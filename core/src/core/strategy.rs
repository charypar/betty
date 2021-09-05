use std::convert::TryFrom;
use std::error::Error;
use std::fmt::Display;

use super::price::{CurrencyAmount, Points, PriceHistory};
use super::trade::{Direction, Entry};

// Tading Strategy estimates the trned of the marekt

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trend {
    Neutral,
    Bullish,
    Bearish,
}

impl TryFrom<Trend> for Direction {
    type Error = &'static str;

    fn try_from(trend: Trend) -> Result<Self, Self::Error> {
        match trend {
            Trend::Neutral => Err("Cannot convert Neutral to a trade direction."),
            Trend::Bullish => Ok(Direction::Buy),
            Trend::Bearish => Ok(Direction::Sell),
        }
    }
}

pub trait TradingStrategy {
    fn trend(&self, history: &PriceHistory) -> Trend;
}

// RiskStrategy decides stop-loss placement and trade size

pub trait RiskStrategy {
    fn stop(
        &self,
        direction: Direction,
        history: &PriceHistory,
    ) -> Result<Points, RiskStrategyError>;

    fn entry(
        &self,
        direction: Direction,
        history: &PriceHistory,
        risk: CurrencyAmount,
    ) -> Result<Entry, RiskStrategyError> {
        let stop = self.stop(direction, history)?;

        // Assuming immediate execution,
        // this may lead to a slight size error in real life due to slippage
        let latest_close = history.history[0].close;
        let price = match direction {
            Direction::Buy => latest_close.ask,
            Direction::Sell => latest_close.bid,
        };

        let time = history.history[0].close_time;

        // Size of the trade (per point) is our total acceptable risk
        // divided by the distance to stop-loss level
        let stop_distance = (price - stop).abs();
        let size = risk / stop_distance;

        let position_id = String::new();

        Ok(Entry {
            position_id,
            direction,
            price,
            stop,
            size,
            time,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum RiskStrategyError {
    NotEnoughHistory, // Not enough history to place a stop-loss safely
}

impl Error for RiskStrategyError {}

impl Display for RiskStrategyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEnoughHistory => write!(f, "Not enough history to set stop-loss"),
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::prelude::*;
    use iso_currency::Currency;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use super::*;
    use crate::core::price::{CurrencyAmount, Frame, Price, PriceHistory, Resolution};
    use crate::core::trade::Entry;

    // RiskStrategy

    #[test]
    fn calculates_entry() {
        let risk = CurrencyAmount::new(dec!(10.1), Currency::GBP);
        let rs_buy = ConstStop { stop: dec!(600.0) };
        let rs_sell = ConstStop { stop: dec!(800.0) };

        let history = PriceHistory {
            resolution: Resolution::Minute(10),
            history: vec![Frame {
                open: Price::new_mid(dec!(100), dec!(2)),
                close: Price::new_mid(dec!(700), dec!(2)), // only close matters
                high: Price::new_mid(dec!(200), dec!(2)),
                low: Price::new_mid(dec!(300), dec!(2)),
                close_time: Utc.ymd(2021, 1, 1).and_hms(12, 30, 0),
            }]
            .into(),
        };

        let expected_buy = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price: dec!(701.0),
            stop: dec!(600.0),
            size: CurrencyAmount::new(dec!(0.1), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(12, 30, 0),
        });
        let expected_sell = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Sell,
            price: dec!(699.0),
            stop: dec!(800.0),
            size: CurrencyAmount::new(dec!(0.1), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(12, 30, 0),
        });

        assert_eq!(rs_buy.entry(Direction::Buy, &history, risk), expected_buy);
        assert_eq!(
            rs_sell.entry(Direction::Sell, &history, risk),
            expected_sell
        );
    }

    // Fixtures

    struct ConstStop {
        stop: Decimal,
    }

    impl RiskStrategy for ConstStop {
        fn stop(
            &self,
            _direction: Direction,
            _history: &PriceHistory,
        ) -> Result<Points, RiskStrategyError> {
            Ok(self.stop)
        }
    }
}
