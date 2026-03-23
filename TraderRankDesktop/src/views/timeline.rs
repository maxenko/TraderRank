use dioxus::prelude::*;
use chrono::Datelike;
use crate::components::*;
use crate::settings_store;
use crate::state::AppState;
use rust_decimal::Decimal;

#[derive(Clone, Copy, PartialEq)]
enum TimelineMode {
    Daily,
    Weekly,
    Monthly,
}

impl TimelineMode {
    fn as_str(&self) -> &'static str {
        match self {
            TimelineMode::Daily => "Daily",
            TimelineMode::Weekly => "Weekly",
            TimelineMode::Monthly => "Monthly",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "Daily" => TimelineMode::Daily,
            "Monthly" => TimelineMode::Monthly,
            _ => TimelineMode::Weekly,
        }
    }
}

/// A flattened row used for sorting across all three modes.
#[derive(Clone)]
#[allow(dead_code)]
struct SortableRow {
    /// Display label for the Period column.
    period: String,
    /// Sortable date key — used as default sort and tiebreaker
    sort_date: chrono::NaiveDate,
    /// Numeric period key — for weekly: (year * 100 + week), monthly: (year * 100 + month), daily: ordinal
    sort_period_num: i64,
    realized_pnl: Decimal,
    r_mult: Decimal,
    win_rate: f64,
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_commission: Decimal,
    is_positive: bool,
}

#[component]
pub fn Timeline() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let saved = settings_store::load_raw();
    let mut mode = use_signal(|| saved.as_ref().map(|s| TimelineMode::from_str(&s.timeline_mode)).unwrap_or(TimelineMode::Weekly));
    let mut max_entries = use_signal(|| saved.as_ref().map(|s| s.timeline_max_entries).unwrap_or(100));
    let mut sort_col = use_signal(|| saved.as_ref().map(|s| s.timeline_sort_col.clone()).unwrap_or_else(|| "period".to_string()));
    let mut sort_asc = use_signal(|| saved.as_ref().map(|s| s.timeline_sort_asc).unwrap_or(false));

    let current_mode = *mode.read();
    let current_sort_col = sort_col.read().clone();
    let current_sort_asc = *sort_asc.read();

    // Build sortable rows with pre-computed R-values
    let mut rows: Vec<SortableRow> = match current_mode {
        TimelineMode::Daily => {
            data.daily_summaries.iter().map(|d| {
                let date_str = d.date.format("%m/%d/%Y").to_string();
                let week_start = d.date.date_naive();
                let days_from_mon = d.date.weekday().num_days_from_monday();
                let monday = week_start - chrono::Duration::days(days_from_mon as i64);
                let r_val = data.r_value_for_week(monday);
                let r_mult = data.pnl_in_r(d.realized_pnl, r_val);
                SortableRow {
                    period: date_str,
                    sort_date: d.date.date_naive(),
                    sort_period_num: d.date.date_naive().ordinal() as i64 + d.date.year() as i64 * 1000,
                    realized_pnl: d.realized_pnl,
                    r_mult,
                    win_rate: d.win_rate,
                    total_trades: d.total_trades,
                    winning_trades: d.winning_trades,
                    losing_trades: d.losing_trades,
                    total_commission: d.total_commission,
                    is_positive: d.realized_pnl >= Decimal::ZERO,
                }
            }).collect()
        }
        TimelineMode::Weekly => {
            data.weekly_summaries.iter().map(|w| {
                let period = format!(
                    "Wk {} ({} - {})",
                    w.week_number,
                    w.start_date.format("%m/%d"),
                    w.end_date.format("%m/%d")
                );
                let r_val = data.r_value_for_week(w.start_date.date_naive());
                let r_mult = data.pnl_in_r(w.realized_pnl, r_val);
                SortableRow {
                    period,
                    sort_date: w.start_date.date_naive(),
                    sort_period_num: w.year as i64 * 100 + w.week_number as i64,
                    realized_pnl: w.realized_pnl,
                    r_mult,
                    win_rate: w.win_rate,
                    total_trades: w.total_trades,
                    winning_trades: w.winning_trades,
                    losing_trades: w.losing_trades,
                    total_commission: w.total_commission,
                    is_positive: w.realized_pnl >= Decimal::ZERO,
                }
            }).collect()
        }
        TimelineMode::Monthly => {
            data.monthly_summaries.iter().filter_map(|m| {
                let period = format!("{} {}", m.month_name, m.year);
                let Some(first_of_month) = chrono::NaiveDate::from_ymd_opt(m.year, m.month, 1) else {
                    return None;
                };
                // Find the Monday of the week containing the 1st (go backwards)
                let days_from_monday = first_of_month.weekday().num_days_from_monday();
                let monday_of_first_week = first_of_month - chrono::Duration::days(days_from_monday as i64);
                let r_val = data.r_value_for_week(monday_of_first_week);
                let r_mult = data.pnl_in_r(m.realized_pnl, r_val);
                Some(SortableRow {
                    period,
                    sort_date: first_of_month,
                    sort_period_num: m.year as i64 * 100 + m.month as i64,
                    realized_pnl: m.realized_pnl,
                    r_mult,
                    win_rate: m.win_rate,
                    total_trades: m.total_trades,
                    winning_trades: m.winning_trades,
                    losing_trades: m.losing_trades,
                    total_commission: m.total_commission,
                    is_positive: m.realized_pnl >= Decimal::ZERO,
                })
            }).collect()
        }
    };

    // Sort the full list before pagination
    rows.sort_by(|a, b| {
        let ordering = match current_sort_col.as_str() {
            "pnl" => a.realized_pnl.cmp(&b.realized_pnl),
            "r" => a.r_mult.cmp(&b.r_mult),
            "win" => a.win_rate.partial_cmp(&b.win_rate).unwrap_or(std::cmp::Ordering::Equal),
            "trades" => a.total_trades.cmp(&b.total_trades),
            "wl" => a.winning_trades.cmp(&b.winning_trades),
            "commission" => a.total_commission.cmp(&b.total_commission),
            _ => a.sort_period_num.cmp(&b.sort_period_num), // "period" or default
        };
        if current_sort_asc { ordering } else { ordering.reverse() }
    });

    let total_items = rows.len();

    // Helper closure to build header class
    let header_class = |col: &str| -> &'static str {
        if current_sort_col == col {
            "sortable sorted"
        } else {
            "sortable"
        }
    };

    // Sort indicator arrow
    let sort_indicator = |col: &str| -> &'static str {
        if current_sort_col == col {
            if current_sort_asc { " \u{25B2}" } else { " \u{25BC}" }
        } else {
            ""
        }
    };

    rsx! {
        div { class: "view timeline-view",
            // Mode selector
            div { class: "timeline-controls",
                div { class: "mode-tabs",
                    button {
                        class: if current_mode == TimelineMode::Daily { "tab active" } else { "tab" },
                        onclick: move |_| {
                            mode.set(TimelineMode::Daily);
                            settings_store::update(|s| s.timeline_mode = TimelineMode::Daily.as_str().to_string());
                        },
                        "Daily"
                    }
                    button {
                        class: if current_mode == TimelineMode::Weekly { "tab active" } else { "tab" },
                        onclick: move |_| {
                            mode.set(TimelineMode::Weekly);
                            settings_store::update(|s| s.timeline_mode = TimelineMode::Weekly.as_str().to_string());
                        },
                        "Weekly"
                    }
                    button {
                        class: if current_mode == TimelineMode::Monthly { "tab active" } else { "tab" },
                        onclick: move |_| {
                            mode.set(TimelineMode::Monthly);
                            settings_store::update(|s| s.timeline_mode = TimelineMode::Monthly.as_str().to_string());
                        },
                        "Monthly"
                    }
                }
                div { class: "window-controls",
                    span { class: "window-info", "Show:" }
                    {
                        let options: Vec<usize> = vec![25, 50, 100, 250, 500, 1000];
                        let cur_max = *max_entries.read();
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
                                                settings_store::update(|s| s.timeline_max_entries = val);
                                            },
                                            "{val}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    span { class: "window-info", "{total_items.min(*max_entries.read())} of {total_items}" }
                }
            }

            // Timeline table
            div { class: "timeline-table-wrap",
                table { class: "timeline-table",
                    thead {
                        tr {
                            th {
                                class: header_class("period"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "period" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("period".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "Period{sort_indicator(\"period\")}"
                            }
                            th {
                                class: header_class("pnl"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "pnl" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("pnl".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "P&L ($){sort_indicator(\"pnl\")}"
                            }
                            th {
                                class: header_class("r"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "r" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("r".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "P&L (R){sort_indicator(\"r\")}"
                            }
                            th {
                                class: header_class("win"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "win" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("win".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "Win %{sort_indicator(\"win\")}"
                            }
                            th {
                                class: header_class("trades"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "trades" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("trades".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "Trades{sort_indicator(\"trades\")}"
                            }
                            th {
                                class: header_class("wl"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "wl" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("wl".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "W/L{sort_indicator(\"wl\")}"
                            }
                            th {
                                class: header_class("commission"),
                                onclick: move |_| {
                                    let col = sort_col.read().clone();
                                    if col == "commission" {
                                        let cur = *sort_asc.read();
                                        sort_asc.set(!cur);
                                    } else {
                                        sort_col.set("commission".to_string());
                                        sort_asc.set(false);
                                    }
                                    let sc = sort_col.read().clone();
                                    let sa = *sort_asc.read();
                                    settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
                                },
                                "Commission{sort_indicator(\"commission\")}"
                            }
                        }
                    }
                    tbody {
                        for row in rows.iter().take(*max_entries.read()) {
                            {
                                let row_class = if row.is_positive { "timeline-row positive" } else { "timeline-row negative" };
                                let period = row.period.clone();
                                let pnl = format_pnl(row.realized_pnl);
                                let r = format_r(row.r_mult);
                                let win = format!("{:.1}%", row.win_rate);
                                let trades = format!("{}", row.total_trades);
                                let wl = format!("{}/{}", row.winning_trades, row.losing_trades);
                                let comm = format_decimal(row.total_commission);
                                rsx! {
                                    tr { class: "{row_class}",
                                        td { "{period}" }
                                        td { class: "pnl", "{pnl}" }
                                        td { class: "r-value", "{r}" }
                                        td { "{win}" }
                                        td { "{trades}" }
                                        td { "{wl}" }
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
