use dioxus::prelude::*;
use chrono::{Datelike, Timelike};
use crate::components::*;
use crate::settings_store;
use crate::state::{AppState, TradeOutcome};
use rust_decimal::Decimal;

#[component]
pub fn Trades() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let stats_config = use_context::<Signal<crate::state::StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;
    let saved = settings_store::load_raw();
    let mut sort_col = use_signal(|| saved.as_ref().map(|s| s.trades_sort_col.clone()).unwrap_or_else(|| "time".to_string()));
    let mut sort_asc = use_signal(|| saved.as_ref().map(|s| s.trades_sort_asc).unwrap_or(false));
    let mut max_entries = use_signal(|| saved.as_ref().map(|s| s.trades_max_entries).unwrap_or(100));
    let mut hide_excluded = use_signal(|| saved.as_ref().map(|s| s.trades_hide_excluded).unwrap_or(false));
    // Modal: selected day as "YYYY-MM-DD" string, or empty = closed
    let mut modal_day = use_signal(|| String::new());

    let data = state.read();
    let matched = &data.matched_trades;
    let hiding = *hide_excluded.read();
    let selected_day = modal_day.read().clone();

    // Summary stats: only non-excluded trades
    let active_trades: Vec<_> = matched.iter().filter(|t| !data.is_trade_excluded(t)).collect();
    let total_round_trips = active_trades.len() as u32;
    let winners: u32 = active_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Winner).count() as u32;
    let lossless: u32 = active_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Lossless).count() as u32;
    let losers: u32 = active_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Loser).count() as u32;
    let total_net_pnl: Decimal = active_trades.iter().map(|t| data.trade_pnl(t, count_commissions)).sum();
    let total_commission: Decimal = active_trades.iter().map(|t| t.commission).sum();

    let excluded_count = matched.len() - active_trades.len();

    let current_col = sort_col.read().clone();
    let ascending = *sort_asc.read();
    let cur_max = *max_entries.read();

    // If "hide excluded" is on, only show non-excluded trades
    let mut sorted_trades: Vec<_> = if hiding {
        active_trades.iter().map(|t| *t).collect()
    } else {
        matched.iter().collect()
    };
    sorted_trades.sort_by(|a, b| {
        let ord = match current_col.as_str() {
            "symbol" => a.symbol.cmp(&b.symbol),
            "side" => a.side.cmp(&b.side),
            "qty" => a.quantity.cmp(&b.quantity),
            "entry" => a.entry_price.cmp(&b.entry_price),
            "exit" => a.exit_price.cmp(&b.exit_price),
            "pnl" => data.trade_pnl(a, count_commissions).cmp(&data.trade_pnl(b, count_commissions)),
            "fills" => (a.entry_fills + a.exit_fills).cmp(&(b.entry_fills + b.exit_fills)),
            _ => a.exit_time.cmp(&b.exit_time), // "time"
        };
        if ascending { ord } else { ord.reverse() }
    });

    let total_items = sorted_trades.len();
    let visible_count = total_items.min(cur_max);

    // Sort column header helpers
    let hdr_class = |col: &str| -> &'static str {
        if current_col == col { "sortable sorted" } else { "sortable" }
    };
    let hdr_arrow = |col: &str| -> &'static str {
        if current_col == col {
            if ascending { " \u{25B2}" } else { " \u{25BC}" }
        } else { "" }
    };

    let cols: Vec<(&str, &str, bool)> = vec![
        ("time", "Date/Time", false),
        ("symbol", "Symbol", true),
        ("side", "Side", true),
        ("qty", "Qty", false),
        ("entry", "Entry", false),
        ("exit", "Exit", false),
        ("pnl", "P&L", false),
        ("fills", "Fills", false),
    ];

    // Pre-collect exclusion state for visible trades (avoid borrow issues in rsx)
    let visible: Vec<_> = sorted_trades.iter().take(cur_max).collect();

    // Build day P&L totals (only from non-excluded trades)
    let mut day_pnls: std::collections::HashMap<String, Decimal> = std::collections::HashMap::new();
    for t in visible.iter() {
        if !data.is_trade_excluded(t) {
            let day_key = t.exit_time.date_naive().to_string();
            *day_pnls.entry(day_key).or_insert(Decimal::ZERO) += data.trade_pnl(t, count_commissions);
        }
    }

    // Pre-build exclusion info for each visible trade
    struct TradeExclInfo {
        trade_excluded: bool,
        day_excluded: bool,
        trade_key: String,
        trade_reason: String,
    }
    let excl_infos: Vec<TradeExclInfo> = visible.iter().map(|t| {
        let day_str = t.exit_time.date_naive().to_string();
        let trade_key = AppState::trade_exclusion_key(t);
        let trade_excluded = data.exclusions.contains_key(&trade_key);
        let day_excluded = data.is_day_excluded(&day_str);
        let trade_reason = data.exclusions.get(&trade_key).cloned().unwrap_or_default();
        TradeExclInfo { trade_excluded, day_excluded, trade_key, trade_reason }
    }).collect();

    // Collect unique day dates for day separator exclusion info
    let mut day_excl_map: std::collections::HashMap<String, (String, bool, String)> = std::collections::HashMap::new();
    for t in visible.iter() {
        let date_naive = t.exit_time.date_naive().to_string();
        let day_display = t.exit_time.date_naive().to_string();
        if !day_excl_map.contains_key(&day_display) {
            let day_key = AppState::day_exclusion_key(&date_naive);
            let is_excl = data.is_day_excluded(&date_naive);
            let reason = data.day_exclusion_reason(&date_naive);
            day_excl_map.insert(day_display, (day_key, is_excl, reason));
        }
    }

    // Build modal data if a day is selected
    let modal_data: Option<DayModalData> = if !selected_day.is_empty() {
        // Find the DailySummary for this date
        let day_summary = data.daily_summaries.iter()
            .find(|d| d.date.date_naive().to_string() == selected_day);
        // Get matched trades for this day (non-excluded)
        let day_trades: Vec<_> = active_trades.iter()
            .filter(|t| t.exit_time.date_naive().to_string() == selected_day)
            .collect();

        let day_date = day_summary
            .map(|d| d.date.format("%A, %B %d, %Y").to_string())
            .unwrap_or_else(|| selected_day.clone());

        let net_pnl: Decimal = day_trades.iter().map(|t| data.trade_pnl(t, count_commissions)).sum();
        let gross_pnl: Decimal = day_trades.iter().map(|t| t.gross_pnl).sum();
        let commission: Decimal = day_trades.iter().map(|t| t.commission).sum();
        let total = day_trades.len() as u32;
        let wins = day_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Winner).count() as u32;
        let lossless_count = day_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Lossless).count() as u32;
        let losses = day_trades.iter().filter(|t| data.trade_outcome_with(t, count_commissions) == TradeOutcome::Loser).count() as u32;
        let win_rate = if wins + losses > 0 { (wins as f64 / (wins + losses) as f64) * 100.0 } else { 0.0 };

        let winning_pnls: Vec<Decimal> = day_trades.iter()
            .map(|t| data.trade_pnl(t, count_commissions))
            .filter(|p| *p > Decimal::ZERO)
            .collect();
        let losing_pnls: Vec<Decimal> = day_trades.iter()
            .map(|t| data.trade_pnl(t, count_commissions))
            .filter(|p| *p < Decimal::ZERO)
            .collect();

        let avg_win = if !winning_pnls.is_empty() {
            winning_pnls.iter().sum::<Decimal>() / Decimal::from(winning_pnls.len() as u32)
        } else { Decimal::ZERO };
        let avg_loss = if !losing_pnls.is_empty() {
            losing_pnls.iter().sum::<Decimal>() / Decimal::from(losing_pnls.len() as u32)
        } else { Decimal::ZERO };
        let largest_win = winning_pnls.iter().max().copied().unwrap_or(Decimal::ZERO);
        let largest_loss = losing_pnls.iter().min().copied().unwrap_or(Decimal::ZERO);

        // R-multiple
        let date_naive = day_summary.map(|d| d.date.date_naive()).unwrap_or_default();
        let days_from_mon = date_naive.weekday().num_days_from_monday();
        let monday = date_naive - chrono::Duration::days(days_from_mon as i64);
        let r_val = data.r_value_for_week(monday);
        let r_mult = data.pnl_in_r(net_pnl, r_val);

        // Symbols traded
        let mut symbols: Vec<String> = day_trades.iter().map(|t| t.symbol.clone()).collect();
        symbols.sort();
        symbols.dedup();

        // Hourly performance from matched trades
        let mut hourly: std::collections::HashMap<u32, (Decimal, u32, u32)> = std::collections::HashMap::new();
        for t in day_trades.iter() {
            let hour = t.exit_time.hour();
            let pnl = data.trade_pnl(t, count_commissions);
            let entry = hourly.entry(hour).or_insert((Decimal::ZERO, 0, 0));
            entry.0 += pnl;
            entry.1 += 1;
            if pnl > Decimal::ZERO { entry.2 += 1; }
        }
        let mut hourly_perf: Vec<HourPerf> = hourly.into_iter()
            .map(|(h, (pnl, trades, wins))| HourPerf { hour: h, pnl, trades, wins })
            .collect();
        hourly_perf.sort_by_key(|h| h.hour);

        Some(DayModalData {
            date_label: day_date,
            net_pnl, gross_pnl, commission,
            total, wins, lossless: lossless_count, losses, win_rate,
            avg_win, avg_loss, largest_win, largest_loss,
            r_mult, r_val,
            symbols,
            hourly_perf,
        })
    } else {
        None
    };

    rsx! {
        div { class: "view trades-view",
            // Day summary modal
            if let Some(md) = &modal_data {
                div { class: "modal-backdrop",
                    onclick: move |_| modal_day.set(String::new()),
                    div { class: "modal-content",
                        onclick: move |e: Event<MouseData>| e.stop_propagation(),

                        div { class: "modal-header",
                            h2 { "{md.date_label}" }
                            button {
                                class: "modal-close",
                                onclick: move |_| modal_day.set(String::new()),
                                "\u{2715}"
                            }
                        }

                        // KPI row
                        div { class: "modal-kpis",
                            div { class: "modal-kpi",
                                span { class: "modal-kpi-label", "Net P&L" }
                                span {
                                    class: if md.net_pnl >= Decimal::ZERO { "modal-kpi-value positive" } else { "modal-kpi-value negative" },
                                    "{format_pnl(md.net_pnl)}"
                                }
                            }
                            div { class: "modal-kpi",
                                span { class: "modal-kpi-label", "P&L (R)" }
                                span {
                                    class: if md.r_mult >= Decimal::ZERO { "modal-kpi-value positive" } else { "modal-kpi-value negative" },
                                    "{format_r(md.r_mult)}"
                                }
                            }
                            div { class: "modal-kpi",
                                span { class: "modal-kpi-label", "Win Rate" }
                                span { class: "modal-kpi-value", "{md.win_rate:.1}%" }
                            }
                            div { class: "modal-kpi",
                                span { class: "modal-kpi-label", "Trades" }
                                span { class: "modal-kpi-value", "{md.total}  ({md.wins}W / {md.lossless}LL / {md.losses}L)" }
                            }
                        }

                        // Detail grid
                        div { class: "modal-details",
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Gross P&L" }
                                span { class: "modal-detail-value", "{format_pnl(md.gross_pnl)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Commission" }
                                span { class: "modal-detail-value negative", "{format_decimal(md.commission)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "R Value" }
                                span { class: "modal-detail-value", "${md.r_val}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Avg Win" }
                                span { class: "modal-detail-value positive", "{format_pnl(md.avg_win)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Avg Loss" }
                                span { class: "modal-detail-value negative", "{format_pnl(md.avg_loss)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Largest Win" }
                                span { class: "modal-detail-value positive", "{format_pnl(md.largest_win)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Largest Loss" }
                                span { class: "modal-detail-value negative", "{format_pnl(md.largest_loss)}" }
                            }
                            div { class: "modal-detail-row",
                                span { class: "modal-detail-label", "Symbols" }
                                span { class: "modal-detail-value", "{md.symbols.join(\", \")}" }
                            }
                        }

                        // Hourly breakdown
                        if !md.hourly_perf.is_empty() {
                            div { class: "modal-section",
                                h3 { "Hourly Breakdown" }
                                table { class: "modal-table",
                                    thead {
                                        tr {
                                            th { "Hour" }
                                            th { "Trades" }
                                            th { "P&L" }
                                            th { "Wins" }
                                        }
                                    }
                                    tbody {
                                        for hp in md.hourly_perf.iter() {
                                            {
                                                let hour_label = format!("{:02}:00", hp.hour);
                                                let pnl_class = if hp.pnl >= Decimal::ZERO { "positive" } else { "negative" };
                                                let pnl_str = format_pnl(hp.pnl);
                                                rsx! {
                                                    tr {
                                                        td { "{hour_label}" }
                                                        td { "{hp.trades}" }
                                                        td { class: "{pnl_class}", "{pnl_str}" }
                                                        td { "{hp.wins}/{hp.trades}" }
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

            // Controls bar
            div { class: "timeline-controls",
                div { class: "trades-summary",
                    div { class: "summary-stat",
                        span { class: "stat-label", "Round Trips" }
                        span { class: "stat-value", "{total_round_trips}" }
                    }
                    div { class: "summary-stat",
                        span { class: "stat-label", "Winners" }
                        span { class: "stat-value positive", "{winners}" }
                    }
                    div { class: "summary-stat",
                        span { class: "stat-label", "Lossless" }
                        span { class: "stat-value", style: "color: var(--accent-yellow);", "{lossless}" }
                    }
                    div { class: "summary-stat",
                        span { class: "stat-label", "Losers" }
                        span { class: "stat-value negative", "{losers}" }
                    }
                    div { class: "summary-stat",
                        span { class: "stat-label", "Net P&L" }
                        span {
                            class: if total_net_pnl >= Decimal::ZERO { "stat-value positive" } else { "stat-value negative" },
                            "{format_pnl(total_net_pnl)}"
                        }
                    }
                    div { class: "summary-stat",
                        span { class: "stat-label", "Commission" }
                        span { class: "stat-value negative", "{format_decimal(total_commission)}" }
                    }
                    if excluded_count > 0 {
                        div { class: "summary-stat",
                            span { class: "stat-label", "Excluded" }
                            span { class: "stat-value", style: "color: var(--text-muted);", "{excluded_count}" }
                        }
                        div { class: "summary-stat",
                            label { class: "hide-excluded-toggle",
                                input {
                                    r#type: "checkbox",
                                    checked: hiding,
                                    onchange: move |e: Event<FormData>| {
                                        let val = e.value() == "true";
                                        hide_excluded.set(val);
                                        settings_store::update(|s| s.trades_hide_excluded = val);
                                    },
                                }
                                span { "Hide excluded" }
                            }
                        }
                    }
                }
                div { class: "window-controls",
                    span { class: "window-info", "Show:" }
                    {
                        let options: Vec<usize> = vec![25, 50, 100, 250, 500, 1000];
                        rsx! {
                            for opt in options.iter() {
                                {
                                    let val = *opt;
                                    let cls = if cur_max == val { "range-tab active" } else { "range-tab" };
                                    rsx! {
                                        button {
                                            class: "{cls}",
                                            onclick: move |_| {
                                                max_entries.set(val);
                                                settings_store::update(|s| s.trades_max_entries = val);
                                            },
                                            "{val}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    span { class: "window-info", "{visible_count} of {total_items}" }
                }
            }

            // Trade table
            div { class: "trade-table-wrap",
                table { class: "trade-table",
                    thead {
                        tr {
                            {
                                rsx! {
                                    for (col_id, col_label, default_asc) in cols.iter() {
                                        {
                                            let col_id = col_id.to_string();
                                            let col_label = col_label.to_string();
                                            let default_asc = *default_asc;
                                            let cls = hdr_class(&col_id);
                                            let arr = hdr_arrow(&col_id);
                                            let label = format!("{}{}", col_label, arr);
                                            rsx! {
                                                th {
                                                    class: "{cls}",
                                                    onclick: {
                                                        let col_id = col_id.clone();
                                                        move |_| {
                                                            let cur = sort_col.read().clone();
                                                            if cur == col_id {
                                                                let cur_asc = *sort_asc.read();
                                                                sort_asc.set(!cur_asc);
                                                            } else {
                                                                sort_col.set(col_id.clone());
                                                                sort_asc.set(default_asc);
                                                            }
                                                            let sc = sort_col.read().clone();
                                                            let sa = *sort_asc.read();
                                                            settings_store::update(|s| { s.trades_sort_col = sc; s.trades_sort_asc = sa; });
                                                        }
                                                    },
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                    th { class: "excl-header", "Excl" }
                                }
                            }
                        }
                    }
                    tbody {
                        {
                            let mut last_day = String::new();
                            let mut row_index: usize = 0;

                            rsx! {
                                for trade in visible.iter() {
                                    {
                                        let idx = row_index;
                                        row_index += 1;

                                        let day_key = trade.exit_time.date_naive().to_string();
                                        let show_separator = day_key != last_day;
                                        last_day = day_key.clone();

                                        let info = &excl_infos[idx];
                                        let is_excluded = info.trade_excluded || info.day_excluded;
                                        let row_class_base = if is_excluded { "trade-row excluded-row" } else { "trade-row" };

                                        let effective_pnl = data.trade_pnl(trade, count_commissions);
                                        let is_pos = effective_pnl >= Decimal::ZERO;
                                        let pnl_class = if is_pos { "pnl positive" } else { "pnl negative" };
                                        let fills_str = format!("{}\u{2192}{}", trade.entry_fills, trade.exit_fills);
                                        let time_str = trade.exit_time.format("%m/%d %H:%M").to_string();
                                        let entry_str = format!("${:.2}", trade.entry_price);
                                        let exit_str = format!("${:.2}", trade.exit_price);
                                        let pnl_str = format_pnl(effective_pnl);
                                        let qty_str = trade.quantity.to_string();
                                        let symbol = trade.symbol.clone();
                                        let side = trade.side.clone();
                                        let side_class = if side == "Long" { "side buy" } else { "side sell" };

                                        let trade_key = info.trade_key.clone();
                                        let trade_is_checked = info.trade_excluded;
                                        let day_is_excluded = info.day_excluded;
                                        let trade_reason = info.trade_reason.clone();

                                        // Pre-compute separator data
                                        let day_label = trade.exit_time.format("%A, %b %d").to_string();
                                        let day_total = day_pnls.get(&day_key).copied().unwrap_or(Decimal::ZERO);
                                        let day_pnl_str = format_pnl(day_total);
                                        let day_pnl_class = if day_total >= Decimal::ZERO { "positive" } else { "negative" };
                                        let (day_excl_key, day_excl_checked, day_excl_reason) = day_excl_map
                                            .get(&day_key)
                                            .cloned()
                                            .unwrap_or_default();
                                        let separator_class = if day_excl_checked { "day-separator excluded-row" } else { "day-separator" };

                                        // For modal click
                                        let modal_day_key = day_key.clone();

                                        rsx! {
                                            // Day separator (conditional)
                                            if show_separator {
                                                tr { class: "{separator_class}",
                                                    td {
                                                        colspan: "6",
                                                        class: "day-separator-label",
                                                        onclick: {
                                                            let k = modal_day_key.clone();
                                                            move |_| modal_day.set(k.clone())
                                                        },
                                                        "{day_label}"
                                                    }
                                                    td { class: "{day_pnl_class}", "{day_pnl_str}" }
                                                    td {}
                                                    td { class: "excl-cell",
                                                        div { class: "excl-day-wrap",
                                                            label { class: "excl-label",
                                                                input {
                                                                    r#type: "checkbox",
                                                                    checked: day_excl_checked,
                                                                    onchange: {
                                                                        let day_excl_key = day_excl_key.clone();
                                                                        move |e: Event<FormData>| {
                                                                            let checked = e.value() == "true";
                                                                            {
                                                                                let mut s = state.write();
                                                                                if checked {
                                                                                    s.exclusions.insert(day_excl_key.clone(), String::new());
                                                                                } else {
                                                                                    s.exclusions.remove(&day_excl_key);
                                                                                }
                                                                            }
                                                                            let excl = state.read().exclusions.clone();
                                                                            settings_store::update(|s| s.exclusions = excl);
                                                                        }
                                                                    },
                                                                }
                                                                span { "Day" }
                                                            }
                                                            if day_excl_checked {
                                                                input {
                                                                    class: "excl-reason-input",
                                                                    r#type: "text",
                                                                    placeholder: "Reason...",
                                                                    value: "{day_excl_reason}",
                                                                    onchange: {
                                                                        let day_excl_key = day_excl_key.clone();
                                                                        move |e: Event<FormData>| {
                                                                            let val = e.value();
                                                                            {
                                                                                let mut s = state.write();
                                                                                s.exclusions.insert(day_excl_key.clone(), val);
                                                                            }
                                                                            let excl = state.read().exclusions.clone();
                                                                            settings_store::update(|s| s.exclusions = excl);
                                                                        }
                                                                    },
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // Trade row (always rendered)
                                            tr { class: "{row_class_base}",
                                                td { "{time_str}" }
                                                td { class: "symbol", "{symbol}" }
                                                td { class: "{side_class}", "{side}" }
                                                td { "{qty_str}" }
                                                td { "{entry_str}" }
                                                td { "{exit_str}" }
                                                td { class: "{pnl_class}", "{pnl_str}" }
                                                td { class: "fills", "{fills_str}" }
                                                td { class: "excl-cell",
                                                    if !day_is_excluded {
                                                        div { class: "excl-trade-wrap",
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: trade_is_checked,
                                                                onchange: {
                                                                    let trade_key = trade_key.clone();
                                                                    move |e: Event<FormData>| {
                                                                        let checked = e.value() == "true";
                                                                        {
                                                                            let mut s = state.write();
                                                                            if checked {
                                                                                s.exclusions.insert(trade_key.clone(), String::new());
                                                                            } else {
                                                                                s.exclusions.remove(&trade_key);
                                                                            }
                                                                        }
                                                                        let excl = state.read().exclusions.clone();
                                                                        settings_store::update(|s| s.exclusions = excl);
                                                                    }
                                                                },
                                                            }
                                                            if trade_is_checked {
                                                                input {
                                                                    class: "excl-reason-input",
                                                                    r#type: "text",
                                                                    placeholder: "Reason...",
                                                                    value: "{trade_reason}",
                                                                    onchange: {
                                                                        let trade_key = trade_key.clone();
                                                                        move |e: Event<FormData>| {
                                                                            let val = e.value();
                                                                            {
                                                                                let mut s = state.write();
                                                                                s.exclusions.insert(trade_key.clone(), val);
                                                                            }
                                                                            let excl = state.read().exclusions.clone();
                                                                            settings_store::update(|s| s.exclusions = excl);
                                                                        }
                                                                    },
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
                    }
                }
            }
        }
    }
}

// Data structs for the day modal
struct HourPerf {
    hour: u32,
    pnl: Decimal,
    trades: u32,
    wins: u32,
}

struct DayModalData {
    date_label: String,
    net_pnl: Decimal,
    gross_pnl: Decimal,
    commission: Decimal,
    total: u32,
    wins: u32,
    lossless: u32,
    losses: u32,
    win_rate: f64,
    avg_win: Decimal,
    avg_loss: Decimal,
    largest_win: Decimal,
    largest_loss: Decimal,
    r_mult: Decimal,
    r_val: Decimal,
    symbols: Vec<String>,
    hourly_perf: Vec<HourPerf>,
}
