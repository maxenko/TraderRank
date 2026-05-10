use dioxus::prelude::*;
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;
use crate::components::*;
use crate::settings_store;
use crate::state::{AppState, TradeOutcome};
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
    let stats_config = use_context::<Signal<crate::state::StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;

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
    let total_pnl: Decimal = visible_summaries.iter().map(|d| data.daily_pnl(d, count_commissions)).sum();
    let total_gross: Decimal = visible_summaries.iter().map(|d| d.gross_pnl).sum();
    let total_trades: u32 = visible_summaries.iter().map(|d| d.total_trades).sum();
    // Classify W/L/Lossless from matched trades using R threshold
    let cutoff_date = visible_summaries.first().map(|d| d.date);
    let visible_matched: Vec<_> = data.matched_trades.iter()
        .filter(|mt| cutoff_date.map_or(true, |c| mt.exit_time >= c))
        .filter(|mt| !data.is_trade_excluded(mt))
        .collect();
    let total_wins: u32 = visible_matched.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Winner).count() as u32;
    let total_lossless: u32 = visible_matched.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Lossless).count() as u32;
    let total_losses: u32 = visible_matched.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Loser).count() as u32;
    let total_commission: Decimal = visible_summaries.iter().map(|d| d.total_commission).sum();

    // Total R for the period — each day's P&L divided by its week's R value
    let total_r: Decimal = visible_summaries.iter().map(|d| {
        let days_from_mon = d.date.weekday().num_days_from_monday();
        let monday = d.date.date_naive() - chrono::Duration::days(days_from_mon as i64);
        let r_val = data.r_value_for_week(monday);
        data.pnl_in_r(data.daily_pnl(d, count_commissions), r_val)
    }).sum();

    let win_rate = if total_wins + total_losses > 0 {
        (total_wins as f64) / ((total_wins + total_losses) as f64) * 100.0
    } else {
        0.0
    };

    // Avg win/loss from matched trades using R classification
    let winning_pnls: Vec<Decimal> = visible_matched.iter()
        .filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Winner)
        .map(|t| data.trade_pnl(t, count_commissions)).collect();
    let losing_pnls: Vec<Decimal> = visible_matched.iter()
        .filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Loser)
        .map(|t| data.trade_pnl(t, count_commissions)).collect();
    let avg_win = if !winning_pnls.is_empty() {
        winning_pnls.iter().sum::<Decimal>() / Decimal::from(winning_pnls.len() as u32)
    } else {
        Decimal::ZERO
    };
    let avg_loss = if !losing_pnls.is_empty() {
        losing_pnls.iter().sum::<Decimal>() / Decimal::from(losing_pnls.len() as u32)
    } else {
        Decimal::ZERO
    };

    // Expectancy (lossless excluded — only W and L determine the probability)
    let decided = total_wins + total_losses;
    let expectancy = if decided > 0 {
        let wp = Decimal::from(total_wins) / Decimal::from(decided);
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
        .map(|d| rust_decimal::prelude::ToPrimitive::to_f64(&data.daily_pnl(d, count_commissions)).unwrap_or(0.0))
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
        let day_pnl = data.daily_pnl(d, count_commissions);
        cum += day_pnl;
        if cum > peak { peak = cum; }
        let dd = peak - cum;
        if dd > max_dd { max_dd = dd; }
        if day_pnl > Decimal::ZERO {
            cur_w += 1; cur_l = 0; current_streak = cur_w as i32;
        } else if day_pnl < Decimal::ZERO {
            cur_l += 1; cur_w = 0; current_streak = -(cur_l as i32);
        }
    }

    // Chart data from visible summaries (already range-filtered and exclusion-filtered)
    let visible_pnls: Vec<(String, Decimal, NaiveDate, Decimal)> = visible_summaries.iter()
        .map(|d| (
            d.date.format("%m/%d").to_string(),
            data.daily_pnl(d, count_commissions),
            d.date.date_naive(),
            d.total_commission,
        ))
        .collect();

    // Per-day W/LL/L counts using R-classification (matches the rest of the dashboard)
    let mut day_stats: HashMap<NaiveDate, (u32, u32, u32)> = HashMap::new();
    for mt in visible_matched.iter() {
        let date = mt.exit_time.date_naive();
        let entry = day_stats.entry(date).or_insert((0, 0, 0));
        match data.trade_outcome_with(mt, count_commissions) {
            TradeOutcome::Winner => entry.0 += 1,
            TradeOutcome::Lossless => entry.1 += 1,
            TradeOutcome::Loser => entry.2 += 1,
        }
    }

    let mut hovered_date = use_signal(|| None::<NaiveDate>);
    let max_abs = visible_pnls.iter().fold(Decimal::ZERO, |acc, (_, pnl, _, _)| {
        let abs = pnl.abs();
        if abs > acc { abs } else { acc }
    });
    let scale = if max_abs > Decimal::ZERO { max_abs } else { Decimal::ONE };

    // Current week — recompute from filtered daily summaries
    let filtered_weekly = crate::analytics::TradingAnalytics::calculate_weekly_from_daily(
        &all_visible.iter().cloned().cloned().collect::<Vec<_>>()
    );
    let current_week_r = data.r_configs.last().map(|c| c.r_value).unwrap_or(data.default_r_value);
    let current_week_pnl: Decimal = filtered_weekly.last().map(|w| {
        all_visible.iter()
            .filter(|d| d.date >= w.start_date && d.date <= w.end_date)
            .map(|d| data.daily_pnl(d, count_commissions))
            .sum()
    }).unwrap_or(Decimal::ZERO);
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
                    value: format!("{} / {}", format_r(total_r), format_pnl(total_pnl)),
                    subtitle: Some(format!("Gross: {} \u{00B7} Comm: {}", format_decimal(total_gross), format_decimal(total_commission))),
                    positive: Some(total_pnl > Decimal::ZERO),
                }
                MetricCard {
                    label: "Win Rate".to_string(),
                    value: format!("{:.1}%", win_rate),
                    subtitle: Some(format!("{} W / {} LL / {} L", total_wins, total_lossless, total_losses)),
                    positive: Some(win_rate >= 50.0),
                }
                MetricCard {
                    label: "Expectancy".to_string(),
                    value: format_pnl(expectancy),
                    subtitle: Some(format!("{} per trade", format_r(if total_trades > 0 { total_r / Decimal::from(total_trades) } else { Decimal::ZERO }))),
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
                    {
                        let hov = *hovered_date.read();
                        if let Some(d) = hov {
                            let (w, ll, l) = day_stats.get(&d).copied().unwrap_or((0, 0, 0));
                            let comm = visible_summaries.iter()
                                .find(|s| s.date.date_naive() == d)
                                .map(|s| s.total_commission)
                                .unwrap_or(Decimal::ZERO);
                            let date_str = d.format("%a %m/%d").to_string();
                            rsx! {
                                span { class: "chart-hover-info",
                                    span { class: "hover-date", "{date_str}" }
                                    span { class: "hover-sep", " \u{00B7} " }
                                    span { class: "hover-positive", "{w} W" }
                                    span { class: "hover-sep", " / " }
                                    span { class: "hover-neutral", "{ll} LL" }
                                    span { class: "hover-sep", " / " }
                                    span { class: "hover-negative", "{l} L" }
                                    span { class: "hover-comm", " ({format_decimal(comm)} comm)" }
                                }
                            }
                        } else {
                            rsx! { span { class: "chart-hover-hint", "Hover a bar for details" } }
                        }
                    }
                }
                div { class: "equity-chart",
                    for (date, pnl, date_naive, _comm) in visible_pnls.iter() {
                        {
                            let max_bar_px = 160.0_f64;
                            let ratio = rust_decimal::prelude::ToPrimitive::to_f64(&pnl.abs()).unwrap_or(0.0)
                                / rust_decimal::prelude::ToPrimitive::to_f64(&scale).unwrap_or(1.0);
                            let bar_px = (ratio * max_bar_px).max(4.0);
                            let is_pos = *pnl >= Decimal::ZERO;
                            let bar_class = if is_pos { "bar positive" } else { "bar negative" };
                            let pnl_label = format_pnl(*pnl);
                            let tooltip = format!("{}: {}", date, pnl_label);
                            let dn = *date_naive;
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
                                        onmouseenter: move |_| hovered_date.set(Some(dn)),
                                        onmouseleave: move |_| hovered_date.set(None),
                                    }
                                    span { class: "bar-date", "{date}" }
                                }
                            }
                        }
                    }
                }
            }

            // Best vs Worst Day by Weekday (own range selector)
            crate::views::best_worst::BestWorstByWeekday {}

            // Performance Trends — 5-tab analytical section
            crate::views::trends::Trends { max_days: max_days }

            // Current Week Summary
            div { class: "card",
                h3 { class: "card-title", "This Week" }
                if let Some(week) = filtered_weekly.last() {
                    {
                    // Recompute this week's win rate from matched trades using R classification
                    let week_wins: u32 = visible_matched.iter()
                        .filter(|mt| mt.exit_time >= week.start_date)
                        .filter(|mt| data.trade_outcome_with(mt, count_commissions) == TradeOutcome::Winner)
                        .count() as u32;
                    let week_losses: u32 = visible_matched.iter()
                        .filter(|mt| mt.exit_time >= week.start_date)
                        .filter(|mt| data.trade_outcome_with(mt, count_commissions) == TradeOutcome::Loser)
                        .count() as u32;
                    let this_week_wr = if week_wins + week_losses > 0 {
                        (week_wins as f64 / (week_wins + week_losses) as f64) * 100.0
                    } else { 0.0 };
                    rsx! {
                    div { class: "week-summary-grid",
                        div { class: "week-stat",
                            span { class: "stat-label", "P&L" }
                            span {
                                class: if current_week_pnl >= Decimal::ZERO { "stat-value positive" } else { "stat-value negative" },
                                "{format_pnl(current_week_pnl)} / {format_r(current_week_r_mult)}"
                            }
                        }
                        div { class: "week-stat",
                            span { class: "stat-label", "Win Rate" }
                            span { class: "stat-value", "{this_week_wr:.1}%" }
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
                    } // rsx
                    } // block
                }
            }
        }
    }
}
