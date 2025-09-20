use crate::models::{Trade, Side};
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::str::FromStr;

pub struct CsvParser;

impl CsvParser {
    pub fn parse_file(file_path: &Path) -> Result<Vec<Trade>> {
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
}