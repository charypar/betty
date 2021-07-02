mod core;

use iso_currency::Currency;
use rust_decimal_macros::dec;

use crate::core::market::Market;
use crate::core::price::{CurrencyAmount, Price, Resolution};
use crate::core::strategy::{Donchian, MACD};
use crate::core::Account;

fn main() {
    let market = Market {
        code: "UKX".to_string(),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(8),
        margin_factor: 20,
    };

    let ts = MACD {
        short_trend_length: 5,
        long_trend_length: 20,
    };

    let rs = Donchian { channel_length: 20 };

    let account = Account::new(
        market,
        ts,
        rs,
        CurrencyAmount::new(dec!(10000.00), Currency::GBP),
        Resolution::Minute(10),
    );

    // TODO feed in a price history and log resulting orders
    let latest_price = Price {
        bid: dec!(110),
        ask: dec!(110),
    };

    for trade in account.trade_log(latest_price) {
        println!("{:?}", trade);
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
