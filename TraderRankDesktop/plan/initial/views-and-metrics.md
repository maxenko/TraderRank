# TraderRankDesktop - Views & Metrics Plan

## Dashboard Tab
- 6 KPI metric cards: Net P&L, Win Rate, Expectancy, Profit Factor, Sharpe Ratio, Max Drawdown
- CSS bar chart equity curve (green/red bars for daily P&L)
- Current week summary with R-multiple performance

## Timeline Tab (key view)
- Toggle: Daily / Weekly / Monthly
- Scrollable window over total data range:
  - Shows "X-Y of N total" with prev/next navigation
  - 8 items visible at a time
- Each row shows:
  - Period label
  - P&L in dollars AND R-multiples (e.g. "+$420 / +4.2R")
  - Win rate %
  - Trade count (W/L)
  - Commission
- Color coded: green positive, red negative

## Trades Tab
- Summary bar: total trades, winners, losers, net P&L, commission
- Scrollable table: Time, Symbol, Side, Qty, Price, P&L, Commission
- Color-coded P&L and side

## Analytics Tab
- KPI cards: Avg Win, Payoff Ratio, Win Streak, Loss Streak
- Symbol performance: horizontal bar chart + P&L + win rate per ticker
- Hourly performance: vertical bar chart by market hour

## Settings Tab
- Theme toggle (dark/light)
- Weekly R-unit configuration: editable R value per week with live P&L/R display
- Data source path placeholder

## R-Unit System
- `WeeklyRConfig { week_start: NaiveDate, r_value: Decimal }`
- Default R = $100
- P&L in R = net_pnl / r_value
- Displayed as "+3.2R" or "-1.5R"
- Editable per week in Settings, reflected in Timeline

**R-value lookup by period:**
- **Daily**: Find the Monday of that day's week, look up R for that week
- **Weekly**: Direct lookup by week start date
- **Monthly**: Use the R-value from the first trading week of the month (find the first Monday on or after the 1st). If no config exists, fall back to default $100.

## Key Metrics Computed

| Metric | Formula |
|--------|---------|
| Expectancy | (Decimal::from(wins)/Decimal::from(total) × avg_win) + (Decimal::from(losses)/Decimal::from(total) × avg_loss). Win% derived from Decimal, never via f64 round-trip. |
| Profit Factor | Sum of individual winning trade P&L / sum of individual losing trade P&L (absolute). Use trade-level, not daily-aggregated. Must be consistent between AppState and model methods. |
| Sharpe Ratio | mean(daily_returns) / stdev(daily_returns, sample N-1) × √252. Use `Decimal::to_f64()` (not string parsing) for the f64 conversion. |
| Max Drawdown | max peak-to-trough decline in cumulative P&L |
| Payoff Ratio | avg_win / abs(avg_loss) |
| Win Rate | winning_trades / total_trades × 100 |

## Day Trader Analytics Research Summary

Critical metrics identified from research across TradeZella, Edgewonk, Tradervue, TradesViz, and quantitative trading resources:

**Implemented in stub:**
- Net/Gross P&L, Win Rate, Expectancy, Profit Factor, Sharpe Ratio
- Max Drawdown, Payoff Ratio, Win/Loss Streaks
- Symbol breakdown, Hourly performance
- R-multiple tracking, Calendar-style daily view

**Future enhancements (prioritized):**
1. Sortino Ratio, Calmar Ratio
2. MAE/MFE (Maximum Adverse/Favorable Excursion)
3. Monte Carlo simulation with confidence intervals
4. Day-of-week performance analysis
5. Revenge trading / overtrading detection
6. Custom trade tagging system
7. Equity curve with moving average overlay
8. P&L distribution histogram
