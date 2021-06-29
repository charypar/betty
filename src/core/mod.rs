use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub mod market;
pub mod price;
pub mod strategy;
pub mod trade;

use market::*;
use price::*;
use strategy::*;
use trade::*;

// Account holds the state of the trading account and history of all the orders placed
// in response to price updates.
pub struct Account {
    pub opening_balance: CurrencyAmount,
    pub market: Market,
    pub price_history: PriceHistory,
    pub trading_strategy: TradingStrategy,
    pub risk_strategy: RiskStrategy,
    pub orders: Vec<Order>,
}

impl Account {
    pub fn new(
        market: Market,
        trading_strategy: TradingStrategy,
        risk_strategy: RiskStrategy,
        opening_balance: CurrencyAmount,
    ) -> Self {
        Account {
            opening_balance,
            market,
            trading_strategy,
            risk_strategy,
            orders: vec![],
            price_history: PriceHistory {
                resolution: Resolution::Minute(10),
                history: vec![],
            },
        }
    }

    pub fn trade_log(&self) -> Vec<Trade> {
        todo!()
    }

    pub fn balance_history(&self) -> CurrencyAmount {
        todo!()
    }

    // Add new price information
    // This potentially results in a new order to be placed
    pub fn update_price(&mut self, frame: Frame) -> Option<Order> {
        todo!()
    }

    // Log an order that has been confirmed by the broker
    pub fn log_order(&mut self, order: Order) {
        todo!()
    }
}

#[derive(Debug)]
pub enum TradeStatus {
    Open,
    Closed,
}

#[derive(Debug)]
pub enum TradeOutcome {
    Profit,
    Loss,
}

// A row in a trade log
#[derive(Debug)]
pub struct Trade {
    id: String,
    status: TradeStatus,
    // Entry
    direction: Direction,
    entry_time: DateTime<Utc>,
    entry_price: Points,
    // Exit
    exit_time: Option<DateTime<Utc>>,
    exit_price: Option<Points>,
    // Risk
    stop: Points,
    size: CurrencyAmount,
    risk: CurrencyAmount,
    // Outcome
    outcome: TradeOutcome,
    price_diff: Points,
    balance: CurrencyAmount,
    risk_reward: Decimal,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}
