use chrono::{DateTime, NaiveDateTime, Utc, Timelike};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

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

impl FromStr for Side {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "buy" | "long" => Ok(Side::Buy),
            "sell" | "short" => Ok(Side::Sell),
            _ => Err(anyhow::anyhow!("Invalid side: {}", s)),
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

impl Trade {
    pub fn hour_of_day(&self) -> u32 {
        self.time.hour()
    }

    pub fn parse_time(time_str: &str) -> anyhow::Result<DateTime<Utc>> {
        let naive = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
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

#[allow(dead_code)]
impl WeeklySummary {
    pub fn new(week_number: u32, year: i32, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Self {
        Self {
            week_number,
            year,
            start_date,
            end_date,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            realized_pnl: Decimal::ZERO,
            gross_pnl: Decimal::ZERO,
            total_commission: Decimal::ZERO,
            total_volume: Decimal::ZERO,
            win_rate: 0.0,
            avg_win: Decimal::ZERO,
            avg_loss: Decimal::ZERO,
            largest_win: Decimal::ZERO,
            largest_loss: Decimal::ZERO,
            best_day: None,
            worst_day: None,
            trading_days: 0,
            profitable_days: 0,
            avg_daily_pnl: Decimal::ZERO,
            symbols_traded: Vec::new(),
            daily_summaries: Vec::new(),
        }
    }

    pub fn update_from_daily_summaries(&mut self, daily_summaries: Vec<DailySummary>) {
        self.daily_summaries = daily_summaries;

        // Reset aggregates
        self.total_trades = 0;
        self.winning_trades = 0;
        self.losing_trades = 0;
        self.realized_pnl = Decimal::ZERO;
        self.gross_pnl = Decimal::ZERO;
        self.total_commission = Decimal::ZERO;
        self.total_volume = Decimal::ZERO;
        let mut all_symbols = HashSet::new();

        let mut total_wins_amount = Decimal::ZERO;
        let mut total_losses_amount = Decimal::ZERO;

        for daily in &self.daily_summaries {
            self.total_trades += daily.total_trades;
            self.winning_trades += daily.winning_trades;
            self.losing_trades += daily.losing_trades;
            self.realized_pnl += daily.realized_pnl;
            self.gross_pnl += daily.gross_pnl;
            self.total_commission += daily.total_commission;
            self.total_volume += daily.total_volume;

            total_wins_amount += daily.avg_win * Decimal::from(daily.winning_trades);
            total_losses_amount += daily.avg_loss * Decimal::from(daily.losing_trades);

            for symbol in &daily.symbols_traded {
                all_symbols.insert(symbol.clone());
            }

            // Track largest win/loss
            if daily.largest_win > self.largest_win {
                self.largest_win = daily.largest_win;
            }
            if daily.largest_loss < self.largest_loss {
                self.largest_loss = daily.largest_loss;
            }
        }

        // Calculate averages
        if self.winning_trades > 0 {
            self.avg_win = total_wins_amount / Decimal::from(self.winning_trades);
        }
        if self.losing_trades > 0 {
            self.avg_loss = total_losses_amount / Decimal::from(self.losing_trades);
        }

        // Calculate win rate
        if self.total_trades > 0 {
            self.win_rate = (self.winning_trades as f64) / (self.total_trades as f64) * 100.0;
        }

        // Update symbols traded
        self.symbols_traded = all_symbols.into_iter().collect();
        self.symbols_traded.sort();

        // Calculate trading days and profitable days
        self.trading_days = self.daily_summaries.len() as u32;
        self.profitable_days = self.daily_summaries.iter()
            .filter(|d| d.realized_pnl > Decimal::ZERO)
            .count() as u32;

        // Calculate average daily P&L
        if self.trading_days > 0 {
            self.avg_daily_pnl = self.realized_pnl / Decimal::from(self.trading_days);
        }

        // Find best and worst days
        self.best_day = self.daily_summaries.iter()
            .max_by_key(|d| d.realized_pnl)
            .map(|d| (d.date, d.realized_pnl));

        self.worst_day = self.daily_summaries.iter()
            .min_by_key(|d| d.realized_pnl)
            .map(|d| (d.date, d.realized_pnl));
    }

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

#[allow(dead_code)]
impl DailySummary {
    pub fn new(date: DateTime<Utc>) -> Self {
        Self {
            date,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            realized_pnl: Decimal::ZERO,
            gross_pnl: Decimal::ZERO,
            total_commission: Decimal::ZERO,
            total_volume: Decimal::ZERO,
            win_rate: 0.0,
            avg_win: Decimal::ZERO,
            avg_loss: Decimal::ZERO,
            largest_win: Decimal::ZERO,
            largest_loss: Decimal::ZERO,
            symbols_traded: Vec::new(),
            time_slot_performance: Vec::new(),
        }
    }

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

impl MonthlySummary {
    pub fn new(year: i32, month: u32) -> Self {
        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }.to_string();

        Self {
            year,
            month,
            month_name,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            realized_pnl: Decimal::ZERO,
            gross_pnl: Decimal::ZERO,
            total_commission: Decimal::ZERO,
            total_volume: Decimal::ZERO,
            win_rate: 0.0,
            trading_days: 0,
            profitable_days: 0,
            avg_daily_pnl: Decimal::ZERO,
            best_day: None,
            worst_day: None,
        }
    }

    pub fn update_from_daily_summaries(&mut self, daily_summaries: &[DailySummary]) {
        self.total_trades = 0;
        self.winning_trades = 0;
        self.losing_trades = 0;
        self.realized_pnl = Decimal::ZERO;
        self.gross_pnl = Decimal::ZERO;
        self.total_commission = Decimal::ZERO;
        self.total_volume = Decimal::ZERO;

        for daily in daily_summaries {
            self.total_trades += daily.total_trades;
            self.winning_trades += daily.winning_trades;
            self.losing_trades += daily.losing_trades;
            self.realized_pnl += daily.realized_pnl;
            self.gross_pnl += daily.gross_pnl;
            self.total_commission += daily.total_commission;
            self.total_volume += daily.total_volume;
        }

        if self.total_trades > 0 {
            self.win_rate = (self.winning_trades as f64) / (self.total_trades as f64) * 100.0;
        }

        self.trading_days = daily_summaries.len() as u32;
        self.profitable_days = daily_summaries.iter()
            .filter(|d| d.realized_pnl > Decimal::ZERO)
            .count() as u32;

        if self.trading_days > 0 {
            self.avg_daily_pnl = self.realized_pnl / Decimal::from(self.trading_days);
        }

        self.best_day = daily_summaries.iter()
            .max_by_key(|d| d.realized_pnl)
            .map(|d| (d.date, d.realized_pnl));

        self.worst_day = daily_summaries.iter()
            .min_by_key(|d| d.realized_pnl)
            .map(|d| (d.date, d.realized_pnl));
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MatchedTrade {
    pub symbol: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub side: String,           // "Long" (bought then sold)
    pub quantity: Decimal,
    pub entry_price: Decimal,   // weighted average if multiple fills
    pub exit_price: Decimal,    // weighted average if multiple fills
    pub gross_pnl: Decimal,     // (exit - entry) * qty
    pub commission: Decimal,    // total commission for this round trip
    pub net_pnl: Decimal,       // gross - commission
    pub entry_fills: u32,       // number of buy executions
    pub exit_fills: u32,        // number of sell executions
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
