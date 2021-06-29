use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct CurrencyAmount(pub (Decimal, String));

// Point value with fixed decimal place position
// Different instruments will differ in this
pub type Points = Decimal;

// Price of an instrument. Excuse my finance n00b comments
pub struct Price {
    ask: Points, // price we buy at (market asks for this price level)
    bid: Points, // price we sell at (market bids to buy at this price level)
}

pub struct Frame {
    close: Price,
    high: Price,
    low: Price,
    open: Price,
    open_time: DateTime<Utc>,
}

pub enum Resolution {
    Second,
    Minute(usize),
    Hour(usize),
    Day,
    Weeek,
    Month,
}

pub struct PriceHistory {
    pub resolution: Resolution,
    pub history: Vec<Frame>, // in reverse order - first frame is the most recent
}
