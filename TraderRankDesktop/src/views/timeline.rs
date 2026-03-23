use dioxus::prelude::*;
use chrono::Datelike;
use crate::components::*;
use crate::state::AppState;
use rust_decimal::Decimal;

#[derive(Clone, Copy, PartialEq)]
enum TimelineMode {
    Daily,
    Weekly,
    Monthly,
}

#[component]
pub fn Timeline() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let mut mode = use_signal(|| TimelineMode::Weekly);
    let mut window_start = use_signal(|| 0_usize);
    let window_size = 8_usize;

    let current_mode = *mode.read();

    // Determine total items and visible slice
    let total_items = match current_mode {
        TimelineMode::Daily => data.daily_summaries.len(),
        TimelineMode::Weekly => data.weekly_summaries.len(),
        TimelineMode::Monthly => data.monthly_summaries.len(),
    };

    let start = (*window_start.read()).min(total_items.saturating_sub(window_size));
    let end = (start + window_size).min(total_items);

    rsx! {
        div { class: "view timeline-view",
            // Mode selector
            div { class: "timeline-controls",
                div { class: "mode-tabs",
                    button {
                        class: if current_mode == TimelineMode::Daily { "tab active" } else { "tab" },
                        onclick: move |_| { mode.set(TimelineMode::Daily); window_start.set(0); },
                        "Daily"
                    }
                    button {
                        class: if current_mode == TimelineMode::Weekly { "tab active" } else { "tab" },
                        onclick: move |_| { mode.set(TimelineMode::Weekly); window_start.set(0); },
                        "Weekly"
                    }
                    button {
                        class: if current_mode == TimelineMode::Monthly { "tab active" } else { "tab" },
                        onclick: move |_| { mode.set(TimelineMode::Monthly); window_start.set(0); },
                        "Monthly"
                    }
                }
                div { class: "window-controls",
                    button {
                        class: "nav-btn",
                        disabled: start == 0,
                        onclick: move |_| {
                            let cur = *window_start.read();
                            window_start.set(cur.saturating_sub(window_size));
                        },
                        "\u{25C0} Prev"
                    }
                    span { class: "window-info",
                        "{start + 1}-{end} of {total_items}"
                    }
                    button {
                        class: "nav-btn",
                        disabled: end >= total_items,
                        onclick: move |_| {
                            let cur = *window_start.read();
                            window_start.set((cur + window_size).min(total_items.saturating_sub(1)));
                        },
                        "Next \u{25B6}"
                    }
                }
            }

            // Timeline table
            div { class: "timeline-table-wrap",
                table { class: "timeline-table",
                    thead {
                        tr {
                            th { "Period" }
                            th { "P&L ($)" }
                            th { "P&L (R)" }
                            th { "Win %" }
                            th { "Trades" }
                            th { "W/L" }
                            th { "Commission" }
                        }
                    }
                    tbody {
                        match current_mode {
                            TimelineMode::Daily => rsx! {
                                for d in data.daily_summaries[start..end].iter() {
                                    {
                                        let date_str = d.date.format("%m/%d/%Y").to_string();
                                        let week_start = d.date.date_naive();
                                        // Find the Monday of this week for R lookup
                                        let days_from_mon = d.date.weekday().num_days_from_monday();
                                        let monday = week_start - chrono::Duration::days(days_from_mon as i64);
                                        let r_val = data.r_value_for_week(monday);
                                        let r_mult = data.pnl_in_r(d.realized_pnl, r_val);
                                        let is_pos = d.realized_pnl >= Decimal::ZERO;
                                        let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                        rsx! {
                                            tr { class: "{row_class}",
                                                td { "{date_str}" }
                                                td { class: "pnl", "{format_pnl(d.realized_pnl)}" }
                                                td { class: "r-value", "{format_r(r_mult)}" }
                                                td { "{d.win_rate:.1}%" }
                                                td { "{d.total_trades}" }
                                                td { "{d.winning_trades}/{d.losing_trades}" }
                                                td { class: "commission", "{format_decimal(d.total_commission)}" }
                                            }
                                        }
                                    }
                                }
                            },
                            TimelineMode::Weekly => rsx! {
                                for w in data.weekly_summaries[start..end].iter() {
                                    {
                                        let period = format!(
                                            "Wk {} ({} - {})",
                                            w.week_number,
                                            w.start_date.format("%m/%d"),
                                            w.end_date.format("%m/%d")
                                        );
                                        let r_val = data.r_value_for_week(w.start_date.date_naive());
                                        let r_mult = data.pnl_in_r(w.realized_pnl, r_val);
                                        let is_pos = w.realized_pnl >= Decimal::ZERO;
                                        let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                        rsx! {
                                            tr { class: "{row_class}",
                                                td { "{period}" }
                                                td { class: "pnl", "{format_pnl(w.realized_pnl)}" }
                                                td { class: "r-value", "{format_r(r_mult)}" }
                                                td { "{w.win_rate:.1}%" }
                                                td { "{w.total_trades}" }
                                                td { "{w.winning_trades}/{w.losing_trades}" }
                                                td { class: "commission", "{format_decimal(w.total_commission)}" }
                                            }
                                        }
                                    }
                                }
                            },
                            TimelineMode::Monthly => rsx! {
                                for m in data.monthly_summaries[start..end].iter() {
                                    {
                                        let period = format!("{} {}", m.month_name, m.year);
                                        // Find first Monday on or after 1st of month for R lookup
                                        let first_of_month = chrono::NaiveDate::from_ymd_opt(m.year, m.month, 1).unwrap();
                                        let days_to_monday = (7 - first_of_month.weekday().num_days_from_monday()) % 7;
                                        let first_monday = first_of_month + chrono::Duration::days(days_to_monday as i64);
                                        let r_val = data.r_value_for_week(first_monday);
                                        let r_mult = data.pnl_in_r(m.realized_pnl, r_val);
                                        let is_pos = m.realized_pnl >= Decimal::ZERO;
                                        let row_class = if is_pos { "timeline-row positive" } else { "timeline-row negative" };
                                        rsx! {
                                            tr { class: "{row_class}",
                                                td { "{period}" }
                                                td { class: "pnl", "{format_pnl(m.realized_pnl)}" }
                                                td { class: "r-value", "{format_r(r_mult)}" }
                                                td { "{m.win_rate:.1}%" }
                                                td { "{m.total_trades}" }
                                                td { "{m.winning_trades}/{m.losing_trades}" }
                                                td { class: "commission", "{format_decimal(m.total_commission)}" }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}
