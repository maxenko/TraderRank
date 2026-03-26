use dioxus::prelude::*;
use crate::components::*;
use crate::settings_store;
use crate::state::AppState;
use rust_decimal::Decimal;

#[derive(Clone, Copy, PartialEq)]
enum ChartRange {
    OneWeek,
    TwoWeeks,
    OneMonth,
    ThreeMonths,
    SixMonths,
    All,
}

impl ChartRange {
    fn label(&self) -> &'static str {
        match self {
            ChartRange::OneWeek => "1W",
            ChartRange::TwoWeeks => "2W",
            ChartRange::OneMonth => "1M",
            ChartRange::ThreeMonths => "3M",
            ChartRange::SixMonths => "6M",
            ChartRange::All => "All",
        }
    }

    fn from_label(s: &str) -> Self {
        match s {
            "1W" => ChartRange::OneWeek,
            "2W" => ChartRange::TwoWeeks,
            "1M" => ChartRange::OneMonth,
            "3M" => ChartRange::ThreeMonths,
            "6M" => ChartRange::SixMonths,
            "All" => ChartRange::All,
            _ => ChartRange::OneMonth,
        }
    }

    fn max_days(&self) -> usize {
        match self {
            ChartRange::OneWeek => 5,
            ChartRange::TwoWeeks => 10,
            ChartRange::OneMonth => 22,
            ChartRange::ThreeMonths => 66,
            ChartRange::SixMonths => 132,
            ChartRange::All => usize::MAX,
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let mut chart_range = use_signal(|| {
        settings_store::load_raw()
            .map(|s| ChartRange::from_label(&s.dashboard_range))
            .unwrap_or(ChartRange::OneMonth)
    });
    let current_range = *chart_range.read();

    // Filter daily summaries by range, excluding excluded days — this drives EVERYTHING
    let max_days = current_range.max_days();
    let all_visible: Vec<_> = data.daily_summaries.iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .collect();
    let total_days = all_visible.len();
    let skip = total_days.saturating_sub(max_days);
    let visible_summaries = &all_visible[skip..];

    // Compute KPIs from filtered data
    let total_pnl: Decimal = visible_summaries.iter().map(|d| d.realized_pnl).sum();
    let total_gross: Decimal = visible_summaries.iter().map(|d| d.gross_pnl).sum();
    let total_trades: u32 = visible_summaries.iter().map(|d| d.total_trades).sum();
    let total_wins: u32 = visible_summaries.iter().map(|d| d.winning_trades).sum();
    let total_losses: u32 = visible_summaries.iter().map(|d| d.losing_trades).sum();
    let total_commission: Decimal = visible_summaries.iter().map(|d| d.total_commission).sum();

    let win_rate = if total_trades > 0 {
        (total_wins as f64) / (total_trades as f64) * 100.0
    } else {
        0.0
    };

    // Avg win/loss from filtered
    let avg_win = if total_wins > 0 {
        let s: Decimal = visible_summaries.iter()
            .map(|d| d.avg_win * Decimal::from(d.winning_trades))
            .sum();
        s / Decimal::from(total_wins)
    } else {
        Decimal::ZERO
    };
    let avg_loss = if total_losses > 0 {
        let s: Decimal = visible_summaries.iter()
            .map(|d| d.avg_loss * Decimal::from(d.losing_trades))
            .sum();
        s / Decimal::from(total_losses)
    } else {
        Decimal::ZERO
    };

    // Expectancy
    let expectancy = if total_trades > 0 {
        let wp = Decimal::from(total_wins) / Decimal::from(total_trades);
        let lp = Decimal::ONE - wp;
        (wp * avg_win) + (lp * avg_loss)
    } else {
        Decimal::ZERO
    };

    // Profit factor
    let total_win_amt = avg_win * Decimal::from(total_wins);
    let total_loss_amt = avg_loss.abs() * Decimal::from(total_losses);
    let profit_factor = if total_loss_amt > Decimal::ZERO {
        Some(total_win_amt / total_loss_amt)
    } else {
        None
    };

    // Payoff ratio
    let payoff_ratio = if avg_loss != Decimal::ZERO {
        Some(avg_win / avg_loss.abs())
    } else {
        None
    };

    // Sharpe from filtered
    let daily_returns: Vec<f64> = visible_summaries.iter()
        .map(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d.realized_pnl).unwrap_or(0.0))
        .collect();
    let n = daily_returns.len() as f64;
    let mean_ret = if n > 0.0 { daily_returns.iter().sum::<f64>() / n } else { 0.0 };
    let sharpe = if n > 1.0 {
        let var = daily_returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / (n - 1.0);
        let sd = var.sqrt();
        if sd > 0.0 { (mean_ret / sd) * 252.0_f64.sqrt() } else { 0.0 }
    } else {
        0.0
    };

    // Max drawdown from filtered
    let mut peak = Decimal::ZERO;
    let mut cum = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    let mut current_streak: i32 = 0;
    let mut cur_w: u32 = 0;
    let mut cur_l: u32 = 0;
    for d in visible_summaries {
        cum += d.realized_pnl;
        if cum > peak { peak = cum; }
        let dd = peak - cum;
        if dd > max_dd { max_dd = dd; }
        if d.realized_pnl > Decimal::ZERO {
            cur_w += 1; cur_l = 0; current_streak = cur_w as i32;
        } else if d.realized_pnl < Decimal::ZERO {
            cur_l += 1; cur_w = 0; current_streak = -(cur_l as i32);
        }
    }

    // Chart data from visible summaries (already range-filtered and exclusion-filtered)
    let visible_pnls: Vec<(String, Decimal)> = visible_summaries.iter()
        .map(|d| (d.date.format("%m/%d").to_string(), d.realized_pnl))
        .collect();
    let max_abs = visible_pnls.iter().fold(Decimal::ZERO, |acc, (_, pnl)| {
        let abs = pnl.abs();
        if abs > acc { abs } else { acc }
    });
    let scale = if max_abs > Decimal::ZERO { max_abs } else { Decimal::ONE };

    // Current week — recompute from filtered daily summaries
    let filtered_weekly = crate::analytics::TradingAnalytics::calculate_weekly_from_daily(
        &all_visible.iter().cloned().cloned().collect::<Vec<_>>()
    );
    let current_week_r = data.r_configs.last().map(|c| c.r_value).unwrap_or(Decimal::new(100, 0));
    let current_week_pnl = filtered_weekly.last().map(|w| w.realized_pnl).unwrap_or(Decimal::ZERO);
    let current_week_r_mult = data.pnl_in_r(current_week_pnl, current_week_r);

    let pf_str = profit_factor.map(|p| format!("{:.2}", p)).unwrap_or("N/A".to_string());
    let pr_str = payoff_ratio.map(|p| format!("{:.2}", p)).unwrap_or("N/A".to_string());

    let ranges = [
        ChartRange::OneWeek, ChartRange::TwoWeeks, ChartRange::OneMonth,
        ChartRange::ThreeMonths, ChartRange::SixMonths, ChartRange::All,
    ];

    let range_label = current_range.label();

    rsx! {
        div { class: "view dashboard-view",
            // Range selector at the top
            div { class: "dashboard-filter-bar",
                span { class: "filter-label", "Showing: {range_label}" }
                span { class: "filter-detail", "{visible_summaries.len()} trading days \u{00B7} {total_trades} trades" }
                div { class: "chart-range-tabs",
                    for r in ranges.iter() {
                        {
                            let r_val = *r;
                            let is_active = current_range == r_val;
                            rsx! {
                                button {
                                    class: if is_active { "range-tab active" } else { "range-tab" },
                                    onclick: move |_| {
                                        chart_range.set(r_val);
                                        settings_store::update(|s| s.dashboard_range = r_val.label().to_string());
                                    },
                                    "{r_val.label()}"
                                }
                            }
                        }
                    }
                }
            }

            // KPI Cards — computed from filtered range
            div { class: "kpi-grid",
                MetricCard {
                    label: "Net P&L".to_string(),
                    value: format_pnl(total_pnl),
                    subtitle: Some(format!("Gross: {} \u{00B7} Comm: {}", format_decimal(total_gross), format_decimal(total_commission))),
                    positive: Some(total_pnl > Decimal::ZERO),
                }
                MetricCard {
                    label: "Win Rate".to_string(),
                    value: format!("{:.1}%", win_rate),
                    subtitle: Some(format!("{} W / {} L", total_wins, total_losses)),
                    positive: Some(win_rate >= 50.0),
                }
                MetricCard {
                    label: "Expectancy".to_string(),
                    value: format_pnl(expectancy),
                    subtitle: Some("Per trade".to_string()),
                    positive: Some(expectancy > Decimal::ZERO),
                }
                MetricCard {
                    label: "Profit Factor".to_string(),
                    value: pf_str,
                    subtitle: Some(format!("Payoff: {}", pr_str)),
                    positive: profit_factor.map(|p| p > Decimal::ONE),
                }
                MetricCard {
                    label: "Sharpe Ratio".to_string(),
                    value: format!("{:.2}", sharpe),
                    subtitle: Some("Annualized".to_string()),
                    positive: Some(sharpe > 0.0),
                }
                MetricCard {
                    label: "Max Drawdown".to_string(),
                    value: format_decimal(max_dd),
                    subtitle: Some(format!("Streak: {} days", current_streak)),
                    positive: Some(false),
                }
            }

            // Equity Curve
            div { class: "card equity-section",
                div { class: "chart-header",
                    h3 { class: "card-title", "Daily P&L" }
                }
                div { class: "equity-chart",
                    for (date, pnl) in visible_pnls.iter() {
                        {
                            let max_bar_px = 160.0_f64;
                            let ratio = rust_decimal::prelude::ToPrimitive::to_f64(&pnl.abs()).unwrap_or(0.0)
                                / rust_decimal::prelude::ToPrimitive::to_f64(&scale).unwrap_or(1.0);
                            let bar_px = (ratio * max_bar_px).max(4.0);
                            let is_pos = *pnl >= Decimal::ZERO;
                            let bar_class = if is_pos { "bar positive" } else { "bar negative" };
                            let pnl_label = format_pnl(*pnl);
                            let tooltip = format!("{}: {}", date, pnl_label);
                            rsx! {
                                div { class: "equity-bar-col",
                                    span { class: "bar-value",
                                        class: if is_pos { "positive" } else { "negative" },
                                        "{pnl_label}"
                                    }
                                    div {
                                        class: "{bar_class}",
                                        style: "height: {bar_px}px;",
                                        title: "{tooltip}",
                                    }
                                    span { class: "bar-date", "{date}" }
                                }
                            }
                        }
                    }
                }
            }

            // Current Week Summary
            div { class: "card",
                h3 { class: "card-title", "This Week" }
                if let Some(week) = filtered_weekly.last() {
                    div { class: "week-summary-grid",
                        div { class: "week-stat",
                            span { class: "stat-label", "P&L" }
                            span {
                                class: if week.realized_pnl >= Decimal::ZERO { "stat-value positive" } else { "stat-value negative" },
                                "{format_pnl(week.realized_pnl)} / {format_r(current_week_r_mult)}"
                            }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "Win Rate" }
                            span { class: "stat-value", "{week.win_rate:.1}%" }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "Trades" }
                            span { class: "stat-value", "{week.total_trades}" }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "R Value" }
                            span { class: "stat-value", "{format_decimal(current_week_r)}" }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "Trading Days" }
                            span { class: "stat-value", "{week.trading_days} / {week.profitable_days} profitable" }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "Commission" }
                            span { class: "stat-value negative", "{format_decimal(week.total_commission)}" }
                        }
                    }
                }
            }
        }
    }
}
