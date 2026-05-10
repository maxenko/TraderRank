// Week page — per-week detailed trade review.
//
// Layout: vertical week-selector strip (left) + main content area (right).
// The content area shows for the selected week:
//   - Header KPI card (net P&L, win rate, trades, W/LL/L, profit factor,
//     trading days, biggest win, biggest loss, commission) with inline
//     week-over-week deltas.
//   - Per-day Mon-Fri mini-heatmap.
//   - Overtrading insight panel (trades/day vs prior 4-week median).
//   - Per-trade detail table with hold time + R-multiple + outcome chip.
//   - Hour x weekday P&L heatmap (selected week only).
//
// All P&L values pass through `data.trade_pnl` / `data.daily_pnl` so the
// "count commissions" toggle from settings is honored. Excluded trades and
// excluded days are filtered out at aggregation time.

use chrono::{Datelike, NaiveDate, Timelike};
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use crate::components::*;
use crate::models::MatchedTrade;
use crate::settings_store;
use crate::state::{AppState, TradeOutcome};

/// One trader-week, populated once and reused across the page.
struct WeekBucket {
    monday: NaiveDate,
    sunday: NaiveDate,
    matched: Vec<MatchedTrade>,
    net_pnl: Decimal,
    r_sum: Decimal,
    wins: u32,
    lossless: u32,
    losers: u32,
    trade_days: u32,
    commission: Decimal,
}

impl WeekBucket {
    fn total_trades(&self) -> u32 {
        self.matched.len() as u32
    }

    fn win_rate(&self) -> f64 {
        let denom = self.wins + self.losers;
        if denom == 0 {
            0.0
        } else {
            (self.wins as f64 / denom as f64) * 100.0
        }
    }

    /// Profit factor = sum(wins) / abs(sum(losses)). None when no losses.
    fn profit_factor(&self, data: &AppState, count_commissions: bool) -> Option<f64> {
        let mut wins = 0.0_f64;
        let mut losses = 0.0_f64;
        for mt in &self.matched {
            let p = data.trade_pnl(mt, count_commissions).to_f64().unwrap_or(0.0);
            if p > 0.0 {
                wins += p;
            } else {
                losses += -p;
            }
        }
        if losses > 0.0 {
            Some(wins / losses)
        } else if wins > 0.0 {
            Some(f64::INFINITY)
        } else {
            None
        }
    }
}

#[component]
pub fn Week() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let stats_config = use_context::<Signal<crate::state::StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;

    // Build all week buckets up front (newest first)
    let weeks = build_week_buckets(&data, count_commissions);

    // Most-recent week with trades — fallback when no week is persisted/selected
    let default_monday: Option<NaiveDate> = weeks.first().map(|w| w.monday);

    // Load persisted selection; fall back to most-recent week
    let saved_str = settings_store::load_raw()
        .map(|s| s.selected_week)
        .unwrap_or_default();
    let initial_monday: Option<NaiveDate> = if saved_str.is_empty() {
        default_monday
    } else {
        saved_str.parse::<NaiveDate>().ok().or(default_monday)
    };

    let mut selected_monday = use_signal(|| initial_monday);
    let mut show_older = use_signal(|| false);

    let cur_monday = *selected_monday.read();
    let older = *show_older.read();

    // Find the selected bucket (or build an empty one for the chosen Monday)
    let selected: WeekBucket = match cur_monday {
        Some(mon) => weeks
            .iter()
            .find(|w| w.monday == mon)
            .map(clone_bucket)
            .unwrap_or_else(|| empty_bucket(mon)),
        None => empty_bucket(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
    };

    // Previous week (prior Monday) — used for inline deltas
    let prev_week: Option<&WeekBucket> = cur_monday.and_then(|mon| {
        let prev = mon - chrono::Duration::days(7);
        weeks.iter().find(|w| w.monday == prev)
    });

    // Visible week-selector cells
    let take_n = if older { 52 } else { 26 };
    let visible_weeks: Vec<&WeekBucket> = weeks.iter().take(take_n).collect();
    let max_abs_net = visible_weeks
        .iter()
        .fold(Decimal::ZERO, |acc, w| {
            let a = w.net_pnl.abs();
            if a > acc {
                a
            } else {
                acc
            }
        });
    let max_abs_net_f = max_abs_net.to_f64().unwrap_or(1.0).max(1.0);

    rsx! {
        div { class: "view week-view",
            // Left: week selector (sticky vertical strip)
            aside { class: "week-selector",
                div { class: "week-selector-title", "Weeks" }
                {
                    let cur = cur_monday;
                    rsx! {
                        for b in visible_weeks.iter() {
                            {
                                let monday = b.monday;
                                let net_pnl = b.net_pnl;
                                let is_selected = cur == Some(monday);
                                let pnl_f = net_pnl.to_f64().unwrap_or(0.0);
                                let intensity = (pnl_f.abs() / max_abs_net_f).min(1.0);
                                let level = ((intensity * 4.0).ceil() as i32).clamp(1, 4);
                                let sign = if pnl_f > 0.0 {
                                    "pos"
                                } else if pnl_f < 0.0 {
                                    "neg"
                                } else {
                                    "flat"
                                };
                                let cell_cls = if is_selected {
                                    "week-cell selected"
                                } else {
                                    "week-cell"
                                };
                                let chip_cls = format!("wc-chip l{} {}", level, sign);
                                let date_label = monday.format("%b %-d").to_string();
                                let pnl_label = format_pnl(net_pnl);
                                rsx! {
                                    button {
                                        class: "{cell_cls}",
                                        onclick: move |_| {
                                            selected_monday.set(Some(monday));
                                            let s = monday.to_string();
                                            settings_store::update(|st| st.selected_week = s);
                                        },
                                        span { class: "wc-date", "{date_label}" }
                                        span { class: "{chip_cls}", "{pnl_label}" }
                                    }
                                }
                            }
                        }
                    }
                }
                if weeks.len() > 26 {
                    button {
                        class: "week-selector-toggle",
                        onclick: move |_| {
                            let cur = *show_older.read();
                            show_older.set(!cur);
                        },
                        if older { "Show fewer" } else { "Show older" }
                    }
                }
            }

            // Right: content
            main { class: "week-content",
                if selected.matched.is_empty() {
                    {render_empty_state(&selected)}
                } else {
                    {render_header(&selected, prev_week, &data, count_commissions)}
                    div { class: "week-mid-row",
                        {render_day_strip(&selected, &data, count_commissions)}
                        {render_overtrade_panel(&selected, &weeks, &data, count_commissions)}
                    }
                    {render_trade_table(&selected, &data, count_commissions)}
                    {render_hour_heatmap(&selected, &data, count_commissions)}
                }
            }
        }
    }
}

// ============================================================================
// Aggregation
// ============================================================================

fn build_week_buckets(data: &AppState, count_commissions: bool) -> Vec<WeekBucket> {
    let mut by_monday: HashMap<NaiveDate, Vec<MatchedTrade>> = HashMap::new();
    for mt in &data.matched_trades {
        if data.is_trade_excluded(mt) {
            continue;
        }
        let d = mt.exit_time.date_naive();
        let monday = d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64);
        by_monday.entry(monday).or_default().push(mt.clone());
    }

    let mut out: Vec<WeekBucket> = by_monday
        .into_iter()
        .map(|(monday, matched)| {
            let sunday = monday + chrono::Duration::days(6);
            let net_pnl: Decimal = matched
                .iter()
                .map(|mt| data.trade_pnl(mt, count_commissions))
                .sum();
            let r_val = data.r_value_for_week(monday);
            let r_sum = data.pnl_in_r(net_pnl, r_val);
            let mut wins = 0_u32;
            let mut lossless = 0_u32;
            let mut losers = 0_u32;
            for mt in &matched {
                match data.trade_outcome_with(mt, count_commissions) {
                    TradeOutcome::Winner => wins += 1,
                    TradeOutcome::Lossless => lossless += 1,
                    TradeOutcome::Loser => losers += 1,
                }
            }
            let mut day_set: HashSet<NaiveDate> = HashSet::new();
            for mt in &matched {
                day_set.insert(mt.exit_time.date_naive());
            }
            let commission: Decimal = matched.iter().map(|mt| mt.commission).sum();
            WeekBucket {
                monday,
                sunday,
                matched,
                net_pnl,
                r_sum,
                wins,
                lossless,
                losers,
                trade_days: day_set.len() as u32,
                commission,
            }
        })
        .collect();
    out.sort_by_key(|b| std::cmp::Reverse(b.monday));
    out
}

fn empty_bucket(monday: NaiveDate) -> WeekBucket {
    WeekBucket {
        monday,
        sunday: monday + chrono::Duration::days(6),
        matched: Vec::new(),
        net_pnl: Decimal::ZERO,
        r_sum: Decimal::ZERO,
        wins: 0,
        lossless: 0,
        losers: 0,
        trade_days: 0,
        commission: Decimal::ZERO,
    }
}

fn clone_bucket(b: &WeekBucket) -> WeekBucket {
    WeekBucket {
        monday: b.monday,
        sunday: b.sunday,
        matched: b.matched.clone(),
        net_pnl: b.net_pnl,
        r_sum: b.r_sum,
        wins: b.wins,
        lossless: b.lossless,
        losers: b.losers,
        trade_days: b.trade_days,
        commission: b.commission,
    }
}

// ============================================================================
// Header KPI card
// ============================================================================

fn render_header(
    sel: &WeekBucket,
    prev: Option<&WeekBucket>,
    data: &AppState,
    count_commissions: bool,
) -> Element {
    let net_pnl = sel.net_pnl;
    let r_sum = sel.r_sum;
    let net_str = format!("{} / {}", format_r(r_sum), format_pnl(net_pnl));

    let win_rate = sel.win_rate();
    let trades_n = sel.total_trades();
    let pf = sel.profit_factor(data, count_commissions);
    let pf_str = match pf {
        Some(v) if v.is_infinite() => "\u{221E}".to_string(),
        Some(v) => format!("{:.2}", v),
        None => "—".to_string(),
    };

    // Best / worst trade by P&L
    let mut best: Option<&MatchedTrade> = None;
    let mut worst: Option<&MatchedTrade> = None;
    for mt in &sel.matched {
        let p = data.trade_pnl(mt, count_commissions);
        if best.is_none_or(|b| p > data.trade_pnl(b, count_commissions)) {
            best = Some(mt);
        }
        if worst.is_none_or(|w| p < data.trade_pnl(w, count_commissions)) {
            worst = Some(mt);
        }
    }
    let (best_str, best_pos) = best
        .map(|mt| {
            let p = data.trade_pnl(mt, count_commissions);
            (format!("{} {}", mt.symbol, format_pnl(p)), p >= Decimal::ZERO)
        })
        .unwrap_or_else(|| ("—".to_string(), true));
    let (worst_str, worst_pos) = worst
        .map(|mt| {
            let p = data.trade_pnl(mt, count_commissions);
            (format!("{} {}", mt.symbol, format_pnl(p)), p >= Decimal::ZERO)
        })
        .unwrap_or_else(|| ("—".to_string(), true));

    // Inline deltas vs previous week
    let pnl_delta = fmt_delta_pnl(net_pnl, prev.map(|p| p.net_pnl));
    let wr_delta = fmt_delta_pp(win_rate, prev.map(|p| p.win_rate()));
    let trades_delta = fmt_delta_int(trades_n as i64, prev.map(|p| p.total_trades() as i64));
    let pf_delta = fmt_delta_f64(
        pf.and_then(|v| if v.is_finite() { Some(v) } else { None }),
        prev.and_then(|p| p.profit_factor(data, count_commissions))
            .and_then(|v| if v.is_finite() { Some(v) } else { None }),
        2,
    );
    let days_delta = fmt_delta_int(sel.trade_days as i64, prev.map(|p| p.trade_days as i64));
    let commission_delta = fmt_delta_pnl(sel.commission, prev.map(|p| p.commission));

    let title = format!(
        "Week of {} – {}",
        sel.monday.format("%b %-d"),
        sel.sunday.format("%b %-d, %Y"),
    );

    rsx! {
        div { class: "card week-header-card",
            h2 { class: "week-title", "{title}" }
            div { class: "kpi-grid week-kpi-grid",
                MetricCard {
                    label: "Net P&L".to_string(),
                    value: net_str,
                    subtitle: pnl_delta,
                    positive: Some(net_pnl >= Decimal::ZERO),
                }
                MetricCard {
                    label: "Win Rate".to_string(),
                    value: format!("{:.1}%", win_rate),
                    subtitle: wr_delta,
                    positive: Some(win_rate >= 50.0),
                }
                MetricCard {
                    label: "Trades".to_string(),
                    value: trades_n.to_string(),
                    subtitle: trades_delta,
                    positive: None,
                }
                MetricCard {
                    label: "W / LL / L".to_string(),
                    value: format!("{} / {} / {}", sel.wins, sel.lossless, sel.losers),
                    subtitle: None,
                    positive: None,
                }
                MetricCard {
                    label: "Profit Factor".to_string(),
                    value: pf_str,
                    subtitle: pf_delta,
                    positive: pf.map(|v| v >= 1.0),
                }
                MetricCard {
                    label: "Trading Days".to_string(),
                    value: sel.trade_days.to_string(),
                    subtitle: days_delta,
                    positive: None,
                }
                MetricCard {
                    label: "Biggest Win".to_string(),
                    value: best_str,
                    subtitle: None,
                    positive: Some(best_pos),
                }
                MetricCard {
                    label: "Biggest Loss".to_string(),
                    value: worst_str,
                    subtitle: None,
                    positive: Some(worst_pos),
                }
                MetricCard {
                    label: "Commission".to_string(),
                    value: format_decimal(sel.commission),
                    subtitle: commission_delta,
                    positive: Some(false),
                }
            }
        }
    }
}

// ============================================================================
// Per-day mini-heatmap (Mon-Fri strip)
// ============================================================================

fn render_day_strip(sel: &WeekBucket, data: &AppState, count_commissions: bool) -> Element {
    let names = ["Mon", "Tue", "Wed", "Thu", "Fri"];
    // (date, pnl, count) for each weekday Mon..Fri
    let mut days: Vec<(NaiveDate, Decimal, u32)> = Vec::with_capacity(5);
    for offset in 0..5 {
        let d = sel.monday + chrono::Duration::days(offset);
        let pnl: Decimal = sel
            .matched
            .iter()
            .filter(|mt| mt.exit_time.date_naive() == d)
            .map(|mt| data.trade_pnl(mt, count_commissions))
            .sum();
        let count = sel
            .matched
            .iter()
            .filter(|mt| mt.exit_time.date_naive() == d)
            .count() as u32;
        days.push((d, pnl, count));
    }
    let max_abs = days
        .iter()
        .fold(Decimal::ZERO, |acc, (_, p, _)| {
            let a = p.abs();
            if a > acc {
                a
            } else {
                acc
            }
        });
    let max_abs_f = max_abs.to_f64().unwrap_or(1.0).max(1.0);

    rsx! {
        div { class: "week-day-strip",
            for (i, (d, pnl, count)) in days.iter().enumerate() {
                {
                    let pnl_f = pnl.to_f64().unwrap_or(0.0);
                    let is_empty = *count == 0;
                    let intensity = (pnl_f.abs() / max_abs_f).min(1.0);
                    let level = ((intensity * 4.0).ceil() as i32).clamp(1, 4);
                    let sign = if pnl_f > 0.0 {
                        "pos"
                    } else if pnl_f < 0.0 {
                        "neg"
                    } else {
                        "flat"
                    };
                    let cls = if is_empty {
                        "wds-cell empty".to_string()
                    } else {
                        format!("wds-cell {} l{}", sign, level)
                    };
                    let title = format!("{}: {} ({} trades)", d.format("%a %b %-d"), format_pnl(*pnl), count);
                    let pnl_str = if is_empty { String::new() } else { format_pnl(*pnl) };
                    let count_str = if is_empty { "no trades".to_string() } else { format!("{} trades", count) };
                    rsx! {
                        div { class: "{cls}", title: "{title}",
                            span { class: "wds-day", "{names[i]}" }
                            span { class: "wds-pnl", "{pnl_str}" }
                            span { class: "wds-count", "{count_str}" }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Overtrading insight panel
// ============================================================================

fn render_overtrade_panel(
    sel: &WeekBucket,
    weeks: &[WeekBucket],
    data: &AppState,
    count_commissions: bool,
) -> Element {
    // Prior 4 weeks (skip selected, take 4 going back)
    let prior: Vec<&WeekBucket> = weeks
        .iter()
        .filter(|w| w.monday < sel.monday)
        .take(4)
        .collect();

    // Per-day trade counts across prior 4 weeks (Mon-Fri only)
    let mut prior_counts: Vec<u32> = Vec::new();
    for w in &prior {
        for offset in 0..5 {
            let d = w.monday + chrono::Duration::days(offset);
            let c = w.matched.iter().filter(|mt| mt.exit_time.date_naive() == d).count() as u32;
            if c > 0 {
                prior_counts.push(c);
            }
        }
    }
    prior_counts.sort_unstable();
    let prior_median = if prior_counts.is_empty() {
        0_u32
    } else if prior_counts.len() % 2 == 1 {
        prior_counts[prior_counts.len() / 2]
    } else {
        (prior_counts[prior_counts.len() / 2 - 1] + prior_counts[prior_counts.len() / 2]) / 2
    };
    let threshold = (prior_median as f64 * 1.5).ceil() as u32;

    // This week's per-day counts + P&L
    let mut flagged: Vec<(NaiveDate, u32, Decimal)> = Vec::new();
    let mut over_loss_cost = Decimal::ZERO;
    for offset in 0..5 {
        let d = sel.monday + chrono::Duration::days(offset);
        let day_trades: Vec<&MatchedTrade> = sel
            .matched
            .iter()
            .filter(|mt| mt.exit_time.date_naive() == d)
            .collect();
        let count = day_trades.len() as u32;
        let pnl: Decimal = day_trades
            .iter()
            .map(|mt| data.trade_pnl(mt, count_commissions))
            .sum();
        if threshold > 0 && count > threshold {
            flagged.push((d, count, pnl));
            if pnl < Decimal::ZERO {
                over_loss_cost += pnl;
            }
        }
    }

    // This week's avg trades/day
    let this_week_avg = if sel.trade_days > 0 {
        sel.matched.len() as f64 / sel.trade_days as f64
    } else {
        0.0
    };
    let avg_class = if prior_median > 0 && this_week_avg > prior_median as f64 * 1.5 {
        "stat-value negative"
    } else if prior_median > 0 && this_week_avg <= prior_median as f64 {
        "stat-value positive"
    } else {
        "stat-value"
    };
    let flagged_str = if flagged.is_empty() {
        "None".to_string()
    } else {
        flagged
            .iter()
            .map(|(d, _, _)| d.format("%a").to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    rsx! {
        div { class: "card week-overtrade-card",
            h3 { "Overtrading Insight" }
            div { class: "trend-stats-row",
                div { class: "trend-stat",
                    span { class: "stat-label", "Trades / Day (this week)" }
                    span { class: "{avg_class}", "{this_week_avg:.1}" }
                }
                div { class: "trend-stat",
                    span { class: "stat-label", "Prior 4-wk Median /day" }
                    span { class: "stat-value", "{prior_median}" }
                }
                div { class: "trend-stat",
                    span { class: "stat-label", "Overtrade Threshold" }
                    span { class: "stat-value",
                        if threshold > 0 { "> {threshold}" } else { "—" }
                    }
                }
                div { class: "trend-stat",
                    span { class: "stat-label", "Days Flagged" }
                    span { class: "stat-value", "{flagged_str}" }
                }
                div { class: "trend-stat",
                    span { class: "stat-label", "Overtrade-Loss Cost" }
                    span { class: "stat-value negative", "{format_pnl(over_loss_cost)}" }
                }
            }
        }
    }
}

// ============================================================================
// Per-trade detail table
// ============================================================================

fn render_trade_table(sel: &WeekBucket, data: &AppState, count_commissions: bool) -> Element {
    // Sort chronologically (earliest first)
    let mut trades: Vec<&MatchedTrade> = sel.matched.iter().collect();
    trades.sort_by_key(|mt| mt.exit_time);

    let r_val = data.r_value_for_week(sel.monday);

    rsx! {
        div { class: "card",
            h3 { class: "card-title", "Trades (chronological)" }
            div { class: "trade-table-wrap",
                table { class: "trade-table week-trade-table",
                    thead {
                        tr {
                            th { "Date/Time" }
                            th { "Symbol" }
                            th { "Side" }
                            th { "Qty" }
                            th { "Entry" }
                            th { "Exit" }
                            th { "Hold" }
                            th { "Gross P&L" }
                            th { "Net P&L" }
                            th { "R" }
                            th { "Outcome" }
                        }
                    }
                    tbody {
                        for mt in trades.iter() {
                            {
                                let net_pnl = data.trade_pnl(mt, count_commissions);
                                let pnl_for_outcome = net_pnl;
                                let outcome = data.trade_outcome_with(mt, count_commissions);
                                let (chip_cls, chip_label) = match outcome {
                                    TradeOutcome::Winner => ("outcome-chip w", "W"),
                                    TradeOutcome::Lossless => ("outcome-chip ll", "LL"),
                                    TradeOutcome::Loser => ("outcome-chip l", "L"),
                                };
                                let net_class = if net_pnl >= Decimal::ZERO { "pnl positive" } else { "pnl negative" };
                                let gross_class = if mt.gross_pnl >= Decimal::ZERO { "pnl positive" } else { "pnl negative" };
                                let r_mult = data.pnl_in_r(pnl_for_outcome, r_val);
                                let r_class = if r_mult >= Decimal::ZERO { "pnl positive" } else { "pnl negative" };
                                let side_class = if mt.side == "Long" { "side buy" } else { "side sell" };
                                let time_str = mt.exit_time.format("%a %H:%M").to_string();
                                let hold_str = format_hold(mt.exit_time - mt.entry_time);
                                let entry_str = format!("${:.2}", mt.entry_price);
                                let exit_str = format!("${:.2}", mt.exit_price);
                                let qty_str = mt.quantity.to_string();
                                let symbol = mt.symbol.clone();
                                let side = mt.side.clone();
                                let r_str = format_r(r_mult);
                                let net_str = format_pnl(net_pnl);
                                let gross_str = format_pnl(mt.gross_pnl);
                                rsx! {
                                    tr { class: "trade-row",
                                        td { "{time_str}" }
                                        td { class: "symbol", "{symbol}" }
                                        td { class: "{side_class}", "{side}" }
                                        td { "{qty_str}" }
                                        td { "{entry_str}" }
                                        td { "{exit_str}" }
                                        td { "{hold_str}" }
                                        td { class: "{gross_class}", "{gross_str}" }
                                        td { class: "{net_class}", "{net_str}" }
                                        td { class: "{r_class}", "{r_str}" }
                                        td {
                                            span { class: "{chip_cls}", "{chip_label}" }
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
}

// ============================================================================
// Hour x Weekday heatmap (selected week only)
// ============================================================================

fn render_hour_heatmap(sel: &WeekBucket, data: &AppState, count_commissions: bool) -> Element {
    // (weekday-from-sunday, hour) -> (pnl, count)
    let mut hour_wd: HashMap<(u32, u32), (Decimal, u32)> = HashMap::new();
    for mt in &sel.matched {
        let wd = mt.exit_time.date_naive().weekday().num_days_from_sunday();
        let hour = mt.exit_time.hour();
        let entry = hour_wd.entry((wd, hour)).or_insert((Decimal::ZERO, 0));
        entry.0 += data.trade_pnl(mt, count_commissions);
        entry.1 += 1;
    }
    let max_abs = hour_wd.values().fold(Decimal::ZERO, |acc, (p, _)| {
        let a = p.abs();
        if a > acc { a } else { acc }
    });
    let max_abs_f = max_abs.to_f64().unwrap_or(1.0).max(1.0);

    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    rsx! {
        div { class: "card week-hour-card",
            div { class: "chart-label", "Hour \u{00D7} Weekday Trade P&L (selected week)" }
            div { class: "hour-heatmap",
                div { class: "hh-row hh-header",
                    div { class: "hh-label", " " }
                    for h in 0..24u32 {
                        div { class: "hh-cell hh-hour-label", "{h}" }
                    }
                }
                for wd in 1..6u32 {
                    div { class: "hh-row",
                        div { class: "hh-label", "{names[wd as usize]}" }
                        for h in 0..24u32 {
                            {
                                let cell = hour_wd.get(&(wd, h));
                                let (cls, title) = if let Some((pnl, count)) = cell {
                                    let pnl_f = pnl.to_f64().unwrap_or(0.0);
                                    let intensity = (pnl_f.abs() / max_abs_f).min(1.0);
                                    let level = ((intensity * 4.0).ceil() as i32).clamp(1, 4);
                                    let sign = if pnl_f > 0.0 { "pos" } else if pnl_f < 0.0 { "neg" } else { "flat" };
                                    (
                                        format!("hh-cell {} l{}", sign, level),
                                        format!("{} {}h: ${:.2} ({} trades)", names[wd as usize], h, pnl_f, count),
                                    )
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

// ============================================================================
// Empty state
// ============================================================================

fn render_empty_state(sel: &WeekBucket) -> Element {
    let title = format!("Week of {}", sel.monday.format("%b %-d"));
    rsx! {
        div { class: "card week-empty",
            h2 { "{title}" }
            p { class: "week-empty-msg", "No trades in this week." }
            p { class: "week-empty-hint", "Pick another week from the selector on the left." }
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Format `mt.exit_time - mt.entry_time` as a compact human hold time.
///
///   < 1 min  -> "<1m"
///   < 1 hr   -> "{m}m"
///   < 1 day  -> "{h}h{mm}m"
///   >= 1 day -> "{d}d{hh}h"
fn format_hold(d: chrono::Duration) -> String {
    let total_secs = d.num_seconds().max(0);
    if total_secs < 60 {
        return "<1m".to_string();
    }
    let mins = total_secs / 60;
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = mins / 60;
    let rem_m = mins % 60;
    if hours < 24 {
        return format!("{}h{:02}m", hours, rem_m);
    }
    let days = hours / 24;
    let rem_h = hours % 24;
    format!("{}d{:02}h", days, rem_h)
}

/// Inline week-over-week delta for a $-denominated metric.
fn fmt_delta_pnl(curr: Decimal, prev: Option<Decimal>) -> Option<String> {
    let prev = prev?;
    let diff = curr - prev;
    let arrow = arrow_for(diff);
    let pct = if prev.abs() > Decimal::ZERO {
        let p = (diff / prev.abs() * Decimal::from(100))
            .round_dp(0)
            .to_f64()
            .unwrap_or(0.0);
        Some(p)
    } else {
        None
    };
    let pct_str = pct.map(|p| format!(" ({:+.0}%)", p)).unwrap_or_default();
    Some(format!("vs last: {} {}{}", arrow, format_pnl(diff), pct_str))
}

/// Inline week-over-week delta for an integer metric (trades, days).
fn fmt_delta_int(curr: i64, prev: Option<i64>) -> Option<String> {
    let prev = prev?;
    let diff = curr - prev;
    let arrow = if diff > 0 {
        "\u{25B2}"
    } else if diff < 0 {
        "\u{25BC}"
    } else {
        "="
    };
    Some(format!("vs last: {} {:+}", arrow, diff))
}

/// Inline week-over-week delta for a percentage-point metric (win rate).
fn fmt_delta_pp(curr: f64, prev: Option<f64>) -> Option<String> {
    let prev = prev?;
    let diff = curr - prev;
    let arrow = if diff > 0.0 {
        "\u{25B2}"
    } else if diff < 0.0 {
        "\u{25BC}"
    } else {
        "="
    };
    Some(format!("vs last: {} {:+.1} pp", arrow, diff))
}

/// Inline week-over-week delta for a generic f64 metric (profit factor).
fn fmt_delta_f64(curr: Option<f64>, prev: Option<f64>, places: usize) -> Option<String> {
    let curr = curr?;
    let prev = prev?;
    let diff = curr - prev;
    let arrow = if diff > 0.0 {
        "\u{25B2}"
    } else if diff < 0.0 {
        "\u{25BC}"
    } else {
        "="
    };
    Some(format!("vs last: {} {:+.*}", arrow, places, diff))
}

fn arrow_for(d: Decimal) -> &'static str {
    if d > Decimal::ZERO {
        "\u{25B2}"
    } else if d < Decimal::ZERO {
        "\u{25BC}"
    } else {
        "="
    }
}

