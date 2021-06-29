use rust_decimal::Decimal;

use super::price::{CurrencyAmount, PriceHistory};
use super::trade::{Direction, Entry};

// TradingStrategy produces buy and sell signals
pub struct TradingStrategy {
    pub short_trend_length: usize,
    pub long_trend_length: usize,
}

impl TradingStrategy {
    pub fn signal(history: &PriceHistory) -> Option<Direction> {
        todo!()
    }
}

// RiskStrategy decides stop-loss placement and trade size
pub struct RiskStrategy {
    pub channel_length: usize,
    pub risk_per_trade: Decimal, // percent
}

impl RiskStrategy {
    pub fn entry(direction: Direction, history: &PriceHistory, balance: CurrencyAmount) -> Entry {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}
