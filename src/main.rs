use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn main() {
    let market = Market {
        code: "UKX".to_string(),
        min_deal_size: CurrencyAmount((dec!(0.50), "GBP".to_string())),
        min_stop_distance: dec!(8),
        margin_factor: 20,
        resolution: Resolution::Minute(10),
        history: vec![],
    };

    let strategy = Strategy {
        short_trend_length: 5,
        long_trend_length: 20,
        limit_channel_length: 20,
    };

    let log = trade(
        market,
        strategy,
        CurrencyAmount((dec!(10000.00), "GBP".to_string())),
        3,
    );

    for trade in log {
        println!("{:?}", trade);
    }
}

pub fn trade(
    market: Market,
    strategy: Strategy,
    capital: CurrencyAmount,
    risk_per_trade: usize,
) -> Log {
    Log {
        market,
        initial_capital: capital,
        risk_per_trade, // percentage point
        strategy,
        trades: vec![],
    }
}

#[derive(Debug)]
pub struct CurrencyAmount((Decimal, String));

// Point value with fixed decimal place position
// Different instruments will differ in this
type Points = Decimal;

// Spot price of an instrument. Excuse my finance n00b comments
struct MarketPrice {
    ask: Points, // price we buy at (market asks for this price level)
    bid: Points, // price we sell at (market bids to buy at this price level)
}

// Market data

struct Frame {
    close: MarketPrice,
    high: MarketPrice,
    low: MarketPrice,
    open: MarketPrice,
    open_time: DateTime<Utc>,
}

struct Rules {}

#[derive(Debug)]
struct Instrument {}

enum Resolution {
    Second,
    Minute(usize),
    Hour(usize),
    Day,
    Weeek,
    Month,
}

pub struct Market {
    code: String,
    margin_factor: u64,
    min_deal_size: CurrencyAmount,
    min_stop_distance: Points,
    resolution: Resolution,
    history: Vec<Frame>, // in reverse order - first frame is the most recent
}

// Trade
#[derive(Debug)]
enum Direction {
    Buy,
    Sell,
}

#[derive(Debug)]
struct Order {
    direction: Direction,
    price: Points,
    time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Trade {
    instrument: Instrument,
    direction: Direction,
    entry: Order,
    exit: Option<Order>,
    size: CurrencyAmount,
    stop: Points,
}

// Strategy

pub struct Strategy {
    short_trend_length: usize,
    long_trend_length: usize,
    limit_channel_length: usize,
}

pub struct Log {
    market: Market,
    initial_capital: CurrencyAmount,
    risk_per_trade: usize, // percentage point
    strategy: Strategy,
    trades: Vec<Trade>,
}

impl IntoIterator for Log {
    type Item = Trade;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.trades.into_iter()
    }
}
