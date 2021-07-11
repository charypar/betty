use rust_decimal::Decimal;

use super::price::{CurrencyAmount, Points};
use super::trade::Entry;

// Market holds information about a particular market and the trading rules that apply
pub struct Market {
    pub code: String,
    pub margin_factor: Decimal,
    pub min_deal_size: CurrencyAmount, // per point
    pub min_stop_distance: Points,
}

#[derive(Debug, PartialEq)]
pub enum MarketError {
    DealTooSmall,        // size below min_deal_size
    StopTooClose,        // stop-loss is not far enough
    InsufficientBalance, // would result in margin call
}

impl Market {
    pub fn validate_entry(
        &self,
        order: &Entry,
        balance: CurrencyAmount,
    ) -> Result<(), MarketError> {
        if order.size < self.min_deal_size {
            return Err(MarketError::DealTooSmall);
        }

        if self.margin_requirement(order) > balance {
            return Err(MarketError::InsufficientBalance);
        }

        if (order.price - order.stop).abs() < self.min_stop_distance {
            return Err(MarketError::StopTooClose);
        }

        Ok(())
    }

    fn margin_requirement(&self, order: &Entry) -> CurrencyAmount {
        order.size * order.price * self.margin_factor
    }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, TimeZone, Utc};
    use iso_currency::Currency;
    use rust_decimal_macros::dec;

    use crate::core::trade::Direction;

    use super::*;

    #[test]
    fn validates_an_ok_trade() {
        let market = market();
        let balance = CurrencyAmount::new(dec!(1000), Currency::GBP);
        let risk_per_trade = dec!(0.01); // 10 GBP
        let price = dec!(15000);
        let stop_distance = dec!(15);

        let entry = Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price,
            stop: price - stop_distance,
            size: balance * risk_per_trade / stop_distance,
            time: date(),
        };

        let expected = Ok(());
        let actual = market.validate_entry(&entry, balance);

        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_entry_below_minimum_deal_size() {
        let market = market();
        let balance = CurrencyAmount::new(dec!(1000), Currency::GBP);
        let risk_per_trade = dec!(0.01); // 10 GBP
        let price = dec!(15000);
        let stop_distance = dec!(21); // size = 0.47GB pp

        let entry = Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price,
            stop: price - stop_distance,
            size: balance * risk_per_trade / stop_distance,
            time: date(),
        };

        let expected = Err(MarketError::DealTooSmall);
        let actual = market.validate_entry(&entry, balance);

        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_entry_with_stop_too_close() {
        let market = market();
        let balance = CurrencyAmount::new(dec!(1000), Currency::GBP);
        let risk_per_trade = dec!(0.01); // 10 GBP
        let price = dec!(15000);
        let stop_distance = dec!(10); // size = 1GB pp => margin 750

        let entry = Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price,
            stop: price - stop_distance,
            size: balance * risk_per_trade / stop_distance,
            time: date(),
        };

        let expected = Err(MarketError::StopTooClose);
        let actual = market.validate_entry(&entry, balance);

        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_entry_with_insufficient_margin() {
        let market = market();
        let balance = CurrencyAmount::new(dec!(1000), Currency::GBP);
        let risk_per_trade = dec!(0.028); // 28 GBP
        let price = dec!(15000);
        let stop_distance = dec!(20); // size = 1.4GB pp => margin 1050

        let entry = Entry {
            position_id: String::new(),
            direction: Direction::Buy,
            price,
            stop: price - stop_distance,
            size: balance * risk_per_trade / stop_distance,
            time: date(),
        };

        let expected = Err(MarketError::InsufficientBalance);
        let actual = market.validate_entry(&entry, balance);

        assert_eq!(actual, expected);
    }

    fn market() -> Market {
        Market {
            code: "GDAXI".to_string(),
            margin_factor: dec!(0.05), // 5%
            min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
            min_stop_distance: dec!(12),
        }
    }

    fn date() -> DateTime<Utc> {
        Utc.ymd(2021, 1, 1).and_hms(10, 1, 0)
    }
}
