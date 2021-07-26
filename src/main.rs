mod core;
mod strategies;

use std::error::Error;
use std::io;

use chrono::{DateTime, TimeZone, Utc};
use iso_currency::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use serde::Deserializer;

use crate::core::market::Market;
use crate::core::price::Frame;
use crate::core::price::{CurrencyAmount, Price, Resolution};
use crate::core::trade::Entry;
use crate::core::trade::Exit;
use crate::core::trade::Order;
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

    // Set up a test run

    let market = Market {
        code: "GDAXI".to_string(),
        margin_factor: dec!(0.05),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(12),
    };

    let ts = MACD {
        short: 16,
        long: 42,
        signal: 20,
        entry_lim: dec!(10),
        exit_lim: dec!(10),
    };

    let rs = Donchian { channel_length: 20 };

    let mut account = Account::new(
        market,
        ts,
        rs,
        dec!(0.01),
        CurrencyAmount::new(dec!(20000.00), Currency::GBP),
        Resolution::Day,
    );

    // Run the test
    let mut p_id = 0;

    for price in &prices {
        for order in account.update_price(*price) {
            match order {
                Order::Open(entry) => {
                    // TODO validate entry

                    let o = Order::Open(Entry {
                        position_id: p_id.to_string(),
                        ..entry
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("!! Position already open");
                    }
                }
                Order::Close(exit) => {
                    let o = Order::Close(Exit {
                        position_id: p_id.to_string(),
                        ..exit
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("Position already closed");
                    }

                    p_id += 1;
                }
                Order::Stop(exit) => {
                    let o = Order::Stop(Exit {
                        position_id: p_id.to_string(),
                        ..exit
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("Position already closed");
                    }

                    p_id += 1;
                }
            }
        }
    }

    // TODO feed in a price history and log resulting orders
    let latest_price = prices.last().unwrap().close;

    // Pretty print a trade log

    for trade in account.trade_log(latest_price) {
        println!(
            "#{} {:?} entry: {}, exit: {:?}, size: {}, risk: {}, balance: {} - {:?}",
            trade.id,
            trade.direction,
            trade.entry_price,
            trade.exit_price,
            trade.size,
            trade.risk,
            trade.balance,
            trade.outcome
        );
    }

    Ok(())
}
