use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::price::{CurrencyAmount, Points, Price};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Entry {
    pub position_id: String,
    pub direction: Direction,
    pub price: Points,
    pub stop: Points,
    pub size: CurrencyAmount,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Exit {
    pub position_id: String,
    pub price: Points,
    pub time: DateTime<Utc>,
}

#[derive(Debug)]
pub enum Order {
    Open(Entry),
    Close(Exit),
    Stop(Exit),
}

#[derive(Debug, PartialEq)]
pub enum TradeStatus {
    Open,
    Closed,
}

#[derive(Debug, PartialEq)]
pub enum TradeOutcome {
    Profit,
    Loss,
}

// A row in a trade log
#[derive(Debug, PartialEq)]
pub struct Trade {
    pub id: String,
    pub status: TradeStatus,
    // Entry
    pub direction: Direction,
    pub entry_time: DateTime<Utc>,
    pub entry_price: Points,
    // Exit
    pub exit_time: Option<DateTime<Utc>>,
    pub exit_price: Option<Points>,
    // Risk
    pub stop: Points,
    pub size: CurrencyAmount,
    pub risk: CurrencyAmount,
    // Outcome
    pub outcome: TradeOutcome,
    pub price_diff: Points,
    pub balance: CurrencyAmount,
    pub risk_reward: Decimal,
}

impl Trade {
    pub fn open(entry: &Entry, latest_price: Price) -> Self {
        let price_diff = match entry.direction {
            Direction::Buy => latest_price.bid - entry.price,
            Direction::Sell => latest_price.ask - entry.price,
        };
        let balance = match entry.direction {
            Direction::Buy => entry.size * (latest_price.bid - entry.price),
            Direction::Sell => entry.size * (entry.price - latest_price.ask),
        };
        let risk = entry.size * (entry.price - entry.stop).abs();

        Trade {
            id: entry.position_id.clone(),
            status: TradeStatus::Open,
            direction: entry.direction,
            entry_time: entry.time,
            entry_price: entry.price,
            exit_time: None,
            exit_price: None,
            stop: entry.stop,
            size: entry.size,
            risk,
            outcome: TradeOutcome::Profit,
            price_diff,
            balance,
            risk_reward: (balance / risk).unwrap(), // both numbers are derived from o.size
        }
    }

    pub fn closed(entry: &Entry, exit: &Exit) -> Self {
        let price_diff = exit.price - entry.price;
        let balance = match entry.direction {
            Direction::Buy => entry.size * (exit.price - entry.price),
            Direction::Sell => entry.size * (entry.price - exit.price),
        };
        let risk = entry.size * (entry.price - entry.stop).abs();

        Trade {
            id: entry.position_id.clone(),
            status: TradeStatus::Closed,
            direction: entry.direction,
            entry_time: entry.time,
            entry_price: entry.price,
            exit_time: Some(exit.time),
            exit_price: Some(exit.price),
            stop: entry.stop,
            size: entry.size,
            risk,
            outcome: TradeOutcome::Profit,
            price_diff,
            balance,
            risk_reward: (balance / risk).unwrap(), // both numbers are derived from o.size
        }
    }
}
