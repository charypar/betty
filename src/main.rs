use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn main() {
    let market = Market {
        code: "UKX".to_string(),
        min_deal_size: CurrencyAmount((dec!(0.50), "GBP".to_string())),
        min_stop_distance: dec!(8),
        margin_factor: 20,
    };

    let trading_strategy = TradingStrategy {
        short_trend_length: 5,
        long_trend_length: 20,
    };

    let risk_strategy = RiskStrategy {
        channel_length: 20,
        risk_per_trade: dec!(3),
    };
}

#[cfg(test)]
mod test {
    use super::*;
}

#[derive(Debug)]
pub struct CurrencyAmount((Decimal, String));

// Point value with fixed decimal place position
// Different instruments will differ in this
type Points = Decimal;

// Spot price of an instrument. Excuse my finance n00b comments
pub struct MarketPrice {
    ask: Points, // price we buy at (market asks for this price level)
    bid: Points, // price we sell at (market bids to buy at this price level)
}

// Market data

pub struct Frame {
    close: MarketPrice,
    high: MarketPrice,
    low: MarketPrice,
    open: MarketPrice,
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
    resolution: Resolution,
    history: Vec<Frame>, // in reverse order - first frame is the most recent
}

pub struct Market {
    code: String,
    margin_factor: u64,            // 1:X
    min_deal_size: CurrencyAmount, // per point
    min_stop_distance: Points,
}

pub enum MarketError {
    DealTooSmall,        // size below min_deal_size
    StopTooClose,        // stop-loss is not far enough
    InsufficientBalance, // would result in margin call
}

impl Market {
    pub fn validate_entry(order: Entry, balance: CurrencyAmount) -> Result<(), MarketError> {
        todo!()
    }
}

// Trade Orders

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

// Strategy

// TradingStrategy produces buy and sell signals
pub struct TradingStrategy {
    short_trend_length: usize,
    long_trend_length: usize,
}

impl TradingStrategy {
    pub fn signal(history: &PriceHistory) -> Option<Direction> {
        todo!()
    }
}

// RiskStrategy decides stop-loss placement and trade size
pub struct RiskStrategy {
    channel_length: usize,
    risk_per_trade: Decimal, // percent
}

impl RiskStrategy {
    pub fn entry(direction: Direction, history: &PriceHistory, balance: CurrencyAmount) -> Entry {
        todo!()
    }
}

// Account

// Account holds the state of the trading account and history of all the orders placed
// in response to price updates.
pub struct Account {
    opening_balance: CurrencyAmount,
    market: Market,
    pub price_history: PriceHistory, // mutable, prepend-only
    trading_strategy: TradingStrategy,
    risk_strategy: RiskStrategy,
    pub orders: Vec<Order>, // mutable, append-only
}

#[derive(Debug)]
pub enum TradeStatus {
    Open,
    Closed,
}

#[derive(Debug)]
pub enum TradeOutcome {
    Profit,
    Loss,
}

// A row in a trade log
#[derive(Debug)]
pub struct Trade {
    id: String,
    status: TradeStatus,
    // Entry
    direction: Direction,
    entry_time: DateTime<Utc>,
    entry_price: Points,
    // Exit
    exit_time: Option<DateTime<Utc>>,
    exit_price: Option<Points>,
    // Risk
    stop: Points,
    size: CurrencyAmount,
    risk: CurrencyAmount,
    // Outcome
    outcome: TradeOutcome,
    price_diff: Points,
    balance: CurrencyAmount,
    risk_reward: Decimal,
}

impl Account {
    pub fn trade_log() -> Vec<Trade> {
        todo!()
    }

    pub fn balance_history() -> CurrencyAmount {
        todo!()
    }

    // Add new price information
    // This potentially results in a new order to be placed
    pub fn update_price(&mut self, frame: Frame) -> Option<Order> {
        todo!()
    }

    // Log an order that has been confirmed by the broker
    pub fn confirm_order(&mut self, order: Order) {
        todo!()
    }
}
