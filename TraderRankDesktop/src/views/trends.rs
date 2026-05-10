// Dashboard "Trends" tabbed section — five performance-over-time visualizations:
//   1. Equity & Drawdown   — cumulative line + underwater pane
//   2. Edge Trend          — rolling expectancy in R + rolling profit factor + rolling win rate
//   3. Calendar Heatmap    — weekday × week PnL heatmap + monthly totals + weekday small-multiples
//   4. Behavior            — trades-per-day with overtrading flags + hour × weekday P&L heatmap
//   5. Streaks & Consistency — win/loss strip + rolling longest loss streak + consistency stats
//
// All tabs respect the dashboard's range filter (passed via `max_days`) and the
// AppState exclusion filtering. R-based outcome classification matches the rest
// of the app (state.rs::TradeOutcome).

use chrono::{Datelike, NaiveDate, Weekday};
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::components::*;
use crate::models::{DailySummary, MatchedTrade};
use crate::settings_store;
use crate::state::{AppState, TradeOutcome};

#[derive(Clone, Copy, PartialEq)]
enum TrendsTab {
    Equity,
    Edge,
    Calendar,
    Behavior,
    Streaks,
}

impl TrendsTab {
    fn as_str(&self) -> &'static str {
        match self {
            TrendsTab::Equity => "Equity",
            TrendsTab::Edge => "Edge",
            TrendsTab::Calendar => "Calendar",
            TrendsTab::Behavior => "Behavior",
            TrendsTab::Streaks => "Streaks",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            TrendsTab::Equity => "Equity & Drawdown",
            TrendsTab::Edge => "Edge Trend",
            TrendsTab::Calendar => "Calendar",
            TrendsTab::Behavior => "Behavior",
            TrendsTab::Streaks => "Streaks",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "Edge" => TrendsTab::Edge,
            "Calendar" => TrendsTab::Calendar,
            "Behavior" => TrendsTab::Behavior,
            "Streaks" => TrendsTab::Streaks,
            _ => TrendsTab::Equity,
        }
    }
}

const TABS: [TrendsTab; 5] = [
    TrendsTab::Equity,
    TrendsTab::Edge,
    TrendsTab::Calendar,
    TrendsTab::Behavior,
    TrendsTab::Streaks,
];

#[component]
pub fn Trends(max_days: usize) -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let stats_config = use_context::<Signal<crate::state::StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;

    let mut active_tab = use_signal(|| {
        settings_store::load_raw()
            .map(|s| TrendsTab::from_str(&s.trends_tab))
            .unwrap_or(TrendsTab::Equity)
    });
    let current_tab = *active_tab.read();

    // Hover state for calendar heatmap cells (hoisted here so the hook is
    // called every render regardless of which tab is active)
    let hovered_day = use_signal(|| None::<NaiveDate>);

    // Filter to non-excluded days, then take the last `max_days`
    let non_excluded: Vec<DailySummary> = data
        .daily_summaries
        .iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .cloned()
        .collect();
    let total_days = non_excluded.len();
    let skip = total_days.saturating_sub(max_days);
    let visible_days: Vec<DailySummary> = non_excluded[skip..].to_vec();

    // Filter matched trades by the same cutoff and exclusions
    let cutoff = visible_days.first().map(|d| d.date);
    let visible_matched: Vec<MatchedTrade> = data
        .matched_trades
        .iter()
        .filter(|mt| cutoff.is_none_or(|c| mt.exit_time >= c))
        .filter(|mt| !data.is_trade_excluded(mt))
        .cloned()
        .collect();
    // Trades come pre-sorted descending by exit_time — flip to ascending for time-series
    let mut chrono_matched = visible_matched.clone();
    chrono_matched.sort_by_key(|mt| mt.exit_time);

    rsx! {
        div { class: "card trends-section",
            div { class: "trends-header",
                h3 { class: "card-title", "Performance Trends" }
                div { class: "trends-tabs",
                    for t in TABS.iter() {
                        {
                            let tv = *t;
                            let cls = if current_tab == tv { "trends-tab active" } else { "trends-tab" };
                            rsx! {
                                button {
                                    class: "{cls}",
                                    onclick: move |_| {
                                        active_tab.set(tv);
                                        settings_store::update(|s| s.trends_tab = tv.as_str().to_string());
                                    },
                                    "{tv.label()}"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "trends-body",
                {
                    if visible_days.is_empty() {
                        rsx! { div { class: "trends-empty", "No data in the selected range." } }
                    } else {
                        match current_tab {
                            TrendsTab::Equity => render_equity_tab(&visible_days, &data, count_commissions),
                            TrendsTab::Edge => render_edge_tab(&chrono_matched, &data, count_commissions),
                            TrendsTab::Calendar => render_calendar_tab(&visible_days, &chrono_matched, &data, hovered_day, count_commissions),
                            TrendsTab::Behavior => render_behavior_tab(&visible_days, &chrono_matched, &data, count_commissions),
                            TrendsTab::Streaks => render_streaks_tab(&visible_days, &chrono_matched, &data, count_commissions),
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Tab 1: Equity & Drawdown
// ============================================================================

fn render_equity_tab(days: &[DailySummary], data: &AppState, count_commissions: bool) -> Element {
    // Compute cumulative equity and underwater drawdown
    let mut cum = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut equity: Vec<(NaiveDate, f64, f64)> = Vec::with_capacity(days.len()); // (date, cum, underwater)
    let mut max_dd = 0.0_f64;
    let mut peak_to_now_days = 0_usize;
    let mut days_since_peak = 0_usize;

    for d in days {
        cum += data.daily_pnl(d, count_commissions);
        if cum > peak {
            peak = cum;
            days_since_peak = 0;
        } else {
            days_since_peak += 1;
            if days_since_peak > peak_to_now_days {
                peak_to_now_days = days_since_peak;
            }
        }
        let cum_f = cum.to_f64().unwrap_or(0.0);
        let dd_f = (cum - peak).to_f64().unwrap_or(0.0); // <= 0
        if dd_f.abs() > max_dd {
            max_dd = dd_f.abs();
        }
        equity.push((d.date.date_naive(), cum_f, dd_f));
    }

    let final_pnl = equity.last().map(|(_, c, _)| *c).unwrap_or(0.0);
    let cur_underwater = equity.last().map(|(_, _, dd)| *dd).unwrap_or(0.0);

    let svg_w = 1000.0_f64;
    let eq_h = 200.0_f64;
    let dd_h = 100.0_f64;

    // Equity bounds
    let max_eq = equity.iter().map(|(_, c, _)| *c).fold(0.0_f64, f64::max);
    let min_eq = equity.iter().map(|(_, c, _)| *c).fold(0.0_f64, f64::min);
    let eq_span = (max_eq - min_eq).max(1.0);

    // Drawdown bounds (always 0..min)
    let min_dd = equity.iter().map(|(_, _, dd)| *dd).fold(0.0_f64, f64::min);
    let dd_span = (-min_dd).max(1.0);

    let n = equity.len() as f64;
    let dx = if n > 1.0 { svg_w / (n - 1.0) } else { svg_w };

    // Build equity polyline points
    let eq_points: String = equity
        .iter()
        .enumerate()
        .map(|(i, (_, c, _))| {
            let x = i as f64 * dx;
            let y = eq_h - ((c - min_eq) / eq_span) * eq_h;
            format!("{:.1},{:.1}", x, y)
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Equity area fill (close to baseline = min_eq)
    let baseline_y = eq_h - ((0.0_f64.max(min_eq) - min_eq) / eq_span) * eq_h;
    let eq_area = format!(
        "M 0,{baseline_y:.1} L {points} L {last_x:.1},{baseline_y:.1} Z",
        baseline_y = baseline_y,
        points = eq_points,
        last_x = (equity.len() as f64 - 1.0).max(0.0) * dx,
    );

    // Drawdown area (from 0 down to underwater value)
    let dd_area = {
        let mut s = String::from("M 0,0 ");
        for (i, (_, _, dd)) in equity.iter().enumerate() {
            let x = i as f64 * dx;
            let y = (-dd / dd_span) * dd_h;
            s.push_str(&format!("L {:.1},{:.1} ", x, y));
        }
        s.push_str(&format!("L {:.1},0 Z", (equity.len() as f64 - 1.0).max(0.0) * dx));
        s
    };

    let zero_y = if min_eq < 0.0 && max_eq > 0.0 {
        Some(eq_h - ((0.0 - min_eq) / eq_span) * eq_h)
    } else {
        None
    };

    let final_class = if final_pnl >= 0.0 { "eq-line positive" } else { "eq-line negative" };

    rsx! {
        div { class: "trend-stats-row",
            div { class: "trend-stat",
                span { class: "stat-label", "Net P&L" }
                span {
                    class: if final_pnl >= 0.0 { "stat-value positive" } else { "stat-value negative" },
                    "{format_pnl(Decimal::from_f64_retain(final_pnl).unwrap_or(Decimal::ZERO))}"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Max Drawdown" }
                span { class: "stat-value negative",
                    "-{format_decimal(Decimal::from_f64_retain(max_dd).unwrap_or(Decimal::ZERO))}"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Currently Underwater" }
                span {
                    class: if cur_underwater < 0.0 { "stat-value negative" } else { "stat-value positive" },
                    if cur_underwater < 0.0 {
                        "-{format_decimal(Decimal::from_f64_retain(-cur_underwater).unwrap_or(Decimal::ZERO))}"
                    } else {
                        "$0.00"
                    }
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Days Since Peak" }
                span { class: "stat-value", "{days_since_peak}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Longest Underwater" }
                span { class: "stat-value", "{peak_to_now_days} days" }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Cumulative Equity" }
            svg {
                class: "trend-svg",
                width: "100%",
                height: "{eq_h}",
                view_box: "0 0 {svg_w} {eq_h}",
                preserve_aspect_ratio: "none",
                // zero line if applicable
                if let Some(zy) = zero_y {
                    line {
                        x1: "0",
                        y1: "{zy}",
                        x2: "{svg_w}",
                        y2: "{zy}",
                        stroke: "var(--border-color)",
                        stroke_dasharray: "4,4",
                        stroke_width: "1",
                    }
                }
                // area fill
                path { d: "{eq_area}", fill: if final_pnl >= 0.0 { "rgba(0, 212, 170, 0.15)" } else { "rgba(255, 77, 106, 0.15)" } }
                // equity line
                polyline {
                    class: "{final_class}",
                    points: "{eq_points}",
                    fill: "none",
                    stroke_width: "2",
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Underwater Drawdown" }
            svg {
                class: "trend-svg",
                width: "100%",
                height: "{dd_h}",
                view_box: "0 0 {svg_w} {dd_h}",
                preserve_aspect_ratio: "none",
                path { d: "{dd_area}", fill: "rgba(255, 77, 106, 0.35)", stroke: "var(--accent-red)", stroke_width: "1" }
            }
        }
    }
}

// ============================================================================
// Tab 2: Edge Trend
// ============================================================================

fn render_edge_tab(matched: &[MatchedTrade], data: &AppState, count_commissions: bool) -> Element {
    if matched.is_empty() {
        return rsx! { div { class: "trends-empty", "Need matched trades to compute edge." } };
    }

    // Convert each trade's net_pnl to R based on its week's R config
    let r_per_trade: Vec<f64> = matched
        .iter()
        .map(|mt| {
            let days_from_mon = mt.exit_time.date_naive().weekday().num_days_from_monday();
            let monday = mt.exit_time.date_naive() - chrono::Duration::days(days_from_mon as i64);
            let r_val = data.r_value_for_week(monday);
            data.pnl_in_r(data.trade_pnl(mt, count_commissions), r_val).to_f64().unwrap_or(0.0)
        })
        .collect();

    // Win/loss flags using R-classification
    let outcomes: Vec<TradeOutcome> = matched.iter().map(|mt| data.trade_outcome_with(mt, count_commissions)).collect();

    let exp_20 = rolling_mean(&r_per_trade, 20);
    let exp_50 = rolling_mean(&r_per_trade, 50);
    let win_rate = rolling_win_rate(&outcomes, 50);
    let pf = rolling_profit_factor(matched, 50, count_commissions, data);

    let svg_w = 1000.0_f64;
    let main_h = 220.0_f64;
    let sub_h = 100.0_f64;

    // Determine y-range for expectancy (symmetric around 0)
    let max_abs_exp = exp_20.iter().chain(exp_50.iter())
        .filter_map(|v| v.map(|x| x.abs()))
        .fold(0.0_f64, f64::max);
    let exp_span = max_abs_exp.max(0.5);

    let exp_to_y = |val: f64| -> f64 {
        let half = main_h / 2.0;
        half - (val / exp_span) * (half * 0.85)
    };

    let n = r_per_trade.len() as f64;
    let dx = if n > 1.0 { svg_w / (n - 1.0) } else { svg_w };

    let zero_y_main = main_h / 2.0;

    let exp_20_path = polyline_from_optional(&exp_20, dx, exp_to_y);
    let exp_50_path = polyline_from_optional(&exp_50, dx, exp_to_y);

    // Win rate (bounded 0-100, baseline at 50%)
    let wr_to_y = |v: f64| -> f64 { sub_h - (v / 100.0) * sub_h };
    let wr_baseline_y = wr_to_y(50.0);
    let wr_path = polyline_from_optional(&win_rate, dx, wr_to_y);

    // Profit factor (>= 0, log-ish; clamp at 5 for display)
    let pf_clamped: Vec<Option<f64>> = pf.iter().map(|v| v.map(|x| x.min(5.0))).collect();
    let pf_to_y = |v: f64| -> f64 { sub_h - (v / 5.0) * sub_h };
    let pf_baseline_y = pf_to_y(1.0);
    let pf_path = polyline_from_optional(&pf_clamped, dx, pf_to_y);

    let total_trades = matched.len();
    let total_r: f64 = r_per_trade.iter().sum();
    let avg_r = if total_trades > 0 { total_r / total_trades as f64 } else { 0.0 };
    let recent_50 = if total_trades >= 50 {
        let s: f64 = r_per_trade[total_trades - 50..].iter().sum();
        s / 50.0
    } else if total_trades > 0 {
        let s: f64 = r_per_trade.iter().sum();
        s / total_trades as f64
    } else { 0.0 };

    rsx! {
        div { class: "trend-stats-row",
            div { class: "trend-stat",
                span { class: "stat-label", "Trades in Range" }
                span { class: "stat-value", "{total_trades}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Avg R per Trade" }
                span {
                    class: if avg_r >= 0.0 { "stat-value positive" } else { "stat-value negative" },
                    "{avg_r:+.2}R"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Recent 50-trade R" }
                span {
                    class: if recent_50 >= avg_r { "stat-value positive" } else { "stat-value negative" },
                    "{recent_50:+.2}R"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Trend" }
                span {
                    class: if recent_50 >= avg_r { "stat-value positive" } else { "stat-value negative" },
                    if recent_50 >= avg_r { "Improving" } else { "Decaying" }
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label",
                "Rolling Expectancy in R"
                span { class: "chart-legend-dot dot-r20", " " }
                span { class: "chart-legend-text", "20-trade" }
                span { class: "chart-legend-dot dot-r50", " " }
                span { class: "chart-legend-text", "50-trade" }
            }
            svg {
                class: "trend-svg",
                width: "100%",
                height: "{main_h}",
                view_box: "0 0 {svg_w} {main_h}",
                preserve_aspect_ratio: "none",
                line { x1: "0", y1: "{zero_y_main}", x2: "{svg_w}", y2: "{zero_y_main}", stroke: "var(--border-color)", stroke_dasharray: "4,4", stroke_width: "1" }
                polyline { points: "{exp_20_path}", fill: "none", stroke: "var(--accent-yellow)", stroke_width: "1.5", opacity: "0.7" }
                polyline { points: "{exp_50_path}", fill: "none", stroke: "var(--accent-primary)", stroke_width: "2.5" }
            }
        }

        div { class: "chart-block-row",
            div { class: "chart-block half",
                div { class: "chart-label", "Rolling Win Rate (50-trade)" }
                svg {
                    class: "trend-svg",
                    width: "100%",
                    height: "{sub_h}",
                    view_box: "0 0 {svg_w} {sub_h}",
                    preserve_aspect_ratio: "none",
                    line { x1: "0", y1: "{wr_baseline_y}", x2: "{svg_w}", y2: "{wr_baseline_y}", stroke: "var(--border-color)", stroke_dasharray: "3,3", stroke_width: "1" }
                    polyline { points: "{wr_path}", fill: "none", stroke: "var(--accent-green)", stroke_width: "2" }
                }
            }
            div { class: "chart-block half",
                div { class: "chart-label", "Rolling Profit Factor (50-trade, capped at 5)" }
                svg {
                    class: "trend-svg",
                    width: "100%",
                    height: "{sub_h}",
                    view_box: "0 0 {svg_w} {sub_h}",
                    preserve_aspect_ratio: "none",
                    line { x1: "0", y1: "{pf_baseline_y}", x2: "{svg_w}", y2: "{pf_baseline_y}", stroke: "var(--border-color)", stroke_dasharray: "3,3", stroke_width: "1" }
                    polyline { points: "{pf_path}", fill: "none", stroke: "var(--accent-primary)", stroke_width: "2" }
                }
            }
        }
    }
}

// ============================================================================
// Tab 3: Calendar Heatmap
// ============================================================================

fn render_calendar_tab(
    days: &[DailySummary],
    matched: &[MatchedTrade],
    data: &AppState,
    mut hovered: Signal<Option<NaiveDate>>,
    count_commissions: bool,
) -> Element {
    if days.is_empty() {
        return rsx! { div { class: "trends-empty", "No data." } };
    }

    let by_date: HashMap<NaiveDate, Decimal> = days
        .iter()
        .map(|d| (d.date.date_naive(), data.daily_pnl(d, count_commissions)))
        .collect();

    // Map date -> full DailySummary (for hover detail panel)
    let day_by_date: HashMap<NaiveDate, &DailySummary> = days
        .iter()
        .map(|d| (d.date.date_naive(), d))
        .collect();

    // Matched trades grouped by exit date
    let mut matched_by_date: HashMap<NaiveDate, Vec<&MatchedTrade>> = HashMap::new();
    for mt in matched {
        matched_by_date.entry(mt.exit_time.date_naive()).or_default().push(mt);
    }

    let first = days.first().map(|d| d.date.date_naive()).unwrap();
    let last = days.last().map(|d| d.date.date_naive()).unwrap();
    // Snap to Sunday to align weeks
    let start_sun = first - chrono::Duration::days(first.weekday().num_days_from_sunday() as i64);
    let end_sat = last + chrono::Duration::days((6 - last.weekday().num_days_from_sunday()) as i64);
    let total_days_span = (end_sat - start_sun).num_days() as usize + 1;
    let n_weeks = total_days_span.div_ceil(7);

    // Find max abs P&L for color scaling
    let max_abs = days.iter().fold(Decimal::ZERO, |acc, d| {
        let abs = data.daily_pnl(d, count_commissions).abs();
        if abs > acc { abs } else { acc }
    });
    let max_abs_f = max_abs.to_f64().unwrap_or(1.0).max(1.0);

    // Build a flat grid: weekday rows × week cols
    let mut cells: Vec<Vec<Option<(NaiveDate, f64)>>> = vec![vec![None; n_weeks]; 7];
    let mut cur = start_sun;
    let mut week_idx = 0_usize;
    while cur <= end_sat {
        let wd = cur.weekday().num_days_from_sunday() as usize;
        let pnl = by_date.get(&cur).map(|p| p.to_f64().unwrap_or(0.0));
        cells[wd][week_idx] = pnl.map(|p| (cur, p));
        cur += chrono::Duration::days(1);
        if cur.weekday() == Weekday::Sun {
            week_idx += 1;
        }
    }

    let weekday_labels = ["S", "M", "T", "W", "T", "F", "S"];

    // Monthly totals for the strip
    let mut monthly: Vec<(i32, u32, Decimal)> = Vec::new();
    for d in days {
        let y = d.date.year();
        let m = d.date.month();
        let day_pnl = data.daily_pnl(d, count_commissions);
        if let Some(last) = monthly.last_mut() {
            if last.0 == y && last.1 == m {
                last.2 += day_pnl;
                continue;
            }
        }
        monthly.push((y, m, day_pnl));
    }
    let monthly_max_abs = monthly.iter().fold(Decimal::ZERO, |acc, (_, _, p)| {
        let a = p.abs();
        if a > acc { a } else { acc }
    });
    let monthly_max_abs_f = monthly_max_abs.to_f64().unwrap_or(1.0).max(1.0);

    // Weekday small-multiples — total P&L by weekday across all visible days
    let mut weekday_pnl: [Decimal; 7] = [Decimal::ZERO; 7];
    let mut weekday_count: [u32; 7] = [0; 7];
    for d in days {
        let wd = d.date.weekday().num_days_from_sunday() as usize;
        weekday_pnl[wd] += data.daily_pnl(d, count_commissions);
        weekday_count[wd] += 1;
    }
    let wd_max_abs = weekday_pnl.iter().fold(Decimal::ZERO, |acc, p| {
        let a = p.abs();
        if a > acc { a } else { acc }
    });
    let wd_max_abs_f = wd_max_abs.to_f64().unwrap_or(1.0).max(1.0);

    let total_pnl_f: f64 = days.iter().map(|d| data.daily_pnl(d, count_commissions).to_f64().unwrap_or(0.0)).sum();
    let green_days = days.iter().filter(|d| data.daily_pnl(d, count_commissions) > Decimal::ZERO).count();
    let red_days = days.iter().filter(|d| data.daily_pnl(d, count_commissions) < Decimal::ZERO).count();
    let flat_days = days.iter().filter(|d| data.daily_pnl(d, count_commissions) == Decimal::ZERO).count();

    rsx! {
        div { class: "trend-stats-row",
            div { class: "trend-stat",
                span { class: "stat-label", "Trading Days" }
                span { class: "stat-value", "{days.len()}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Green Days" }
                span { class: "stat-value positive", "{green_days}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Red Days" }
                span { class: "stat-value negative", "{red_days}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Flat Days" }
                span { class: "stat-value", "{flat_days}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Total" }
                span {
                    class: if total_pnl_f >= 0.0 { "stat-value positive" } else { "stat-value negative" },
                    "{format_pnl(Decimal::from_f64_retain(total_pnl_f).unwrap_or(Decimal::ZERO))}"
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Daily P&L Heatmap" }
            div { class: "calendar-row",
                div { class: "calendar-heatmap",
                    div { class: "cal-row cal-labels",
                        div { class: "cal-day-label", " " }
                        for w in 0..n_weeks {
                            // Show month label only at first week of a month (rough)
                            {
                                let week_start = start_sun + chrono::Duration::days((w * 7) as i64);
                                let label = if w == 0 || week_start.day() <= 7 {
                                    week_start.format("%b").to_string()
                                } else {
                                    String::new()
                                };
                                rsx! { div { class: "cal-week-label", "{label}" } }
                            }
                        }
                    }
                    for wd in 0..7 {
                        div { class: "cal-row",
                            div { class: "cal-day-label", "{weekday_labels[wd]}" }
                            for w in 0..n_weeks {
                                {
                                    let cell = cells[wd][w];
                                    if let Some((date, pnl)) = cell {
                                        let intensity = (pnl.abs() / max_abs_f).min(1.0);
                                        let level = ((intensity * 4.0).ceil() as i32).clamp(1, 4);
                                        let sign_cls = if pnl > 0.0 { "pos" } else if pnl < 0.0 { "neg" } else { "flat" };
                                        let cls = format!("cal-cell {} l{}", sign_cls, level);
                                        let title = format!("{} — ${:.2}", date.format("%a %m/%d"), pnl);
                                        rsx! {
                                            div {
                                                class: "{cls}",
                                                title: "{title}",
                                                onmouseenter: move |_| hovered.set(Some(date)),
                                                onmouseleave: move |_| hovered.set(None),
                                            }
                                        }
                                    } else {
                                        rsx! { div { class: "cal-cell empty" } }
                                    }
                                }
                            }
                        }
                    }
                }

                // Detail / summary side panel
                {
                    let hov = *hovered.read();
                    render_calendar_side_panel(hov, &day_by_date, &matched_by_date, days, data, count_commissions)
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Monthly P&L" }
            div { class: "monthly-strip",
                for (y, m, pnl) in monthly.iter() {
                    {
                        let pnl_f = pnl.to_f64().unwrap_or(0.0);
                        let height_pct = ((pnl.abs().to_f64().unwrap_or(0.0) / monthly_max_abs_f) * 100.0).max(2.0);
                        let is_pos = *pnl >= Decimal::ZERO;
                        let bar_cls = if is_pos { "month-bar positive" } else { "month-bar negative" };
                        let label = format!("{}/{:02}", y % 100, m);
                        let val_cls = if is_pos { "month-val positive" } else { "month-val negative" };
                        rsx! {
                            div { class: "month-col",
                                span { class: "{val_cls}", "{format_pnl(*pnl)}" }
                                div { class: "month-bar-wrap",
                                    div {
                                        class: "{bar_cls}",
                                        style: "height: {height_pct}%;",
                                        title: "{label}: ${pnl_f:.2}",
                                    }
                                }
                                span { class: "month-label", "{label}" }
                            }
                        }
                    }
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Performance by Weekday" }
            div { class: "weekday-strip",
                {
                    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                    rsx! {
                        for i in 1..6 { // skip weekend (no trading)
                            {
                                let pnl = weekday_pnl[i];
                                let count = weekday_count[i];
                                let pnl_f = pnl.to_f64().unwrap_or(0.0);
                                let height_pct = ((pnl.abs().to_f64().unwrap_or(0.0) / wd_max_abs_f) * 100.0).max(2.0);
                                let is_pos = pnl >= Decimal::ZERO;
                                let bar_cls = if is_pos { "wd-bar positive" } else { "wd-bar negative" };
                                let val_cls = if is_pos { "wd-val positive" } else { "wd-val negative" };
                                let avg = if count > 0 { pnl_f / count as f64 } else { 0.0 };
                                rsx! {
                                    div { class: "wd-col",
                                        span { class: "{val_cls}", "{format_pnl(pnl)}" }
                                        div { class: "wd-bar-wrap",
                                            div {
                                                class: "{bar_cls}",
                                                style: "height: {height_pct}%;",
                                                title: "{names[i]}: total ${pnl_f:.2}, avg ${avg:.2}/day, {count} days",
                                            }
                                        }
                                        span { class: "wd-label", "{names[i]}" }
                                        span { class: "wd-sub", "{count}d \u{00B7} avg ${avg:.0}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Side panel next to the calendar heatmap.
/// On hover: full breakdown of the hovered day. Default: range summary stats.
fn render_calendar_side_panel(
    hovered: Option<NaiveDate>,
    day_by_date: &HashMap<NaiveDate, &DailySummary>,
    matched_by_date: &HashMap<NaiveDate, Vec<&MatchedTrade>>,
    days: &[DailySummary],
    data: &AppState,
    count_commissions: bool,
) -> Element {
    if let Some(date) = hovered {
        if let Some(d) = day_by_date.get(&date) {
            return render_day_detail(date, d, matched_by_date.get(&date), data, count_commissions);
        }
        // Hovered an empty cell — show "no trading" message
        return rsx! {
            aside { class: "cal-side-panel",
                div { class: "cal-side-title", "{date.format(\"%a, %b %-d %Y\")}" }
                div { class: "cal-side-empty", "No trading activity" }
            }
        };
    }

    render_range_summary(days, data, count_commissions)
}

fn render_day_detail(
    date: NaiveDate,
    d: &DailySummary,
    matched_today: Option<&Vec<&MatchedTrade>>,
    data: &AppState,
    count_commissions: bool,
) -> Element {
    // R-multiple for the day using its week's R config
    let monday = date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64);
    let r_val = data.r_value_for_week(monday);
    let day_pnl = data.daily_pnl(d, count_commissions);
    let pnl_r = data.pnl_in_r(day_pnl, r_val);

    // R-based W/LL/L from matched trades (matches rest of app)
    let (mut wins, mut lossless, mut losses) = (0_u32, 0_u32, 0_u32);
    let mut best_trade: Option<&MatchedTrade> = None;
    let mut worst_trade: Option<&MatchedTrade> = None;
    if let Some(trades) = matched_today {
        for mt in trades {
            match data.trade_outcome_with(mt, count_commissions) {
                TradeOutcome::Winner => wins += 1,
                TradeOutcome::Lossless => lossless += 1,
                TradeOutcome::Loser => losses += 1,
            }
            if best_trade.is_none_or(|b| data.trade_pnl(mt, count_commissions) > data.trade_pnl(b, count_commissions)) {
                best_trade = Some(mt);
            }
            if worst_trade.is_none_or(|w| data.trade_pnl(mt, count_commissions) < data.trade_pnl(w, count_commissions)) {
                worst_trade = Some(mt);
            }
        }
    }
    let wr = if wins + losses > 0 {
        (wins as f64 / (wins + losses) as f64) * 100.0
    } else {
        0.0
    };

    let symbols_top: Vec<String> = d.symbols_traded.iter().take(5).cloned().collect();
    let symbols_str = if symbols_top.is_empty() {
        "—".to_string()
    } else if d.symbols_traded.len() > 5 {
        format!("{} (+{} more)", symbols_top.join(", "), d.symbols_traded.len() - 5)
    } else {
        symbols_top.join(", ")
    };

    let is_pos = day_pnl >= Decimal::ZERO;

    rsx! {
        aside { class: "cal-side-panel",
            div { class: "cal-side-title",
                "{date.format(\"%a, %b %-d %Y\")}"
            }

            div { class: "cal-side-pnl-row",
                div {
                    class: if is_pos { "cal-side-pnl positive" } else { "cal-side-pnl negative" },
                    "{format_pnl(day_pnl)}"
                }
                div {
                    class: if pnl_r >= Decimal::ZERO { "cal-side-r positive" } else { "cal-side-r negative" },
                    "{format_r(pnl_r)}"
                }
            }

            div { class: "cal-side-grid",
                div { class: "csg-row",
                    span { class: "csg-label", "Trades" }
                    span { class: "csg-val", "{d.total_trades}" }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "W / LL / L" }
                    span { class: "csg-val",
                        span { class: "positive", "{wins}" }
                        " / "
                        span { class: "neutral", "{lossless}" }
                        " / "
                        span { class: "negative", "{losses}" }
                    }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Win Rate" }
                    span {
                        class: if wr >= 50.0 { "csg-val positive" } else { "csg-val negative" },
                        "{wr:.1}%"
                    }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Gross P&L" }
                    span { class: "csg-val", "{format_pnl(d.gross_pnl)}" }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Commission" }
                    span { class: "csg-val negative", "{format_decimal(d.total_commission)}" }
                }
                if d.avg_win != Decimal::ZERO || d.avg_loss != Decimal::ZERO {
                    div { class: "csg-row",
                        span { class: "csg-label", "Avg W / Avg L" }
                        span { class: "csg-val",
                            span { class: "positive", "{format_pnl(d.avg_win)}" }
                            " / "
                            span { class: "negative", "{format_pnl(d.avg_loss)}" }
                        }
                    }
                }
                if d.largest_win != Decimal::ZERO || d.largest_loss != Decimal::ZERO {
                    div { class: "csg-row",
                        span { class: "csg-label", "Best / Worst" }
                        span { class: "csg-val",
                            span { class: "positive", "{format_pnl(d.largest_win)}" }
                            " / "
                            span { class: "negative", "{format_pnl(d.largest_loss)}" }
                        }
                    }
                }
                if let Some(bt) = best_trade {
                    {
                        let bt_pnl = data.trade_pnl(bt, count_commissions);
                        rsx! {
                            div { class: "csg-row",
                                span { class: "csg-label", "Best Trade" }
                                span { class: "csg-val",
                                    span { class: "positive", "{bt.symbol} {format_pnl(bt_pnl)}" }
                                }
                            }
                        }
                    }
                }
                if let Some(wt) = worst_trade {
                    {
                        let wt_pnl = data.trade_pnl(wt, count_commissions);
                        if wt_pnl < Decimal::ZERO {
                            rsx! {
                                div { class: "csg-row",
                                    span { class: "csg-label", "Worst Trade" }
                                    span { class: "csg-val",
                                        span { class: "negative", "{wt.symbol} {format_pnl(wt_pnl)}" }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }

            div { class: "cal-side-symbols",
                span { class: "csg-label", "Symbols: " }
                span { class: "csg-val", "{symbols_str}" }
            }
        }
    }
}

fn render_range_summary(days: &[DailySummary], data: &AppState, count_commissions: bool) -> Element {
    if days.is_empty() {
        return rsx! { aside { class: "cal-side-panel", div { class: "cal-side-empty", "No data" } } };
    }

    // Best & worst day in range
    let best = days.iter().max_by_key(|d| data.daily_pnl(d, count_commissions)).unwrap();
    let worst = days.iter().min_by_key(|d| data.daily_pnl(d, count_commissions)).unwrap();
    let best_pnl = data.daily_pnl(best, count_commissions);
    let worst_pnl = data.daily_pnl(worst, count_commissions);

    let total_pnl: Decimal = days.iter().map(|d| data.daily_pnl(d, count_commissions)).sum();
    let total_trades: u32 = days.iter().map(|d| d.total_trades).sum();
    let avg_pnl = total_pnl / Decimal::from(days.len() as u32);
    let avg_trades = total_trades as f64 / days.len() as f64;

    // Hot streak: longest consecutive green run; cold streak: longest red run
    let (mut max_green, mut max_red) = (0_u32, 0_u32);
    let (mut cur_green, mut cur_red) = (0_u32, 0_u32);
    for d in days {
        let p = data.daily_pnl(d, count_commissions);
        if p > Decimal::ZERO {
            cur_green += 1; cur_red = 0;
            if cur_green > max_green { max_green = cur_green; }
        } else if p < Decimal::ZERO {
            cur_red += 1; cur_green = 0;
            if cur_red > max_red { max_red = cur_red; }
        } else {
            cur_green = 0; cur_red = 0;
        }
    }
    // Current streak (positive number = green run, negative = red run, 0 = flat)
    let current = if cur_green > 0 { cur_green as i32 } else { -(cur_red as i32) };

    // Best & worst weekday by avg P&L
    let mut wd_sum: [Decimal; 7] = [Decimal::ZERO; 7];
    let mut wd_count: [u32; 7] = [0; 7];
    for d in days {
        let wd = d.date.weekday().num_days_from_sunday() as usize;
        wd_sum[wd] += data.daily_pnl(d, count_commissions);
        wd_count[wd] += 1;
    }
    let wd_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mut best_wd = (None::<usize>, f64::MIN);
    let mut worst_wd = (None::<usize>, f64::MAX);
    for i in 0..7 {
        if wd_count[i] == 0 { continue; }
        let avg = wd_sum[i].to_f64().unwrap_or(0.0) / wd_count[i] as f64;
        if avg > best_wd.1 { best_wd = (Some(i), avg); }
        if avg < worst_wd.1 { worst_wd = (Some(i), avg); }
    }

    let avg_is_pos = avg_pnl >= Decimal::ZERO;

    rsx! {
        aside { class: "cal-side-panel",
            div { class: "cal-side-title", "Range Summary" }
            div { class: "cal-side-hint", "Hover a cell for day details" }

            div { class: "cal-side-grid",
                div { class: "csg-row",
                    span { class: "csg-label", "Best Day" }
                    span { class: "csg-val",
                        span { class: "positive", "{format_pnl(best_pnl)}" }
                        span { class: "csg-sub", " on {best.date.format(\"%a %m/%d\")}" }
                    }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Worst Day" }
                    span { class: "csg-val",
                        span { class: "negative", "{format_pnl(worst_pnl)}" }
                        span { class: "csg-sub", " on {worst.date.format(\"%a %m/%d\")}" }
                    }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Avg / Day" }
                    span {
                        class: if avg_is_pos { "csg-val positive" } else { "csg-val negative" },
                        "{format_pnl(avg_pnl)}"
                    }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Avg Trades / Day" }
                    span { class: "csg-val", "{avg_trades:.1}" }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Longest Hot Streak" }
                    span { class: "csg-val positive", "{max_green} days" }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Longest Cold Streak" }
                    span { class: "csg-val negative", "{max_red} days" }
                }
                div { class: "csg-row",
                    span { class: "csg-label", "Current Streak" }
                    span {
                        class: if current >= 0 { "csg-val positive" } else { "csg-val negative" },
                        if current >= 0 { "{current} green" } else { "{current.abs()} red" }
                    }
                }
                if let Some(i) = best_wd.0 {
                    div { class: "csg-row",
                        span { class: "csg-label", "Best Weekday" }
                        span { class: "csg-val positive",
                            "{wd_names[i]} ({format_pnl(Decimal::from_f64_retain(best_wd.1).unwrap_or(Decimal::ZERO))}/day)"
                        }
                    }
                }
                if let Some(i) = worst_wd.0 {
                    div { class: "csg-row",
                        span { class: "csg-label", "Worst Weekday" }
                        span { class: "csg-val negative",
                            "{wd_names[i]} ({format_pnl(Decimal::from_f64_retain(worst_wd.1).unwrap_or(Decimal::ZERO))}/day)"
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Tab 4: Behavior
// ============================================================================

fn render_behavior_tab(days: &[DailySummary], matched: &[MatchedTrade], data: &AppState, count_commissions: bool) -> Element {
    // Trades-per-day series (one entry per visible trading day)
    let trades_per_day: Vec<(NaiveDate, u32, Decimal)> = days
        .iter()
        .map(|d| (d.date.date_naive(), d.total_trades, data.daily_pnl(d, count_commissions)))
        .collect();

    // Median trade count
    let mut counts: Vec<u32> = trades_per_day.iter().map(|(_, c, _)| *c).collect();
    counts.sort_unstable();
    let median = if counts.is_empty() {
        0
    } else if counts.len() % 2 == 1 {
        counts[counts.len() / 2]
    } else {
        (counts[counts.len() / 2 - 1] + counts[counts.len() / 2]) / 2
    };
    let overtrade_threshold = (median as f64 * 1.5).ceil() as u32;

    let max_count = counts.last().copied().unwrap_or(1).max(1);
    let avg_count = if counts.is_empty() { 0.0 } else {
        counts.iter().sum::<u32>() as f64 / counts.len() as f64
    };
    let overtrade_days = trades_per_day.iter()
        .filter(|(_, c, p)| *c > overtrade_threshold && *p < Decimal::ZERO)
        .count();
    let overtrade_pnl: Decimal = trades_per_day.iter()
        .filter(|(_, c, p)| *c > overtrade_threshold && *p < Decimal::ZERO)
        .map(|(_, _, p)| *p)
        .sum();

    // Hour × weekday matrix from matched trades (P&L per cell)
    let mut hour_wd_pnl: HashMap<(u32, u32), (Decimal, u32)> = HashMap::new(); // (weekday, hour) -> (pnl, count)
    for mt in matched {
        let wd = mt.exit_time.date_naive().weekday().num_days_from_sunday();
        let hour = mt.exit_time.format("%H").to_string().parse::<u32>().unwrap_or(0);
        let entry = hour_wd_pnl.entry((wd, hour)).or_insert((Decimal::ZERO, 0));
        entry.0 += data.trade_pnl(mt, count_commissions);
        entry.1 += 1;
    }
    let hour_max_abs = hour_wd_pnl.values().fold(Decimal::ZERO, |acc, (p, _)| {
        let a = p.abs();
        if a > acc { a } else { acc }
    });
    let hour_max_abs_f = hour_max_abs.to_f64().unwrap_or(1.0).max(1.0);

    rsx! {
        div { class: "trend-stats-row",
            div { class: "trend-stat",
                span { class: "stat-label", "Median Trades/Day" }
                span { class: "stat-value", "{median}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Avg Trades/Day" }
                span { class: "stat-value", "{avg_count:.1}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Overtrade Threshold" }
                span { class: "stat-value", ">{overtrade_threshold} (1.5x median)" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Overtrade-Loss Days" }
                span { class: "stat-value negative", "{overtrade_days}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Cost of Overtrading" }
                span { class: "stat-value negative", "{format_pnl(overtrade_pnl)}" }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label",
                "Trades per Day"
                span { class: "chart-legend-text", " (red = overtraded AND lost)" }
            }
            div { class: "behavior-chart",
                for (date, count, pnl) in trades_per_day.iter() {
                    {
                        let h_pct = (*count as f64 / max_count as f64 * 100.0).max(2.0);
                        let is_overtrade_loss = *count > overtrade_threshold && *pnl < Decimal::ZERO;
                        let cls = if is_overtrade_loss {
                            "behavior-bar overtrade"
                        } else if *count > overtrade_threshold {
                            "behavior-bar warn"
                        } else if *pnl >= Decimal::ZERO {
                            "behavior-bar normal-pos"
                        } else {
                            "behavior-bar normal-neg"
                        };
                        let pnl_f = pnl.to_f64().unwrap_or(0.0);
                        let title = format!("{}: {} trades, ${:.2}", date.format("%a %m/%d"), count, pnl_f);
                        rsx! {
                            div { class: "behavior-col",
                                div {
                                    class: "{cls}",
                                    style: "height: {h_pct}%;",
                                    title: "{title}",
                                }
                            }
                        }
                    }
                }
            }
            // Median line marker
            div { class: "behavior-axis",
                span { class: "axis-marker", "median: {median}" }
                span { class: "axis-marker", "max: {max_count}" }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Hour × Weekday P&L Heatmap (trade-exit time, UTC)" }
            div { class: "hour-heatmap",
                // Header row with hours
                div { class: "hh-row hh-header",
                    div { class: "hh-label", " " }
                    for h in 0..24u32 {
                        div { class: "hh-cell hh-hour-label", "{h}" }
                    }
                }
                {
                    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                    rsx! {
                        for wd in 1..6u32 {
                            div { class: "hh-row",
                                div { class: "hh-label", "{names[wd as usize]}" }
                                for h in 0..24u32 {
                                    {
                                        let cell = hour_wd_pnl.get(&(wd, h));
                                        let (cls, title) = if let Some((pnl, count)) = cell {
                                            let pnl_f = pnl.to_f64().unwrap_or(0.0);
                                            let intensity = (pnl_f.abs() / hour_max_abs_f).min(1.0);
                                            let level = ((intensity * 4.0).ceil() as i32).clamp(1, 4);
                                            let sign = if pnl_f > 0.0 { "pos" } else if pnl_f < 0.0 { "neg" } else { "flat" };
                                            let cls = format!("hh-cell {} l{}", sign, level);
                                            let title = format!("{} {}h: ${:.2} ({} trades)", names[wd as usize], h, pnl_f, count);
                                            (cls, title)
                                        } else {
                                            ("hh-cell empty".to_string(), String::new())
                                        };
                                        rsx! { div { class: "{cls}", title: "{title}" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Tab 5: Streaks & Consistency
// ============================================================================

fn render_streaks_tab(days: &[DailySummary], matched: &[MatchedTrade], data: &AppState, count_commissions: bool) -> Element {
    if matched.is_empty() {
        return rsx! { div { class: "trends-empty", "No matched trades in range." } };
    }

    // Trade-by-trade outcomes (chronological)
    let outcomes: Vec<TradeOutcome> = matched.iter().map(|mt| data.trade_outcome_with(mt, count_commissions)).collect();

    // Compute current and longest loss/win streaks
    let mut max_win = 0_u32;
    let mut max_loss = 0_u32;
    let mut cur_w = 0_u32;
    let mut cur_l = 0_u32;
    let mut rolling_max_loss: Vec<u32> = Vec::with_capacity(outcomes.len());
    for o in &outcomes {
        match o {
            TradeOutcome::Winner => { cur_w += 1; cur_l = 0; }
            TradeOutcome::Loser => { cur_l += 1; cur_w = 0; }
            TradeOutcome::Lossless => {}
        }
        if cur_w > max_win { max_win = cur_w; }
        if cur_l > max_loss { max_loss = cur_l; }
        rolling_max_loss.push(max_loss);
    }
    let final_streak = if cur_w > 0 { cur_w as i32 } else { -(cur_l as i32) };

    // Weekly P&L series (Monday-anchored)
    let mut weekly: HashMap<NaiveDate, Decimal> = HashMap::new();
    for d in days {
        let dt = d.date.date_naive();
        let mon = dt - chrono::Duration::days(dt.weekday().num_days_from_monday() as i64);
        *weekly.entry(mon).or_insert(Decimal::ZERO) += data.daily_pnl(d, count_commissions);
    }
    let mut weekly_sorted: Vec<(NaiveDate, Decimal)> = weekly.into_iter().collect();
    weekly_sorted.sort_by_key(|(d, _)| *d);

    let weeks_n = weekly_sorted.len();
    let weeks_profitable = weekly_sorted.iter().filter(|(_, p)| *p > Decimal::ZERO).count();
    let pct_profitable = if weeks_n > 0 { (weeks_profitable as f64 / weeks_n as f64) * 100.0 } else { 0.0 };

    // Last 12 weeks profitability rate
    let last_12: Vec<&(NaiveDate, Decimal)> = weekly_sorted.iter().rev().take(12).collect();
    let l12_n = last_12.len();
    let l12_profitable = last_12.iter().filter(|(_, p)| *p > Decimal::ZERO).count();
    let l12_pct = if l12_n > 0 { (l12_profitable as f64 / l12_n as f64) * 100.0 } else { 0.0 };

    // Std-dev of weekly P&L (rolling 8-week)
    let weekly_pnls: Vec<f64> = weekly_sorted.iter().map(|(_, p)| p.to_f64().unwrap_or(0.0)).collect();
    let recent_8 = if weekly_pnls.len() >= 8 { &weekly_pnls[weekly_pnls.len() - 8..] } else { &weekly_pnls[..] };
    let recent_mean = if !recent_8.is_empty() { recent_8.iter().sum::<f64>() / recent_8.len() as f64 } else { 0.0 };
    let recent_sd = if recent_8.len() > 1 {
        let v = recent_8.iter().map(|x| (x - recent_mean).powi(2)).sum::<f64>() / (recent_8.len() - 1) as f64;
        v.sqrt()
    } else { 0.0 };

    // Trade strip render data
    let svg_w = 1000.0_f64;
    let strip_h = 80.0_f64;
    let bar_w = (svg_w / outcomes.len() as f64).max(0.5);

    // Rolling longest loss streak overlay (line on top)
    let max_rolling = *rolling_max_loss.iter().max().unwrap_or(&1) as f64;
    let rl_to_y = |v: u32| -> f64 { strip_h - (v as f64 / max_rolling.max(1.0)) * (strip_h * 0.85) };
    let rl_path: String = rolling_max_loss.iter().enumerate()
        .map(|(i, v)| format!("{:.1},{:.1}", i as f64 * bar_w + bar_w / 2.0, rl_to_y(*v)))
        .collect::<Vec<_>>().join(" ");

    rsx! {
        div { class: "trend-stats-row",
            div { class: "trend-stat",
                span { class: "stat-label", "Current Streak" }
                span {
                    class: if final_streak >= 0 { "stat-value positive" } else { "stat-value negative" },
                    if final_streak >= 0 { "{final_streak} W" } else { "{final_streak.abs()} L" }
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Longest Win Streak" }
                span { class: "stat-value positive", "{max_win}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Longest Loss Streak" }
                span { class: "stat-value negative", "{max_loss}" }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "% Weeks Profitable" }
                span {
                    class: if pct_profitable >= 50.0 { "stat-value positive" } else { "stat-value negative" },
                    "{pct_profitable:.0}%"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Last 12 Weeks" }
                span {
                    class: if l12_pct >= 50.0 { "stat-value positive" } else { "stat-value negative" },
                    "{l12_pct:.0}% ({l12_profitable}/{l12_n})"
                }
            }
            div { class: "trend-stat",
                span { class: "stat-label", "Weekly Std-Dev (8w)" }
                span { class: "stat-value", "${recent_sd:.0}" }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label",
                "Trade-by-Trade Outcomes"
                span { class: "chart-legend-text", " (chronological — green=W, red=L, gray=lossless; line=rolling longest loss streak)" }
            }
            svg {
                class: "trend-svg",
                width: "100%",
                height: "{strip_h}",
                view_box: "0 0 {svg_w} {strip_h}",
                preserve_aspect_ratio: "none",
                for (i, o) in outcomes.iter().enumerate() {
                    {
                        let x = i as f64 * bar_w;
                        let (color, h) = match o {
                            TradeOutcome::Winner => ("var(--accent-green)", strip_h * 0.85),
                            TradeOutcome::Loser => ("var(--accent-red)", strip_h * 0.85),
                            TradeOutcome::Lossless => ("var(--text-muted)", strip_h * 0.4),
                        };
                        let y = strip_h - h;
                        rsx! {
                            rect {
                                x: "{x:.2}",
                                y: "{y:.1}",
                                width: "{bar_w:.2}",
                                height: "{h:.1}",
                                fill: "{color}",
                                opacity: "0.85",
                            }
                        }
                    }
                }
                polyline {
                    points: "{rl_path}",
                    fill: "none",
                    stroke: "var(--accent-yellow)",
                    stroke_width: "2",
                    opacity: "0.9",
                }
            }
        }

        div { class: "chart-block",
            div { class: "chart-label", "Weekly P&L Bars" }
            div { class: "weekly-bars",
                {
                    let max_abs = weekly_sorted.iter().fold(0.0_f64, |acc, (_, p)| {
                        let a = p.abs().to_f64().unwrap_or(0.0);
                        if a > acc { a } else { acc }
                    }).max(1.0);
                    rsx! {
                        for (mon, pnl) in weekly_sorted.iter() {
                            {
                                let pnl_f = pnl.to_f64().unwrap_or(0.0);
                                let h_pct = (pnl_f.abs() / max_abs * 100.0).max(2.0);
                                let is_pos = pnl_f >= 0.0;
                                let cls = if is_pos { "wkbar positive" } else { "wkbar negative" };
                                let title = format!("Week of {}: ${:.2}", mon, pnl_f);
                                rsx! {
                                    div { class: "wkcol",
                                        div { class: "{cls}", style: "height: {h_pct}%;", title: "{title}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn rolling_mean(values: &[f64], window: usize) -> Vec<Option<f64>> {
    let mut out = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        if i + 1 < window {
            out.push(None);
            continue;
        }
        let slice = &values[i + 1 - window..=i];
        let s: f64 = slice.iter().sum();
        out.push(Some(s / window as f64));
    }
    out
}

fn rolling_win_rate(outcomes: &[TradeOutcome], window: usize) -> Vec<Option<f64>> {
    let mut out = Vec::with_capacity(outcomes.len());
    for i in 0..outcomes.len() {
        if i + 1 < window {
            out.push(None);
            continue;
        }
        let slice = &outcomes[i + 1 - window..=i];
        let wins = slice.iter().filter(|o| matches!(o, TradeOutcome::Winner)).count() as f64;
        let losses = slice.iter().filter(|o| matches!(o, TradeOutcome::Loser)).count() as f64;
        let denom = wins + losses;
        if denom > 0.0 {
            out.push(Some((wins / denom) * 100.0));
        } else {
            out.push(None);
        }
    }
    out
}

fn rolling_profit_factor(matched: &[MatchedTrade], window: usize, count_commissions: bool, data: &AppState) -> Vec<Option<f64>> {
    let mut out = Vec::with_capacity(matched.len());
    for i in 0..matched.len() {
        if i + 1 < window {
            out.push(None);
            continue;
        }
        let slice = &matched[i + 1 - window..=i];
        let mut wins = 0.0_f64;
        let mut losses = 0.0_f64;
        for mt in slice {
            let p = data.trade_pnl(mt, count_commissions).to_f64().unwrap_or(0.0);
            if p > 0.0 { wins += p; } else { losses += -p; }
        }
        if losses > 0.0 {
            out.push(Some(wins / losses));
        } else if wins > 0.0 {
            out.push(Some(5.0)); // capped infinity
        } else {
            out.push(None);
        }
    }
    out
}

fn polyline_from_optional<F: Fn(f64) -> f64>(values: &[Option<f64>], dx: f64, to_y: F) -> String {
    values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|val| format!("{:.1},{:.1}", i as f64 * dx, to_y(val))))
        .collect::<Vec<_>>()
        .join(" ")
}
