use std::{
    collections::VecDeque,
    ops::{Add, Div, Mul, Sub},
};

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use iso_currency::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const CURRENCY_DECIMAL_PLACES: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurrencyAmount {
    amount: Decimal,
    currency: Currency,
}

impl CurrencyAmount {
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }
}

impl Mul<Decimal> for CurrencyAmount {
    type Output = CurrencyAmount;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self::Output::new(
            (self.amount * rhs).round_dp(CURRENCY_DECIMAL_PLACES),
            self.currency,
        )
    }
}

impl Div<Decimal> for CurrencyAmount {
    type Output = CurrencyAmount;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self::Output::new(
            (self.amount / rhs).round_dp(CURRENCY_DECIMAL_PLACES),
            self.currency,
        )
    }
}

impl Div<CurrencyAmount> for CurrencyAmount {
    type Output = Option<Decimal>;

    fn div(self, rhs: CurrencyAmount) -> Self::Output {
        if self.currency == rhs.currency {
            self.amount.checked_div(rhs.amount)
        } else {
            None
        }
    }
}

impl PartialOrd<CurrencyAmount> for CurrencyAmount {
    fn partial_cmp(&self, rhs: &CurrencyAmount) -> Option<std::cmp::Ordering> {
        if self.currency == rhs.currency {
            self.amount.partial_cmp(&rhs.amount)
        } else {
            None
        }
    }
}

// Point value with fixed decimal place position
// Different instruments will differ in this
pub type Points = Decimal;

// Price of an instrument. Excuse my finance n00b comments
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Price {
    pub ask: Points, // price we buy at (market asks for this price level)
    pub bid: Points, // price we sell at (market bids to buy at this price level)
}

impl Price {
    pub fn new_mid(price: Points, spread: Points) -> Self {
        Self {
            ask: price + spread / dec!(2.0),
            bid: price - spread / dec!(2.0),
        }
    }

    pub fn mid_price(&self) -> Points {
        (self.bid + self.ask) / dec!(2.0)
    }

    pub fn spread(&self) -> Points {
        self.ask - self.bid
    }
}

impl Sub for Price {
    type Output = Points;

    fn sub(self, rhs: Self) -> Self::Output {
        self.mid_price() - rhs.mid_price()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Frame {
    pub close: Price,
    pub high: Price,
    pub low: Price,
    pub open: Price,
    pub close_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    Second,
    Minute(usize),
    Hour(usize),
    Day,
    Week,
    Month,
}

impl<TZ> Add<Resolution> for DateTime<TZ>
where
    TZ: TimeZone,
{
    type Output = DateTime<TZ>;

    fn add(self, rhs: Resolution) -> Self::Output {
        match rhs {
            Resolution::Second => self + Duration::seconds(1),
            Resolution::Minute(t) => self + Duration::minutes(t as i64),
            Resolution::Hour(t) => self + Duration::hours(t as i64),
            Resolution::Day => self + Duration::days(1),
            Resolution::Week => self + Duration::weeks(1),
            Resolution::Month => self.with_month(self.month() + 1).unwrap_or(
                self.timezone().ymd(self.year() + 1, 1, self.day()).and_hms(
                    self.hour(),
                    self.minute(),
                    self.second(),
                ),
            ),
        }
    }
}

pub struct PriceHistory {
    pub resolution: Resolution,
    pub history: VecDeque<Frame>, // in reverse order - first frame is the most recent
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn adds_seconds_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Second;
        let expected = Utc.ymd(2021, 1, 1).and_hms(10, 0, 1);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 59) + Resolution::Second;
        let expected = Utc.ymd(2021, 1, 1).and_hms(10, 1, 0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn adds_minutes_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Minute(5);
        let expected = Utc.ymd(2021, 1, 1).and_hms(10, 5, 0);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 56, 0) + Resolution::Minute(5);
        let expected = Utc.ymd(2021, 1, 1).and_hms(11, 1, 0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn adds_hours_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Hour(4);
        let expected = Utc.ymd(2021, 1, 1).and_hms(14, 0, 0);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 1, 1).and_hms(22, 0, 0) + Resolution::Hour(4);
        let expected = Utc.ymd(2021, 1, 2).and_hms(2, 0, 0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn adds_days_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Day;
        let expected = Utc.ymd(2021, 1, 2).and_hms(10, 0, 0);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 1, 31).and_hms(10, 0, 0) + Resolution::Day;
        let expected = Utc.ymd(2021, 2, 1).and_hms(10, 0, 0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn adds_weeks_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Week;
        let expected = Utc.ymd(2021, 1, 8).and_hms(10, 0, 0);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 1, 28).and_hms(10, 0, 0) + Resolution::Week;
        let expected = Utc.ymd(2021, 2, 4).and_hms(10, 0, 0);

        assert_eq!(actual, expected);
    }
    #[test]
    fn adds_months_to_date() {
        let actual = Utc.ymd(2021, 1, 1).and_hms(10, 0, 0) + Resolution::Month;
        let expected = Utc.ymd(2021, 2, 1).and_hms(10, 0, 0);

        assert_eq!(actual, expected);

        let actual = Utc.ymd(2021, 12, 1).and_hms(10, 0, 0) + Resolution::Month;
        let expected = Utc.ymd(2022, 1, 1).and_hms(10, 0, 0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn makes_price_from_mid_market_and_spread() {
        let expected = Price {
            ask: dec!(100.5),
            bid: dec!(99.5),
        };
        let actual = Price::new_mid(dec!(100.0), dec!(1.0));

        assert_eq!(actual, expected)
    }
}
