use dioxus::prelude::*;
use chrono::NaiveDate;
use crate::theme::Theme;
use crate::state::{AppState, StatsConfig};
use crate::settings_store;
use rust_decimal::Decimal;

fn persist(theme: &Signal<Theme>, state: &Signal<AppState>) {
    let t = *theme.read();
    let s = state.read();
    settings_store::save_settings(&t, &s.r_configs);
}

#[derive(Clone, PartialEq)]
enum FetchStatus {
    Idle,
    Fetching,
    Success(String),
    Error(String),
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
    let mut app_log = use_context::<Signal<Vec<(String, String)>>>();
    let mut stats_config = use_context::<Signal<StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;

    let current_theme = *theme.read();

    let saved_settings = settings_store::load_raw();
    let mut flex_token = use_signal(|| saved_settings.as_ref().map(|s| s.flex_token.clone()).unwrap_or_default());
    let mut flex_query_id = use_signal(|| saved_settings.as_ref().map(|s| s.flex_query_id.clone()).unwrap_or_default());
    let mut fetch_status = use_signal(|| FetchStatus::Idle);

    let mut sort_col = use_signal(|| "week".to_string());
    let mut sort_asc = use_signal(|| true);

    let current_sort_col = sort_col.read().clone();
    let current_sort_asc = *sort_asc.read();

    // Default R value input — seeded from AppState (which loaded from settings)
    let initial_default_r = state.read().default_r_value;
    let mut default_r_input = use_signal(|| {
        let s = initial_default_r.to_string();
        // Trim trailing zeros for nicer display ("100" not "100.00")
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    });

    // Max hold days input — seeded from AppState
    let initial_max_hold = state.read().max_hold_days;
    let mut max_hold_input = use_signal(|| initial_max_hold.to_string());

    // Build sortable rows from filtered weekly data (excludes excluded days)
    let data = state.read();
    let filtered_daily: Vec<_> = data.daily_summaries.iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .cloned()
        .collect();
    let filtered_weekly = crate::analytics::TradingAnalytics::calculate_weekly_from_daily(&filtered_daily);
    let mut rows: Vec<RConfigRow> = filtered_weekly.iter().map(|w| {
        let week_start_date = w.start_date.date_naive();
        let r_val = data.r_value_for_week(week_start_date);
        let r_mult = if r_val != Decimal::ZERO {
            w.realized_pnl / r_val
        } else {
            Decimal::ZERO
        };
        RConfigRow {
            week_start: week_start_date,
            period: format!("Wk {} ({}  -  {})", w.week_number, w.start_date.format("%m/%d"), w.end_date.format("%m/%d")),
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
            // Stats accounting — affects EVERYTHING in the app instantly
            div { class: "card",
                div { class: "setting-row",
                    div { class: "setting-stack",
                        span { class: "setting-label", "Include Commissions in Stats" }
                        span { class: "setting-hint",
                            if count_commissions {
                                "ON — all P&L, R-multiples, and W/L classifications are NET (after broker fees)."
                            } else {
                                "OFF — stats use GROSS P&L. Useful for evaluating raw strategy edge separate from execution costs."
                            }
                        }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if count_commissions { "toggle-btn active" } else { "toggle-btn" },
                            onclick: move |_| {
                                stats_config.write().count_commissions = true;
                                settings_store::update(|s| s.count_commissions = true);
                            },
                            "Net (count)"
                        }
                        button {
                            class: if !count_commissions { "toggle-btn active" } else { "toggle-btn" },
                            onclick: move |_| {
                                stats_config.write().count_commissions = false;
                                settings_store::update(|s| s.count_commissions = false);
                            },
                            "Gross (ignore)"
                        }
                    }
                }
            }

            // Day-trader filter — drop multi-day round trips from stats
            div { class: "card",
                div { class: "setting-row",
                    div { class: "setting-stack",
                        span { class: "setting-label", "Day Trader Filter — Max Hold (days)" }
                        span { class: "setting-hint",
                            "Round-trip trades held longer than this are dropped from all stats. "
                            "Catches pre-existing positions whose CLOSE landed in the dataset (e.g. selling a long-term hold) — "
                            "those would otherwise be matched against unrelated later trades as phantom shorts. "
                            "Default 2 days. Changes apply on next data load (Refresh)."
                        }
                    }
                    div { class: "max-hold-wrap",
                        input {
                            class: "max-hold-input",
                            r#type: "number",
                            min: "0",
                            step: "1",
                            value: "{max_hold_input}",
                            oninput: move |e| max_hold_input.set(e.value()),
                            onchange: move |_| {
                                let raw = max_hold_input.read().clone();
                                if let Ok(parsed) = raw.parse::<u32>() {
                                    settings_store::update(|s| s.max_hold_days = parsed);
                                    crate::reload_app_state(&mut state);
                                }
                            },
                        }
                        span { class: "max-hold-suffix", "days" }
                    }
                }
            }

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
                div { class: "rconfig-header",
                    h3 { class: "card-title", "Risk Unit (R) Configuration" }
                    div { class: "rconfig-default-wrap",
                        label { class: "rconfig-default-label", r#for: "default-r-input", "Default R for new weeks: $" }
                        input {
                            id: "default-r-input",
                            class: "rconfig-default-input",
                            r#type: "number",
                            min: "0.01",
                            step: "1",
                            value: "{default_r_input}",
                            oninput: move |e| default_r_input.set(e.value()),
                            onchange: move |_| {
                                let raw = default_r_input.read().clone();
                                if let Ok(parsed) = raw.parse::<Decimal>() {
                                    if parsed > Decimal::ZERO {
                                        state.write().default_r_value = parsed;
                                        settings_store::update(|s| s.default_r_value = parsed.to_string());
                                    }
                                }
                            },
                        }
                    }
                }
                p { class: "setting-desc",
                    "Set the dollar value of 1R for each week. P&L will be displayed in R-multiples throughout the app. New weeks roll in using the default value above."
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

            // IB Flex Web Service
            div { class: "card",
                h3 { class: "card-title", "Interactive Brokers — Flex Web Service" }
                p { class: "setting-desc",
                    "Pull trade history directly from your IB account. One-time setup (2 minutes):"
                }
                div { class: "ib-setup-steps",
                    ol {
                        li {
                            "Log into "
                            a {
                                href: "https://www.interactivebrokers.com/portal",
                                target: "_blank",
                                class: "ib-link",
                                "IB Account Management"
                            }
                        }
                        li {
                            "Go to "
                            span { class: "ib-path", "Settings" }
                            " \u{2192} "
                            span { class: "ib-path", "Reporting" }
                            " \u{2192} "
                            span { class: "ib-path", "Flex Queries" }
                        }
                        li {
                            "Create a new "
                            strong { "Activity" }
                            " Flex Query — select "
                            strong { "Trades" }
                            " section with all fields, date period = Last 365 days"
                        }
                        li {
                            "Note the "
                            strong { "Query ID" }
                            " (shown next to the query name)"
                        }
                        li {
                            "At the bottom of the Flex Queries page, click "
                            span { class: "ib-path", "Generate Flex Web Service Token" }
                            " — copy the token"
                        }
                    }
                }

                div { class: "ib-flex-form",
                    div { class: "setting-row",
                        span { class: "setting-label", "Flex Token" }
                        input {
                            r#type: "password",
                            class: "flex-input",
                            placeholder: "Paste your Flex Web Service token",
                            value: "{flex_token.read()}",
                            oninput: move |e: Event<FormData>| {
                                let val = e.value().to_string();
                                flex_token.set(val.clone());
                                settings_store::update(|s| s.flex_token = val);
                            }
                        }
                    }
                    div { class: "setting-row",
                        span { class: "setting-label", "Query ID" }
                        input {
                            r#type: "text",
                            class: "flex-input",
                            placeholder: "e.g. 123456",
                            value: "{flex_query_id.read()}",
                            oninput: move |e: Event<FormData>| {
                                let val = e.value().to_string();
                                flex_query_id.set(val.clone());
                                settings_store::update(|s| s.flex_query_id = val);
                            }
                        }
                    }
                    div { class: "ib-flex-actions",
                        button {
                            class: "fetch-btn",
                            disabled: matches!(*fetch_status.read(), FetchStatus::Fetching),
                            onclick: move |_| {
                                let token = flex_token.read().clone();
                                let qid = flex_query_id.read().clone();
                                fetch_status.set(FetchStatus::Fetching);
                                crate::log_message(&mut app_log, "Fetching trades from IB Flex Web Service...");
                                spawn(async move {
                                    match crate::flex_fetcher::fetch_and_save(&token, &qid).await {
                                        Ok(count) => {
                                            crate::log_message(&mut app_log, &format!("Fetched {} trades from IB. Reloading...", count));
                                            crate::reload_app_state(&mut state);
                                            let msg = format!("Imported {} trades. Data reloaded.", count);
                                            crate::log_message(&mut app_log, &msg);
                                            fetch_status.set(FetchStatus::Success(msg));
                                        }
                                        Err(e) => {
                                            let msg = format!("{}", e);
                                            crate::log_message(&mut app_log, &format!("ERROR: {}", msg));
                                            fetch_status.set(FetchStatus::Error(msg));
                                        }
                                    }
                                });
                            },
                            {match *fetch_status.read() {
                                FetchStatus::Fetching => "Fetching...",
                                _ => "Fetch Trades from IB",
                            }}
                        }
                        {match &*fetch_status.read() {
                            FetchStatus::Success(msg) => rsx! {
                                span { class: "fetch-status success", "{msg}" }
                            },
                            FetchStatus::Error(msg) => rsx! {
                                span { class: "fetch-status error", "{msg}" }
                            },
                            FetchStatus::Fetching => rsx! {
                                span { class: "fetch-status fetching", "Connecting to IB..." }
                            },
                            FetchStatus::Idle => rsx! {},
                        }}
                    }
                }
                p { class: "setting-desc muted",
                    "Trades are saved to %LOCALAPPDATA%\\TraderRank\\imports\\. The app will use them on next launch."
                }
            }

            // Activity Log
            div { class: "card",
                div { class: "log-header",
                    h3 { class: "card-title", "Activity Log" }
                    if !app_log.read().is_empty() {
                        button {
                            class: "log-clear-btn",
                            onclick: move |_| {
                                app_log.write().clear();
                            },
                            "Clear"
                        }
                    }
                }
                div { class: "log-scroll",
                    if app_log.read().is_empty() {
                        p { class: "log-empty", "No activity yet." }
                    }
                    for (ts, msg) in app_log.read().iter().rev() {
                        {
                            let is_error = msg.starts_with("ERROR:");
                            let line_class = if is_error { "log-line error" } else { "log-line" };
                            let ts = ts.clone();
                            let msg = msg.clone();
                            rsx! {
                                div { class: "{line_class}",
                                    span { class: "log-ts", "{ts}" }
                                    span { class: "log-msg", "{msg}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
