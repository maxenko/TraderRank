# TraderRankDesktop - Architecture Plan

## Context

TraderRank is an existing Rust CLI that processes trading CSV files and displays analytics in the terminal. This desktop GUI version uses Dioxus 0.6 and the mailbox_processor actor library. KISS methodology — keep it simple, functional, and clean.

## Key Design Principles

- **KISS** — minimal files, minimal abstraction, get data on screen clearly
- **Top tab navigation** — tabs for different views: Dashboard | Timeline | Trades | Analytics | Settings
- **R-unit support** — configurable R (risk unit) per week, show P&L in both $ and R-multiples
- **Theme support** — dark/light toggle via CSS custom properties
- **Decimal for money** — use `rust_decimal::Decimal` for all monetary amounts (P&L, commission, volume, prices). Ratios and percentages (win_rate, sharpe_ratio) may use f64 since they are display-only and not accumulated into financial totals. When computing metrics that mix ratios with money (e.g., expectancy), derive the ratio from Decimal directly: `Decimal::from(wins) / Decimal::from(total)` — never round-trip through f64.

## File Structure (~15 files total)

```
TraderRankDesktop/
├── Cargo.toml
├── assets/
│   └── main.css                # Theme system + all styles
└── src/
    ├── main.rs                 # Entry point + App component + route enum + layout
    ├── theme.rs                # Theme enum (Dark/Light)
    ├── models.rs               # Trade, Side, DailySummary, WeeklySummary (ported from CLI)
    ├── sample_data.rs          # Hardcoded realistic sample data
    ├── state.rs                # AppState + R-unit config + helper methods
    ├── components.rs           # Reusable: MetricCard, TradeRow, formatting helpers
    └── views/
        ├── mod.rs
        ├── dashboard.rs        # Overview: KPI cards + equity curve + current week
        ├── timeline.rs         # Daily/Weekly/Monthly P&L with R-multiples (scrollable window)
        ├── trades.rs           # Full trade log table
        ├── analytics.rs        # Symbol breakdown, hourly performance, streaks
        └── settings.rs         # Theme toggle, R-unit config per week
```

## Technology Stack

- **Dioxus 0.6** — desktop renderer (WebView2 on Windows)
- **rust_decimal** — financial precision (Decimal for all monetary values)
- **chrono** — date/time handling
- **serde/serde_json** — serialization (future persistence)

**Deferred dependencies** (add when needed, not before):
- **mailbox_processor** — actor-model state management, add when async CSV data loading is implemented. Path: `../../ControlPlugin/Shared/mailbox_processor`
- **tokio** — async runtime, add alongside mailbox_processor

## Reference Files

- Models ported from: `TraderRank/src/models/trade.rs`, `summary.rs` (design reference — desktop duplicates these locally for now)
- Analytics logic reference: `TraderRank/src/analytics/metrics.rs`
- Mailbox API: `ControlPlugin/Shared/mailbox_processor/src/lib.rs`

## Model Duplication Strategy

Desktop models are currently duplicated from the CLI to avoid coupling. They are field-identical today. **Before real CSV data loading is implemented**, extract a shared `trader_rank_models` crate that both CLI and desktop depend on. This is the natural trigger since the parser and persistence layers need identical structs.

## Code Alignment Tasks (completed)

These fixes were applied to align the code with the plan specifications:

1. **Cargo.toml** — Removed `mailbox_processor` and `tokio` (deferred until async CSV loading)
2. **sample_data.rs:242** — Expectancy now uses `Decimal::from(total_wins) / Decimal::from(total_trades)` instead of f64 round-trip
3. **sample_data.rs:247-261** — Profit factor now computed at trade-level (`all_trades` iteration) instead of daily-aggregated
4. **sample_data.rs:279-291** — Sharpe ratio now uses sample variance (N-1) and `ToPrimitive::to_f64()` instead of string parsing
5. **sample_data.rs:379-385** — R-configs now keyed by ISO Monday (computed from week start date)
6. **timeline.rs:155** — Monthly R-value lookup now finds first Monday on or after 1st of month

## Future Architecture Needs

Prerequisites before moving beyond sample data:

1. **Error handling** — Replace `.unwrap()` calls with proper error propagation (`anyhow::Result` or custom error types). Define how parse/load errors surface in the UI.
2. **Testing** — Add unit tests for pure functions first: `format_decimal`, `format_pnl`, `format_r`, `pnl_in_r`, `r_value_for_week`, and metric calculations. Gate real data integration on test coverage.
3. **Data lifecycle** — Define how CSV files are loaded into AppState. Likely: mailbox_processor actor receives file paths, parses async, updates state signal. Needs architectural design when the time comes.
4. **State persistence** — R-unit configs and user preferences are currently in-memory only (lost on restart). Add JSON persistence to `Data/` directory before shipping.
5. **Trades pagination** — Current trades view renders up to 200 rows. Add windowed pagination (reuse Timeline's pattern) before real data integration.
