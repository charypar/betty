use chrono::{DateTime, Utc};

fn main() {
    println!("Hello, world!");
}

struct CurrencyAmount {
    code: String,
    amount: u128,
}

// A smallest possible price increment
type Ticks = u64;

// Point value with fixed decimal place position
// Different instruments will differ in this
type Points = (Ticks, usize);

// Price per point used for instrument and trad sizing
type PricePerPoint = u64; // in pence

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

struct Rules {
    min_deal_size: Points,
    stop_distance_limts: (Points, Points), // min, max
}

struct Instrument {
    code: String,
    contract_size: PricePerPoint,
    margin_factor: u64,
}

enum Resolution {
    Second,
    Minute(usize),
    Hour(usize),
    Day,
    Weeek,
    Month,
}

struct History {
    resolution: Resolution,
    frames: Vec<Frame>, // in reverse order - first frame is the most recent
}

struct Market {
    rules: Rules,
    instrument: Instrument,
    history: History,
}

// Trade

enum Direction {
    Buy,
    Sell,
}

struct Order {
    direction: Direction,
    price: Points,
    time: DateTime<Utc>,
}

struct Trade {
    instrument: Instrument,
    direction: Direction,
    entry: Order,
    exit: Option<Order>,
    size: PricePerPoint,
    stop: Points,
}

// Strategy

struct Strategy {
    short_trend_length: usize,
    long_trend_length: usize,
    limit_channel_length: usize,
}
struct Log {
    market: Market,
    initial_capital: CurrencyAmount,
    risk_per_trade: usize, // percentage point
    strategy: Strategy,
    trades: Vec<Trade>,
}
