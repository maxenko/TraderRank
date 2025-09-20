use crate::models::Trade;
use rust_decimal::Decimal;

pub struct TimePatternAnalyzer;

impl TimePatternAnalyzer {
    pub fn identify_best_trading_periods(trades: &[Trade]) -> Vec<TradingPeriod> {
        let mut periods = vec![
            TradingPeriod::new("Pre-Market", 4, 9),
            TradingPeriod::new("Market Open", 9, 10),
            TradingPeriod::new("Morning", 10, 12),
            TradingPeriod::new("Lunch", 12, 13),
            TradingPeriod::new("Afternoon", 13, 15),
            TradingPeriod::new("Power Hour", 15, 16),
            TradingPeriod::new("After-Hours", 16, 20),
        ];

        for period in &mut periods {
            period.calculate_metrics(trades);
        }

        periods.sort_by(|a, b| b.total_pnl.cmp(&a.total_pnl));
        periods
    }
}

#[derive(Debug, Clone)]
pub struct TradingPeriod {
    pub name: String,
    pub start_hour: u32,
    pub end_hour: u32,
    pub total_trades: u32,
    pub total_pnl: Decimal,
    pub win_rate: f64,
    pub avg_pnl_per_trade: Decimal,
}

impl TradingPeriod {
    fn new(name: &str, start_hour: u32, end_hour: u32) -> Self {
        Self {
            name: name.to_string(),
            start_hour,
            end_hour,
            total_trades: 0,
            total_pnl: Decimal::ZERO,
            win_rate: 0.0,
            avg_pnl_per_trade: Decimal::ZERO,
        }
    }

    fn calculate_metrics(&mut self, trades: &[Trade]) {
        let period_trades: Vec<&Trade> = trades
            .iter()
            .filter(|t| {
                let hour = t.hour_of_day();
                hour >= self.start_hour && hour < self.end_hour
            })
            .collect();

        self.total_trades = period_trades.len() as u32;

        let mut wins = 0;
        let mut losses = 0;

        for trade in &period_trades {
            let pnl = trade.net_pnl();
            self.total_pnl += pnl;

            if pnl > Decimal::ZERO {
                wins += 1;
            } else if pnl < Decimal::ZERO {
                losses += 1;
            }
        }

        if self.total_trades > 0 {
            self.avg_pnl_per_trade = self.total_pnl / Decimal::from(self.total_trades);
            self.win_rate = if wins + losses > 0 {
                (wins as f64) / ((wins + losses) as f64) * 100.0
            } else {
                0.0
            };
        }
    }
}