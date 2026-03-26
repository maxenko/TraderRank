use dioxus::prelude::*;
use chrono::Datelike;
use crate::components::*;
use crate::state::AppState;
use crate::settings_store;
use rust_decimal::Decimal;

#[component]
pub fn VisualTimeline() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();

    let saved = settings_store::load_raw();
    let mut zoom = use_signal(|| saved.as_ref().map(|s| s.vtl_zoom).unwrap_or(1.0));
    let mut range_start = use_signal(|| saved.as_ref().map(|s| s.vtl_range_start).unwrap_or(0.0));
    let mut range_end = use_signal(|| saved.as_ref().map(|s| s.vtl_range_end).unwrap_or(1.0));

    // Filter out excluded days for visual timeline
    let non_excluded_days: Vec<_> = data.daily_summaries.iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .collect();
    let total_days = non_excluded_days.len();
    if total_days == 0 {
        return rsx! {
            div { class: "view", p { "No trading data available." } }
        };
    }

    // Compute visible range from the range selector
    let rs = *range_start.read();
    let re = *range_end.read();
    let vis_start = (rs * total_days as f64).floor() as usize;
    let vis_end = ((re * total_days as f64).ceil() as usize).min(total_days);
    let vis_end = vis_end.max(vis_start);
    let visible_days = &non_excluded_days[vis_start..vis_end];
    let visible_count = visible_days.len();

    // Track width driven by zoom
    let z = *zoom.read();
    let base_node_width = 160.0;
    let track_width = (visible_count as f64 * base_node_width * z).max(400.0);

    // Max absolute P&L for sizing dots
    let max_pnl = visible_days
        .iter()
        .map(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d.realized_pnl.abs()).unwrap_or(0.0))
        .fold(1.0_f64, f64::max);

    // Date range label
    let date_range = if !visible_days.is_empty() {
        let first = visible_days.first().unwrap().date.format("%b %d, %Y");
        let last = visible_days.last().unwrap().date.format("%b %d, %Y");
        format!("{} — {}", first, last)
    } else {
        "No data".to_string()
    };

    // Cumulative P&L for the gradient line color (from filtered list)
    let mut cum_pnl = Decimal::ZERO;
    for d in &non_excluded_days[..vis_start] {
        cum_pnl += d.realized_pnl;
    }

    // Build node data
    struct NodeData {
        date: String,
        pnl_str: String,
        r_str: String,
        win_rate: f64,
        trades: u32,
        is_positive: bool,
        dot_size: f64,
        cum_pnl_str: String,
    }

    let nodes: Vec<NodeData> = visible_days
        .iter()
        .map(|d| {
            cum_pnl += d.realized_pnl;
            let pnl_f = rust_decimal::prelude::ToPrimitive::to_f64(&d.realized_pnl.abs()).unwrap_or(0.0);
            let dot_size = 10.0 + (pnl_f / max_pnl) * 16.0;

            // R-value lookup
            let days_from_mon = d.date.weekday().num_days_from_monday();
            let monday = d.date.date_naive() - chrono::Duration::days(days_from_mon as i64);
            let r_val = data.r_value_for_week(monday);
            let r_mult = data.pnl_in_r(d.realized_pnl, r_val);

            NodeData {
                date: d.date.format("%b %d").to_string(),
                pnl_str: format_pnl(d.realized_pnl),
                r_str: format_r(r_mult),
                win_rate: d.win_rate,
                trades: d.total_trades,
                is_positive: d.realized_pnl >= Decimal::ZERO,
                dot_size,
                cum_pnl_str: format_pnl(cum_pnl),
            }
        })
        .collect();

    // Range slider max (total days mapped to 0-100 integer for input)
    let slider_max = 100;

    rsx! {
        div { class: "view vtl-view",
            // Toolbar: zoom + range
            div { class: "card vtl-toolbar",
                div { class: "vtl-toolbar-left",
                    span { class: "vtl-date-range", "{date_range}" }
                    span { class: "vtl-day-count", "{visible_count} trading days" }
                }
                div { class: "vtl-toolbar-right",
                    // Zoom controls
                    div { class: "vtl-zoom",
                        button {
                            class: "zoom-btn",
                            onclick: move |_| {
                                let cur = *zoom.read();
                                let new_val = (cur * 0.75).max(0.5);
                                zoom.set(new_val);
                                settings_store::update(|s| s.vtl_zoom = new_val);
                            },
                            "\u{2212}"
                        }
                        span { class: "zoom-label", "{(z * 100.0) as u32}%" }
                        button {
                            class: "zoom-btn",
                            onclick: move |_| {
                                let cur = *zoom.read();
                                let new_val = (cur * 1.33).min(5.0);
                                zoom.set(new_val);
                                settings_store::update(|s| s.vtl_zoom = new_val);
                            },
                            "+"
                        }
                    }
                }
            }

            // Range selector
            div { class: "card vtl-range-card",
                div { class: "vtl-range-label", "Time Range" }
                div { class: "vtl-range-row",
                    span { class: "vtl-range-date",
                        {non_excluded_days.first().map(|d| d.date.format("%b %d").to_string()).unwrap_or_default()}
                    }
                    div { class: "vtl-range-inputs",
                        input {
                            r#type: "range",
                            class: "vtl-slider vtl-slider-start",
                            min: "0",
                            max: "{slider_max}",
                            value: "{(rs * slider_max as f64) as i32}",
                            oninput: move |e: Event<FormData>| {
                                if let Ok(v) = e.value().parse::<f64>() {
                                    let new_val = (v / slider_max as f64).min(*range_end.read() - 0.05).max(0.0);
                                    range_start.set(new_val);
                                }
                            },
                            onchange: move |_| {
                                let val = *range_start.read();
                                settings_store::update(|s| s.vtl_range_start = val);
                            }
                        }
                        input {
                            r#type: "range",
                            class: "vtl-slider vtl-slider-end",
                            min: "0",
                            max: "{slider_max}",
                            value: "{(re * slider_max as f64) as i32}",
                            oninput: move |e: Event<FormData>| {
                                if let Ok(v) = e.value().parse::<f64>() {
                                    let new_val = (v / slider_max as f64).max(*range_start.read() + 0.05).min(1.0);
                                    range_end.set(new_val);
                                }
                            },
                            onchange: move |_| {
                                let val = *range_end.read();
                                settings_store::update(|s| s.vtl_range_end = val);
                            }
                        }
                        // Fill indicator
                        div {
                            class: "vtl-range-fill",
                            style: "left: {rs * 100.0}%; width: {(re - rs) * 100.0}%;",
                        }
                    }
                    span { class: "vtl-range-date",
                        {non_excluded_days.last().map(|d| d.date.format("%b %d").to_string()).unwrap_or_default()}
                    }
                }
            }

            // Timeline viewport
            div { class: "card vtl-card",
                div { class: "vtl-viewport",
                    div {
                        class: "vtl-track",
                        style: "width: {track_width}px;",

                        // The horizontal line
                        div { class: "vtl-line" }

                        // Event nodes
                        div { class: "vtl-events",
                            for (_i, node) in nodes.iter().enumerate() {
                                {
                                    let event_class = if node.is_positive { "vtl-event win" } else { "vtl-event loss" };
                                    let dot_px = node.dot_size;
                                    let pnl_class = if node.is_positive { "vtl-pnl positive" } else { "vtl-pnl negative" };
                                    let detail = format!("{} trades \u{00B7} {:.0}%", node.trades, node.win_rate);
                                    rsx! {
                                        div { class: "{event_class}",
                                            div { class: "vtl-content",
                                                div { class: "vtl-content-top",
                                                    span { class: "{pnl_class}", "{node.pnl_str}" }
                                                    span { class: "vtl-r", "{node.r_str}" }
                                                }
                                                div { class: "vtl-detail", "{detail}" }
                                            }
                                            div { class: "vtl-stem" }
                                            div {
                                                class: "vtl-dot",
                                                style: "width: {dot_px}px; height: {dot_px}px;",
                                                title: "Cum: {node.cum_pnl_str}",
                                            }
                                            div { class: "vtl-date", "{node.date}" }
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
