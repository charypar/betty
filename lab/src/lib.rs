use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use wasm_bindgen::prelude::*;

use betty::{
    price::{Frame, Price},
    strategies::{Donchian, MACD},
};
use serde::{Deserialize, Deserializer, Serialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn console_log(s: String) {
    #[allow(unused_unsafe)] // shut up rust-analyzer
    unsafe {
        log(&s);
    }
}

#[derive(Deserialize, Debug, Clone)]
struct PriceRecord {
    #[serde(deserialize_with = "parse_date")]
    date: DateTime<Utc>,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

#[derive(Serialize, Debug)]
struct StrategyRecord {
    long_stop: Decimal,
    short_stop: Decimal,
    short_ema: Decimal,
    long_ema: Decimal,
    macd: Decimal,
    macd_signal: Decimal,
    macd_trend: Decimal,
    trade_signal: Option<String>,
}

#[derive(Serialize, Debug)]
struct Trade {
    open_date: String, // FIXME
    open_price: Decimal,
    stop: Decimal,
    close_date: String, // FIXME
    close_price: Decimal,
}

#[derive(Serialize, Debug)]
struct TestResult {
    indicators: Vec<StrategyRecord>,
    trades: Vec<Trade>,
}

#[wasm_bindgen]
pub fn run_test(prices: JsValue) -> JsValue {
    let prices = prices.into_serde::<Vec<PriceRecord>>();
    if let Err(e) = prices {
        console_log(format!("Error parsing price data: {}", e));

        return JsValue::from_serde(&()).unwrap();
    }

    let spread = dec!(5);
    let price_history: Vec<_> = prices
        .expect("Couldn't parse prices")
        .iter()
        .map(|r| frame_from(r, spread))
        .collect();

    let indicators: Vec<_> = MACD::macd(&price_history, 16, 42, 20, dec!(20), dec!(10))
        .iter()
        .zip(Donchian::channel(&price_history, 20))
        .map(|(ts, rs)| StrategyRecord {
            short_ema: ts.short_ema,
            long_ema: ts.long_ema,
            macd: ts.macd,
            macd_signal: ts.macd_signal,
            macd_trend: ts.macd_trend,
            trade_signal: ts.trade_signal.map(|s| format!("{:?}", s)),
            long_stop: rs.1,
            short_stop: rs.0,
        })
        .collect();

    let result = TestResult {
        indicators,
        trades: vec![], // TODO
    };

    JsValue::from_serde(&result).unwrap()
}

fn frame_from(price_record: &PriceRecord, spread: Decimal) -> Frame {
    Frame {
        close_time: price_record.date,
        open: Price::new_mid(price_record.open, spread),
        high: Price::new_mid(price_record.high, spread),
        low: Price::new_mid(price_record.low, spread),
        close: Price::new_mid(price_record.close, spread),
    }
}

const DATE_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S.%fZ";

fn parse_date<'de, D>(de: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;

    Utc.datetime_from_str(&s, DATE_FORMAT)
        .map_err(serde::de::Error::custom)
}
