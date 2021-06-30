use std::cmp::{max, min};

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
    pub risk_per_trade: Decimal,
}

#[derive(Debug, PartialEq)]
enum RiskStrategyError {
    NotEnoughHistory,
}

impl RiskStrategy {
    pub fn entry(
        &self,
        direction: Direction,
        history: &PriceHistory,
        balance: CurrencyAmount,
    ) -> Result<Entry, RiskStrategyError> {
        if history.history.len() < self.channel_length {
            return Err(RiskStrategyError::NotEnoughHistory);
        }

        let time = history.history[0].open_time + history.resolution;

        // The lower end of the channel is a bid price - we are selling to exit a long position that didn't go our way
        // The higher end of the channel is an ask price - we are buying to exit a short position that didn't go our way
        let channel_limits = history.history[1..self.channel_length].iter().fold(
            (history.history[0].low.bid, history.history[0].high.ask),
            |limits, frame| (min(limits.0, frame.low.bid), max(limits.1, frame.high.ask)),
        );

        // Assuming immediate execution,
        // this may lead to a slight size error in real life due to slippage
        let latest_close = history.history[0].close;
        let price = match direction {
            Direction::Buy => latest_close.ask,
            Direction::Sell => latest_close.bid,
        };

        let stop = match direction {
            Direction::Buy => channel_limits.0,
            Direction::Sell => channel_limits.1,
        };

        // Size of the trade (per point) is our total acceptable risk
        // divided by the distance to stop-loss level
        let stop_distance = (price - stop).abs();
        let risk = balance * self.risk_per_trade;
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

#[cfg(test)]
mod test {
    use std::iter;

    use chrono::prelude::*;
    use iso_currency::Currency;
    use rust_decimal_macros::dec;

    use super::*;
    use crate::core::price::{CurrencyAmount, Frame, Price, PriceHistory, Resolution};
    use crate::core::trade::Entry;

    #[test]
    fn it_rejects_entry_without_enough_history() {
        let balance = CurrencyAmount::new(dec!(1020), Currency::GBP);
        let rs = RiskStrategy {
            channel_length: 4,
            risk_per_trade: dec!(0.01),
        };
        let history = oscilating_history(
            dec!(600),
            dec!(1000),
            dec!(2),
            Utc.ymd(2021, 1, 1).and_hms(12, 0, 0),
            Resolution::Minute(10),
            3,
        );

        assert_eq!(
            rs.entry(Direction::Buy, &history, balance),
            Err(RiskStrategyError::NotEnoughHistory)
        );
    }

    #[test]
    fn it_calculates_entry_with_stable_history() {
        let balance = CurrencyAmount::new(dec!(1020), Currency::GBP);
        let rs = RiskStrategy {
            channel_length: 2,
            risk_per_trade: dec!(0.01),
        };
        let history = oscilating_history(
            dec!(600),
            dec!(1000),
            dec!(2),
            Utc.ymd(2021, 1, 1).and_hms(12, 0, 0),
            Resolution::Minute(10),
            3,
        );

        let expected_buy = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price: dec!(701.0),
            stop: dec!(599.0),
            size: CurrencyAmount::new(dec!(0.1), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(12, 30, 0),
        });
        let expected_sell = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Sell,
            price: dec!(699.0),
            stop: dec!(1001.0),
            size: CurrencyAmount::new(dec!(0.033775), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(12, 30, 0),
        });

        assert_eq!(rs.entry(Direction::Buy, &history, balance), expected_buy);
        assert_eq!(rs.entry(Direction::Sell, &history, balance), expected_sell);
    }

    #[test]
    fn it_sets_stop_based_on_recent_history() {
        let balance = CurrencyAmount::new(dec!(1000), Currency::GBP);
        let short_rs = RiskStrategy {
            channel_length: 2,
            risk_per_trade: dec!(0.01),
        };

        let long_rs = RiskStrategy {
            channel_length: 8,
            risk_per_trade: dec!(0.01),
        };

        let mut h = oscilating_history(
            dec!(600),
            dec!(1000),
            dec!(2),
            Utc.ymd(2021, 1, 1).and_hms(12, 50, 0),
            Resolution::Minute(10),
            5,
        )
        .history;
        let mut h2 = oscilating_history(
            dec!(200),
            dec!(2000),
            dec!(2),
            Utc.ymd(2021, 1, 1).and_hms(12, 0, 0),
            Resolution::Minute(10),
            5,
        )
        .history;
        h.append(&mut h2);

        let history = PriceHistory {
            resolution: Resolution::Minute(10),
            history: h,
        };

        let short_expected_buy = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price: dec!(701.0),
            stop: dec!(599.0),
            size: CurrencyAmount::new(dec!(0.098039), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(13, 40, 0),
        });
        let short_expected_sell = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Sell,
            price: dec!(699.0),
            stop: dec!(1001.0),
            size: CurrencyAmount::new(dec!(0.033113), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(13, 40, 0),
        });

        let long_expected_buy = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price: dec!(701.0),
            stop: dec!(199.0),
            size: CurrencyAmount::new(dec!(0.019920), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(13, 40, 0),
        });
        let long_expected_sell = Ok(Entry {
            position_id: String::new(),
            direction: Direction::Sell,
            price: dec!(699.0),
            stop: dec!(2001.0),
            size: CurrencyAmount::new(dec!(0.007680), Currency::GBP),
            time: Utc.ymd(2021, 1, 1).and_hms(13, 40, 0),
        });

        assert_eq!(
            short_rs.entry(Direction::Buy, &history, balance),
            short_expected_buy
        );
        assert_eq!(
            short_rs.entry(Direction::Sell, &history, balance),
            short_expected_sell
        );

        assert_eq!(
            long_rs.entry(Direction::Buy, &history, balance),
            long_expected_buy
        );
        assert_eq!(
            long_rs.entry(Direction::Sell, &history, balance),
            long_expected_sell
        );
    }

    // Fixtures

    // History that jumps between two prices starting up
    fn oscilating_history(
        min_level: Decimal,
        max_level: Decimal,
        spread: Decimal,
        start_time: DateTime<Utc>,
        resolution: Resolution,
        length: usize,
    ) -> PriceHistory {
        let max = Price::new_mid(max_level, spread);
        let min = Price::new_mid(min_level, spread);
        let high = Price::new_mid(max_level - dec!(100), spread);
        let low = Price::new_mid(min_level + dec!(100), spread);

        let cycle = [
            Frame {
                open: high.clone(),
                close: low,
                high: max,
                low: min,
                open_time: start_time,
            },
            Frame {
                open: low,
                close: high,
                high: max,
                low: min,
                open_time: start_time,
            },
        ];
        let timeline = iter::successors(Some(start_time), |t| Some(*t + resolution));

        let mut history: Vec<Frame> = std::iter::repeat(cycle)
            .flatten()
            .zip(timeline)
            .map(|(frame, time)| Frame {
                open_time: time,
                ..frame
            })
            .take(length)
            .collect();

        history.reverse();

        PriceHistory {
            resolution,
            history,
        }
    }
}
