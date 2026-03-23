use dioxus::prelude::*;
use crate::components::*;
use crate::state::AppState;
use rust_decimal::Decimal;

#[component]
pub fn Trades() -> Element {
    let state = use_context::<Signal<AppState>>();
    let mut sort_col = use_signal(|| "time".to_string());
    let mut sort_asc = use_signal(|| false); // most recent first
    let mut max_entries = use_signal(|| 100_usize);

    let data = state.read();
    let matched = &data.matched_trades;

    let total_round_trips = matched.len() as u32;
    let winners: u32 = matched.iter().filter(|t| t.net_pnl > Decimal::ZERO).count() as u32;
    let losers: u32 = matched.iter().filter(|t| t.net_pnl < Decimal::ZERO).count() as u32;
    let total_net_pnl: Decimal = matched.iter().map(|t| t.net_pnl).sum();
    let total_commission: Decimal = matched.iter().map(|t| t.commission).sum();

    let current_col = sort_col.read().clone();
    let ascending = *sort_asc.read();
    let cur_max = *max_entries.read();

    let mut sorted_trades: Vec<_> = matched.iter().collect();
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
                                            onclick: move |_| max_entries.set(val),
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
                                                        }
                                                    },
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tbody {
                        {
                            // Pre-compute day groups for separator rows
                            let visible: Vec<_> = sorted_trades.iter().take(cur_max).collect();
                            let mut last_day = String::new();

                            // Build day P&L totals for separator labels
                            let mut day_pnls: std::collections::HashMap<String, Decimal> = std::collections::HashMap::new();
                            for t in visible.iter() {
                                let day_key = t.exit_time.format("%m/%d").to_string();
                                *day_pnls.entry(day_key).or_insert(Decimal::ZERO) += t.net_pnl;
                            }

                            rsx! {
                                for trade in visible.iter() {
                                    {
                                        let day_key = trade.exit_time.format("%m/%d").to_string();
                                        let show_separator = day_key != last_day;
                                        last_day = day_key.clone();

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

                                        if show_separator {
                                            let day_label = trade.exit_time.format("%A, %b %d").to_string();
                                            let day_total = day_pnls.get(&day_key).copied().unwrap_or(Decimal::ZERO);
                                            let day_pnl_str = format_pnl(day_total);
                                            let day_pnl_class = if day_total >= Decimal::ZERO { "positive" } else { "negative" };
                                            rsx! {
                                                tr { class: "day-separator",
                                                    td { colspan: "6", "{day_label}" }
                                                    td { class: "{day_pnl_class}", "{day_pnl_str}" }
                                                    td {}
                                                }
                                                tr { class: "trade-row",
                                                    td { "{time_str}" }
                                                    td { class: "symbol", "{symbol}" }
                                                    td { class: "{side_class}", "{side}" }
                                                    td { "{qty_str}" }
                                                    td { "{entry_str}" }
                                                    td { "{exit_str}" }
                                                    td { class: "{pnl_class}", "{pnl_str}" }
                                                    td { class: "fills", "{fills_str}" }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                tr { class: "trade-row",
                                                    td { "{time_str}" }
                                                    td { class: "symbol", "{symbol}" }
                                                    td { class: "{side_class}", "{side}" }
                                                    td { "{qty_str}" }
                                                    td { "{entry_str}" }
                                                    td { "{exit_str}" }
                                                    td { class: "{pnl_class}", "{pnl_str}" }
                                                    td { class: "fills", "{fills_str}" }
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
