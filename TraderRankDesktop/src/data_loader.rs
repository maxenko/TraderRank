use crate::models::{ProcessedData, TradingSummary};
use crate::state::{AppState, WeeklyRConfig, SymbolStats, HourlyStats};
use anyhow::{Context, Result};
use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::path::PathBuf;

/// Find the Data directory — same logic as CLI:
/// project parent / Data (i.e. D:\GitHub\TraderRank\Data)
fn find_data_dir() -> Option<PathBuf> {
    // Try relative to executable first
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            // From target/debug/ -> go up to TraderRankDesktop, then up to TraderRank
            let candidates = [
                parent.join("../../Data"),       // target/debug -> TraderRankDesktop -> TraderRank
                parent.join("../../../Data"),     // deeper nesting
                parent.join("Data"),              // next to executable
            ];
            for c in &candidates {
                if c.exists() {
                    return Some(c.canonicalize().unwrap_or_else(|_| c.to_path_buf()));
                }
            }
        }
    }

    // Try relative to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join("../Data"),          // TraderRankDesktop -> TraderRank/Data
            cwd.join("Data"),             // cwd is TraderRank
            cwd.join("../../Data"),       // nested deeper
        ];
        for c in &candidates {
            if c.exists() {
                return Some(c.canonicalize().unwrap_or_else(|_| c.to_path_buf()));
            }
        }
    }

    None
}

/// Load cached analysis from Data/processed_data.json
pub fn load_processed_data() -> Result<Option<ProcessedData>> {
    let data_dir = match find_data_dir() {
        Some(d) => d,
        None => return Ok(None),
    };

    let json_path = data_dir.join("processed_data.json");
    if !json_path.exists() {
        return Ok(None);
    }

    let json_str = std::fs::read_to_string(&json_path)
        .with_context(|| format!("Failed to read {:?}", json_path))?;

    let data: ProcessedData = serde_json::from_str(&json_str)
        .with_context(|| "Failed to deserialize processed_data.json")?;

    Ok(Some(data))
}

/// Convert CLI's TradingSummary into the desktop's AppState
pub fn trading_summary_to_app_state(summary: TradingSummary) -> AppState {
    let daily_summaries = summary.daily_summaries;
    let weekly_summaries = summary.weekly_summaries;
    let monthly_summaries = summary.monthly_summaries;

    let total_pnl = summary.total_pnl;
    let total_trades = summary.total_trades;
    let total_wins: u32 = daily_summaries.iter().map(|d| d.winning_trades).sum();
    let total_losses: u32 = daily_summaries.iter().map(|d| d.losing_trades).sum();
    let total_commission: Decimal = daily_summaries.iter().map(|d| d.total_commission).sum();
    let total_gross: Decimal = daily_summaries.iter().map(|d| d.gross_pnl).sum();
    let overall_win_rate = summary.overall_win_rate;

    // Compute avg win/loss from daily summaries
    let avg_win = if total_wins > 0 {
        let sum_wins: Decimal = daily_summaries
            .iter()
            .map(|d| d.avg_win * Decimal::from(d.winning_trades))
            .sum();
        sum_wins / Decimal::from(total_wins)
    } else {
        Decimal::ZERO
    };

    let avg_loss = if total_losses > 0 {
        let sum_losses: Decimal = daily_summaries
            .iter()
            .map(|d| d.avg_loss * Decimal::from(d.losing_trades))
            .sum();
        sum_losses / Decimal::from(total_losses)
    } else {
        Decimal::ZERO
    };

    // Expectancy — pure Decimal path
    let win_pct = if total_trades > 0 {
        Decimal::from(total_wins) / Decimal::from(total_trades)
    } else {
        Decimal::ZERO
    };
    let loss_pct = Decimal::ONE - win_pct;
    let expectancy = (win_pct * avg_win) + (loss_pct * avg_loss);

    // Profit factor from daily aggregated avg_win/avg_loss (trade-level data not available from cache)
    let total_win_amount = avg_win * Decimal::from(total_wins);
    let total_loss_amount = avg_loss.abs() * Decimal::from(total_losses);
    let profit_factor = if total_loss_amount > Decimal::ZERO {
        Some(total_win_amount / total_loss_amount)
    } else {
        None
    };

    // Max drawdown
    let mut peak = Decimal::ZERO;
    let mut cumulative = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for d in &daily_summaries {
        cumulative += d.realized_pnl;
        if cumulative > peak {
            peak = cumulative;
        }
        let dd = peak - cumulative;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    // Sharpe ratio (annualized, sample variance N-1)
    let daily_returns: Vec<f64> = daily_summaries
        .iter()
        .map(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d.realized_pnl).unwrap_or(0.0))
        .collect();
    let n = daily_returns.len() as f64;
    let mean_return = if n > 0.0 { daily_returns.iter().sum::<f64>() / n } else { 0.0 };
    let sharpe = if n > 1.0 {
        let variance = daily_returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        if std_dev > 0.0 {
            (mean_return / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Payoff ratio
    let payoff_ratio = if avg_loss != Decimal::ZERO {
        Some(avg_win / avg_loss.abs())
    } else {
        None
    };

    // Streaks (day-level)
    let mut current_streak: i32 = 0;
    let mut max_win_streak: u32 = 0;
    let mut max_loss_streak: u32 = 0;
    let mut cur_win: u32 = 0;
    let mut cur_loss: u32 = 0;
    for d in &daily_summaries {
        if d.realized_pnl > Decimal::ZERO {
            cur_win += 1;
            cur_loss = 0;
            current_streak = cur_win as i32;
            if cur_win > max_win_streak { max_win_streak = cur_win; }
        } else if d.realized_pnl < Decimal::ZERO {
            cur_loss += 1;
            cur_win = 0;
            current_streak = -(cur_loss as i32);
            if cur_loss > max_loss_streak { max_loss_streak = cur_loss; }
        }
    }

    // Symbol stats — aggregate from daily summaries
    let mut sym_map: HashMap<String, (Decimal, u32, u32)> = HashMap::new();
    for d in &daily_summaries {
        if d.symbols_traded.is_empty() || d.total_trades == 0 {
            continue;
        }
        let per_sym_pnl = d.realized_pnl / Decimal::from(d.symbols_traded.len().max(1) as u32);
        let per_sym_trades = d.total_trades / d.symbols_traded.len().max(1) as u32;
        let per_sym_wins = d.winning_trades / d.symbols_traded.len().max(1) as u32;
        for sym in &d.symbols_traded {
            let entry = sym_map.entry(sym.clone()).or_insert((Decimal::ZERO, 0, 0));
            entry.0 += per_sym_pnl;
            entry.1 += per_sym_trades;
            entry.2 += per_sym_wins;
        }
    }
    let mut symbol_stats: Vec<SymbolStats> = sym_map
        .into_iter()
        .map(|(sym, (pnl, trades, wins))| {
            let wr = if trades > 0 { (wins as f64 / trades as f64) * 100.0 } else { 0.0 };
            SymbolStats { symbol: sym, total_pnl: pnl, trade_count: trades, win_rate: wr }
        })
        .collect();
    symbol_stats.sort_by(|a, b| b.total_pnl.cmp(&a.total_pnl));

    // Hourly stats
    let mut hourly_map: HashMap<u32, (Decimal, u32, f64)> = HashMap::new();
    for d in &daily_summaries {
        for ts in &d.time_slot_performance {
            let entry = hourly_map.entry(ts.hour).or_insert((Decimal::ZERO, 0, 0.0));
            entry.0 += ts.pnl;
            entry.1 += ts.trades;
            entry.2 += ts.win_rate;
        }
    }
    let day_count = daily_summaries.len().max(1) as f64;
    let mut hourly_stats: Vec<HourlyStats> = hourly_map
        .into_iter()
        .map(|(hour, (pnl, trades, wr_sum))| HourlyStats {
            hour,
            total_pnl: pnl,
            trade_count: trades,
            avg_win_rate: wr_sum / day_count,
        })
        .collect();
    hourly_stats.sort_by_key(|h| h.hour);

    // Daily P&L for equity chart
    let daily_pnls: Vec<(String, Decimal)> = daily_summaries
        .iter()
        .map(|d| (d.date.format("%m/%d").to_string(), d.realized_pnl))
        .collect();

    // Weekly R configs (default R=$100)
    let r_configs: Vec<WeeklyRConfig> = weekly_summaries
        .iter()
        .map(|w| {
            let date = w.start_date.date_naive();
            let days_from_mon = date.weekday().num_days_from_monday();
            let monday = date - chrono::Duration::days(days_from_mon as i64);
            WeeklyRConfig {
                week_start: monday,
                r_value: dec!(100),
            }
        })
        .collect();

    AppState {
        daily_summaries,
        weekly_summaries,
        monthly_summaries,
        trades: Vec::new(), // trades not stored in processed_data.json
        total_pnl,
        total_trades,
        total_wins,
        total_losses,
        total_commission,
        total_gross,
        overall_win_rate,
        avg_win,
        avg_loss,
        expectancy,
        profit_factor,
        sharpe_ratio: sharpe,
        max_drawdown: max_dd,
        payoff_ratio,
        current_streak,
        max_win_streak,
        max_loss_streak,
        symbol_stats,
        hourly_stats,
        daily_pnls,
        r_configs,
    }
}

/// Try to load real data, fall back to sample data
pub fn load_app_state() -> AppState {
    match load_processed_data() {
        Ok(Some(data)) => {
            eprintln!("Loaded {} trading days from processed_data.json (last processed: {})",
                data.summary.daily_summaries.len(),
                data.last_processed.format("%Y-%m-%d %H:%M"));
            trading_summary_to_app_state(data.summary)
        }
        Ok(None) => {
            eprintln!("No processed_data.json found — using sample data. Run the TraderRank CLI first to process CSVs.");
            crate::sample_data::generate_sample_data()
        }
        Err(e) => {
            eprintln!("Error loading data: {} — falling back to sample data", e);
            crate::sample_data::generate_sample_data()
        }
    }
}
