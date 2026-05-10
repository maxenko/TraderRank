# Feature 3 — "Week" Page: Per-Week Detailed Trade Review

## 1. Goal

Add a dedicated `/week` page that lets the trader pick any past week and study every trade in it as a focused review. The page combines a minimalist heatmap-style week selector (each cell = one week, color-coded by net P&L) with a week-header KPI block, a per-trade detail table, a within-week per-day mini-heatmap, week-over-week comparison deltas, and overtrading insights. The view is the core artifact for the trader's "Sunday review" workflow — pick last week, see what happened at trade granularity, and identify one behavioral lesson.

## 2. Research findings

### Sources
- TradeZella's 30-Minute Weekly Review Framework — calendar heat-map, metrics vs 30-day baseline, winners/losers separation, pattern breaks (time-of-day, setup, ticker), three written observations. https://www.tradezella.com/blog/weekly-trade-review-process
- "7 Metrics Every Serious Trader Should Track Weekly" — net P&L, win rate, R-multiple, profit factor, max drawdown, expectancy, process-adherence rate. Emphasizes trend across consecutive weeks ("one bad week is noise, three in a row is a signal"). https://www.worldnewsresearch.com/blog/the-7-metrics-every-serious-trader-should-track-weekly/
- Tradervue / TraderSync / Edgewonk feature comparison — Edgewonk Tiltmeter (emotional state), Edge Finder weekly auto-report, time-of-day analysis, setup tagging. https://www.tradervue.com/blog/best-trading-journal
- Plancana day-trading-journal guide — emphasis on session-level and per-trade row with hold time, R-multiple, time-of-day band, setup tag. https://plancana.com/blog/trader-segments/day-trading-journal
- Overtrading detection literature — flag when daily count is >1.5x median or N-week median; combine count with sign of P&L (overtrade-AND-lost is the dangerous cell). https://www.heygotrade.com/en/blog/how-to-avoid-overtrading/
- Revenge-trading literature — sudden spike in size or frequency right after a loss; 3-question test before any post-loss trade. https://www.tradezella.com/blog/revenge-trading

### 10 candidate ideas surfaced
1. Calendar heat-map for week selection
2. Per-trade detail table with hold time + R-multiple
3. Week-over-week delta indicators on every KPI
4. Per-day mini-heatmap inside the week (Mon-Fri colored by P&L)
5. Overtrading flag — trades/day vs prior 4-week median
6. Hour-x-weekday heatmap for the selected week
7. Biggest winner / biggest loser callout cards
8. Discipline metrics (rule adherence) — defer, requires per-trade tags
9. Tiltmeter / emotional-state log — defer, requires journaling
10. Setup-type breakdown — defer, requires tagging

### Chosen 6 features for v1 (ordered by value)
1. **Per-trade detail table** scoped to the selected week (the primary artifact).
2. **Week-header KPI card** with net P&L, R-sum, W/LL/L counts, win rate, profit factor, # trading days, biggest single win/loss, total commission.
3. **Week-over-week comparison row** — same metrics shown as `current (Δ vs prior week)` with up/down arrows.
4. **Within-week per-day mini-heatmap** (Mon-Fri strip).
5. **Overtrading insight panel** — trades/day this week vs prior-4-week median; flag any day >1.5x; sum P&L on those flagged days.
6. **Hour-x-weekday heatmap** (P&L per cell) for the selected week, reusing the trends Behavior tab styling.

## 3. Design decisions (locked)

### A. Week selector — VERTICAL STRIP, last 26 weeks
A vertical column of up to 26 cells along the **left side** of the page, newest at the top. Each cell is a small rounded rectangle (height ~28px, full width of the column ~120px) showing the Monday date label (`Mon Apr 28`) on the left and a colored chip on the right that reflects that week's net P&L. The currently-selected cell is outlined.

**Rationale:** Vertical strip keeps the entire week list visible without scrolling, leaves the wide right area for content, and the date label is always readable (unlike a tiny year-overview cell). 26 weeks ≈ 6 months covers the typical review horizon; older weeks accessible via a "Show older" expander that doubles to 52.

### B. Week-content layout
- Header card (full-width): selected week's title + KPI grid + week-over-week comparison row.
- Below: 2-column row — left = per-day mini-heatmap (Mon-Fri colored bar), right = overtrading insight panel.
- Below: per-trade detail table (full-width).
- Below: hour-x-weekday heatmap (full-width).

### C. Comparison vs previous week — INLINE DELTA
Each KPI card shows: big primary value, then a small "vs last week: ▲ +$1,240 (+12%)" subtitle line. Green arrow if metric improved (note: for `Loss/Drawdown/Commission` "improved" means lower; helper handles direction). One-line subtitle, no side-by-side mini-table.

**Rationale:** Side-by-side doubles visual real estate; an inline delta is the standard pattern (matches the existing `MetricCard` subtitle slot).

### D. Overtrading insights
- **Trades/day this week** (single number, color-coded vs baseline).
- **Prior-4-week median trades/day** (shown as baseline).
- **Threshold = 1.5× median** (matches `trends.rs` Behavior tab convention).
- **Days flagged**: list each flagged day with its trade count and P&L.
- **Cost of overtrading**: sum of P&L on flagged-AND-losing days.
- **Worst red day in week**: callout card.

### E. Persistence
Add `selected_week: String` (ISO date of Monday, e.g. `"2026-05-04"`) field to `PersistedSettings`. Default = the empty string, which the view interprets as "current week" (most recent week with trades). Saved on every selector click via `settings_store::update`.

### F. Routing
Add `Week {}` variant to `Route` enum in `src/main.rs`, register a `Week()` route component delegating to `views::week::Week`, and add a nav link `Link { ... to: Route::Week {}, "Week" }` placed **between** "Visual" and "Trades" (logical: Dashboard → Timeline → Visual → **Week** → Trades → Analytics → Settings).

### G. File structure
- New file: `src/views/week.rs` (single component file).
- Register module in `src/views/mod.rs`: `pub mod week;`.

### H. Reactivity
Read `Signal<AppState>` and `Signal<StatsConfig>` from context. All P&L values pass through `data.trade_pnl(mt, count_commissions)` or `data.daily_pnl(d, count_commissions)`. All trade outcomes use `data.trade_outcome_with(mt, count_commissions)`. Filter out excluded trades and excluded days via `data.is_trade_excluded(mt)` and `data.is_day_excluded(date_str)`.

### I. Empty state
If selected week has zero trades after exclusions: show "No trades in this week" message in the header KPI card area, hide the per-trade table and the within-week mini-heatmap, but **still show the week-selector strip on the left** so the user can pick another week. Do NOT auto-fall-back to the prior week (it would mask the user's explicit choice and confuse them).

### J. Range scope
A trade belongs to a week iff its `exit_time` falls within that Monday-Sunday window. Matches the dashboard convention and the existing `trends.rs` weekly aggregation. No special handling for entry-before-week (ok if entry was earlier — only exit_time defines week assignment).

## 4. Files to create / modify

**Create:**
- `D:\GitHub\TraderRank\TraderRankDesktop\src\views\week.rs`

**Modify:**
- `D:\GitHub\TraderRank\TraderRankDesktop\src\main.rs` (Route enum, nav link, route component)
- `D:\GitHub\TraderRank\TraderRankDesktop\src\views\mod.rs` (register `pub mod week;`)
- `D:\GitHub\TraderRank\TraderRankDesktop\src\settings_store.rs` (add `selected_week: String` field)
- `D:\GitHub\TraderRank\TraderRankDesktop\assets\main.css` (add new CSS classes — see step 7 below)

## 5. Detailed steps

### Step 1 — Persistence field
In `src/settings_store.rs`, inside `PersistedSettings`, add:
```rust
#[serde(default)]
pub selected_week: String, // ISO date "YYYY-MM-DD" of the week's Monday, "" = current
```
Place it next to other view-state fields (after `trades_hide_excluded`). No default-fn helper needed — empty string is the sentinel for "current week".

### Step 2 — Module registration
In `src/views/mod.rs`, add a single line:
```rust
pub mod week;
```

### Step 3 — Routing
In `src/main.rs`:
1. Inside the `Route` enum, add (between `VisualTimeline` and `Trades`):
```rust
#[route("/week")]
Week {},
```
2. Add a nav `Link` between the Visual and Trades links inside `AppLayout`:
```rust
Link { class: "nav-tab", to: Route::Week {}, "Week" }
```
3. Add a route component near the bottom of `main.rs`:
```rust
#[component]
fn Week() -> Element {
    rsx! { views::week::Week {} }
}
```

### Step 4 — `src/views/week.rs` skeleton
Top-level layout (Dioxus 0.6 `#[component]` style; signature `pub fn Week() -> Element`):

```rust
use dioxus::prelude::*;
use chrono::{Datelike, NaiveDate, Weekday, Timelike};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::components::*;
use crate::models::MatchedTrade;
use crate::settings_store;
use crate::state::{AppState, TradeOutcome};
```

Inside the component:
- Read `state` and `stats_config` contexts.
- `let count_commissions = stats_config.read().count_commissions;`
- Load saved selected week into a `use_signal::<Option<NaiveDate>>(|| ...)`. If empty/parse-fail, use the most recent week (Monday) that has trades.
- Compute `weeks: Vec<WeekBucket>` from `data.matched_trades` (see step 5 below).
- Compute `selected: WeekBucket` (matched_trades + days summaries scoped to that week).
- Compute `prev_week: Option<WeekBucket>` (same logic for week prior).
- Render: top-level `div { class: "view week-view" }` containing `aside { class: "week-selector" }` + `main { class: "week-content" }`.

### Step 5 — Week aggregation helper
Add a private struct + helper inside `week.rs`:
```rust
struct WeekBucket {
    monday: NaiveDate,           // Monday of week
    sunday: NaiveDate,           // Sunday of week
    matched: Vec<MatchedTrade>,  // non-excluded trades in this week
    net_pnl: Decimal,
    r_sum: Decimal,
    wins: u32,
    lossless: u32,
    losers: u32,
    trade_days: u32,             // distinct exit dates
    commission: Decimal,
}

fn build_week_buckets(data: &AppState, count_commissions: bool) -> Vec<WeekBucket> {
    let mut buckets: HashMap<NaiveDate, Vec<MatchedTrade>> = HashMap::new();
    for mt in &data.matched_trades {
        if data.is_trade_excluded(mt) { continue; }
        let d = mt.exit_time.date_naive();
        let monday = d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64);
        buckets.entry(monday).or_default().push(mt.clone());
    }
    let mut out: Vec<WeekBucket> = buckets.into_iter().map(|(monday, matched)| {
        let sunday = monday + chrono::Duration::days(6);
        let net_pnl: Decimal = matched.iter().map(|mt| data.trade_pnl(mt, count_commissions)).sum();
        let r_val = data.r_value_for_week(monday);
        let r_sum = data.pnl_in_r(net_pnl, r_val);
        let mut wins=0u32; let mut lossless=0u32; let mut losers=0u32;
        for mt in &matched {
            match data.trade_outcome_with(mt, count_commissions) {
                TradeOutcome::Winner => wins += 1,
                TradeOutcome::Lossless => lossless += 1,
                TradeOutcome::Loser => losers += 1,
            }
        }
        let mut day_set = std::collections::HashSet::new();
        for mt in &matched { day_set.insert(mt.exit_time.date_naive()); }
        let commission: Decimal = matched.iter().map(|mt| mt.commission).sum();
        WeekBucket { monday, sunday, matched, net_pnl, r_sum, wins, lossless, losers,
            trade_days: day_set.len() as u32, commission }
    }).collect();
    out.sort_by_key(|b| std::cmp::Reverse(b.monday));  // newest first
    out
}
```

### Step 6 — Week selector (vertical strip)
Render `aside { class: "week-selector" }` with up to 26 cells (truncate via `.iter().take(26)`); selecting a cell:
```rust
button {
    class: if is_selected { "week-cell selected" } else { "week-cell" },
    onclick: move |_| {
        selected_monday.set(Some(b.monday));
        let s = b.monday.to_string();
        settings_store::update(|st| st.selected_week = s);
    },
    span { class: "wc-date", "{b.monday.format(\"%b %-d\")}" }
    span {
        class: format!("wc-chip l{} {}", level, sign),
        "{format_pnl(b.net_pnl)}"
    }
}
```
Where `level` is computed from `(|net_pnl| / max_abs_net_pnl).clamp(0..1) * 4` ceiling, `sign` is `pos`/`neg`/`flat`. `max_abs_net_pnl` is computed once over all visible buckets.

Add a "Show older" toggle (a `use_signal::<bool>(|| false)`) — when enabled, take 52 instead of 26.

### Step 7 — Week-header KPI card
Use a 2-row layout inside `div { class: "card week-header-card" }`:

Row 1 — title bar:
```rust
h2 { class: "week-title",
    "Week of {selected.monday.format(\"%b %-d\")} – {selected.sunday.format(\"%b %-d, %Y\")}"
}
```

Row 2 — KPI grid (`div { class: "kpi-grid week-kpi-grid" }`) with these `MetricCard`s, each `subtitle: Some(delta_string)`:
- **Net P&L** — `format!("{} / {}", format_r(r_sum), format_pnl(net_pnl))`, subtitle = WoW delta on net_pnl.
- **Win Rate** — `wins / (wins + losers) * 100`, subtitle = WoW pct-points delta.
- **Trades** — total count, subtitle = WoW absolute delta.
- **W / LL / L** — `"{wins} / {lossless} / {losers}"`, no delta.
- **Profit Factor** — sum(wins)/abs(sum(losses)), subtitle = WoW delta.
- **Trading Days** — count of distinct exit dates, subtitle = WoW delta.
- **Biggest Win** — `data.trade_pnl(best, count_commissions)` and symbol.
- **Biggest Loss** — `data.trade_pnl(worst, count_commissions)` and symbol.
- **Commission** — sum, subtitle = WoW delta (lower is better).

### Step 8 — Week-over-week delta helper
Add this helper:
```rust
/// Returns subtitle string like "vs last: ▲ +$120 (+12%)" or "vs last: ▼ -$50 (-5%)".
/// `lower_is_better` flips the up/down arrow color logic (used for commission, drawdown).
fn fmt_delta(curr: Decimal, prev: Option<Decimal>, lower_is_better: bool, as_pnl: bool) -> Option<String> {
    let prev = prev?;
    let diff = curr - prev;
    let pct = if prev.abs() > Decimal::ZERO {
        Some((diff / prev.abs() * Decimal::from(100)).round_dp(0))
    } else { None };
    let arrow = if diff > Decimal::ZERO { "\u{25B2}" } else if diff < Decimal::ZERO { "\u{25BC}" } else { "=" };
    let value_str = if as_pnl { format_pnl(diff) } else { format_decimal(diff) };
    let pct_str = pct.map(|p| format!(" ({:+}%)", p)).unwrap_or_default();
    let _ = lower_is_better; // direction info consumed by caller via CSS class if needed
    Some(format!("vs last: {} {}{}", arrow, value_str, pct_str))
}
```
For the f64 metrics (win rate, profit factor) write a parallel `fmt_delta_f64`.

### Step 9 — Per-trade detail table
Same skeleton as `trades.rs` but pre-filtered to the selected week and **without** the exclusion column (already filtered out). Columns:
1. Date/Time (`exit_time` formatted `Mon 14:32`)
2. Symbol
3. Side
4. Qty
5. Entry price
6. Exit price
7. **Hold time** (NEW) — see step 10
8. Gross P&L
9. Net P&L
10. R-multiple (per-trade `data.pnl_in_r(net, r_val_for_week)`)
11. Outcome chip (`W` green / `LL` yellow / `L` red)

Sort default = `exit_time` ascending (earliest of the week first — matches a chronological review). No persisted sort state for v1 (kept simple).

CSS: reuse existing `.trade-table` class, add `.week-trade-table` modifier for any week-specific overrides.

### Step 10 — Hold-time formatter
Add this free helper at the bottom of `week.rs`:
```rust
/// Format a chrono::Duration as a compact human-readable hold time.
///   < 1 min  -> "<1m"
///   < 1 hr   -> "{m}m"
///   < 1 day  -> "{h}h{m}m" (e.g. "2h15m")
///   >= 1 day -> "{d}d{h}h"
fn format_hold(d: chrono::Duration) -> String {
    let total_secs = d.num_seconds().max(0);
    if total_secs < 60 { return "<1m".to_string(); }
    let mins = total_secs / 60;
    if mins < 60 { return format!("{}m", mins); }
    let hours = mins / 60;
    let rem_m = mins % 60;
    if hours < 24 { return format!("{}h{:02}m", hours, rem_m); }
    let days = hours / 24;
    let rem_h = hours % 24;
    format!("{}d{:02}h", days, rem_h)
}
```
Call sites compute `mt.exit_time - mt.entry_time` and pass.

### Step 11 — Within-week per-day mini-heatmap
Inside the content area, render `div { class: "week-day-strip" }` with 5 cells (Mon-Fri). Each cell:
```rust
div {
    class: "wds-cell {sign} l{level}",
    title: "{day_label}: {pnl_str}",
    span { class: "wds-day", "{Mon|Tue|...}" }
    span { class: "wds-pnl", "{format_pnl(day_pnl)}" }
    span { class: "wds-count", "{trades_count} trades" }
}
```
`level` computed against `max_abs` over the 5 days (so within-week color scale is local — emphasizes the worst day in the week even on a flat week). Empty days (no trades) get class `empty` and no P&L.

### Step 12 — Overtrading insight panel
Right of the day strip, render `div { class: "card week-overtrade-card" }`:
- Compute `prior_4_weeks: &[WeekBucket]` = `weeks` skipping the selected one, taking up to 4 going back.
- Compute `prior_median_per_day` = median of trades/day across all trading days in those 4 weeks.
- Compute `this_week_avg_per_day` = `selected.matched.len() / selected.trade_days as f64`.
- Threshold = `prior_median_per_day * 1.5` (ceil to int).
- Iterate selected week's days (Mon-Fri); flag each whose trade count > threshold; sum P&L on `flagged AND P&L<0` days.
- Render: a `trend-stats-row` (reusing existing class) with cells:
  - "Trades / Day (this week)" — color green if ≤ baseline, red if > 1.5× baseline.
  - "Prior 4-week median /day"
  - "Overtrade threshold" — `"> {N}"`
  - "Days flagged" — list of `Mon, Wed` if any
  - "Overtrade-loss cost" — `format_pnl(cost)` in negative red.

### Step 13 — Hour × weekday heatmap
Reuse the structure from `trends.rs::render_behavior_tab`'s `hour-heatmap` block, scoped to `selected.matched`. Place it under `div { class: "card week-hour-card" }` with title "Hour × Weekday Trade P&L (selected week)". No new CSS — reuses `.hour-heatmap`, `.hh-row`, `.hh-cell`, `.hh-label`, etc.

### Step 14 — Empty state
Wrap the content rendering in:
```rust
if selected.matched.is_empty() {
    rsx! {
        div { class: "card week-empty",
            h2 { "Week of {selected.monday.format(\"%b %-d\")}" }
            p { class: "week-empty-msg", "No trades in this week." }
            p { class: "week-empty-hint", "Pick another week from the selector on the left." }
        }
    }
} else {
    // full content
}
```

### Step 15 — CSS additions
Append the following to `D:\GitHub\TraderRank\TraderRankDesktop\assets\main.css`:

```css
/* ===== Week page ===== */
.week-view {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 16px;
}

/* Week selector (vertical strip) */
.week-selector {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 8px;
    max-height: calc(100vh - 120px);
    overflow-y: auto;
    position: sticky;
    top: 12px;
    align-self: start;
}
.week-selector-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 4px 6px;
}
.week-cell {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 6px 8px;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 11px;
    text-align: left;
    transition: background 0.1s ease, border-color 0.1s ease;
}
.week-cell:hover { background: var(--bg-input); }
.week-cell.selected { border-color: var(--accent-primary); background: var(--bg-input); }
.wc-date { font-weight: 600; }
.wc-chip {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 3px;
    color: var(--text-primary);
}
.wc-chip.pos.l1 { background: rgba(0, 212, 170, 0.20); }
.wc-chip.pos.l2 { background: rgba(0, 212, 170, 0.40); }
.wc-chip.pos.l3 { background: rgba(0, 212, 170, 0.65); }
.wc-chip.pos.l4 { background: rgba(0, 212, 170, 0.90); }
.wc-chip.neg.l1 { background: rgba(255, 77, 106, 0.20); }
.wc-chip.neg.l2 { background: rgba(255, 77, 106, 0.40); }
.wc-chip.neg.l3 { background: rgba(255, 77, 106, 0.65); }
.wc-chip.neg.l4 { background: rgba(255, 77, 106, 0.90); }
.wc-chip.flat  { background: var(--text-muted); opacity: 0.4; }
.week-selector-toggle {
    background: transparent;
    border: 1px dashed var(--border-color);
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 10px;
    padding: 4px 6px;
    margin-top: 4px;
    cursor: pointer;
}
.week-selector-toggle:hover { color: var(--text-primary); border-color: var(--text-muted); }

/* Week content */
.week-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
}
.week-header-card { padding: 18px 20px; }
.week-title {
    margin: 0 0 12px 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
}
.week-kpi-grid { /* identical layout to .kpi-grid; selector kept for tweaks */ }

/* Per-day mini-heatmap (Mon-Fri strip) */
.week-day-strip {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 12px;
}
.wds-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 12px 4px;
    border-radius: 4px;
    color: var(--text-primary);
    min-height: 64px;
}
.wds-day  { font-size: 10px; color: var(--text-muted); text-transform: uppercase; }
.wds-pnl  { font-size: 14px; font-weight: 700; }
.wds-count{ font-size: 10px; color: var(--text-muted); }
.wds-cell.empty { background: var(--bg-input); opacity: 0.5; }
.wds-cell.pos.l1 { background: rgba(0, 212, 170, 0.18); }
.wds-cell.pos.l2 { background: rgba(0, 212, 170, 0.32); }
.wds-cell.pos.l3 { background: rgba(0, 212, 170, 0.55); }
.wds-cell.pos.l4 { background: rgba(0, 212, 170, 0.80); }
.wds-cell.neg.l1 { background: rgba(255, 77, 106, 0.18); }
.wds-cell.neg.l2 { background: rgba(255, 77, 106, 0.32); }
.wds-cell.neg.l3 { background: rgba(255, 77, 106, 0.55); }
.wds-cell.neg.l4 { background: rgba(255, 77, 106, 0.80); }

/* 2-col row for day strip + overtrade panel */
.week-mid-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
}
@media (max-width: 1100px) { .week-mid-row { grid-template-columns: 1fr; } }

.week-overtrade-card { padding: 14px 18px; }
.week-overtrade-card h3 { margin: 0 0 10px 0; font-size: 13px; }

/* Empty state */
.week-empty { padding: 40px 20px; text-align: center; }
.week-empty h2 { margin: 0 0 12px; font-size: 18px; }
.week-empty-msg { color: var(--text-muted); font-size: 14px; margin: 0 0 4px; }
.week-empty-hint { color: var(--text-muted); font-size: 12px; font-style: italic; margin: 0; }

/* Per-trade outcome chip in week table */
.outcome-chip {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 700;
}
.outcome-chip.w  { background: rgba(0, 212, 170, 0.25); color: var(--accent-green); }
.outcome-chip.ll { background: rgba(255, 200, 0, 0.20); color: var(--accent-yellow); }
.outcome-chip.l  { background: rgba(255, 77, 106, 0.25); color: var(--accent-red); }

/* Light-theme overrides for week cells */
[data-theme="light"] .wc-chip.pos.l1 { background: rgba(5, 150, 105, 0.15); }
[data-theme="light"] .wc-chip.pos.l2 { background: rgba(5, 150, 105, 0.35); }
[data-theme="light"] .wc-chip.pos.l3 { background: rgba(5, 150, 105, 0.60); color: white; }
[data-theme="light"] .wc-chip.pos.l4 { background: rgba(5, 150, 105, 0.85); color: white; }
[data-theme="light"] .wc-chip.neg.l1 { background: rgba(220, 38, 38, 0.15); }
[data-theme="light"] .wc-chip.neg.l2 { background: rgba(220, 38, 38, 0.35); }
[data-theme="light"] .wc-chip.neg.l3 { background: rgba(220, 38, 38, 0.60); color: white; }
[data-theme="light"] .wc-chip.neg.l4 { background: rgba(220, 38, 38, 0.85); color: white; }
[data-theme="light"] .wds-cell.pos.l3,
[data-theme="light"] .wds-cell.pos.l4 { color: white; }
[data-theme="light"] .wds-cell.neg.l3,
[data-theme="light"] .wds-cell.neg.l4 { color: white; }
```

## 6. Verification

1. `cargo build` — must complete with no warnings beyond the pre-existing baseline.
2. `cargo run` — app launches.
3. Navigate to `/week` via the new nav link. Confirm:
   - Vertical strip on the left lists last 26 weeks newest-first, with colored chips.
   - The most recent week is auto-selected.
   - Header card shows correct net P&L, win rate, etc., and a "vs last: ▲/▼ ..." subtitle on each delta-bearing card.
   - Per-day Mon-Fri heatmap shows 5 cells, colored per that day's P&L.
   - Per-trade table lists every non-excluded trade for the week with a hold time column showing values like `42m`, `2h15m`, `1d03h` for varied holds.
   - Overtrading card shows `Trades/day this week`, `Prior 4-week median /day`, `Overtrade threshold > N`, and any flagged days.
   - Hour × weekday heatmap renders.
4. Click a different week cell — content updates instantly. Restart the app — same week is still selected.
5. Click a week with no trades — empty-state card shown, selector still functional.
6. Toggle the dark/light theme — week cells, chips, day-strip cells all readable in both themes.
7. Toggle "Hide commissions" in Settings — KPI numbers change consistently across the page.

## 7. Out of scope (defer to v2)

- Per-trade journal notes / tags / setup classification
- Screenshots or chart attachments per trade
- Automatic written observations or AI weekly summary
- Discipline / rule-adherence scoring (requires per-trade tagging)
- Tilt-meter / emotional-state tracking
- Side-by-side compare-mode (this week vs picked week vs prior 4-week median in a single table)
- Export / print-friendly layout
- Setup-type breakdown table (requires per-trade setup tagging)
- 12-month year-overview heatmap as alternative selector
- Drag-to-zoom on the hour heatmap
