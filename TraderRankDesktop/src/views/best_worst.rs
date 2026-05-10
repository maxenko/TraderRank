// "Best vs Worst Day by Weekday" — diverging bar chart with its own range
// selector. Per weekday: best single day (green up), worst single day (red
// down), with avg marker and day count.

use chrono::Datelike;
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::components::{format_decimal, format_pnl};
use crate::settings_store;
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq)]
enum BWRange {
    OneWeek,
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    All,
}

impl BWRange {
    fn label(&self) -> &'static str {
        match self {
            BWRange::OneWeek => "1W",
            BWRange::OneMonth => "1M",
            BWRange::ThreeMonths => "3M",
            BWRange::SixMonths => "6M",
            BWRange::OneYear => "1Y",
            BWRange::All => "All",
        }
    }

    fn from_label(s: &str) -> Self {
        match s {
            "1W" => BWRange::OneWeek,
            "3M" => BWRange::ThreeMonths,
            "6M" => BWRange::SixMonths,
            "1Y" => BWRange::OneYear,
            "All" => BWRange::All,
            _ => BWRange::OneMonth,
        }
    }

    fn max_days(&self) -> usize {
        match self {
            BWRange::OneWeek => 5,
            BWRange::OneMonth => 22,
            BWRange::ThreeMonths => 66,
            BWRange::SixMonths => 132,
            BWRange::OneYear => 252,
            BWRange::All => usize::MAX,
        }
    }
}

const RANGES: [BWRange; 6] = [
    BWRange::OneWeek,
    BWRange::OneMonth,
    BWRange::ThreeMonths,
    BWRange::SixMonths,
    BWRange::OneYear,
    BWRange::All,
];

const WEEKDAY_NAMES: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];

#[derive(Clone, Copy, PartialEq)]
enum BWMode {
    BestWorst,
    Average,
}

impl BWMode {
    fn as_str(&self) -> &'static str {
        match self {
            BWMode::BestWorst => "BestWorst",
            BWMode::Average => "Average",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "Average" => BWMode::Average,
            _ => BWMode::BestWorst,
        }
    }
}

#[component]
pub fn BestWorstByWeekday() -> Element {
    let state = use_context::<Signal<AppState>>();
    let data = state.read();
    let stats_config = use_context::<Signal<crate::state::StatsConfig>>();
    let count_commissions = stats_config.read().count_commissions;

    let mut range = use_signal(|| {
        settings_store::load_raw()
            .map(|s| BWRange::from_label(&s.bestworst_range))
            .unwrap_or(BWRange::OneMonth)
    });
    let current = *range.read();

    let mut mode = use_signal(|| {
        settings_store::load_raw()
            .map(|s| BWMode::from_str(&s.bestworst_mode))
            .unwrap_or(BWMode::BestWorst)
    });
    let current_mode = *mode.read();

    // Filter to non-excluded days, then last N
    let non_excluded: Vec<_> = data
        .daily_summaries
        .iter()
        .filter(|d| !data.is_day_excluded(&d.date.date_naive().to_string()))
        .collect();
    let total = non_excluded.len();
    let skip = total.saturating_sub(current.max_days());
    let visible = &non_excluded[skip..];

    // Per weekday (Mon=0..Fri=4): (best, worst, sum, count)
    let mut buckets: [(Decimal, Decimal, Decimal, u32); 5] = [
        (Decimal::MIN, Decimal::MAX, Decimal::ZERO, 0),
        (Decimal::MIN, Decimal::MAX, Decimal::ZERO, 0),
        (Decimal::MIN, Decimal::MAX, Decimal::ZERO, 0),
        (Decimal::MIN, Decimal::MAX, Decimal::ZERO, 0),
        (Decimal::MIN, Decimal::MAX, Decimal::ZERO, 0),
    ];
    for d in visible {
        let wd = d.date.weekday().num_days_from_monday() as usize;
        if wd >= 5 { continue; } // skip weekends
        let pnl = data.daily_pnl(d, count_commissions);
        let b = &mut buckets[wd];
        if pnl > b.0 { b.0 = pnl; }
        if pnl < b.1 { b.1 = pnl; }
        b.2 += pnl;
        b.3 += 1;
    }

    // Scale depends on mode: BestWorst uses extreme values; Average uses avg values only.
    let scale = match current_mode {
        BWMode::BestWorst => {
            let mut max_abs = 0.0_f64;
            for (best, worst, _, count) in buckets.iter() {
                if *count == 0 { continue; }
                let b = best.to_f64().unwrap_or(0.0).abs();
                let w = worst.to_f64().unwrap_or(0.0).abs();
                if b > max_abs { max_abs = b; }
                if w > max_abs { max_abs = w; }
            }
            max_abs.max(1.0)
        }
        BWMode::Average => {
            let mut max_abs = 0.0_f64;
            for (_, _, sum, count) in buckets.iter() {
                if *count == 0 { continue; }
                let avg = (sum.to_f64().unwrap_or(0.0) / *count as f64).abs();
                if avg > max_abs { max_abs = avg; }
            }
            max_abs.max(1.0)
        }
    };

    // Per-weekday best & worst date for tooltip
    let mut best_dates: [Option<chrono::NaiveDate>; 5] = [None; 5];
    let mut worst_dates: [Option<chrono::NaiveDate>; 5] = [None; 5];
    for d in visible {
        let wd = d.date.weekday().num_days_from_monday() as usize;
        if wd >= 5 { continue; }
        let pnl = data.daily_pnl(d, count_commissions);
        if pnl == buckets[wd].0 { best_dates[wd] = Some(d.date.date_naive()); }
        if pnl == buckets[wd].1 { worst_dates[wd] = Some(d.date.date_naive()); }
    }

    let title = match current_mode {
        BWMode::BestWorst => "Best vs Worst Day by Weekday",
        BWMode::Average => "Average Day by Weekday",
    };
    let info = match current_mode {
        BWMode::BestWorst => "green = best single day, red = worst single day, dot = average",
        BWMode::Average => "single bar per weekday = average daily P&L over the selected range",
    };

    rsx! {
        div { class: "card bestworst-section",
            div { class: "bestworst-header",
                h3 { class: "card-title", "{title}" }
                div { class: "bw-controls",
                    div { class: "chart-range-tabs",
                        button {
                            class: if current_mode == BWMode::BestWorst { "range-tab active" } else { "range-tab" },
                            onclick: move |_| {
                                mode.set(BWMode::BestWorst);
                                settings_store::update(|s| s.bestworst_mode = BWMode::BestWorst.as_str().to_string());
                            },
                            "Best/Worst"
                        }
                        button {
                            class: if current_mode == BWMode::Average { "range-tab active" } else { "range-tab" },
                            onclick: move |_| {
                                mode.set(BWMode::Average);
                                settings_store::update(|s| s.bestworst_mode = BWMode::Average.as_str().to_string());
                            },
                            "Average"
                        }
                    }
                    div { class: "chart-range-tabs",
                        for r in RANGES.iter() {
                            {
                                let rv = *r;
                                let cls = if current == rv { "range-tab active" } else { "range-tab" };
                                rsx! {
                                    button {
                                        class: "{cls}",
                                        onclick: move |_| {
                                            range.set(rv);
                                            settings_store::update(|s| s.bestworst_range = rv.label().to_string());
                                        },
                                        "{rv.label()}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "bestworst-info",
                span { "{visible.len()} trading days \u{00B7} {info}" }
            }

            div { class: "bw-chart",
                for wd in 0..5usize {
                    {
                        let (best, worst, sum, count) = buckets[wd];
                        let has_data = count > 0;
                        let best_f = if has_data { best.to_f64().unwrap_or(0.0) } else { 0.0 };
                        let worst_f = if has_data { worst.to_f64().unwrap_or(0.0) } else { 0.0 };
                        let avg = if count > 0 { sum / Decimal::from(count) } else { Decimal::ZERO };
                        let avg_f = avg.to_f64().unwrap_or(0.0);

                        // Heights as percent of half-pane (each half is 50% of chart)
                        let best_pct = if best_f > 0.0 { (best_f / scale * 100.0).min(100.0) } else { 0.0 };
                        let worst_pct = if worst_f < 0.0 { ((-worst_f) / scale * 100.0).min(100.0) } else { 0.0 };
                        // Avg position / bar height
                        let avg_pct_signed = (avg_f / scale * 100.0).clamp(-100.0, 100.0);
                        let avg_offset_style = if avg_pct_signed >= 0.0 {
                            format!("bottom: 50%; transform: translateY(-{}%);", avg_pct_signed)
                        } else {
                            format!("bottom: 50%; transform: translateY({}%);", -avg_pct_signed)
                        };
                        let avg_bar_pct = avg_pct_signed.abs();

                        let best_title = best_dates[wd].map(|d|
                            format!("Best {}: {} on {}", WEEKDAY_NAMES[wd], format_pnl(best), d.format("%a %m/%d"))
                        ).unwrap_or_else(|| format!("No {} trading", WEEKDAY_NAMES[wd]));
                        let worst_title = worst_dates[wd].map(|d|
                            format!("Worst {}: {} on {}", WEEKDAY_NAMES[wd], format_pnl(worst), d.format("%a %m/%d"))
                        ).unwrap_or_default();
                        let avg_title = format!("Avg {}: {} over {} days", WEEKDAY_NAMES[wd], format_pnl(avg), count);

                        rsx! {
                            div { class: "bw-col",
                                div { class: "bw-chart-area",
                                    // top half — best (Best/Worst mode) OR positive avg (Average mode)
                                    div { class: "bw-half top",
                                        match current_mode {
                                            BWMode::BestWorst => rsx! {
                                                if has_data && best_f > 0.0 {
                                                    span { class: "bw-val best", "{format_pnl(best)}" }
                                                }
                                                div { class: "bw-bar-wrap top",
                                                    if has_data && best_f > 0.0 {
                                                        div {
                                                            class: "bw-bar best",
                                                            style: "height: {best_pct}%;",
                                                            title: "{best_title}",
                                                        }
                                                    }
                                                }
                                            },
                                            BWMode::Average => rsx! {
                                                if has_data && avg_f > 0.0 {
                                                    span { class: "bw-val best", "{format_pnl(avg)}" }
                                                }
                                                div { class: "bw-bar-wrap top",
                                                    if has_data && avg_f > 0.0 {
                                                        div {
                                                            class: "bw-bar best",
                                                            style: "height: {avg_bar_pct}%;",
                                                            title: "{avg_title}",
                                                        }
                                                    }
                                                }
                                            },
                                        }
                                    }
                                    // zero axis line (avg marker only in BestWorst mode)
                                    div { class: "bw-axis-line",
                                        if has_data && current_mode == BWMode::BestWorst {
                                            div {
                                                class: if avg_f >= 0.0 { "bw-avg-marker positive" } else { "bw-avg-marker negative" },
                                                style: "{avg_offset_style}",
                                                title: "{avg_title}",
                                            }
                                        }
                                    }
                                    // bottom half — worst (Best/Worst mode) OR negative avg (Average mode)
                                    div { class: "bw-half bottom",
                                        match current_mode {
                                            BWMode::BestWorst => rsx! {
                                                div { class: "bw-bar-wrap bottom",
                                                    if has_data && worst_f < 0.0 {
                                                        div {
                                                            class: "bw-bar worst",
                                                            style: "height: {worst_pct}%;",
                                                            title: "{worst_title}",
                                                        }
                                                    }
                                                }
                                                if has_data && worst_f < 0.0 {
                                                    span { class: "bw-val worst", "{format_pnl(worst)}" }
                                                }
                                            },
                                            BWMode::Average => rsx! {
                                                div { class: "bw-bar-wrap bottom",
                                                    if has_data && avg_f < 0.0 {
                                                        div {
                                                            class: "bw-bar worst",
                                                            style: "height: {avg_bar_pct}%;",
                                                            title: "{avg_title}",
                                                        }
                                                    }
                                                }
                                                if has_data && avg_f < 0.0 {
                                                    span { class: "bw-val worst", "{format_pnl(avg)}" }
                                                }
                                            },
                                        }
                                    }
                                }
                                div { class: "bw-label", "{WEEKDAY_NAMES[wd]}" }
                                div { class: "bw-meta",
                                    if has_data {
                                        "{count}d \u{00B7} avg "
                                        span {
                                            class: if avg_f >= 0.0 { "positive" } else { "negative" },
                                            "{format_pnl(avg)}"
                                        }
                                    } else {
                                        "no data"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Compact summary: best/worst weekday overall
            {
                let mut best_wd_idx = None;
                let mut worst_wd_idx = None;
                let mut best_avg = f64::MIN;
                let mut worst_avg = f64::MAX;
                for (i, (_, _, sum, count)) in buckets.iter().enumerate() {
                    if *count == 0 { continue; }
                    let avg = sum.to_f64().unwrap_or(0.0) / *count as f64;
                    if avg > best_avg { best_avg = avg; best_wd_idx = Some(i); }
                    if avg < worst_avg { worst_avg = avg; worst_wd_idx = Some(i); }
                }
                rsx! {
                    div { class: "bw-summary",
                        if let Some(i) = best_wd_idx {
                            div { class: "bw-summary-item",
                                span { class: "bw-summary-label", "Best weekday on average:" }
                                span { class: "bw-summary-val positive",
                                    "{WEEKDAY_NAMES[i]} ({format_decimal(Decimal::from_f64_retain(best_avg).unwrap_or(Decimal::ZERO))}/day)"
                                }
                            }
                        }
                        if let Some(i) = worst_wd_idx {
                            div { class: "bw-summary-item",
                                span { class: "bw-summary-label", "Worst weekday on average:" }
                                span { class: "bw-summary-val negative",
                                    "{WEEKDAY_NAMES[i]} ({format_decimal(Decimal::from_f64_retain(worst_avg).unwrap_or(Decimal::ZERO))}/day)"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
