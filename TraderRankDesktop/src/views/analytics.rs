use dioxus::prelude::*;
use crate::components::*;
use crate::state::AppState;
use rust_decimal::Decimal;

#[component]
pub fn Analytics() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let max_sym_pnl = data.symbol_stats
        .iter()
        .map(|s| s.total_pnl.abs())
        .max()
        .unwrap_or(Decimal::ONE);

    let max_hourly_pnl = data.hourly_stats
        .iter()
        .map(|h| h.total_pnl.abs())
        .max()
        .unwrap_or(Decimal::ONE);

    rsx! {
        div { class: "view analytics-view",
            // Streak & ratio cards
            div { class: "kpi-grid kpi-grid-4",
                MetricCard {
                    label: "Avg Win".to_string(),
                    value: format_decimal(data.avg_win),
                    subtitle: Some(format!("Avg Loss: {}", format_decimal(data.avg_loss))),
                    positive: Some(true),
                }
                MetricCard {
                    label: "Payoff Ratio".to_string(),
                    value: data.payoff_ratio.map(|p| format!("{:.2}:1", p)).unwrap_or("N/A".to_string()),
                    subtitle: Some("Avg Win / Avg Loss".to_string()),
                    positive: data.payoff_ratio.map(|p| p > Decimal::ONE),
                }
                MetricCard {
                    label: "Win Streak".to_string(),
                    value: format!("{} days", data.max_win_streak),
                    subtitle: Some(format!("Current: {} days", data.current_streak)),
                    positive: Some(data.current_streak > 0),
                }
                MetricCard {
                    label: "Loss Streak".to_string(),
                    value: format!("{} days", data.max_loss_streak),
                    subtitle: Some("Max consecutive".to_string()),
                    positive: Some(false),
                }
            }

            // Symbol Breakdown
            div { class: "card",
                h3 { class: "card-title", "Symbol Performance" }
                div { class: "symbol-list",
                    for sym in data.symbol_stats.iter() {
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
                    for h in data.hourly_stats.iter() {
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
}
