use dioxus::prelude::*;
use crate::components::*;
use crate::settings_store;
use crate::state::AppState;
use rust_decimal::Decimal;

#[component]
pub fn Trades() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let saved = settings_store::load_raw();
    let mut sort_col = use_signal(|| saved.as_ref().map(|s| s.trades_sort_col.clone()).unwrap_or_else(|| "time".to_string()));
    let mut sort_asc = use_signal(|| saved.as_ref().map(|s| s.trades_sort_asc).unwrap_or(false));
    let mut max_entries = use_signal(|| saved.as_ref().map(|s| s.trades_max_entries).unwrap_or(100));
    let mut hide_excluded = use_signal(|| false);

    let data = state.read();
    let matched = &data.matched_trades;
    let hiding = *hide_excluded.read();

    // Summary stats: only non-excluded trades
    let active_trades: Vec<_> = matched.iter().filter(|t| !data.is_trade_excluded(t)).collect();
    let total_round_trips = active_trades.len() as u32;
    let winners: u32 = active_trades.iter().filter(|t| t.net_pnl > Decimal::ZERO).count() as u32;
    let losers: u32 = active_trades.iter().filter(|t| t.net_pnl < Decimal::ZERO).count() as u32;
    let total_net_pnl: Decimal = active_trades.iter().map(|t| t.net_pnl).sum();
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
            "pnl" => a.net_pnl.cmp(&b.net_pnl),
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
            *day_pnls.entry(day_key).or_insert(Decimal::ZERO) += t.net_pnl;
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

    rsx! {
        div { class: "view trades-view",
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
                                        hide_excluded.set(e.value() == "true");
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

                                        let is_pos = trade.net_pnl >= Decimal::ZERO;
                                        let pnl_class = if is_pos { "pnl positive" } else { "pnl negative" };
                                        let fills_str = format!("{}\u{2192}{}", trade.entry_fills, trade.exit_fills);
                                        let time_str = trade.exit_time.format("%m/%d %H:%M").to_string();
                                        let entry_str = format!("${:.2}", trade.entry_price);
                                        let exit_str = format!("${:.2}", trade.exit_price);
                                        let pnl_str = format_pnl(trade.net_pnl);
                                        let qty_str = trade.quantity.to_string();
                                        let symbol = trade.symbol.clone();
                                        let side = trade.side.clone();
                                        let side_class = if side == "Long" { "side buy" } else { "side sell" };

                                        let trade_key = info.trade_key.clone();
                                        let trade_is_checked = info.trade_excluded;
                                        let day_is_excluded = info.day_excluded;
                                        let trade_reason = info.trade_reason.clone();

                                        // Pre-compute separator data (used inside rsx conditionally)
                                        let day_label = trade.exit_time.format("%A, %b %d").to_string();
                                        let day_total = day_pnls.get(&day_key).copied().unwrap_or(Decimal::ZERO);
                                        let day_pnl_str = format_pnl(day_total);
                                        let day_pnl_class = if day_total >= Decimal::ZERO { "positive" } else { "negative" };
                                        let (day_excl_key, day_excl_checked, day_excl_reason) = day_excl_map
                                            .get(&day_key)
                                            .cloned()
                                            .unwrap_or_default();
                                        let separator_class = if day_excl_checked { "day-separator excluded-row" } else { "day-separator" };

                                        rsx! {
                                            // Day separator (conditional)
                                            if show_separator {
                                                tr { class: "{separator_class}",
                                                    td { colspan: "6", "{day_label}" }
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
