use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: String,
    pub side: Side,
    pub quantity: Decimal,
    pub fill_price: Decimal,
    pub time: DateTime<Utc>,
    pub net_amount: Decimal,
    pub commission: Decimal,
}

impl PartialEq for Trade {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
            && self.side == other.side
            && self.quantity == other.quantity
            && self.fill_price == other.fill_price
            && self.time == other.time
    }
}

impl Eq for Trade {}

impl Hash for Trade {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.symbol.hash(state);
        self.side.hash(state);
        self.quantity.hash(state);
        self.fill_price.hash(state);
        self.time.hash(state);
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: DateTime<Utc>,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub realized_pnl: Decimal,
    pub gross_pnl: Decimal,
    pub total_commission: Decimal,
    pub total_volume: Decimal,
    pub win_rate: f64,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub largest_win: Decimal,
    pub largest_loss: Decimal,
    pub symbols_traded: Vec<String>,
    pub time_slot_performance: Vec<TimeSlotPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlotPerformance {
    pub hour: u32,
    pub trades: u32,
    pub pnl: Decimal,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklySummary {
    pub week_number: u32,
    pub year: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub realized_pnl: Decimal,
    pub gross_pnl: Decimal,
    pub total_commission: Decimal,
    pub total_volume: Decimal,
    pub win_rate: f64,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub largest_win: Decimal,
    pub largest_loss: Decimal,
    pub best_day: Option<(DateTime<Utc>, Decimal)>,
    pub worst_day: Option<(DateTime<Utc>, Decimal)>,
    pub trading_days: u32,
    pub profitable_days: u32,
    pub avg_daily_pnl: Decimal,
    pub symbols_traded: Vec<String>,
    pub daily_summaries: Vec<DailySummary>,
}

impl WeeklySummary {
    pub fn profit_factor(&self) -> Option<Decimal> {
        if self.avg_loss != Decimal::ZERO && self.losing_trades > 0 {
            let total_wins = self.avg_win * Decimal::from(self.winning_trades);
            let total_losses = self.avg_loss.abs() * Decimal::from(self.losing_trades);
            if total_losses != Decimal::ZERO {
                return Some(total_wins / total_losses);
            }
        }
        None
    }
}

impl DailySummary {
    pub fn profit_factor(&self) -> Option<Decimal> {
        if self.avg_loss != Decimal::ZERO && self.losing_trades > 0 {
            let total_wins = self.avg_win * Decimal::from(self.winning_trades);
            let total_losses = self.avg_loss.abs() * Decimal::from(self.losing_trades);
            if total_losses != Decimal::ZERO {
                return Some(total_wins / total_losses);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySummary {
    pub year: i32,
    pub month: u32,
    pub month_name: String,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub realized_pnl: Decimal,
    pub gross_pnl: Decimal,
    pub total_commission: Decimal,
    pub total_volume: Decimal,
    pub win_rate: f64,
    pub trading_days: u32,
    pub profitable_days: u32,
    pub avg_daily_pnl: Decimal,
    pub best_day: Option<(DateTime<Utc>, Decimal)>,
    pub worst_day: Option<(DateTime<Utc>, Decimal)>,
}

/// Matches the CLI's TradingSummary for JSON deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSummary {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub daily_summaries: Vec<DailySummary>,
    pub weekly_summaries: Vec<WeeklySummary>,
    #[serde(default)]
    pub monthly_summaries: Vec<MonthlySummary>,
    pub total_pnl: Decimal,
    pub total_volume: Decimal,
    pub total_trades: u32,
    pub overall_win_rate: f64,
    pub best_day: Option<(DateTime<Utc>, Decimal)>,
    pub worst_day: Option<(DateTime<Utc>, Decimal)>,
    pub best_week: Option<((i32, u32), Decimal)>,
    pub worst_week: Option<((i32, u32), Decimal)>,
    #[serde(default)]
    pub best_month: Option<((i32, u32), Decimal)>,
    #[serde(default)]
    pub worst_month: Option<((i32, u32), Decimal)>,
    pub most_profitable_hour: Option<(u32, Decimal)>,
    pub least_profitable_hour: Option<(u32, Decimal)>,
}

/// Matches the CLI's ProcessedData for loading cached analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedData {
    pub last_processed: DateTime<Utc>,
    pub processed_files: Vec<String>,
    pub summary: TradingSummary,
}
