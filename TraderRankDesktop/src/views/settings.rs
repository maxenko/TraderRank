use dioxus::prelude::*;
use chrono::NaiveDate;
use crate::theme::Theme;
use crate::state::AppState;
use crate::settings_store;
use rust_decimal::Decimal;

fn persist(theme: &Signal<Theme>, state: &Signal<AppState>) {
    let t = *theme.read();
    let s = state.read();
    settings_store::save_settings(&t, &s.r_configs);
}

#[derive(Clone)]
struct RConfigRow {
    week_start: NaiveDate,
    period: String,
    sort_key: i64,
    r_value: Decimal,
    week_pnl: Decimal,
    r_mult: Decimal,
    is_positive: bool,
}

#[component]
pub fn Settings() -> Element {
    let mut theme = use_context::<Signal<Theme>>();
    let mut state = use_context::<Signal<AppState>>();

    let current_theme = *theme.read();

    let mut sort_col = use_signal(|| "week".to_string());
    let mut sort_asc = use_signal(|| true);

    let current_sort_col = sort_col.read().clone();
    let current_sort_asc = *sort_asc.read();

    // Build sortable rows
    let data = state.read();
    let mut rows: Vec<RConfigRow> = data.weekly_summaries.iter().map(|w| {
        let week_start_date = w.start_date.date_naive();
        let r_val = data.r_value_for_week(week_start_date);
        let r_mult = if r_val != Decimal::ZERO {
            w.realized_pnl / r_val
        } else {
            Decimal::ZERO
        };
        RConfigRow {
            week_start: week_start_date,
            period: format!("Wk {} ({}/{})", w.week_number, w.start_date.format("%m/%d"), w.end_date.format("%m/%d")),
            sort_key: w.year as i64 * 100 + w.week_number as i64,
            r_value: r_val,
            week_pnl: w.realized_pnl,
            r_mult,
            is_positive: w.realized_pnl >= Decimal::ZERO,
        }
    }).collect();
    drop(data);

    // Sort
    rows.sort_by(|a, b| {
        let ord = match current_sort_col.as_str() {
            "r_value" => a.r_value.cmp(&b.r_value),
            "pnl" => a.week_pnl.cmp(&b.week_pnl),
            "r_mult" => a.r_mult.cmp(&b.r_mult),
            _ => a.sort_key.cmp(&b.sort_key),
        };
        if current_sort_asc { ord } else { ord.reverse() }
    });

    rsx! {
        div { class: "view settings-view",
            // Theme — compact single-row card
            div { class: "card",
                div { class: "setting-row",
                    span { class: "setting-label", "Theme" }
                    div { class: "toggle-group",
                        button {
                            class: if current_theme == Theme::Dark { "toggle-btn active" } else { "toggle-btn" },
                            onclick: move |_| {
                                theme.set(Theme::Dark);
                                persist(&theme, &state);
                            },
                            "\u{1F319} Dark"
                        }
                        button {
                            class: if current_theme == Theme::Light { "toggle-btn active" } else { "toggle-btn" },
                            onclick: move |_| {
                                theme.set(Theme::Light);
                                persist(&theme, &state);
                            },
                            "\u{2600}\u{FE0F} Light"
                        }
                    }
                }
            }

            // R-Unit Configuration
            div { class: "card",
                h3 { class: "card-title", "Risk Unit (R) Configuration" }
                p { class: "setting-desc",
                    "Set the dollar value of 1R for each week. P&L will be displayed in R-multiples throughout the app."
                }
                div { class: "r-config-table-wrap",
                    table { class: "r-config-table",
                        thead {
                            tr {
                                {
                                    let cols = vec![
                                        ("week", "Week", true),
                                        ("r_value", "R Value ($)", false),
                                        ("pnl", "Week P&L", false),
                                        ("r_mult", "P&L in R", false),
                                    ];
                                    rsx! {
                                        for (col_id, col_label, default_asc) in cols.iter() {
                                            {
                                                let col_id = col_id.to_string();
                                                let col_label = col_label.to_string();
                                                let default_asc = *default_asc;
                                                let cls = if current_sort_col == col_id { "sortable sorted" } else { "sortable" };
                                                let arr = if current_sort_col == col_id {
                                                    if current_sort_asc { " \u{25B2}" } else { " \u{25BC}" }
                                                } else { "" };
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
                            for row in rows.iter() {
                                {
                                    let week_start_date = row.week_start;
                                    let period = row.period.clone();
                                    let r_val = row.r_value;
                                    let is_pos = row.is_positive;
                                    rsx! {
                                        tr {
                                            td { "{period}" }
                                            td {
                                                input {
                                                    r#type: "number",
                                                    class: "r-input",
                                                    value: "{r_val}",
                                                    min: "1",
                                                    step: "10",
                                                    oninput: move |e: Event<FormData>| {
                                                        if let Ok(val) = e.value().parse::<i64>() {
                                                            if val > 0 {
                                                                {
                                                                    let mut s = state.write();
                                                                    if let Some(config) = s.r_configs.iter_mut().find(|c| c.week_start == week_start_date) {
                                                                        config.r_value = Decimal::new(val, 0);
                                                                    }
                                                                }
                                                                persist(&theme, &state);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            td { class: if is_pos { "pnl positive" } else { "pnl negative" },
                                                "{crate::components::format_pnl(row.week_pnl)}"
                                            }
                                            td { class: if is_pos { "pnl positive" } else { "pnl negative" },
                                                "{crate::components::format_r(row.r_mult)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Data Source (placeholder)
            div { class: "card",
                h3 { class: "card-title", "Data Source" }
                div { class: "setting-row",
                    span { class: "setting-label", "CSV Directory" }
                    input {
                        r#type: "text",
                        class: "path-input",
                        placeholder: "D:\\Trading\\Data\\Source",
                        disabled: true,
                    }
                }
                p { class: "setting-desc muted", "Data source configuration coming soon." }
            }
        }
    }
}
