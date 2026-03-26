use dioxus::prelude::*;
use chrono::Datelike;
use crate::components::*;
use crate::state::AppState;
use crate::settings_store;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum AnalyticsTab {
    Overview,
    TimeOfDay,
    DayOfWeek,
    Symbols,
    TradeQuality,
    Progression,
}

impl AnalyticsTab {
    fn as_str(&self) -> &'static str {
        match self {
            AnalyticsTab::Overview => "Overview",
            AnalyticsTab::TimeOfDay => "TimeOfDay",
            AnalyticsTab::DayOfWeek => "DayOfWeek",
            AnalyticsTab::Symbols => "Symbols",
            AnalyticsTab::TradeQuality => "TradeQuality",
            AnalyticsTab::Progression => "Progression",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "TimeOfDay" => AnalyticsTab::TimeOfDay,
            "DayOfWeek" => AnalyticsTab::DayOfWeek,
            "Symbols" => AnalyticsTab::Symbols,
            "TradeQuality" => AnalyticsTab::TradeQuality,
            "Progression" => AnalyticsTab::Progression,
            _ => AnalyticsTab::Overview,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TimeRange {
    OneWeek,
    TwoWeeks,
    OneMonth,
    ThreeMonths,
    SixMonths,
    All,
}

impl TimeRange {
    fn label(&self) -> &'static str {
        match self {
            TimeRange::OneWeek => "1W",
            TimeRange::TwoWeeks => "2W",
            TimeRange::OneMonth => "1M",
            TimeRange::ThreeMonths => "3M",
            TimeRange::SixMonths => "6M",
            TimeRange::All => "All",
        }
    }
    fn max_days(&self) -> usize {
        match self {
            TimeRange::OneWeek => 5,
            TimeRange::TwoWeeks => 10,
            TimeRange::OneMonth => 22,
            TimeRange::ThreeMonths => 66,
            TimeRange::SixMonths => 132,
            TimeRange::All => usize::MAX,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            TimeRange::OneWeek => "1W",
            TimeRange::TwoWeeks => "2W",
            TimeRange::OneMonth => "1M",
            TimeRange::ThreeMonths => "3M",
            TimeRange::SixMonths => "6M",
            TimeRange::All => "All",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "1W" => TimeRange::OneWeek,
            "2W" => TimeRange::TwoWeeks,
            "1M" => TimeRange::OneMonth,
            "3M" => TimeRange::ThreeMonths,
            "6M" => TimeRange::SixMonths,
            _ => TimeRange::All,
        }
    }
}

#[component]
pub fn Analytics() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let saved = settings_store::load_raw();
    let mut active_tab = use_signal(|| saved.as_ref().map(|s| AnalyticsTab::from_str(&s.analytics_tab)).unwrap_or(AnalyticsTab::Overview));
    let mut time_range = use_signal(|| saved.as_ref().map(|s| TimeRange::from_str(&s.analytics_range)).unwrap_or(TimeRange::All));
    let mut sym_sort_col = use_signal(|| "pnl".to_string());
    let mut sym_sort_asc = use_signal(|| false);
    let mut prog_sort_col = use_signal(|| "date".to_string());
    let mut prog_sort_asc = use_signal(|| true);
    let mut month_sort_col = use_signal(|| "month".to_string());
    let mut month_sort_asc = use_signal(|| true);
    let mut hour_sort_col = use_signal(|| "hour".to_string());
    let mut hour_sort_asc = use_signal(|| true);
    let mut dow_sort_col = use_signal(|| "day".to_string());
    let mut dow_sort_asc = use_signal(|| true);
    let mut bucket_sort_col = use_signal(|| "range".to_string());
    let mut bucket_sort_asc = use_signal(|| true);

    let current_tab = *active_tab.read();
    let current_range = *time_range.read();

    // Filter daily summaries by time range and exclusions
    let max_days = current_range.max_days();
    let non_excluded_days: Vec<_> = data.daily_summaries.iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .collect();
    let total_days = non_excluded_days.len();
    let skip = total_days.saturating_sub(max_days);
    let filtered_days = &non_excluded_days[skip..];

    // Filter matched trades by the same date cutoff and exclusions
    let cutoff_date = filtered_days.first().map(|d| d.date);
    let filtered_matched: Vec<_> = data.matched_trades.iter()
        .filter(|mt| cutoff_date.map_or(true, |c| mt.exit_time >= c))
        .filter(|mt| !data.is_trade_excluded(mt))
        .collect();

    let ranges = [
        TimeRange::OneWeek, TimeRange::TwoWeeks, TimeRange::OneMonth,
        TimeRange::ThreeMonths, TimeRange::SixMonths, TimeRange::All,
    ];

    rsx! {
        div { class: "view analytics-view",
            // Range filter + sub-tab selector
            div { class: "timeline-controls",
                div { class: "mode-tabs",
                    button {
                        class: if current_tab == AnalyticsTab::Overview { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::Overview);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::Overview.as_str().to_string());
                        },
                        "Overview"
                    }
                    button {
                        class: if current_tab == AnalyticsTab::TimeOfDay { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::TimeOfDay);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::TimeOfDay.as_str().to_string());
                        },
                        "Time of Day"
                    }
                    button {
                        class: if current_tab == AnalyticsTab::DayOfWeek { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::DayOfWeek);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::DayOfWeek.as_str().to_string());
                        },
                        "Day of Week"
                    }
                    button {
                        class: if current_tab == AnalyticsTab::Symbols { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::Symbols);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::Symbols.as_str().to_string());
                        },
                        "Symbols"
                    }
                    button {
                        class: if current_tab == AnalyticsTab::TradeQuality { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::TradeQuality);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::TradeQuality.as_str().to_string());
                        },
                        "Trade Quality"
                    }
                    button {
                        class: if current_tab == AnalyticsTab::Progression { "tab active" } else { "tab" },
                        onclick: move |_| {
                            active_tab.set(AnalyticsTab::Progression);
                            settings_store::update(|s| s.analytics_tab = AnalyticsTab::Progression.as_str().to_string());
                        },
                        "Progression"
                    }
                }
                div { class: "window-controls",
                    span { class: "window-info", "Range:" }
                    for r in ranges.iter() {
                        {
                            let r_val = *r;
                            let cls = if current_range == r_val { "range-tab active" } else { "range-tab" };
                            rsx! {
                                button {
                                    class: "{cls}",
                                    onclick: move |_| {
                                        time_range.set(r_val);
                                        settings_store::update(|s| s.analytics_range = r_val.as_str().to_string());
                                    },
                                    "{r_val.label()}"
                                }
                            }
                        }
                    }
                    span { class: "window-info", "{filtered_days.len()} days" }
                }
            }

            // Recompute symbol stats from filtered matched trades
            {
            let mut sym_map: HashMap<String, (Decimal, u32, u32)> = HashMap::new();
            for mt in filtered_matched.iter() {
                let entry = sym_map.entry(mt.symbol.clone()).or_insert((Decimal::ZERO, 0, 0));
                entry.0 += mt.net_pnl;
                entry.1 += 1;
                if mt.net_pnl > Decimal::ZERO { entry.2 += 1; }
            }
            let filtered_symbol_stats: Vec<crate::state::SymbolStats> = sym_map.into_iter()
                .map(|(sym, (pnl, trades, wins))| {
                    let wr = if trades > 0 { (wins as f64 / trades as f64) * 100.0 } else { 0.0 };
                    crate::state::SymbolStats { symbol: sym, total_pnl: pnl, trade_count: trades, win_rate: wr }
                })
                .collect();

            // Recompute hourly stats from filtered daily summaries
            let mut hourly_map: HashMap<u32, (Decimal, u32, u32, u32)> = HashMap::new();
            for d in filtered_days.iter() {
                for ts in d.time_slot_performance.iter() {
                    let entry = hourly_map.entry(ts.hour).or_insert((Decimal::ZERO, 0, 0, 0));
                    entry.0 += ts.pnl;
                    entry.1 += ts.trades;
                    if ts.pnl > Decimal::ZERO { entry.2 += 1; }
                    else if ts.pnl < Decimal::ZERO { entry.3 += 1; }
                }
            }
            let mut filtered_hourly_stats: Vec<crate::state::HourlyStats> = hourly_map.into_iter()
                .map(|(hour, (pnl, trades, wins, losses))| {
                    let wr = if wins + losses > 0 { (wins as f64 / (wins + losses) as f64) * 100.0 } else { 0.0 };
                    crate::state::HourlyStats { hour, total_pnl: pnl, trade_count: trades, avg_win_rate: wr }
                })
                .collect();
            filtered_hourly_stats.sort_by_key(|h| h.hour);

            // Recompute monthly summaries from filtered daily data
            let filtered_monthly = crate::analytics::TradingAnalytics::calculate_monthly_from_daily(
                &filtered_days.iter().cloned().cloned().collect::<Vec<_>>()
            );

            // Recompute overview KPIs from filtered data
            let _f_total_trades: u32 = filtered_days.iter().map(|d| d.total_trades).sum();
            let f_total_wins: u32 = filtered_days.iter().map(|d| d.winning_trades).sum();
            let f_total_losses: u32 = filtered_days.iter().map(|d| d.losing_trades).sum();
            let f_avg_win = if f_total_wins > 0 {
                let s: Decimal = filtered_days.iter().map(|d| d.avg_win * Decimal::from(d.winning_trades)).sum();
                s / Decimal::from(f_total_wins)
            } else { Decimal::ZERO };
            let f_avg_loss = if f_total_losses > 0 {
                let s: Decimal = filtered_days.iter().map(|d| d.avg_loss * Decimal::from(d.losing_trades)).sum();
                s / Decimal::from(f_total_losses)
            } else { Decimal::ZERO };
            let f_payoff_ratio = if f_avg_loss != Decimal::ZERO {
                Some(f_avg_win / f_avg_loss.abs())
            } else { None };

            // Streaks from filtered data
            let mut f_current_streak: i32 = 0;
            let mut f_max_win_streak: u32 = 0;
            let mut f_max_loss_streak: u32 = 0;
            let mut cw: u32 = 0;
            let mut cl: u32 = 0;
            for d in filtered_days.iter() {
                if d.realized_pnl > Decimal::ZERO {
                    cw += 1; cl = 0; f_current_streak = cw as i32;
                    if cw > f_max_win_streak { f_max_win_streak = cw; }
                } else if d.realized_pnl < Decimal::ZERO {
                    cl += 1; cw = 0; f_current_streak = -(cl as i32);
                    if cl > f_max_loss_streak { f_max_loss_streak = cl; }
                }
            }

            // Tab content
            match current_tab {
                AnalyticsTab::Overview => {
                    // ── Overview ──────────────────────────────────────────────
                    let max_sym_pnl = filtered_symbol_stats
                        .iter()
                        .map(|s| s.total_pnl.abs())
                        .max()
                        .unwrap_or(Decimal::ONE);

                    let max_hourly_pnl = filtered_hourly_stats
                        .iter()
                        .map(|h| h.total_pnl.abs())
                        .max()
                        .unwrap_or(Decimal::ONE);

                    rsx! {
                        div { class: "kpi-grid kpi-grid-4",
                            MetricCard {
                                label: "Avg Win".to_string(),
                                value: format_decimal(f_avg_win),
                                subtitle: Some(format!("Avg Loss: {}", format_decimal(f_avg_loss))),
                                positive: Some(true),
                            }
                            MetricCard {
                                label: "Payoff Ratio".to_string(),
                                value: f_payoff_ratio.map(|p| format!("{:.2}:1", p)).unwrap_or("N/A".to_string()),
                                subtitle: Some("Avg Win / Avg Loss".to_string()),
                                positive: f_payoff_ratio.map(|p| p > Decimal::ONE),
                            }
                            MetricCard {
                                label: "Win Streak".to_string(),
                                value: format!("{} days", f_max_win_streak),
                                subtitle: Some(format!("Current: {} days", f_current_streak)),
                                positive: Some(f_current_streak > 0),
                            }
                            MetricCard {
                                label: "Loss Streak".to_string(),
                                value: format!("{} days", f_max_loss_streak),
                                subtitle: Some("Max consecutive".to_string()),
                                positive: Some(false),
                            }
                        }

                        // Symbol Breakdown
                        div { class: "card",
                            h3 { class: "card-title", "Symbol Performance" }
                            div { class: "symbol-list",
                                for sym in filtered_symbol_stats.iter() {
                                    {
                                        let bar_width = (sym.total_pnl.abs() * Decimal::new(100, 0) / max_sym_pnl)
                                            .to_string()
                                            .parse::<f64>()
                                            .unwrap_or(5.0)
                                            .max(5.0);
                                        let is_pos = sym.total_pnl >= Decimal::ZERO;
                                        let bar_class = if is_pos { "sym-bar positive" } else { "sym-bar negative" };
                                        rsx! {
                                            div { class: "symbol-row",
                                                span { class: "sym-name", "{sym.symbol}" }
                                                div { class: "sym-bar-wrap",
                                                    div {
                                                        class: "{bar_class}",
                                                        style: "width: {bar_width}%;",
                                                    }
                                                }
                                                span { class: if is_pos { "sym-pnl positive" } else { "sym-pnl negative" },
                                                    "{format_pnl(sym.total_pnl)}"
                                                }
                                                span { class: "sym-meta",
                                                    "{sym.trade_count} trades | {sym.win_rate:.0}% win"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Hourly Performance
                        div { class: "card",
                            h3 { class: "card-title", "Performance by Hour" }
                            div { class: "hourly-chart",
                                for h in filtered_hourly_stats.iter() {
                                    {
                                        let bar_height = (h.total_pnl.abs() * Decimal::new(100, 0) / max_hourly_pnl)
                                            .to_string()
                                            .parse::<f64>()
                                            .unwrap_or(5.0)
                                            .max(5.0);
                                        let is_pos = h.total_pnl >= Decimal::ZERO;
                                        let bar_class = if is_pos { "bar positive" } else { "bar negative" };
                                        let hour_label = format!("{}:00", h.hour);
                                        rsx! {
                                            div { class: "hourly-col",
                                                div { class: "hourly-bar-wrap",
                                                    div {
                                                        class: "{bar_class}",
                                                        style: "height: {bar_height}%;",
                                                        title: "{format_pnl(h.total_pnl)} | {h.trade_count} trades | {h.avg_win_rate:.0}% win",
                                                    }
                                                }
                                                span { class: "hour-label", "{hour_label}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                AnalyticsTab::TimeOfDay => {
                    // ── Time of Day ──────────────────────────────────────────
                    // Aggregate time_slot_performance across all daily summaries
                    let mut hour_map: HashMap<u32, (u32, Decimal, f64, u32)> = HashMap::new(); // (trades, pnl, win_rate_sum, day_count)
                    for d in filtered_days.iter() {
                        for ts in d.time_slot_performance.iter() {
                            let entry = hour_map.entry(ts.hour).or_insert((0, Decimal::ZERO, 0.0, 0));
                            entry.0 += ts.trades;
                            entry.1 += ts.pnl;
                            entry.2 += ts.win_rate * ts.trades as f64;
                            entry.3 += 1;
                        }
                    }

                    // Build sorted rows for market hours 9-16
                    struct HourRow {
                        hour: u32,
                        trades: u32,
                        pnl: Decimal,
                        win_rate: f64,
                        avg_pnl: Decimal,
                    }

                    let mut hour_rows: Vec<HourRow> = hour_map
                        .iter()
                        .filter(|(h, _)| **h >= 9 && **h <= 16)
                        .map(|(h, (trades, pnl, wr_sum, _count))| {
                            let win_rate = if *trades > 0 { wr_sum / *trades as f64 } else { 0.0 };
                            let avg_pnl = if *trades > 0 { *pnl / Decimal::from(*trades) } else { Decimal::ZERO };
                            HourRow { hour: *h, trades: *trades, pnl: *pnl, win_rate, avg_pnl }
                        })
                        .collect();
                    // Sort by user selection
                    let h_col = hour_sort_col.read().clone();
                    let h_asc = *hour_sort_asc.read();
                    hour_rows.sort_by(|a, b| {
                        let ord = match h_col.as_str() {
                            "trades" => a.trades.cmp(&b.trades),
                            "pnl" => a.pnl.cmp(&b.pnl),
                            "wr" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
                            "avg" => a.avg_pnl.cmp(&b.avg_pnl),
                            _ => a.hour.cmp(&b.hour),
                        };
                        if h_asc { ord } else { ord.reverse() }
                    });
                    let h_cls = |col: &str| -> &'static str { if h_col == col { "sortable sorted" } else { "sortable" } };
                    let h_arr = |col: &str| -> &'static str { if h_col == col { if h_asc { " \u{25B2}" } else { " \u{25BC}" } } else { "" } };

                    // Find best and worst hours
                    let best_hour = hour_rows.iter().max_by(|a, b| a.pnl.cmp(&b.pnl));
                    let worst_hour = hour_rows.iter().min_by(|a, b| a.pnl.cmp(&b.pnl));

                    let best_label = best_hour.map(|h| format!("{}:00", h.hour)).unwrap_or("N/A".to_string());
                    let best_val = best_hour.map(|h| format_pnl(h.pnl)).unwrap_or("N/A".to_string());
                    let worst_label = worst_hour.map(|h| format!("{}:00", h.hour)).unwrap_or("N/A".to_string());
                    let worst_val = worst_hour.map(|h| format_pnl(h.pnl)).unwrap_or("N/A".to_string());

                    rsx! {
                        div { class: "kpi-grid kpi-grid-4",
                            MetricCard {
                                label: "Best Hour".to_string(),
                                value: best_label,
                                subtitle: Some(best_val),
                                positive: Some(true),
                            }
                            MetricCard {
                                label: "Worst Hour".to_string(),
                                value: worst_label,
                                subtitle: Some(worst_val),
                                positive: Some(false),
                            }
                        }

                        div { class: "card",
                            h3 { class: "card-title", "Performance by Hour" }
                            div { class: "timeline-table-wrap",
                                table { class: "timeline-table",
                                    thead {
                                        tr {
                                            th { class: h_cls("hour"), onclick: move |_| { let c = hour_sort_col.read().clone(); if c == "hour" { let v = *hour_sort_asc.read(); hour_sort_asc.set(!v); } else { hour_sort_col.set("hour".to_string()); hour_sort_asc.set(true); } }, "Hour{h_arr(\"hour\")}" }
                                            th { class: h_cls("trades"), onclick: move |_| { let c = hour_sort_col.read().clone(); if c == "trades" { let v = *hour_sort_asc.read(); hour_sort_asc.set(!v); } else { hour_sort_col.set("trades".to_string()); hour_sort_asc.set(false); } }, "Trades{h_arr(\"trades\")}" }
                                            th { class: h_cls("pnl"), onclick: move |_| { let c = hour_sort_col.read().clone(); if c == "pnl" { let v = *hour_sort_asc.read(); hour_sort_asc.set(!v); } else { hour_sort_col.set("pnl".to_string()); hour_sort_asc.set(false); } }, "P&L{h_arr(\"pnl\")}" }
                                            th { class: h_cls("wr"), onclick: move |_| { let c = hour_sort_col.read().clone(); if c == "wr" { let v = *hour_sort_asc.read(); hour_sort_asc.set(!v); } else { hour_sort_col.set("wr".to_string()); hour_sort_asc.set(false); } }, "Win Rate{h_arr(\"wr\")}" }
                                            th { class: h_cls("avg"), onclick: move |_| { let c = hour_sort_col.read().clone(); if c == "avg" { let v = *hour_sort_asc.read(); hour_sort_asc.set(!v); } else { hour_sort_col.set("avg".to_string()); hour_sort_asc.set(false); } }, "Avg P&L/Trade{h_arr(\"avg\")}" }
                                        }
                                    }
                                    tbody {
                                        for row in hour_rows.iter() {
                                            {
                                                let is_pos = row.pnl >= Decimal::ZERO;
                                                let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                                let pnl_str = format_pnl(row.pnl);
                                                let avg_str = format_pnl(row.avg_pnl);
                                                let hour_label = format!("{}:00", row.hour);
                                                let wr = format!("{:.1}%", row.win_rate);
                                                rsx! {
                                                    tr { class: "{row_class}",
                                                        td { "{hour_label}" }
                                                        td { "{row.trades}" }
                                                        td { class: "pnl", "{pnl_str}" }
                                                        td { "{wr}" }
                                                        td { class: "pnl", "{avg_str}" }
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

                AnalyticsTab::DayOfWeek => {
                    // ── Day of Week ──────────────────────────────────────────
                    // Group daily summaries by weekday
                    // (trading_days, total_trades, wins, pnl)
                    let mut dow_map: HashMap<u32, (u32, u32, u32, Decimal)> = HashMap::new();
                    for d in filtered_days.iter() {
                        let wd = d.date.weekday().num_days_from_monday(); // 0=Mon, 4=Fri
                        let entry = dow_map.entry(wd).or_insert((0, 0, 0, Decimal::ZERO));
                        entry.0 += 1; // trading days
                        entry.1 += d.total_trades;
                        entry.2 += d.winning_trades;
                        entry.3 += d.realized_pnl;
                    }

                    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

                    struct DowRow {
                        day_name: &'static str,
                        day_idx: u32,
                        trading_days: u32,
                        trades: u32,
                        pnl: Decimal,
                        win_rate: f64,
                        avg_daily_pnl: Decimal,
                    }

                    let mut dow_rows: Vec<DowRow> = (0u32..5)
                        .map(|wd| {
                            let (days, trades, wins, pnl) = dow_map.get(&wd).copied().unwrap_or((0, 0, 0, Decimal::ZERO));
                            let win_rate = if trades > 0 { (wins as f64 / trades as f64) * 100.0 } else { 0.0 };
                            let avg_daily_pnl = if days > 0 { pnl / Decimal::from(days) } else { Decimal::ZERO };
                            DowRow {
                                day_name: day_names[wd as usize],
                                day_idx: wd,
                                trading_days: days,
                                trades,
                                pnl,
                                win_rate,
                                avg_daily_pnl,
                            }
                        })
                        .collect();
                    let d_col = dow_sort_col.read().clone();
                    let d_asc = *dow_sort_asc.read();
                    dow_rows.sort_by(|a, b| {
                        let ord = match d_col.as_str() {
                            "days" => a.trading_days.cmp(&b.trading_days),
                            "trades" => a.trades.cmp(&b.trades),
                            "pnl" => a.pnl.cmp(&b.pnl),
                            "wr" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
                            "avg" => a.avg_daily_pnl.cmp(&b.avg_daily_pnl),
                            _ => a.day_idx.cmp(&b.day_idx),
                        };
                        if d_asc { ord } else { ord.reverse() }
                    });
                    let d_cls = |col: &str| -> &'static str { if d_col == col { "sortable sorted" } else { "sortable" } };
                    let d_arr = |col: &str| -> &'static str { if d_col == col { if d_asc { " \u{25B2}" } else { " \u{25BC}" } } else { "" } };

                    // Find best, worst, most active
                    let best_day = dow_rows.iter().max_by(|a, b| a.pnl.cmp(&b.pnl));
                    let worst_day = dow_rows.iter().min_by(|a, b| a.pnl.cmp(&b.pnl));
                    let most_active = dow_rows.iter().max_by_key(|r| r.trades);

                    let best_name = best_day.map(|d| d.day_name.to_string()).unwrap_or("N/A".to_string());
                    let best_pnl_str = best_day.map(|d| format_pnl(d.pnl)).unwrap_or("N/A".to_string());
                    let worst_name = worst_day.map(|d| d.day_name.to_string()).unwrap_or("N/A".to_string());
                    let worst_pnl_str = worst_day.map(|d| format_pnl(d.pnl)).unwrap_or("N/A".to_string());
                    let active_name = most_active.map(|d| d.day_name.to_string()).unwrap_or("N/A".to_string());
                    let active_trades = most_active.map(|d| format!("{} trades", d.trades)).unwrap_or("N/A".to_string());

                    rsx! {
                        div { class: "kpi-grid kpi-grid-4",
                            MetricCard {
                                label: "Best Day".to_string(),
                                value: best_name,
                                subtitle: Some(best_pnl_str),
                                positive: Some(true),
                            }
                            MetricCard {
                                label: "Worst Day".to_string(),
                                value: worst_name,
                                subtitle: Some(worst_pnl_str),
                                positive: Some(false),
                            }
                            MetricCard {
                                label: "Most Active Day".to_string(),
                                value: active_name,
                                subtitle: Some(active_trades),
                                positive: None,
                            }
                        }

                        div { class: "card",
                            h3 { class: "card-title", "Performance by Day of Week" }
                            div { class: "timeline-table-wrap",
                                table { class: "timeline-table",
                                    thead {
                                        tr {
                                            th { class: d_cls("day"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "day" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("day".to_string()); dow_sort_asc.set(true); } }, "Day{d_arr(\"day\")}" }
                                            th { class: d_cls("days"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "days" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("days".to_string()); dow_sort_asc.set(false); } }, "Trading Days{d_arr(\"days\")}" }
                                            th { class: d_cls("trades"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "trades" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("trades".to_string()); dow_sort_asc.set(false); } }, "Trades{d_arr(\"trades\")}" }
                                            th { class: d_cls("pnl"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "pnl" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("pnl".to_string()); dow_sort_asc.set(false); } }, "P&L{d_arr(\"pnl\")}" }
                                            th { class: d_cls("wr"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "wr" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("wr".to_string()); dow_sort_asc.set(false); } }, "Win Rate{d_arr(\"wr\")}" }
                                            th { class: d_cls("avg"), onclick: move |_| { let c = dow_sort_col.read().clone(); if c == "avg" { let v = *dow_sort_asc.read(); dow_sort_asc.set(!v); } else { dow_sort_col.set("avg".to_string()); dow_sort_asc.set(false); } }, "Avg Daily P&L{d_arr(\"avg\")}" }
                                        }
                                    }
                                    tbody {
                                        for row in dow_rows.iter() {
                                            {
                                                let is_pos = row.pnl >= Decimal::ZERO;
                                                let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                                let pnl_str = format_pnl(row.pnl);
                                                let avg_str = format_pnl(row.avg_daily_pnl);
                                                let wr = format!("{:.1}%", row.win_rate);
                                                rsx! {
                                                    tr { class: "{row_class}",
                                                        td { "{row.day_name}" }
                                                        td { "{row.trading_days}" }
                                                        td { "{row.trades}" }
                                                        td { class: "pnl", "{pnl_str}" }
                                                        td { "{wr}" }
                                                        td { class: "pnl", "{avg_str}" }
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

                AnalyticsTab::Symbols => {
                    // ── Symbols ──────────────────────────────────────────────
                    // Use pre-computed symbol_stats from AppState (actual per-symbol data from matched_trades)
                    struct SymRow {
                        symbol: String,
                        trades: u32,
                        win_rate: f64,
                        total_pnl: Decimal,
                        avg_pnl: Decimal,
                        commission: Decimal,
                    }

                    let current_sym_col = sym_sort_col.read().clone();
                    let current_sym_asc = *sym_sort_asc.read();

                    let mut sym_rows: Vec<SymRow> = filtered_symbol_stats
                        .iter()
                        .map(|s| {
                            let avg_pnl = if s.trade_count > 0 { s.total_pnl / Decimal::from(s.trade_count) } else { Decimal::ZERO };
                            SymRow {
                                symbol: s.symbol.clone(),
                                trades: s.trade_count,
                                win_rate: s.win_rate,
                                total_pnl: s.total_pnl,
                                avg_pnl,
                                commission: Decimal::ZERO, // commission not tracked per-symbol in SymbolStats
                            }
                        })
                        .collect();

                    // Sort
                    sym_rows.sort_by(|a, b| {
                        let ordering = match current_sym_col.as_str() {
                            "symbol" => a.symbol.cmp(&b.symbol),
                            "trades" => a.trades.cmp(&b.trades),
                            "winrate" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
                            "avg" => a.avg_pnl.cmp(&b.avg_pnl),
                            "commission" => a.commission.cmp(&b.commission),
                            _ => a.total_pnl.cmp(&b.total_pnl), // "pnl" default
                        };
                        if current_sym_asc { ordering } else { ordering.reverse() }
                    });

                    // MetricCards: most traded, most profitable, least profitable
                    let most_traded = sym_rows.iter().max_by_key(|r| r.trades);
                    let most_profitable = sym_rows.iter().max_by(|a, b| a.total_pnl.cmp(&b.total_pnl));
                    let least_profitable = sym_rows.iter().min_by(|a, b| a.total_pnl.cmp(&b.total_pnl));

                    let mt_name = most_traded.map(|r| r.symbol.clone()).unwrap_or("N/A".to_string());
                    let mt_sub = most_traded.map(|r| format!("{} trades", r.trades)).unwrap_or("N/A".to_string());
                    let mp_name = most_profitable.map(|r| r.symbol.clone()).unwrap_or("N/A".to_string());
                    let mp_sub = most_profitable.map(|r| format_pnl(r.total_pnl)).unwrap_or("N/A".to_string());
                    let lp_name = least_profitable.map(|r| r.symbol.clone()).unwrap_or("N/A".to_string());
                    let lp_sub = least_profitable.map(|r| format_pnl(r.total_pnl)).unwrap_or("N/A".to_string());

                    // Header helpers
                    let header_class = |col: &str| -> &'static str {
                        if current_sym_col == col { "sortable sorted" } else { "sortable" }
                    };
                    let sort_indicator = |col: &str| -> &'static str {
                        if current_sym_col == col {
                            if current_sym_asc { " \u{25B2}" } else { " \u{25BC}" }
                        } else { "" }
                    };

                    rsx! {
                        div { class: "kpi-grid kpi-grid-4",
                            MetricCard {
                                label: "Most Traded".to_string(),
                                value: mt_name,
                                subtitle: Some(mt_sub),
                                positive: None,
                            }
                            MetricCard {
                                label: "Most Profitable".to_string(),
                                value: mp_name,
                                subtitle: Some(mp_sub),
                                positive: Some(true),
                            }
                            MetricCard {
                                label: "Least Profitable".to_string(),
                                value: lp_name,
                                subtitle: Some(lp_sub),
                                positive: Some(false),
                            }
                        }

                        div { class: "card",
                            h3 { class: "card-title", "Symbol Breakdown" }
                            div { class: "timeline-table-wrap",
                                table { class: "timeline-table",
                                    thead {
                                        tr {
                                            th {
                                                class: header_class("symbol"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "symbol" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("symbol".to_string()); sym_sort_asc.set(true); }
                                                },
                                                "Symbol{sort_indicator(\"symbol\")}"
                                            }
                                            th {
                                                class: header_class("trades"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "trades" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("trades".to_string()); sym_sort_asc.set(false); }
                                                },
                                                "Trades{sort_indicator(\"trades\")}"
                                            }
                                            th {
                                                class: header_class("winrate"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "winrate" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("winrate".to_string()); sym_sort_asc.set(false); }
                                                },
                                                "Win Rate{sort_indicator(\"winrate\")}"
                                            }
                                            th {
                                                class: header_class("pnl"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "pnl" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("pnl".to_string()); sym_sort_asc.set(false); }
                                                },
                                                "Total P&L{sort_indicator(\"pnl\")}"
                                            }
                                            th {
                                                class: header_class("avg"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "avg" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("avg".to_string()); sym_sort_asc.set(false); }
                                                },
                                                "Avg P&L/Trade{sort_indicator(\"avg\")}"
                                            }
                                            th {
                                                class: header_class("commission"),
                                                onclick: move |_| {
                                                    let col = sym_sort_col.read().clone();
                                                    if col == "commission" { let cur = *sym_sort_asc.read(); sym_sort_asc.set(!cur); }
                                                    else { sym_sort_col.set("commission".to_string()); sym_sort_asc.set(false); }
                                                },
                                                "Commission{sort_indicator(\"commission\")}"
                                            }
                                        }
                                    }
                                    tbody {
                                        for row in sym_rows.iter() {
                                            {
                                                let is_pos = row.total_pnl >= Decimal::ZERO;
                                                let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                                let pnl_str = format_pnl(row.total_pnl);
                                                let avg_str = format_pnl(row.avg_pnl);
                                                let wr = format!("{:.1}%", row.win_rate);
                                                let comm = format_decimal(row.commission);
                                                let sym = row.symbol.clone();
                                                rsx! {
                                                    tr { class: "{row_class}",
                                                        td { class: "symbol", "{sym}" }
                                                        td { "{row.trades}" }
                                                        td { "{wr}" }
                                                        td { class: "pnl", "{pnl_str}" }
                                                        td { class: "pnl", "{avg_str}" }
                                                        td { class: "commission", "{comm}" }
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

                AnalyticsTab::TradeQuality => {
                    // ── Trade Quality ─────────────────────────────────────────
                    // Compute hold times from matched_trades
                    let hold_durations: Vec<i64> = filtered_matched
                        .iter()
                        .map(|mt| (mt.exit_time - mt.entry_time).num_seconds())
                        .filter(|s| *s > 0)
                        .collect();

                    let avg_hold_secs = if !hold_durations.is_empty() {
                        hold_durations.iter().sum::<i64>() / hold_durations.len() as i64
                    } else { 0 };
                    let min_hold_secs = hold_durations.iter().min().copied().unwrap_or(0);
                    let max_hold_secs = hold_durations.iter().max().copied().unwrap_or(0);

                    let format_duration = |secs: i64| -> String {
                        if secs <= 0 { return "N/A".to_string(); }
                        let hours = secs / 3600;
                        let mins = (secs % 3600) / 60;
                        let s = secs % 60;
                        if hours > 0 {
                            format!("{}h {}m {}s", hours, mins, s)
                        } else if mins > 0 {
                            format!("{}m {}s", mins, s)
                        } else {
                            format!("{}s", s)
                        }
                    };

                    // Average entry/exit fills
                    let total_matched = filtered_matched.len().max(1) as f64;
                    let avg_entry_fills: f64 = filtered_matched.iter().map(|mt| mt.entry_fills as f64).sum::<f64>() / total_matched;
                    let avg_exit_fills: f64 = filtered_matched.iter().map(|mt| mt.exit_fills as f64).sum::<f64>() / total_matched;

                    // P&L bucket distribution
                    // Buckets: losses > -$50, -$50 to $0, $0 to $50, $50+
                    struct PnlBucket {
                        label: &'static str,
                        count: u32,
                        wins: u32,
                        total_pnl: Decimal,
                    }

                    let mut buckets = vec![
                        PnlBucket { label: "< -$50", count: 0, wins: 0, total_pnl: Decimal::ZERO },
                        PnlBucket { label: "-$50 to $0", count: 0, wins: 0, total_pnl: Decimal::ZERO },
                        PnlBucket { label: "$0 to $50", count: 0, wins: 0, total_pnl: Decimal::ZERO },
                        PnlBucket { label: "> $50", count: 0, wins: 0, total_pnl: Decimal::ZERO },
                    ];

                    let neg50 = Decimal::new(-50, 0);
                    let pos50 = Decimal::new(50, 0);

                    for mt in filtered_matched.iter() {
                        let idx = if mt.net_pnl < neg50 { 0 }
                            else if mt.net_pnl < Decimal::ZERO { 1 }
                            else if mt.net_pnl < pos50 { 2 }
                            else { 3 };
                        buckets[idx].count += 1;
                        buckets[idx].total_pnl += mt.net_pnl;
                        if mt.net_pnl >= Decimal::ZERO {
                            buckets[idx].wins += 1;
                        }
                    }

                    let has_matched = !filtered_matched.is_empty();

                    // Sort buckets
                    let bk_col = bucket_sort_col.read().clone();
                    let bk_asc = *bucket_sort_asc.read();
                    let _total_for_pct = filtered_matched.len().max(1) as f64;
                    buckets.sort_by(|a, b| {
                        let ord = match bk_col.as_str() {
                            "count" => a.count.cmp(&b.count),
                            "pnl" => a.total_pnl.cmp(&b.total_pnl),
                            "wr" => {
                                let a_wr = if a.count > 0 { a.wins as f64 / a.count as f64 } else { 0.0 };
                                let b_wr = if b.count > 0 { b.wins as f64 / b.count as f64 } else { 0.0 };
                                a_wr.partial_cmp(&b_wr).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            "pct" => a.count.cmp(&b.count), // same as count sort
                            _ => std::cmp::Ordering::Equal, // keep original order for range
                        };
                        if bk_asc { ord } else { ord.reverse() }
                    });
                    let bk_cls = |col: &str| -> &'static str { if bk_col == col { "sortable sorted" } else { "sortable" } };
                    let bk_arr = |col: &str| -> &'static str { if bk_col == col { if bk_asc { " \u{25B2}" } else { " \u{25BC}" } } else { "" } };

                    rsx! {
                        div { class: "kpi-grid kpi-grid-4",
                            MetricCard {
                                label: "Avg Hold Time".to_string(),
                                value: format_duration(avg_hold_secs),
                                subtitle: Some(format!("{} round trips", filtered_matched.len())),
                                positive: None,
                            }
                            MetricCard {
                                label: "Shortest Hold".to_string(),
                                value: format_duration(min_hold_secs),
                                subtitle: None,
                                positive: None,
                            }
                            MetricCard {
                                label: "Longest Hold".to_string(),
                                value: format_duration(max_hold_secs),
                                subtitle: None,
                                positive: None,
                            }
                            MetricCard {
                                label: "Avg Fills".to_string(),
                                value: format!("{:.1} / {:.1}", avg_entry_fills, avg_exit_fills),
                                subtitle: Some("Entry / Exit".to_string()),
                                positive: None,
                            }
                        }

                        if has_matched {
                            div { class: "card",
                                h3 { class: "card-title", "P&L Distribution" }
                                div { class: "timeline-table-wrap",
                                    table { class: "timeline-table",
                                        thead {
                                            tr {
                                                th { class: bk_cls("range"), onclick: move |_| { let c = bucket_sort_col.read().clone(); if c == "range" { let v = *bucket_sort_asc.read(); bucket_sort_asc.set(!v); } else { bucket_sort_col.set("range".to_string()); bucket_sort_asc.set(true); } }, "P&L Range{bk_arr(\"range\")}" }
                                                th { class: bk_cls("count"), onclick: move |_| { let c = bucket_sort_col.read().clone(); if c == "count" { let v = *bucket_sort_asc.read(); bucket_sort_asc.set(!v); } else { bucket_sort_col.set("count".to_string()); bucket_sort_asc.set(false); } }, "Count{bk_arr(\"count\")}" }
                                                th { class: bk_cls("pnl"), onclick: move |_| { let c = bucket_sort_col.read().clone(); if c == "pnl" { let v = *bucket_sort_asc.read(); bucket_sort_asc.set(!v); } else { bucket_sort_col.set("pnl".to_string()); bucket_sort_asc.set(false); } }, "Total P&L{bk_arr(\"pnl\")}" }
                                                th { class: bk_cls("wr"), onclick: move |_| { let c = bucket_sort_col.read().clone(); if c == "wr" { let v = *bucket_sort_asc.read(); bucket_sort_asc.set(!v); } else { bucket_sort_col.set("wr".to_string()); bucket_sort_asc.set(false); } }, "Win Rate{bk_arr(\"wr\")}" }
                                                th { class: bk_cls("pct"), onclick: move |_| { let c = bucket_sort_col.read().clone(); if c == "pct" { let v = *bucket_sort_asc.read(); bucket_sort_asc.set(!v); } else { bucket_sort_col.set("pct".to_string()); bucket_sort_asc.set(false); } }, "% of Trades{bk_arr(\"pct\")}" }
                                            }
                                        }
                                        tbody {
                                            for bucket in buckets.iter() {
                                                {
                                                    let total_trades_f = filtered_matched.len().max(1) as f64;
                                                    let pct = (bucket.count as f64 / total_trades_f) * 100.0;
                                                    let wr = if bucket.count > 0 { (bucket.wins as f64 / bucket.count as f64) * 100.0 } else { 0.0 };
                                                    let is_pos = bucket.total_pnl >= Decimal::ZERO;
                                                    let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                                    let pnl_str = format_pnl(bucket.total_pnl);
                                                    rsx! {
                                                        tr { class: "{row_class}",
                                                            td { "{bucket.label}" }
                                                            td { "{bucket.count}" }
                                                            td { class: "pnl", "{pnl_str}" }
                                                            td { "{wr:.1}%" }
                                                            td { "{pct:.1}%" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if !has_matched {
                            div { class: "card",
                                h3 { class: "card-title", "Trade Quality" }
                                p { "No matched round-trip trades available. Load CSV source files to enable trade matching." }
                            }
                        }
                    }
                }

                AnalyticsTab::Progression => {
                    // ── Progression ──────────────────────────────────────────
                    struct ProgressionRow {
                        sort_date: chrono::NaiveDate,
                        date: String,
                        cumulative_wr: f64,
                        rolling_wr: f64,
                    }

                    let mut progression_rows: Vec<ProgressionRow> = Vec::new();
                    let mut cum_trades: u32 = 0;
                    let mut cum_wins: u32 = 0;
                    let summaries = filtered_days;

                    for (i, d) in summaries.iter().enumerate() {
                        cum_trades += d.total_trades;
                        cum_wins += d.winning_trades;
                        let cumulative_wr = if cum_trades > 0 {
                            (cum_wins as f64 / cum_trades as f64) * 100.0
                        } else { 0.0 };
                        let window_start = if i >= 10 { i - 9 } else { 0 };
                        let window = &summaries[window_start..=i];
                        let window_trades: u32 = window.iter().map(|dd| dd.total_trades).sum();
                        let window_wins: u32 = window.iter().map(|dd| dd.winning_trades).sum();
                        let rolling_wr = if window_trades > 0 {
                            (window_wins as f64 / window_trades as f64) * 100.0
                        } else { 0.0 };
                        progression_rows.push(ProgressionRow {
                            sort_date: d.date.date_naive(),
                            date: d.date.format("%m/%d/%Y").to_string(),
                            cumulative_wr,
                            rolling_wr,
                        });
                    }

                    // Sort progression
                    let p_col = prog_sort_col.read().clone();
                    let p_asc = *prog_sort_asc.read();
                    progression_rows.sort_by(|a, b| {
                        let ord = match p_col.as_str() {
                            "cum" => a.cumulative_wr.partial_cmp(&b.cumulative_wr).unwrap_or(std::cmp::Ordering::Equal),
                            "roll" => a.rolling_wr.partial_cmp(&b.rolling_wr).unwrap_or(std::cmp::Ordering::Equal),
                            _ => a.sort_date.cmp(&b.sort_date),
                        };
                        if p_asc { ord } else { ord.reverse() }
                    });

                    let p_cls = |col: &str| -> &'static str {
                        if p_col == col { "sortable sorted" } else { "sortable" }
                    };
                    let p_arr = |col: &str| -> &'static str {
                        if p_col == col { if p_asc { " \u{25B2}" } else { " \u{25BC}" } } else { "" }
                    };

                    // Monthly comparison — sortable
                    struct MonthRow {
                        sort_key: i64,
                        period: String,
                        trades: u32,
                        pnl: Decimal,
                        win_rate: f64,
                        trading_days: u32,
                        avg_daily_pnl: Decimal,
                    }

                    let mut month_rows: Vec<MonthRow> = filtered_monthly.iter().map(|m| {
                        MonthRow {
                            sort_key: m.year as i64 * 100 + m.month as i64,
                            period: format!("{} {}", m.month_name, m.year),
                            trades: m.total_trades,
                            pnl: m.realized_pnl,
                            win_rate: m.win_rate,
                            trading_days: m.trading_days,
                            avg_daily_pnl: m.avg_daily_pnl,
                        }
                    }).collect();

                    let m_col = month_sort_col.read().clone();
                    let m_asc = *month_sort_asc.read();
                    month_rows.sort_by(|a, b| {
                        let ord = match m_col.as_str() {
                            "trades" => a.trades.cmp(&b.trades),
                            "pnl" => a.pnl.cmp(&b.pnl),
                            "wr" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
                            "days" => a.trading_days.cmp(&b.trading_days),
                            "avg" => a.avg_daily_pnl.cmp(&b.avg_daily_pnl),
                            _ => a.sort_key.cmp(&b.sort_key),
                        };
                        if m_asc { ord } else { ord.reverse() }
                    });

                    let m_cls = |col: &str| -> &'static str {
                        if m_col == col { "sortable sorted" } else { "sortable" }
                    };
                    let m_arr = |col: &str| -> &'static str {
                        if m_col == col { if m_asc { " \u{25B2}" } else { " \u{25BC}" } } else { "" }
                    };

                    rsx! {
                        div { class: "card",
                            h3 { class: "card-title", "Win Rate Progression" }
                            div { class: "timeline-table-wrap",
                                table { class: "timeline-table",
                                    thead {
                                        tr {
                                            th {
                                                class: p_cls("date"),
                                                onclick: move |_| {
                                                    let c = prog_sort_col.read().clone();
                                                    if c == "date" { let v = *prog_sort_asc.read(); prog_sort_asc.set(!v); }
                                                    else { prog_sort_col.set("date".to_string()); prog_sort_asc.set(true); }
                                                },
                                                "Date{p_arr(\"date\")}"
                                            }
                                            th {
                                                class: p_cls("cum"),
                                                onclick: move |_| {
                                                    let c = prog_sort_col.read().clone();
                                                    if c == "cum" { let v = *prog_sort_asc.read(); prog_sort_asc.set(!v); }
                                                    else { prog_sort_col.set("cum".to_string()); prog_sort_asc.set(false); }
                                                },
                                                "Cumulative Win Rate{p_arr(\"cum\")}"
                                            }
                                            th {
                                                class: p_cls("roll"),
                                                onclick: move |_| {
                                                    let c = prog_sort_col.read().clone();
                                                    if c == "roll" { let v = *prog_sort_asc.read(); prog_sort_asc.set(!v); }
                                                    else { prog_sort_col.set("roll".to_string()); prog_sort_asc.set(false); }
                                                },
                                                "Rolling 10-Day Win Rate{p_arr(\"roll\")}"
                                            }
                                        }
                                    }
                                    tbody {
                                        for row in progression_rows.iter() {
                                            {
                                                let cum_str = format!("{:.1}%", row.cumulative_wr);
                                                let roll_str = format!("{:.1}%", row.rolling_wr);
                                                let date_str = row.date.clone();
                                                rsx! {
                                                    tr { class: "timeline-row",
                                                        td { "{date_str}" }
                                                        td { "{cum_str}" }
                                                        td { "{roll_str}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card",
                            h3 { class: "card-title", "Monthly Comparison" }
                            div { class: "timeline-table-wrap",
                                table { class: "timeline-table",
                                    thead {
                                        tr {
                                            th {
                                                class: m_cls("month"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "month" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("month".to_string()); month_sort_asc.set(true); }
                                                },
                                                "Month{m_arr(\"month\")}"
                                            }
                                            th {
                                                class: m_cls("trades"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "trades" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("trades".to_string()); month_sort_asc.set(false); }
                                                },
                                                "Trades{m_arr(\"trades\")}"
                                            }
                                            th {
                                                class: m_cls("pnl"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "pnl" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("pnl".to_string()); month_sort_asc.set(false); }
                                                },
                                                "P&L{m_arr(\"pnl\")}"
                                            }
                                            th {
                                                class: m_cls("wr"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "wr" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("wr".to_string()); month_sort_asc.set(false); }
                                                },
                                                "Win Rate{m_arr(\"wr\")}"
                                            }
                                            th {
                                                class: m_cls("days"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "days" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("days".to_string()); month_sort_asc.set(false); }
                                                },
                                                "Trading Days{m_arr(\"days\")}"
                                            }
                                            th {
                                                class: m_cls("avg"),
                                                onclick: move |_| {
                                                    let c = month_sort_col.read().clone();
                                                    if c == "avg" { let v = *month_sort_asc.read(); month_sort_asc.set(!v); }
                                                    else { month_sort_col.set("avg".to_string()); month_sort_asc.set(false); }
                                                },
                                                "Avg Daily P&L{m_arr(\"avg\")}"
                                            }
                                        }
                                    }
                                    tbody {
                                        for m in month_rows.iter() {
                                            {
                                                let is_pos = m.pnl >= Decimal::ZERO;
                                                let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                                let pnl_str = format_pnl(m.pnl);
                                                let wr = format!("{:.1}%", m.win_rate);
                                                let avg_str = format_pnl(m.avg_daily_pnl);
                                                rsx! {
                                                    tr { class: "{row_class}",
                                                        td { "{m.period}" }
                                                        td { "{m.trades}" }
                                                        td { class: "pnl", "{pnl_str}" }
                                                        td { "{wr}" }
                                                        td { "{m.trading_days}" }
                                                        td { class: "pnl", "{avg_str}" }
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
            } // end filtered stats block
        }
    }
}
