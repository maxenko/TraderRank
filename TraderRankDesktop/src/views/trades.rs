use dioxus::prelude::*;
use crate::components::*;
use crate::state::AppState;
use rust_decimal::Decimal;

#[component]
pub fn Trades() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    rsx! {
        div { class: "view trades-view",
            // Summary bar
            div { class: "trades-summary",
                div { class: "summary-stat",
                    span { class: "stat-label", "Total Trades" }
                    span { class: "stat-value", "{data.total_trades}" }
                }
                div { class: "summary-stat",
                    span { class: "stat-label", "Winners" }
                    span { class: "stat-value positive", "{data.total_wins}" }
                }
                div { class: "summary-stat",
                    span { class: "stat-label", "Losers" }
                    span { class: "stat-value negative", "{data.total_losses}" }
                }
                div { class: "summary-stat",
                    span { class: "stat-label", "Net P&L" }
                    span {
                        class: if data.total_pnl >= Decimal::ZERO { "stat-value positive" } else { "stat-value negative" },
                        "{format_pnl(data.total_pnl)}"
                    }
                }
                div { class: "summary-stat",
                    span { class: "stat-label", "Commission" }
                    span { class: "stat-value negative", "{format_decimal(data.total_commission)}" }
                }
            }

            // Trade table
            div { class: "trade-table-wrap",
                table { class: "trade-table",
                    thead {
                        tr {
                            th { "Time" }
                            th { "Symbol" }
                            th { "Side" }
                            th { "Qty" }
                            th { "Price" }
                            th { "P&L" }
                            th { "Comm" }
                        }
                    }
                    tbody {
                        for trade in data.trades.iter().rev().take(200) {
                            {
                                let is_pos = trade.net_amount >= Decimal::ZERO;
                                rsx! {
                                    TradeRow {
                                        time: trade.time.format("%m/%d %H:%M").to_string(),
                                        symbol: trade.symbol.clone(),
                                        side: trade.side.to_string(),
                                        qty: trade.quantity.to_string(),
                                        price: format!("${:.2}", trade.fill_price),
                                        pnl: format_pnl(trade.net_amount),
                                        commission: format!("${:.2}", trade.commission),
                                        is_positive: is_pos,
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
