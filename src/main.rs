mod core;
mod strategies;

use std::error::Error;
use std::io;

use chrono::Date;
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use iso_currency::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use serde::Deserializer;

use crate::core::market::Market;
use crate::core::price::Frame;
use crate::core::price::{CurrencyAmount, Price, Resolution};
use crate::core::Account;
use crate::strategies::{Donchian, MACD};

// CSV processing
// FIXME move this somewhere else

#[derive(Deserialize, Debug)]
struct PriceRecord {
    #[serde(rename = "Date", deserialize_with = "parse_date")]
    date: DateTime<Utc>,
    #[serde(rename = "Open")]
    open: Decimal,
    #[serde(rename = "High")]
    high: Decimal,
    #[serde(rename = "Low")]
    low: Decimal,
    #[serde(rename = "Close")]
    close: Decimal,
}

const DATE_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S";

fn parse_date<'de, D>(de: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;

    Utc.datetime_from_str(&s, DATE_FORMAT)
        .map_err(serde::de::Error::custom)
}

fn frame_from(price_record: PriceRecord, spread: Decimal) -> Frame {
    Frame {
        close_time: price_record.date,
        open: Price::new_mid(price_record.open, spread),
        high: Price::new_mid(price_record.high, spread),
        low: Price::new_mid(price_record.low, spread),
        close: Price::new_mid(price_record.close, spread),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Read the price

    let mut reader = csv::Reader::from_reader(io::stdin());
    let prices: Vec<_> = reader
        .deserialize()
        .flat_map(|line| -> Result<Frame, csv::Error> { Ok(frame_from(line?, dec!(5))) })
        .collect();

    for price in prices {
        println!(
            "{:}: open:{:?} low:{:?} high:{:?} close:{:?}",
            price.close_time, price.open, price.low, price.high, price.close
        );
    }

    // Set up a test run

    let market = Market {
        code: "GDAXI".to_string(),
        margin_factor: dec!(0.05),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(12),
    };

    let ts = MACD {
        short_trend_length: 12,
        long_trend_length: 40,
        macd_signal_length: 10,
        entry_signal_diff_limit: dec!(10),
        exit_signal_diff_limit: dec!(10),
    };

    let rs = Donchian { channel_length: 20 };

    let account = Account::new(
        market,
        ts,
        rs,
        dec!(0.01),
        CurrencyAmount::new(dec!(10000.00), Currency::GBP),
        Resolution::Day,
    );

    // Run the test

    // TODO feed in a price history and log resulting orders
    let latest_price = Price {
        bid: dec!(110),
        ask: dec!(110),
    };

    // Pretty print a trade log

    for trade in account.trade_log(latest_price) {
        println!("{:?}", trade);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
}
