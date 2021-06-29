use chrono::{DateTime, Utc};

use super::price::{CurrencyAmount, Points};

#[derive(Debug)]
pub enum Direction {
    Buy,
    Sell,
}

#[derive(Debug)]
pub struct Entry {
    position_id: String,
    direction: Direction,
    price: Points,
    stop: Points,
    size: CurrencyAmount,
    time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Exit {
    position_id: String,
    price: Points,
    time: DateTime<Utc>,
}

#[derive(Debug)]
pub enum Order {
    Open(Entry),
    Close(Exit),
}
