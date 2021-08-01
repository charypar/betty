use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Deserializer};

use crate::core::price::{Frame, Price};

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

pub fn read_prices_csv<R>(io: R) -> Vec<Frame>
where
    R: std::io::Read,
{
    let mut reader = csv::Reader::from_reader(io);

    reader
        .deserialize()
        .flat_map(|line| -> Result<Frame, csv::Error> { Ok(frame_from(line?, dec!(5))) })
        .collect()
}
