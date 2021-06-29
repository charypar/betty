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
    pub fn entry(
        &self,
        direction: Direction,
        history: &PriceHistory,
        balance: CurrencyAmount,
    ) -> Entry {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use chrono::prelude::*;
    use rust_decimal_macros::dec;

    use super::*;
    use crate::core::price::{CurrencyAmount, PriceHistory, Resolution};
    use crate::core::trade::Entry;

    #[test]
    fn it_calculates_correct_entry() {
        let rs = RiskStrategy {
            channel_length: 10,
            risk_per_trade: dec!(10.0),
        };

        let history = PriceHistory {
            resolution: Resolution::Minute(10),
            history: vec![],
        };

        let expected_buy = Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price: dec!(800.0),
            stop: dec!(800.0),
            size: CurrencyAmount((dec!(0.40), "GBP".to_string())),
            time: Utc.ymd(2021, 7, 29).and_hms(20, 30, 0),
        };
        let expected_sell = Entry {
            position_id: String::new(),
            direction: Direction::Sell,
            price: dec!(800.0),
            stop: dec!(800.0),
            size: CurrencyAmount((dec!(0.40), "GBP".to_string())),
            time: Utc.ymd(2021, 7, 29).and_hms(20, 30, 0),
        };

        assert_eq!(
            rs.entry(
                Direction::Buy,
                &history,
                CurrencyAmount((dec!(1000), "GBP".to_string()))
            ),
            expected_buy
        );
        assert_eq!(
            rs.entry(
                Direction::Buy,
                &history,
                CurrencyAmount((dec!(1000), "GBP".to_string()))
            ),
            expected_sell
        );
    }
}
