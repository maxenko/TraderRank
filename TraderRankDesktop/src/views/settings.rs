use dioxus::prelude::*;
use crate::theme::Theme;
use crate::state::AppState;
use crate::settings_store;
use rust_decimal::Decimal;

fn persist(theme: &Signal<Theme>, state: &Signal<AppState>) {
    let t = *theme.read();
    let s = state.read();
    settings_store::save_settings(&t, &s.r_configs);
}

#[component]
pub fn Settings() -> Element {
    let mut theme = use_context::<Signal<Theme>>();
    let mut state = use_context::<Signal<AppState>>();

    let current_theme = *theme.read();

    rsx! {
        div { class: "view settings-view",
            // Theme
            div { class: "card",
                h3 { class: "card-title", "Appearance" }
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
                                th { "Week" }
                                th { "R Value ($)" }
                                th { "Week P&L" }
                                th { "P&L in R" }
                            }
                        }
                        tbody {
                            {
                                let data = state.read();
                                let weeks = &data.weekly_summaries;
                                let r_configs = &data.r_configs;
                                rsx! {
                                    for (i, w) in weeks.iter().enumerate() {
                                        {
                                            let r_val = r_configs.get(i).map(|c| c.r_value).unwrap_or(Decimal::new(100, 0));
                                            let r_mult = if r_val != Decimal::ZERO {
                                                w.realized_pnl / r_val
                                            } else {
                                                Decimal::ZERO
                                            };
                                            let period = format!("Wk {} ({}/{})", w.week_number, w.start_date.format("%m/%d"), w.end_date.format("%m/%d"));
                                            let is_pos = w.realized_pnl >= Decimal::ZERO;
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
                                                            oninput: {
                                                                let idx = i;
                                                                move |e: Event<FormData>| {
                                                                    if let Ok(val) = e.value().parse::<i64>() {
                                                                        if val > 0 {
                                                                            {
                                                                                let mut s = state.write();
                                                                                if let Some(config) = s.r_configs.get_mut(idx) {
                                                                                    config.r_value = Decimal::new(val, 0);
                                                                                }
                                                                            }
                                                                            persist(&theme, &state);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    td { class: if is_pos { "pnl positive" } else { "pnl negative" },
                                                        "{crate::components::format_pnl(w.realized_pnl)}"
                                                    }
                                                    td { class: if is_pos { "pnl positive" } else { "pnl negative" },
                                                        "{crate::components::format_r(r_mult)}"
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
