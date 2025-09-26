use chrono::{DateTime, Utc, NaiveDateTime, Timelike};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum Side {
    Buy,
    Sell,
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
    pub fn gross_pnl(&self) -> Decimal {
        match self.side {
            Side::Buy => -self.net_amount,
            Side::Sell => self.net_amount,
        }
    }

    pub fn net_pnl(&self) -> Decimal {
        self.gross_pnl() - self.commission
    }

    pub fn hour_of_day(&self) -> u32 {
        self.time.hour()
    }

    pub fn parse_time(time_str: &str) -> anyhow::Result<DateTime<Utc>> {
        let naive = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }
}