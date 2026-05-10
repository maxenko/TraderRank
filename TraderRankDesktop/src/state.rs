use crate::models::*;
use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Trade outcome classification using R-based threshold.
/// Winner: pnl >= 0.5R, Lossless: -$1 <= pnl < 0.5R, Loser: pnl < -$1
/// "pnl" is net (after commissions) or gross depending on `StatsConfig`.
/// Sub-dollar losses are treated as Lossless — they're noise (commission residue,
/// rounding) rather than meaningful losing trades.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeOutcome {
    Winner,
    Lossless,
    Loser,
}

/// User-toggleable display config that affects how stats are computed.
/// Provided as `Signal<StatsConfig>` via Dioxus context so any view can read it
/// and react instantly when the toggle changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsConfig {
    /// When true, all P&L values used for stats are NET of commissions
    /// (default — matches how brokers report). When false, GROSS — useful
    /// for evaluating raw strategy edge separate from execution costs.
    pub count_commissions: bool,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self { count_commissions: true }
    }
}

#[derive(Debug, Clone)]
pub struct WeeklyRConfig {
    pub week_start: NaiveDate,
    pub r_value: Decimal,
}

#[derive(Debug, Clone)]
pub struct SymbolStats {
    pub symbol: String,
    pub total_pnl: Decimal,
    pub trade_count: u32,
    pub win_rate: f64,
}

#[derive(Debug, Clone)]
pub struct HourlyStats {
    pub hour: u32,
    pub total_pnl: Decimal,
    pub trade_count: u32,
    pub avg_win_rate: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppState {
    // Raw data
    pub daily_summaries: Vec<DailySummary>,
    pub weekly_summaries: Vec<WeeklySummary>,
    pub monthly_summaries: Vec<MonthlySummary>,
    pub trades: Vec<Trade>,
    pub matched_trades: Vec<MatchedTrade>,

    // Overall metrics
    pub total_pnl: Decimal,
    pub total_trades: u32,
    pub total_wins: u32,
    pub total_losses: u32,
    pub total_commission: Decimal,
    pub total_gross: Decimal,
    pub overall_win_rate: f64,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub expectancy: Decimal,
    pub profit_factor: Option<Decimal>,
    pub sharpe_ratio: f64,
    pub max_drawdown: Decimal,
    pub payoff_ratio: Option<Decimal>,

    // Streaks
    pub current_streak: i32,
    pub max_win_streak: u32,
    pub max_loss_streak: u32,

    // Breakdowns
    pub symbol_stats: Vec<SymbolStats>,
    pub hourly_stats: Vec<HourlyStats>,
    pub daily_pnls: Vec<(String, Decimal)>,

    // R-unit config
    pub r_configs: Vec<WeeklyRConfig>,
    /// Fallback R value for any week without an explicit `WeeklyRConfig` entry.
    /// User-settable in Settings; new weeks pick this up as they roll in.
    pub default_r_value: Decimal,

    /// Maximum hold duration (days) for a round-trip to be counted as a daytrader
    /// trade. Trades exceeding this were already filtered out of `matched_trades`
    /// at load time; this field is kept for display/reference in Settings.
    pub max_hold_days: u32,

    // Exclusions: key -> reason
    pub exclusions: HashMap<String, String>,
}

impl AppState {
    pub fn r_value_for_week(&self, week_start: NaiveDate) -> Decimal {
        self.r_configs
            .iter()
            .find(|c| c.week_start == week_start)
            .map(|c| c.r_value)
            .unwrap_or(self.default_r_value)
    }

    pub fn pnl_in_r(&self, pnl: Decimal, r_value: Decimal) -> Decimal {
        if r_value == Decimal::ZERO {
            Decimal::ZERO
        } else {
            pnl / r_value
        }
    }

    /// Get the trade's effective P&L for stats — net (after commissions) when
    /// `count_commissions` is true, else gross. This is the single source of
    /// truth for "what counts as the trade's P&L" across the app.
    pub fn trade_pnl(&self, mt: &MatchedTrade, count_commissions: bool) -> Decimal {
        if count_commissions { mt.net_pnl } else { mt.gross_pnl }
    }

    /// Get the day's effective P&L for stats — net realized (default) when
    /// `count_commissions` is true, else gross.
    pub fn daily_pnl(&self, d: &DailySummary, count_commissions: bool) -> Decimal {
        if count_commissions { d.realized_pnl } else { d.gross_pnl }
    }

    /// Classify a matched trade as Winner/Lossless/Loser using R threshold.
    /// Winner: pnl >= 0.5R, Lossless: -$1 <= pnl < 0.5R, Loser: pnl < -$1.
    /// Sub-dollar losses are noise (commission residue, rounding) — folded into Lossless.
    /// Honors the `count_commissions` flag for the P&L value used in classification.
    pub fn trade_outcome_with(&self, mt: &MatchedTrade, count_commissions: bool) -> TradeOutcome {
        let pnl = self.trade_pnl(mt, count_commissions);
        if pnl < -Decimal::ONE {
            return TradeOutcome::Loser;
        }
        let days_from_mon = mt.exit_time.date_naive().weekday().num_days_from_monday();
        let monday = mt.exit_time.date_naive() - chrono::Duration::days(days_from_mon as i64);
        let r_val = self.r_value_for_week(monday);
        let threshold = r_val / Decimal::from(2); // 0.5R
        if pnl >= threshold {
            TradeOutcome::Winner
        } else {
            TradeOutcome::Lossless
        }
    }

    /// Generate exclusion key for a day (date_str = "YYYY-MM-DD")
    pub fn day_exclusion_key(date_str: &str) -> String {
        format!("day:{}", date_str)
    }

    /// Generate exclusion key for a matched trade
    pub fn trade_exclusion_key(mt: &MatchedTrade) -> String {
        format!("trade:{}:{}", mt.symbol, mt.exit_time.format("%Y-%m-%dT%H:%M:%S"))
    }

    /// Check if a day is excluded
    pub fn is_day_excluded(&self, date_str: &str) -> bool {
        self.exclusions.contains_key(&Self::day_exclusion_key(date_str))
    }

    /// Check if a matched trade is excluded (directly or via its day)
    pub fn is_trade_excluded(&self, mt: &MatchedTrade) -> bool {
        let day_str = mt.exit_time.date_naive().to_string();
        self.exclusions.contains_key(&Self::trade_exclusion_key(mt))
            || self.is_day_excluded(&day_str)
    }

    /// Get the exclusion reason for a day
    pub fn day_exclusion_reason(&self, date_str: &str) -> String {
        self.exclusions
            .get(&Self::day_exclusion_key(date_str))
            .cloned()
            .unwrap_or_default()
    }

    /// Get the exclusion reason for a matched trade
    #[allow(dead_code)]
    pub fn trade_exclusion_reason(&self, mt: &MatchedTrade) -> String {
        self.exclusions
            .get(&Self::trade_exclusion_key(mt))
            .cloned()
            .unwrap_or_default()
    }
}
