# TraderRankDesktop - Desktop Trading Analytics

Dioxus 0.6 desktop app (Rust) for analyzing IB Flex trading data.

## Build & Run

```bash
cargo run          # debug build
cargo build --release
```

## Architecture

```
src/
  main.rs            # App entry, routing, ThemeToggle, RefreshButton, app-wide log signal
  models.rs          # Domain types: Trade, DailySummary, WeeklySummary, MonthlySummary, MatchedTrade
  state.rs           # AppState (all runtime data), TradeOutcome enum, exclusion/R helpers
  analytics.rs       # TradingAnalytics: position tracking, daily/weekly/monthly summary computation
  trade_matcher.rs   # Matches raw fills into round-trip MatchedTrade entries
  data_loader.rs     # Loads CSV from IB imports dir, runs analytics, builds AppState
  parser.rs          # CSV parser (IB format + custom format detection)
  flex_fetcher.rs    # IB Flex Web Service: fetch XML, parse trades, merge into CSV
  settings_store.rs  # PersistedSettings: JSON read/write to %LOCALAPPDATA%\TraderRank\
  app_dirs.rs        # Path helpers: settings_path(), imports_dir()
  components.rs      # Reusable UI: MetricCard, format_pnl(), format_r(), format_decimal()
  theme.rs           # Dark/Light theme enum
  sample_data.rs     # Synthetic demo data fallback
  views/
    dashboard.rs     # KPI cards, equity chart, "This Week" card
    timeline.rs      # Daily/Weekly/Monthly sortable table
    trades.rs        # Matched trades table, day separator, exclusion UI, day modal
    analytics.rs     # 6-tab deep analysis (Overview, TimeOfDay, DayOfWeek, Symbols, TradeQuality, Progression)
    visual_timeline.rs # Animated horizontal timeline with zoom/pan
    settings.rs      # Theme, R-config, IB Flex credentials, activity log
```

## Data Flow

```
IB Flex API  --(fetch_and_save)--> %LOCALAPPDATA%\TraderRank\imports\ib_flex_import.csv
                                       |
                                  load_trades_from_imports()
                                       |
                              CsvParser::parse_file() --> Vec<Trade>
                                       |
                        TradingAnalytics::analyze_trades() --> TradingSummary
                        trade_matcher::match_trades()      --> Vec<MatchedTrade>
                                       |
                            trading_summary_to_app_state() --> AppState
                                       |
                              Signal<AppState> (Dioxus context)
                                       |
                              All views read from this signal
```

## Key Data Types

### Trade (raw fill)
`symbol, side, quantity, fill_price, time: DateTime<Utc>, net_amount, commission`

### MatchedTrade (round trip)
`symbol, side, entry_time, exit_time, entry_price, exit_price, quantity, gross_pnl, net_pnl, commission, entry_fills, exit_fills`

### DailySummary
`date, total_trades, winning_trades, losing_trades, realized_pnl, gross_pnl, total_commission, win_rate, avg_win, avg_loss, largest_win, largest_loss, symbols_traded, time_slot_performance`

### AppState
All summaries + trades + matched_trades + r_configs + exclusions. Provided as `Signal<AppState>` context to all views.

## Trade Classification (R-based)

Defined in `state.rs::TradeOutcome`:
- **Winner**: `net_pnl >= 0.5 * R_value` (half-R threshold for the trade's week)
- **Lossless**: `0 <= net_pnl < 0.5 * R_value`
- **Loser**: `net_pnl < 0`

Win rate everywhere = `winners / (winners + losers)`. Lossless trades excluded from denominator.

Use `AppState::trade_outcome(&self, mt: &MatchedTrade) -> TradeOutcome` for classification.

## R-Unit System

Per-week configurable R value (Settings tab). Default $100/R.
- `AppState::r_value_for_week(monday: NaiveDate) -> Decimal`
- `AppState::pnl_in_r(pnl: Decimal, r_value: Decimal) -> Decimal`

## Exclusion System

Trades/days can be excluded from all analytics. Stored in `settings.json` as `HashMap<String, String>` (key -> optional reason).

**Key formats:**
- Day: `"day:YYYY-MM-DD"`
- Trade: `"trade:SYMBOL:YYYY-MM-DDTHH:MM:SS"`

**Helpers on AppState:**
- `day_exclusion_key(date_str) -> String`
- `trade_exclusion_key(mt: &MatchedTrade) -> String`
- `is_day_excluded(date_str) -> bool`
- `is_trade_excluded(mt: &MatchedTrade) -> bool` (also true if day excluded)
- `day_exclusion_reason(date_str) -> String`

**View filtering pattern:**
```rust
let filtered_days: Vec<_> = data.daily_summaries.iter()
    .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
    .collect();
let filtered_matched: Vec<_> = data.matched_trades.iter()
    .filter(|mt| !data.is_trade_excluded(mt))
    .collect();
```

Every view filters at display time. No pre-computed aggregates from AppState are used directly (total_pnl, overall_win_rate, etc. are stale — always recompute from filtered data).

## Persistence

| What | Where | Format |
|------|-------|--------|
| Trade data | `%LOCALAPPDATA%\TraderRank\imports\ib_flex_import.csv` | CSV |
| Settings | `%LOCALAPPDATA%\TraderRank\settings.json` | JSON |
| Styling | `assets/main.css` | CSS |

### PersistedSettings fields
`theme, dashboard_range, timeline_mode, timeline_max_entries, timeline_sort_col, timeline_sort_asc, trades_max_entries, trades_sort_col, trades_sort_asc, trades_hide_excluded, analytics_tab, analytics_range, vtl_zoom, vtl_range_start, vtl_range_end, flex_token, flex_query_id, r_configs, exclusions`

Save pattern: `settings_store::update(|s| s.field = value)`

## IB Flex Integration

`flex_fetcher::fetch_and_save(token, query_id) -> Result<usize>`

Fetches XML from IB, parses trades, **merges** with existing CSV (deduplicates by line). Returns trade count. The date range is controlled by the Flex Query configuration on IB's website.

## App-Wide Activity Log

`Signal<Vec<(String, String)>>` context — `(timestamp, message)` pairs.

```rust
// From main.rs (public)
crate::log_message(&mut app_log, "message");
crate::log_message(&mut app_log, &format!("ERROR: {}", e));
```

Displayed in Settings view as scrollable card. Errors prefixed with "ERROR:" render in red.

## Refresh Button

`RefreshButton` component in nav bar. If IB credentials set, fetches from broker then reloads. Otherwise reloads local CSV. Use `crate::reload_app_state(&mut state)` to reload preserving R-configs.

## CSS Theming

All colors via CSS custom properties on `.app-root[data-theme]`. Key vars:
`--bg-primary, --bg-card, --bg-input, --text-primary, --text-muted, --accent-primary, --accent-green, --accent-red, --accent-yellow, --border-color`

## Code Conventions

- `rust_decimal::Decimal` for all money — never f64
- `chrono::DateTime<Utc>` for timestamps, `NaiveDate` for date keys
- Formatting: `format_pnl()` for +$X.XX/-$X.XX, `format_r()` for +X.XR, `format_decimal()` for raw $X.XX
- Settings auto-save on change via `settings_store::update()`
- Analytics empty days (0 round trips) are filtered at the source in `analytics.rs`
