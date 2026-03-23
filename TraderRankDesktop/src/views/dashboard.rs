use dioxus::prelude::*;
use crate::components::*;
use crate::state::AppState;
use rust_decimal::Decimal;

#[component]
pub fn Dashboard() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let max_abs = data.daily_pnls.iter().fold(Decimal::ZERO, |acc, (_, pnl)| {
        let abs = pnl.abs();
        if abs > acc { abs } else { acc }
    });
    let scale = if max_abs > Decimal::ZERO { max_abs } else { Decimal::ONE };

    // Current week R
    let current_week_r = data.r_configs.last().map(|c| c.r_value).unwrap_or(Decimal::new(100, 0));
    let current_week_pnl = data.weekly_summaries.last().map(|w| w.realized_pnl).unwrap_or(Decimal::ZERO);
    let current_week_r_mult = data.pnl_in_r(current_week_pnl, current_week_r);

    let pf_str = data.profit_factor.map(|p| format!("{:.2}", p)).unwrap_or("N/A".to_string());
    let pr_str = data.payoff_ratio.map(|p| format!("{:.2}", p)).unwrap_or("N/A".to_string());

    rsx! {
        div { class: "view dashboard-view",
            // KPI Cards
            div { class: "kpi-grid",
                MetricCard {
                    label: "Net P&L".to_string(),
                    value: format_pnl(data.total_pnl),
                    subtitle: Some(format!("Gross: {}", format_decimal(data.total_gross))),
                    positive: Some(data.total_pnl > Decimal::ZERO),
                }
                MetricCard {
                    label: "Win Rate".to_string(),
                    value: format!("{:.1}%", data.overall_win_rate),
                    subtitle: Some(format!("{} W / {} L", data.total_wins, data.total_losses)),
                    positive: Some(data.overall_win_rate >= 50.0),
                }
                MetricCard {
                    label: "Expectancy".to_string(),
                    value: format_pnl(data.expectancy),
                    subtitle: Some("Per trade".to_string()),
                    positive: Some(data.expectancy > Decimal::ZERO),
                }
                MetricCard {
                    label: "Profit Factor".to_string(),
                    value: pf_str,
                    subtitle: Some(format!("Payoff: {}", pr_str)),
                    positive: data.profit_factor.map(|p| p > Decimal::ONE),
                }
                MetricCard {
                    label: "Sharpe Ratio".to_string(),
                    value: format!("{:.2}", data.sharpe_ratio),
                    subtitle: Some("Annualized".to_string()),
                    positive: Some(data.sharpe_ratio > 0.0),
                }
                MetricCard {
                    label: "Max Drawdown".to_string(),
                    value: format_decimal(data.max_drawdown),
                    subtitle: Some(format!("Current streak: {}", data.current_streak)),
                    positive: Some(false),
                }
            }

            // Equity Curve
            div { class: "card equity-section",
                h3 { class: "card-title", "Daily P&L" }
                div { class: "equity-chart",
                    for (date, pnl) in data.daily_pnls.iter() {
                        {
                            let height_pct = (pnl.abs() * Decimal::new(100, 0) / scale)
                                .to_string()
                                .parse::<f64>()
                                .unwrap_or(2.0)
                                .max(3.0);
                            let is_pos = *pnl >= Decimal::ZERO;
                            let bar_class = if is_pos { "bar positive" } else { "bar negative" };
                            let tooltip = format!("{}: {}", date, format_pnl(*pnl));
                            rsx! {
                                div {
                                    class: "{bar_class}",
                                    style: "height: {height_pct}%;",
                                    title: "{tooltip}",
                                }
                            }
                        }
                    }
                }
            }

            // Current Week Summary
            div { class: "card",
                h3 { class: "card-title", "This Week" }
                if let Some(week) = data.weekly_summaries.last() {
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
