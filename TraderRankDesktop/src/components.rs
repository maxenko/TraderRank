use dioxus::prelude::*;
use rust_decimal::Decimal;

#[component]
pub fn MetricCard(
    label: String,
    value: String,
    subtitle: Option<String>,
    positive: Option<bool>,
) -> Element {
    let value_class = match positive {
        Some(true) => "metric-value positive",
        Some(false) => "metric-value negative",
        None => "metric-value",
    };

    rsx! {
        div { class: "metric-card",
            div { class: "metric-label", "{label}" }
            div { class: "{value_class}", "{value}" }
            if let Some(sub) = subtitle {
                div { class: "metric-subtitle", "{sub}" }
            }
        }
    }
}

#[component]
pub fn TradeRow(
    time: String,
    symbol: String,
    side: String,
    qty: String,
    price: String,
    pnl: String,
    commission: String,
    is_positive: bool,
) -> Element {
    let pnl_class = if is_positive { "pnl positive" } else { "pnl negative" };
    let side_class = if side == "Buy" { "side buy" } else { "side sell" };

    rsx! {
        tr { class: "trade-row",
            td { "{time}" }
            td { class: "symbol", "{symbol}" }
            td { class: "{side_class}", "{side}" }
            td { "{qty}" }
            td { "{price}" }
            td { class: "{pnl_class}", "{pnl}" }
            td { class: "commission", "{commission}" }
        }
    }
}

pub fn format_decimal(d: Decimal) -> String {
    let s = format!("{:.2}", d);
    // Add thousand separators
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let is_neg = int_part.starts_with('-');
    let digits: String = int_part.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut result = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    let formatted: String = result.chars().rev().collect();
    if parts.len() > 1 {
        format!(
            "{}${}.{}",
            if is_neg { "-" } else { "" },
            formatted,
            parts[1]
        )
    } else {
        format!("{}${}", if is_neg { "-" } else { "" }, formatted)
    }
}

pub fn format_r(r_val: Decimal) -> String {
    if r_val >= Decimal::ZERO {
        format!("+{:.1}R", r_val)
    } else {
        format!("{:.1}R", r_val)
    }
}

pub fn format_pnl(d: Decimal) -> String {
    if d >= Decimal::ZERO {
        format!("+{}", format_decimal(d))
    } else {
        format_decimal(d)
    }
}
