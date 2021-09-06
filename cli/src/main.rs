mod print;
mod read;

use std::io;

use betty::backtest::Backtest;
use iso_currency::Currency;
use rust_decimal_macros::dec;

use betty::account::Account;
use betty::market::Market;
use betty::price::{CurrencyAmount, Resolution};
use betty::strategies::{Donchian, MACD};

use crate::print::format_trade_log;
use crate::read::read_prices_csv;

fn main() {
    let prices = read_prices_csv(io::stdin());
    let latest_price = prices.last().unwrap().close;

    let market = Market {
        code: "GDAXI".to_string(),
        margin_factor: dec!(0.05),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(12),
    };

    let ts = MACD {
        short: 12,
        long: 42,
        signal: 10,
        entry_lim: dec!(40),
        exit_lim: dec!(40),
    };
    let rs = Donchian { channel_length: 20 };

    let opening_balance = CurrencyAmount::new(dec!(20000.00), Currency::GBP);

    let account = Account::new(market, ts, rs, dec!(0.03), opening_balance, Resolution::Day);

    let mut backtest = Backtest::new(account);
    backtest.run(&prices);

    let trade_log = backtest.account.trade_log(latest_price);

    let log = format_trade_log(&trade_log, opening_balance, latest_price);
    println!("{}", log);
}
