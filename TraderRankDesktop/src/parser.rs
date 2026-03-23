use crate::models::{Trade, Side};
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::str::FromStr;

pub enum FileFormat {
    Trades,
    InteractiveBrokers,
    Positions,
    Unknown,
}

pub struct CsvParser;

impl CsvParser {
    pub fn detect_format(file_path: &Path) -> Result<FileFormat> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        // Read up to 20 lines to detect format (IB files have headers in first ~10 lines)
        let lines: Vec<String> = reader.lines()
            .take(20)
            .filter_map(|l| l.ok())
            .collect();

        // Check for Interactive Brokers format
        // Look for "Transaction History,Header" line which defines the columns
        for line in &lines {
            let line_lower = line.to_lowercase();
            if line_lower.starts_with("transaction history,header,") {
                // Verify it has the expected IB columns
                if line_lower.contains("date") &&
                   line_lower.contains("symbol") &&
                   line_lower.contains("transaction type") &&
                   line_lower.contains("quantity") {
                    return Ok(FileFormat::InteractiveBrokers);
                }
            }
        }

        // Check first line for other formats
        if let Some(header) = lines.first() {
            let header_lower = header.to_lowercase();

            // Check for positions file indicators
            if header_lower.contains("unrealized") ||
               header_lower.contains("avg price") ||
               header_lower.contains("last price") ||
               header_lower.contains("position id") {
                return Ok(FileFormat::Positions);
            }

            // Check for trades file indicators (original format)
            if header_lower.contains("symbol") &&
               header_lower.contains("side") &&
               (header_lower.contains("qty") || header_lower.contains("quantity")) &&
               header_lower.contains("fill price") {
                return Ok(FileFormat::Trades);
            }
        }

        Ok(FileFormat::Unknown)
    }

    pub fn parse_file(file_path: &Path) -> Result<Vec<Trade>> {
        // First detect the file format
        let format = Self::detect_format(file_path)?;

        match format {
            FileFormat::Positions => {
                let file_name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                eprintln!("  Skipping positions file: {} (not a trades file)", file_name);
                return Ok(Vec::new());
            }
            FileFormat::Unknown => {
                let file_name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                eprintln!("  Skipping unrecognized file format: {}", file_name);
                return Ok(Vec::new());
            }
            FileFormat::InteractiveBrokers => {
                return Self::parse_ib_file(file_path);
            }
            FileFormat::Trades => {
                // Continue with normal parsing below
            }
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut trades = Vec::new();
        let mut lines = reader.lines();

        // Skip header
        if let Some(header_line) = lines.next() {
            let _header = header_line?;
        }

        for (line_num, line) in lines.enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let trade = Self::parse_line(&line)
                .with_context(|| format!("Failed to parse line {} in {:?}", line_num + 2, file_path))?;
            trades.push(trade);
        }

        Ok(trades)
    }

    fn parse_line(line: &str) -> Result<Trade> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 7 {
            return Err(anyhow::anyhow!("Invalid CSV format: expected 7 fields, got {}", parts.len()));
        }

        Ok(Trade {
            symbol: parts[0].to_string(),
            side: Side::from_str(parts[1])?,
            quantity: Decimal::from_str(parts[2])
                .with_context(|| format!("Invalid quantity: {}", parts[2]))?,
            fill_price: Decimal::from_str(parts[3])
                .with_context(|| format!("Invalid fill price: {}", parts[3]))?,
            time: Trade::parse_time(parts[4])?,
            net_amount: Decimal::from_str(parts[5])
                .with_context(|| format!("Invalid net amount: {}", parts[5]))?,
            commission: if parts.len() > 6 && !parts[6].trim().is_empty() {
                Decimal::from_str(parts[6])
                    .with_context(|| format!("Invalid commission: {}", parts[6]))?
            } else {
                Decimal::from(0)
            },
        })
    }

    /// Parse Interactive Brokers transaction history CSV format
    /// Format: Transaction History,Data,Date,Account,Description,Transaction Type,Symbol,Quantity,Price,Gross Amount,Commission,Net Amount
    fn parse_ib_file(file_path: &Path) -> Result<Vec<Trade>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut trades = Vec::new();
        let mut line_num = 0;

        for line_result in reader.lines() {
            line_num += 1;
            let line = line_result?;

            // Only process "Transaction History,Data," lines
            if !line.starts_with("Transaction History,Data,") {
                continue;
            }

            match Self::parse_ib_line(&line, line_num) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    // Log but don't fail on individual line errors
                    eprintln!("Warning: Skipping line {} in {:?}: {}", line_num, file_path, e);
                }
            }
        }

        Ok(trades)
    }

    /// Parse a single Interactive Brokers transaction line
    /// Format: Transaction History,Data,Date,Account,Description,Transaction Type,Symbol,Quantity,Price,Gross Amount,Commission,Net Amount
    /// Index:  0                   1    2    3       4           5                6      7        8     9            10         11
    fn parse_ib_line(line: &str, line_num: usize) -> Result<Trade> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 12 {
            return Err(anyhow::anyhow!("Invalid IB CSV format: expected 12 fields, got {}", parts.len()));
        }

        // Index mapping for IB format:
        // 0: "Transaction History"
        // 1: "Data"
        // 2: Date (YYYY-MM-DD)
        // 3: Account
        // 4: Description
        // 5: Transaction Type (Buy/Sell)
        // 6: Symbol
        // 7: Quantity (negative for sells)
        // 8: Price
        // 9: Gross Amount
        // 10: Commission (may be "-" for zero)
        // 11: Net Amount

        let date_str = parts[2].trim();
        let transaction_type = parts[5].trim();
        let symbol = parts[6].trim();
        let quantity_str = parts[7].trim();
        let price_str = parts[8].trim();
        let commission_str = parts[10].trim();
        let net_amount_str = parts[11].trim();

        // Parse date (IB format is YYYY-MM-DD, no time)
        // Assign a default time based on transaction order within the day
        // We'll use market open (09:30) as default
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .with_context(|| format!("Invalid date: {}", date_str))?;
        // Spread line_num across seconds and milliseconds to support up to 60,000 unique timestamps per day
        let secs = (line_num as u32 / 1000) % 30;
        let millis = line_num as u32 % 1000;
        let datetime = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_milli_opt(9, 30 + secs, 0, millis).unwrap());
        let time = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);

        // Parse side from transaction type
        let side = Side::from_str(transaction_type)?;

        // Parse quantity (IB uses negative for sells, we want absolute value)
        let quantity = Decimal::from_str(quantity_str)
            .with_context(|| format!("Invalid quantity: {}", quantity_str))?
            .abs();

        // Parse price
        let fill_price = Decimal::from_str(price_str)
            .with_context(|| format!("Invalid price: {}", price_str))?;

        // Parse commission (IB uses "-" for zero commission, and sometimes scientific notation)
        let commission = if commission_str == "-" || commission_str.is_empty() {
            Decimal::ZERO
        } else if commission_str.contains('E') || commission_str.contains('e') {
            // Handle scientific notation (e.g., "-6.6E-4")
            let float_val: f64 = commission_str.parse()
                .with_context(|| format!("Invalid commission (scientific notation): {}", commission_str))?;
            Decimal::from_f64_retain(float_val)
                .unwrap_or(Decimal::ZERO)
                .abs()
        } else {
            Decimal::from_str(commission_str)
                .with_context(|| format!("Invalid commission: {}", commission_str))?
                .abs()  // Commission should always be positive
        };

        // Parse net amount
        let net_amount = Decimal::from_str(net_amount_str)
            .with_context(|| format!("Invalid net amount: {}", net_amount_str))?;

        Ok(Trade {
            symbol: symbol.to_string(),
            side,
            quantity,
            fill_price,
            time,
            net_amount,
            commission,
        })
    }
}
