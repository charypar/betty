use std::fmt::Display;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

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

#[derive(Debug, PartialEq, Clone)]
pub struct Exit {
    pub position_id: String,
    pub price: Points,
    pub time: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub enum Order {
    Open(Entry),
    Close(Exit),
    Stop(Exit),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TradeStatus {
    Open,
    Closed,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TradeOutcome {
    Profit,
    Loss,
}

impl Display for TradeOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeOutcome::Profit => write!(f, "Profit"),
            TradeOutcome::Loss => write!(f, "Loss"),
        }
    }
}

// A row in a trade log
#[derive(Debug, PartialEq, Clone)]
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
    pub profit: CurrencyAmount,
    pub risk_reward: Decimal,
}

impl Trade {
    pub fn open(entry: &Entry, latest_price: Price) -> Self {
        let price_diff = match entry.direction {
            Direction::Buy => latest_price.bid - entry.price,
            Direction::Sell => latest_price.ask - entry.price,
        };
        let profit = match entry.direction {
            Direction::Buy => entry.size * (latest_price.bid - entry.price),
            Direction::Sell => entry.size * (entry.price - latest_price.ask),
        };
        let outcome = if profit.amount > dec!(0) {
            TradeOutcome::Profit
        } else {
            TradeOutcome::Loss
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
            outcome,
            price_diff,
            profit,
            risk_reward: (profit / risk).unwrap(), // both numbers are derived from o.size
        }
    }

    pub fn closed(entry: &Entry, exit: &Exit) -> Self {
        let price_diff = exit.price - entry.price;
        let profit = match entry.direction {
            Direction::Buy => entry.size * (exit.price - entry.price),
            Direction::Sell => entry.size * (entry.price - exit.price),
        };
        let outcome = if profit.amount > dec!(0) {
            TradeOutcome::Profit
        } else {
            TradeOutcome::Loss
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
            outcome,
            price_diff,
            profit,
            risk_reward: (profit / risk).unwrap(), // both numbers are derived from o.size
        }
    }

    pub fn exit(&self, price: Price, time: DateTime<Utc>) -> Exit {
        Exit {
            position_id: self.id.clone(),
            price: match self.direction {
                Direction::Buy => price.bid,
                Direction::Sell => price.ask,
            },
            time,
        }
    }
}
