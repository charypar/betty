use chrono::{DateTime, Utc};
use iso_currency::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use wasm_bindgen::prelude::*;

use betty::{
    account::Account,
    backtest::Backtest,
    market::Market,
    price::{CurrencyAmount, Frame, Price, Resolution},
    strategies::{Donchian, MACD},
};
use serde::{Deserialize, Serialize};

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
    date: DateTime<Utc>,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

#[derive(Deserialize)]
struct TestParameters {
    short: usize,
    long: usize,
    signal: usize,
    entry: Decimal,
    exit: Decimal,
    channel: usize,
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
    trend: String,
}

#[derive(Serialize, Debug)]
struct Trade {
    open_date: DateTime<Utc>,
    open_price: Decimal,
    stop: Decimal,
    close_date: Option<DateTime<Utc>>,
    close_price: Option<Decimal>,
    outcome: String,
}

#[derive(Serialize, Debug)]
struct TestResult {
    indicators: Vec<StrategyRecord>,
    trades: Vec<Trade>,
}

#[wasm_bindgen]
pub fn run_test(prices: JsValue, parameters: JsValue) -> JsValue {
    let prices = prices.into_serde::<Vec<PriceRecord>>();
    if let Err(e) = prices {
        console_log(format!("Error parsing price data: {}", e));

        return JsValue::from_serde(&()).unwrap();
    }

    let tp = parameters.into_serde::<TestParameters>();
    if let Err(e) = tp {
        console_log(format!("Error parsing test parameters: {}", e));

        return JsValue::from_serde(&()).unwrap();
    }
    let opts = tp.expect("");

    let spread = dec!(5);
    let price_history: Vec<_> = prices
        .expect("Couldn't parse prices")
        .iter()
        .map(|r| frame_from(r, spread))
        .collect();
    let latest_price = price_history
        .last()
        .expect("Expected at least one price frame")
        .close;

    let ts = MACD {
        short: opts.short,
        long: opts.long,
        signal: opts.signal,
        entry_lim: opts.entry,
        exit_lim: opts.exit,
    };
    let rs = Donchian {
        channel_length: opts.channel,
    };

    let market = Market {
        code: "GDAXI".to_string(),
        margin_factor: dec!(0.05),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(12),
    };

    let account = Account::new(
        market,
        ts,
        rs,
        dec!(0.03),
        CurrencyAmount::new(dec!(20000), Currency::GBP),
        Resolution::Day,
    );

    let indicators: Vec<_> = account
        .trading_strategy
        .macd(&price_history)
        .iter()
        .zip(account.risk_strategy.channel(&price_history))
        .map(|(ts, rs)| StrategyRecord {
            short_ema: ts.short_ema,
            long_ema: ts.long_ema,
            macd: ts.macd,
            macd_signal: ts.macd_signal,
            macd_trend: ts.macd_trend,
            trend: format!("{:?}", ts.trend),
            long_stop: rs.1,
            short_stop: rs.0,
        })
        .collect();

    let mut test = Backtest::new(account);
    test.run(&price_history);

    let trades = test
        .account
        .trade_log(latest_price)
        .iter()
        .map(|t| Trade {
            open_date: t.entry_time,
            open_price: t.entry_price,
            stop: t.stop,
            close_date: t.exit_time,
            close_price: t.exit_price,
            outcome: format!("{}", t.outcome),
        })
        .collect();

    let result = TestResult { indicators, trades };

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
