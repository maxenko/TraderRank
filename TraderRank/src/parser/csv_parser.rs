use crate::models::{Trade, Side};
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::str::FromStr;

pub enum FileFormat {
    Trades,
    Positions,
    Unknown,
}

pub struct CsvParser;

impl CsvParser {
    pub fn detect_format(file_path: &Path) -> Result<FileFormat> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        if let Some(header_line) = lines.next() {
            let header = header_line?;
            let header_lower = header.to_lowercase();

            // Check for positions file indicators
            if header_lower.contains("unrealized") ||
               header_lower.contains("avg price") ||
               header_lower.contains("last price") ||
               header_lower.contains("position id") {
                return Ok(FileFormat::Positions);
            }

            // Check for trades file indicators
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
                println!("  ⚠️  Skipping positions file: {} (not a trades file)", file_name);
                return Ok(Vec::new());
            }
            FileFormat::Unknown => {
                let file_name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                println!("  ⚠️  Skipping unrecognized file format: {}", file_name);
                return Ok(Vec::new());
            }
            FileFormat::Trades => {
                // Continue with normal parsing
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
}