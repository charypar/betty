use chrono::{DateTime, Utc};

use super::price::{CurrencyAmount, Points};

#[derive(Debug, PartialEq)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub position_id: String,
    pub direction: Direction,
    pub price: Points,
    pub stop: Points,
    pub size: CurrencyAmount,
    pub time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Exit {
    pub position_id: String,
    pub price: Points,
    pub time: DateTime<Utc>,
}

#[derive(Debug)]
pub enum Order {
    Open(Entry),
    Close(Exit),
}
