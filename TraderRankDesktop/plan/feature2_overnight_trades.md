# Feature 2 — Overnight Trades: Cross-Day Position Matching

## 1. Goal

Trades opened on one calendar day and closed on a later day (overnight holds, multi-day swings) are currently invisible to the analytics engine. They never produce a `MatchedTrade` and they corrupt the per-day P&L because the buy and the sell are processed by separate per-day passes that each see only one side of the round trip.

Success looks like:
- Every closed round trip — intraday or multi-day — appears in `AppState::matched_trades`.
- `DailySummary.realized_pnl` reflects only **realized** P&L on that day (positions closed that day), not raw cash flow. A multi-day trade's full P&L is attributed to the **exit day**.
- Hold-time stats in the Trade Quality tab correctly show day-spanning durations.

## 2. Root Cause

Both the matcher and the analytics engine bucket raw fills by calendar date *before* running their FIFO position tracker, then start each day from a flat position.

- `src/trade_matcher.rs:18-22` — `daily_trades: HashMap<String, Vec<&Trade>>` groups raw fills by `trade.time.date_naive()`.
- `src/trade_matcher.rs:26-194` — outer loop `for (_date_key, day_trades) in &daily_trades` iterates **per day**, instantiates a fresh `position = ZERO` per day, and at line 192 silently drops any non-zero position with the comment `Position left open at end of day: skip`.
- `src/analytics.rs:10-22` — same per-day grouping (`daily_trades: HashMap<String, Vec<Trade>>`), then `calculate_daily_summary` does its own per-day FIFO at lines 124-258.
- Consequence: a Mon BUY + Tue SELL for the same symbol is processed as two unmatched orphans — Mon prints `Warning: Unmatched trade` (analytics.rs:133), Tue prints the same. No `MatchedTrade` is ever created. Neither day's `realized_pnl` reflects the actual round-trip P&L.

Note: `analytics.rs` happens to dodge the cash-flow-leak concern because `realized_pnl` is built from the per-day FIFO's `realized_trades` vector (line 117) — not from `trade.net_amount`. So the bug is a **missed match**, not a phantom cash spike: the trade's P&L silently disappears entirely, and the unmatched fill's commission is also dropped (commission is only added when `symbol_had_trades = true`). After the fix it will appear on the exit day.

## 3. Design Decisions

| # | Question | Decision | Rationale |
|---|---|---|---|
| A | Match scope | **Per symbol, FIFO across the entire dataset.** Drop the per-day outer grouping in both matcher and analytics. | Simplest correct algorithm. The CSV is already chronological. Same O(n) per symbol. KISS. |
| B | Which day owns an overnight trade's P&L? | **Exit day** (the day the close fill landed). | Standard realized-P&L accounting. `MatchedTrade.exit_time` already drives the day grouping in views (e.g. `trades.rs:457`, `analytics.rs:1098`). |
| C | DailySummary computation | **Position-realized accounting** — only count P&L on a CLOSE, attribute to the close fill's day. (Already the model, but extend across day boundaries.) Re-derive `DailySummary` from the global match pass instead of running a second per-day FIFO. | Eliminates duplication of position-tracking logic between `analytics.rs` and `trade_matcher.rs` (currently two near-identical implementations). One source of truth: matched trades. Daily summaries become an aggregation of MatchedTrades grouped by `exit_time.date_naive()`. |
| D | Open positions at end of dataset | **Skip** — don't emit a MatchedTrade. Log a warning. | Unrealized P&L is not in scope; the user explicitly said "real trades" = closed round trips. Same semantics as today. |
| E | CSV ordering | Trades are already loaded sorted (`data_loader.rs:71`: `trades.sort_by(...)`). No change needed. | Confirmed. |
| F | Hold-time formatting | Extend `format_duration` in `views/analytics.rs:865-877` to handle days. New format: `Xd Yh`, `Xh Ym`, `Xm Ys`, `Xs`. Trades view displays times with `%m/%d %H:%M` (`views/trades.rs:469`) which already shows the date — no change needed there. | The TradeQuality KPI cards will otherwise read e.g. "47h 12m" which is hard to parse. Days are clearer for swing trades. |
| G | Buy-only / sell-only positions still open at end | Skip (same as D). Emit a warning for visibility. | No change in behavior. |
| H | Migration | None — no persisted matched_trades. `data_loader::load_app_state()` recomputes on every load. | Confirmed by `data_loader.rs:336-342`. |
| I | Performance | Algorithm remains O(n) per symbol. For typical IB Flex data sizes (thousands of fills) this is trivial. | No concern. |
| J | Tests | `grep -r "fn test_"` returned **zero** matches under `src/`. There are no unit tests to update or break. Verification will be done manually via `cargo build` + spot-check against live CSV. | Confirmed. |

## 4. Files to Modify

- `D:\GitHub\TraderRank\TraderRankDesktop\src\trade_matcher.rs` — remove per-day grouping; switch outer loop to per-symbol-only across all trades.
- `D:\GitHub\TraderRank\TraderRankDesktop\src\analytics.rs` — replace per-day FIFO inside `calculate_daily_summary` with aggregation from MatchedTrades; add new entry point that takes both `&[Trade]` and `&[MatchedTrade]`.
- `D:\GitHub\TraderRank\TraderRankDesktop\src\data_loader.rs` — change call order so `match_trades` runs first, then pass the matched output into analytics.
- `D:\GitHub\TraderRank\TraderRankDesktop\src\views\analytics.rs` — extend `format_duration` to render days for multi-day holds.

No model/state changes. `MatchedTrade` already has `entry_time` and `exit_time` as full `DateTime<Utc>`; nothing structural needs to grow.

## 5. Detailed Steps

### Step 1 — `trade_matcher.rs`: drop per-day grouping

Replace the outer loop. The inner per-symbol FIFO body is correct as-is and must be preserved verbatim — the only change is **what feeds it**.

New top of `match_trades`:

```rust
pub fn match_trades(trades: &[Trade]) -> Vec<MatchedTrade> {
    // Group by symbol across the ENTIRE dataset (no day boundary).
    // Same FIFO algorithm — just don't reset position at midnight.
    let mut by_symbol: HashMap<&str, Vec<&Trade>> = HashMap::new();
    for trade in trades {
        by_symbol.entry(trade.symbol.as_str()).or_default().push(trade);
    }

    let mut all_matched: Vec<MatchedTrade> = Vec::new();

    for (symbol, mut symbol_trades) in by_symbol {
        if symbol_trades.len() < 2 { continue; }
        symbol_trades.sort_by_key(|t| t.time);

        // ... existing per-symbol FIFO body, lines 42-191 of current file, UNCHANGED ...
        // (position, cost_basis, opening_commission, entry_fills, first_entry_time
        //  state machine; emits MatchedTrade on close; handles long & short.)

        // Position left open at end of dataset: skip (unrealized).
        if position != Decimal::ZERO {
            eprintln!(
                "Warning: {} ended with open position of {} (unrealized — not matched)",
                symbol, position
            );
        }
    }

    all_matched.sort_by(|a, b| b.exit_time.cmp(&a.exit_time));
    all_matched
}
```

Concretely:
- Delete lines 18-22 (`daily_trades` HashMap and the date-keyed insert).
- Delete the outer `for (_date_key, day_trades) in &daily_trades` loop and the `by_symbol` rebuild that happens *inside* it (current lines 26-31). Move the per-symbol grouping to the top level.
- Keep the entire per-symbol FIFO body byte-for-byte identical (Buy/Sell match arms, weighted cost basis, partial closes, side flips). It is already correct.
- Replace the per-day "skip open position" comment at line 192 with the warning above (only fires once per symbol now, at end of dataset).

### Step 2 — `analytics.rs`: rebuild `DailySummary` from MatchedTrades

The cleanest refactor is to **delete the per-day FIFO entirely** from `analytics.rs` and rebuild `DailySummary` by grouping matched trades by `exit_time.date_naive()`. This eliminates the duplicated position-tracking logic and guarantees `analytics` and `trade_matcher` agree.

Add a new public entry point and have the old one delegate to it:

```rust
impl TradingAnalytics {
    /// New primary entry point: takes raw trades AND the already-matched round trips.
    /// DailySummary is built by grouping matched trades by exit_day.
    /// Raw trades are still needed for: total_volume, time_slot_performance,
    /// and the symbols_traded set (which now includes a symbol on the day its
    /// trade CLOSES — consistent with realized-P&L accounting).
    pub fn analyze_trades_with_matched(
        trades: &[Trade],
        matched: &[MatchedTrade],
    ) -> TradingSummary {
        // 1. Group matched by exit_day
        let mut by_exit_day: HashMap<NaiveDate, Vec<&MatchedTrade>> = HashMap::new();
        for mt in matched {
            by_exit_day.entry(mt.exit_time.date_naive()).or_default().push(mt);
        }

        // 2. Group raw trades by day for volume + hourly performance
        let mut raw_by_day: HashMap<NaiveDate, Vec<Trade>> = HashMap::new();
        for t in trades {
            raw_by_day.entry(t.time.date_naive()).or_default().push(t.clone());
        }

        // 3. Union of all days that have either a close or any fill
        let all_days: HashSet<NaiveDate> = by_exit_day.keys()
            .copied()
            .chain(raw_by_day.keys().copied())
            .collect();

        let mut daily_summaries: Vec<DailySummary> = all_days.into_iter()
            .map(|day| Self::build_daily_summary(
                day,
                by_exit_day.get(&day).map(|v| v.as_slice()).unwrap_or(&[]),
                raw_by_day.get(&day).map(|v| v.as_slice()).unwrap_or(&[]),
            ))
            .filter(|s| s.total_trades > 0)  // drop days with no closes
            .collect();
        daily_summaries.sort_by_key(|s| s.date);

        // 4. Aggregates (unchanged from current logic — same fields)
        // ... build TradingSummary from daily_summaries ...
    }

    fn build_daily_summary(
        day: NaiveDate,
        closes: &[&MatchedTrade],     // matched trades whose exit_time is this day
        raw: &[Trade],                // ALL fills on this day (for volume + hourly)
    ) -> DailySummary {
        let date_utc = DateTime::<Utc>::from_naive_utc_and_offset(
            day.and_hms_opt(0, 0, 0).unwrap(), Utc
        );
        let mut summary = DailySummary::new(date_utc);

        // Volume = notional of every fill that touched this day (entry side may
        // span days, but for "how active was this day" we count all fills).
        summary.total_volume = raw.iter()
            .map(|t| t.quantity * t.fill_price)
            .sum();

        // Realized P&L, wins, losses, commission — straight aggregation of closes.
        let mut symbols = HashSet::new();
        let mut winning_pnls = Vec::new();
        let mut losing_pnls = Vec::new();
        for mt in closes {
            symbols.insert(mt.symbol.clone());
            summary.total_commission += mt.commission;
            // gross classification (matches existing convention at analytics.rs:267)
            if mt.gross_pnl > Decimal::ZERO {
                summary.winning_trades += 1;
                winning_pnls.push(mt.net_pnl);
                if mt.net_pnl > summary.largest_win { summary.largest_win = mt.net_pnl; }
            } else if mt.gross_pnl < Decimal::ZERO {
                summary.losing_trades += 1;
                losing_pnls.push(mt.net_pnl);
                if mt.net_pnl < summary.largest_loss { summary.largest_loss = mt.net_pnl; }
            }
            summary.realized_pnl += mt.net_pnl;
        }
        summary.gross_pnl = summary.realized_pnl + summary.total_commission;
        summary.total_trades = (summary.winning_trades + summary.losing_trades) as u32;
        summary.symbols_traded = symbols.into_iter().collect();

        if !winning_pnls.is_empty() {
            summary.avg_win = winning_pnls.iter().sum::<Decimal>() / Decimal::from(winning_pnls.len() as u32);
        }
        if !losing_pnls.is_empty() {
            summary.avg_loss = losing_pnls.iter().sum::<Decimal>() / Decimal::from(losing_pnls.len() as u32);
        }
        summary.win_rate = if summary.total_trades > 0 {
            (summary.winning_trades as f64) / (summary.total_trades as f64) * 100.0
        } else { 0.0 };

        // Hourly performance from raw fills — keep the existing impl.
        // NOTE: calculate_hourly_performance currently runs its own per-day FIFO
        // (analytics.rs:314-470). For Step 2, leave it intact — the hourly
        // breakdown attributes intraday round trips to the closing-fill hour,
        // which is still correct for the *intraday* portion of activity.
        // Overnight trades will simply not show in the hour bars for either
        // their open or close day — acceptable trade-off for v1; revisit in
        // a follow-up if user requests overnight P&L attribution by hour.
        summary.time_slot_performance = Self::calculate_hourly_performance(raw);

        summary
    }
}
```

Then keep the original `analyze_trades` as a thin wrapper for backward compatibility and so external callers (if any) don't break:

```rust
pub fn analyze_trades(trades: &[Trade]) -> TradingSummary {
    let matched = crate::trade_matcher::match_trades(trades);
    Self::analyze_trades_with_matched(trades, &matched)
}
```

Concretely:
- Delete `calculate_daily_summary` (lines 98-312) — its job is taken by `build_daily_summary`.
- Keep `calculate_hourly_performance` (lines 314-470) — still per-day, still correct for intraday.
- Keep `calculate_weekly_summaries`, `calculate_monthly_summaries`, `analyze_hourly_performance`, etc. — they only depend on `DailySummary` and don't care how it was built.
- Add `build_daily_summary` and `analyze_trades_with_matched` per the sketch above.
- Rewrite `analyze_trades` to delegate.

### Step 3 — `data_loader.rs`: match first, then analyze

Current order (`data_loader.rs:336-338`):
```rust
let summary = crate::analytics::TradingAnalytics::analyze_trades(&trades);
let matched = crate::trade_matcher::match_trades(&trades);
```

Change to:
```rust
let matched = crate::trade_matcher::match_trades(&trades);
let summary = crate::analytics::TradingAnalytics::analyze_trades_with_matched(&trades, &matched);
```

This avoids running `match_trades` twice (once via the new `analyze_trades` wrapper, once explicitly).

### Step 4 — `views/analytics.rs`: format multi-day holds

Replace `format_duration` at lines 865-877:

```rust
let format_duration = |secs: i64| -> String {
    if secs <= 0 { return "N/A".to_string(); }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if days > 0 {
        if hours > 0 { format!("{}d {}h", days, hours) } else { format!("{}d", days) }
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, s)
    } else {
        format!("{}s", s)
    }
};
```

(Drops seconds once we're showing hours-or-larger — cleaner KPI card, prevents overflow.)

### Step 5 — Sanity audit: no other code assumes intraday-only

Quick checks (already done during planning, recorded for executor):
- `views/trades.rs:469` formats time as `%m/%d %H:%M` — already includes the date, will read fine for overnight closes.
- `views/trades.rs:457` groups by `trade.exit_time.date_naive()` — overnight trades land on their close day, correct.
- `views/analytics.rs:1098` filters monthly rows by `mt.exit_time.year/month` — same, correct.
- `state.rs:136-145` computes the R-week from `mt.exit_time.date_naive().weekday()` — correct: an overnight trade is classified by the week it closes in.
- `views/dashboard.rs` and `views/timeline.rs` consume `DailySummary`, which is now reconstructed correctly from matched trades — no changes needed.

No other call sites touch the matching logic.

## 6. Verification

1. **Build**
   ```pwsh
   cd D:\GitHub\TraderRank\TraderRankDesktop
   cargo build
   ```
   Must compile with **zero new warnings**. (The `Warning: Unmatched trade ...` runtime eprintlns from `analytics.rs:133-141` will go away — they were caused by overnight trades being orphaned.)

2. **Run**
   ```pwsh
   cargo run
   ```
   Watch stdout — count of "Matched N round-trip trades" in `data_loader.rs:338` should **increase** vs prior runs on the same CSV.

3. **Spot-check a known overnight trade**
   - Open `%LOCALAPPDATA%\TraderRank\imports\ib_flex_import.csv` in a viewer.
   - Find a symbol with a BUY on date X and a SELL on date Y > X (or SELL/BUY for shorts) where the running position would close.
   - In the Trades view, scroll/sort by time and confirm a `MatchedTrade` row exists with `entry_time` on day X and `exit_time` on day Y.
   - Open the day modal for day Y and confirm the trade is in its detail list.

4. **Day P&L sanity**
   - Pick a day previously known to show a suspicious lone fill (e.g. a single BUY warning in the prior console output). That day's `realized_pnl` previously skipped the trade entirely.
   - After fix: the trade's P&L now shows on the day it CLOSES, not the day it OPENS. The open day's P&L should not change vs prior (it was already excluding the orphan); the close day's P&L should now include the round trip's net P&L.
   - Specifically, sum `MatchedTrade.net_pnl` for trades closing on day D and confirm it equals `daily_summaries[D].realized_pnl` (already the case for intraday, must hold for cross-day).

5. **Trade Quality tab**
   - Navigate to Analytics > Trade Quality.
   - "Longest Hold" KPI should now potentially show "Xd Yh" if any overnight trades exist.
   - "Avg Hold Time" may notably increase if there are many swing trades.

6. **Aggregate sanity**
   - Total Net P&L on the Trades view (sum of `data.trade_pnl(t, ...)`) should equal the sum of `daily_summaries[*].realized_pnl` to within rounding. This is the strongest invariant — both numbers now derive from the same MatchedTrades.

## 7. Risks

- **Hourly bars miss overnight trades.** `calculate_hourly_performance` still runs per-day. An overnight trade contributes to neither the open day's hour bars nor the close day's. This is a known v1 limitation; documented in the comment in `build_daily_summary`. Acceptable because intraday remains the dominant pattern. Follow-up: attribute overnight P&L to the hour of the closing fill in the hourly view.
- **`total_volume` definition shift.** Previously: notional of fills on this day, including unmatched orphans. Now: same — `build_daily_summary` still sums all `raw` fills. So volume is unchanged, but it now will not artificially equal "matched volume" for overnight days. Acceptable; volume as "how busy was the desk" is the right semantic.
- **`symbols_traded` semantic shift.** Previously a symbol appeared on a day if it had any matched intraday fill. Now it appears on the day it CLOSES (the realized day). For pure intraday, identical. For overnight, the symbol shows on the close day only — not the open day. This matches realized-P&L accounting and is more useful than the prior phantom-orphan behavior, but worth flagging if a UI element expected "symbols I traded today" rather than "symbols I closed today". (Quick scan: nothing in the views relies on the open-day appearance.)
- **Hold-time KPI surprise.** "Avg Hold Time" may jump dramatically when a long swing trade is added. Solved by Step 4's new `Xd Yh` formatting; user just needs to expect the change.
- **Win rate may shift.** Previously orphaned overnight trades contributed nothing. Now they classify as winners/losers based on net P&L vs R threshold. If the user happened to be carrying losing overnight positions, win rate will worsen — this is a true correction, not a regression.
- **Performance.** Building one global `by_symbol` HashMap and one global `by_exit_day` HashMap is still O(n). For datasets up to ~100k fills (well past realistic IB Flex sizes), no perceptible slowdown.
- **Backward compatibility of `analyze_trades`.** Kept as a thin delegator so any future caller (or test fixture) using the original signature still works.
