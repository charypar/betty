mod core;

use crate::core::market::Market;
use crate::core::price::CurrencyAmount;
use crate::core::strategy::{RiskStrategy, TradingStrategy};
use crate::core::Account;

use rust_decimal_macros::dec;

fn main() {
    let market = Market {
        code: "UKX".to_string(),
        min_deal_size: CurrencyAmount((dec!(0.50), "GBP".to_string())),
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
        CurrencyAmount((dec!(10000.00), "GBP".to_string())),
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
