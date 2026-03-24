use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, DateTime, Utc};
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

const SEND_URL: &str = "https://ndcdyn.interactivebrokers.com/AccountManagement/FlexWebService/SendRequest";
const GET_URL: &str = "https://ndcdyn.interactivebrokers.com/AccountManagement/FlexWebService/GetStatement";
const MAX_RETRIES: u32 = 5;
const RETRY_DELAY_MS: u64 = 5000;

/// A single trade execution parsed from the Flex XML response.
struct FlexTrade {
    symbol: String,
    side: String,
    quantity: Decimal,
    price: Decimal,
    date: NaiveDate,
    time: Option<NaiveTime>,
    commission: Decimal,
    net_amount: Decimal,
}

/// Fetch trades from IB Flex Web Service and save as CSV to Data/Source/.
/// Returns the number of trades fetched.
pub async fn fetch_and_save(token: &str, query_id: &str) -> Result<usize> {
    if token.is_empty() || query_id.is_empty() {
        bail!("Token and Query ID are required");
    }

    let client = reqwest::Client::builder()
        .user_agent("TraderRank/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    // Step 1: SendRequest — get a reference code
    let send_url = format!("{}?t={}&q={}&v=3", SEND_URL, token, query_id);
    let send_resp = client.get(&send_url).send().await
        .context("Failed to connect to IB Flex Web Service")?;
    let send_body = send_resp.text().await
        .context("Failed to read SendRequest response")?;

    let ref_code = parse_reference_code(&send_body)?;

    // Step 2: GetStatement — poll until ready, with retries
    let mut statement_xml = String::new();
    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
        }

        let get_url = format!("{}?t={}&q={}&v=3", GET_URL, token, ref_code);
        let get_resp = client.get(&get_url).send().await
            .context("Failed to fetch statement")?;
        let body = get_resp.text().await
            .context("Failed to read GetStatement response")?;

        // Check for "still generating" response codes
        if body.contains("<ErrorCode>1019</ErrorCode>") || body.contains("<ErrorCode>1009</ErrorCode>") {
            continue; // Server busy, retry
        }
        if body.contains("<ErrorCode>") {
            // Extract error message
            let err_msg = extract_xml_text(&body, "ErrorMessage")
                .unwrap_or_else(|| "Unknown IB error".to_string());
            bail!("IB Flex error: {}", err_msg);
        }

        statement_xml = body;
        break;
    }

    if statement_xml.is_empty() {
        bail!("IB Flex Web Service did not return data after {} retries", MAX_RETRIES);
    }

    // Step 3: Parse trades from XML
    let trades = parse_flex_trades(&statement_xml)?;
    if trades.is_empty() {
        bail!("No trades found in the Flex response. Check your Flex Query configuration — it must include the Trades section.");
    }

    // Step 4: Save as CSV to %LOCALAPPDATA%\TraderRank\imports\
    let output_path = imports_dir()?;
    let csv_path = output_path.join("ib_flex_import.csv");
    write_trades_csv(&csv_path, &trades)?;

    Ok(trades.len())
}

/// Parse the ReferenceCode from the SendRequest XML response.
fn parse_reference_code(xml: &str) -> Result<String> {
    // Check for errors first
    if xml.contains("<ErrorCode>") {
        let code = extract_xml_text(xml, "ErrorCode").unwrap_or_default();
        let msg = extract_xml_text(xml, "ErrorMessage")
            .unwrap_or_else(|| "Unknown error".to_string());
        bail!("IB SendRequest error ({}): {}", code, msg);
    }

    extract_xml_text(xml, "ReferenceCode")
        .context("No ReferenceCode found in SendRequest response. Verify your token and query ID.")
}

/// Parse trade executions from the Flex statement XML.
/// Handles both Activity Statement (<Trade>) and Trade Confirmation (<TradeConfirm>) formats.
fn parse_flex_trades(xml: &str) -> Result<Vec<FlexTrade>> {
    let doc = roxmltree::Document::parse(xml)
        .context("Failed to parse Flex XML response")?;

    let mut trades = Vec::new();

    // Look for <Trade> elements (Activity Statement) and <TradeConfirm> elements
    for node in doc.descendants() {
        let tag = node.tag_name().name();
        if tag != "Trade" && tag != "TradeConfirm" {
            continue;
        }

        // Skip summary/header rows — only want EXECUTION level detail
        if let Some(level) = node.attribute("levelOfDetail") {
            if level != "EXECUTION" && level != "ORDER" {
                continue;
            }
        }

        // Only process stock and option trades (skip forex conversions, dividends, etc.)
        if let Some(asset) = node.attribute("assetCategory") {
            if asset == "CASH" || asset == "BOND" {
                continue;
            }
        }

        let symbol = match node.attribute("symbol") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        let buy_sell = match node.attribute("buySell") {
            Some(s) => s.to_string(),
            None => continue,
        };

        let side = match buy_sell.to_uppercase().as_str() {
            "BUY" => "Buy",
            "SELL" => "Sell",
            "BUY (Ca.)" => "Buy",   // cancelled/corrected
            "SELL (Ca.)" => "Sell",
            _ => continue,
        }.to_string();

        let quantity = parse_attr_decimal(node, "quantity")?.abs();
        if quantity == Decimal::ZERO {
            continue;
        }
        let price = parse_attr_decimal(node, "tradePrice")?;
        let commission = parse_attr_decimal(node, "ibCommission")?.abs();
        let net_amount = parse_attr_decimal(node, "netCash")
            .or_else(|_| parse_attr_decimal(node, "proceeds"))
            .unwrap_or(quantity * price);

        // Parse date and time
        // IB Flex uses multiple formats:
        //   - Combined: dateTime="YYYYMMDD;HHMMSS" (Activity Statements)
        //   - Separate: tradeDate="YYYYMMDD" + tradeTime="HHMMSS"
        //   - orderTime="YYYYMMDD;HHMMSS" as fallback
        let (date, time) = parse_trade_datetime(&node)?;

        trades.push(FlexTrade {
            symbol,
            side,
            quantity,
            price,
            date,
            time,
            commission,
            net_amount,
        });
    }

    // Sort by date+time so FIFO matching works correctly
    trades.sort_by(|a, b| {
        let dt_a = NaiveDateTime::new(a.date, a.time.unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
        let dt_b = NaiveDateTime::new(b.date, b.time.unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
        dt_a.cmp(&dt_b)
    });

    Ok(trades)
}

/// Parse date and time from an IB Flex XML trade node.
/// Tries multiple attribute names and formats.
fn parse_trade_datetime(node: &roxmltree::Node) -> Result<(NaiveDate, Option<NaiveTime>)> {
    // Try combined dateTime first (most common in Activity Statements): "YYYYMMDD;HHMMSS"
    if let Some(dt_str) = node.attribute("dateTime").or_else(|| node.attribute("orderTime")) {
        if let Some((date_part, time_part)) = dt_str.split_once(';') {
            if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y%m%d") {
                let time = NaiveTime::parse_from_str(time_part, "%H%M%S")
                    .or_else(|_| NaiveTime::parse_from_str(time_part, "%H:%M:%S"))
                    .ok();
                return Ok((date, time));
            }
        }
        // Try as date-only
        if let Ok(date) = NaiveDate::parse_from_str(dt_str, "%Y%m%d") {
            return Ok((date, None));
        }
    }

    // Try separate tradeDate + tradeTime
    let date_str = node.attribute("tradeDate")
        .or_else(|| node.attribute("reportDate"))
        .unwrap_or("");
    let date = NaiveDate::parse_from_str(date_str, "%Y%m%d")
        .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y-%m-%d"))
        .with_context(|| format!("Invalid trade date: '{}' (no dateTime or tradeDate found)", date_str))?;

    let time = node.attribute("tradeTime").and_then(|t| {
        NaiveTime::parse_from_str(t, "%H%M%S")
            .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
            .ok()
    });

    Ok((date, time))
}

/// Parse a Decimal from an XML attribute, returning ZERO for empty/missing.
fn parse_attr_decimal(node: roxmltree::Node, attr: &str) -> Result<Decimal> {
    let val = node.attribute(attr).unwrap_or("0");
    if val.is_empty() || val == "--" || val == "-" {
        return Ok(Decimal::ZERO);
    }
    Decimal::from_str(val)
        .with_context(|| format!("Invalid decimal in {}: {}", attr, val))
}

/// Extract text content between <Tag>...</Tag> using simple string search.
/// Used for the simple SendRequest response (not the full statement).
fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Get or create the imports directory: %LOCALAPPDATA%\TraderRank\imports\
fn imports_dir() -> Result<PathBuf> {
    let dir = crate::app_dirs::imports_dir()
        .context("Could not determine %LOCALAPPDATA% path")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create imports directory: {:?}", dir))?;
    Ok(dir)
}

/// Write trades as CSV in the standard format our parser reads:
/// Symbol,Side,Quantity,Fill Price,Time,Net Amount,Commission
fn write_trades_csv(path: &PathBuf, trades: &[FlexTrade]) -> Result<()> {
    let mut lines = Vec::with_capacity(trades.len() + 1);
    lines.push("Symbol,Side,Qty,Fill Price,Time,Net Amount,Commission".to_string());

    for (i, t) in trades.iter().enumerate() {
        let time = if let Some(tm) = t.time {
            NaiveDateTime::new(t.date, tm)
        } else {
            // Assign synthetic time preserving order
            let secs = (i as u32 / 1000) % 30;
            let millis = i as u32 % 1000;
            let tm = NaiveTime::from_hms_milli_opt(9, 30 + secs, 0, millis).unwrap();
            NaiveDateTime::new(t.date, tm)
        };
        let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(time, Utc);

        lines.push(format!(
            "{},{},{},{},{},{},{}",
            t.symbol,
            t.side,
            t.quantity,
            t.price,
            dt.format("%Y-%m-%d %H:%M:%S"),
            t.net_amount,
            t.commission,
        ));
    }

    std::fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write CSV to {:?}", path))?;

    eprintln!("Saved {} trades to {:?}", trades.len(), path);
    Ok(())
}
