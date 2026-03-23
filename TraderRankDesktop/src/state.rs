use crate::models::*;
use chrono::NaiveDate;
use rust_decimal::Decimal;

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
}

impl AppState {
    pub fn r_value_for_week(&self, week_start: NaiveDate) -> Decimal {
        self.r_configs
            .iter()
            .find(|c| c.week_start == week_start)
            .map(|c| c.r_value)
            .unwrap_or(Decimal::new(100, 0))
    }

    pub fn pnl_in_r(&self, pnl: Decimal, r_value: Decimal) -> Decimal {
        if r_value == Decimal::ZERO {
            Decimal::ZERO
        } else {
            pnl / r_value
        }
    }
}
