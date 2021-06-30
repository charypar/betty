mod core;

use iso_currency::Currency;
use rust_decimal_macros::dec;

use crate::core::market::Market;
use crate::core::price::CurrencyAmount;
use crate::core::strategy::{RiskStrategy, TradingStrategy};
use crate::core::Account;

fn main() {
    let market = Market {
        code: "UKX".to_string(),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(8),
        margin_factor: 20,
    };

    let ts = TradingStrategy {
        short_trend_length: 5,
        long_trend_length: 20,
    };

    let rs = RiskStrategy {
        channel_length: 20,
        risk_per_trade: dec!(3),
    };

    let account = Account::new(
        market,
        ts,
        rs,
        CurrencyAmount::new(dec!(10000.00), Currency::GBP),
    );

    // TODO feed in a price history and log resulting orders

    for trade in account.trade_log() {
        println!("{:?}", trade);
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
