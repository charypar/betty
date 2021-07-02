pub mod market;
pub mod price;
pub mod strategy;
pub mod trade;

use std::collections::VecDeque;

use self::market::Market;
use self::price::{CurrencyAmount, Frame, Price, PriceHistory, Resolution};
use self::strategy::{RiskStrategy, TradingStrategy};
use self::trade::{Direction, Exit, Order, Trade, TradeStatus};

// Account holds the state of the trading account and history of all the orders placed
// in response to price updates.
pub struct Account<TS, RS>
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    pub opening_balance: CurrencyAmount,
    pub market: Market,
    pub price_history: PriceHistory,
    pub trading_strategy: TS,
    pub risk_strategy: RS,
    orders: Vec<Order>,
}

#[derive(Debug, PartialEq)]
pub enum AccountError {
    NoMatchingEntry(String),
}

impl<TS, RS> Account<TS, RS>
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    pub fn new(
        market: Market,
        trading_strategy: TS,
        risk_strategy: RS,
        opening_balance: CurrencyAmount,
        resolution: Resolution,
    ) -> Self {
        Account {
            opening_balance,
            market,
            trading_strategy,
            risk_strategy,
            orders: vec![],
            price_history: PriceHistory {
                resolution,
                history: VecDeque::new(),
            },
        }
    }

    // FIXME make the trade log calculation incremental on price update
    pub fn trade_log(&self, latest_price: Price) -> Vec<Trade> {
        if self.orders.len() < 1 {
            return vec![];
        }

        let (mut entries, mut exits) = (vec![], vec![]);
        for order in &self.orders {
            match order {
                Order::Open(en) => entries.push(en.clone()),
                Order::Close(ex) | Order::Stop(ex) => exits.push(ex.clone()),
            }
        }

        let mut trades: Vec<Trade> = (&entries)
            .into_iter()
            .map(|entry| {
                (&exits)
                    .into_iter()
                    .find(|exit| exit.position_id == entry.position_id)
                    .map_or_else(
                        || Trade::open(entry, latest_price),
                        |exit| Trade::closed(entry, exit),
                    )
            })
            .collect();

        trades.sort_by(|a, b| a.entry_time.cmp(&b.entry_time));

        trades
    }

    pub fn balance_history(&self) -> CurrencyAmount {
        todo!()
    }

    // Add new price information
    // This potentially results in new orders to be executed
    pub fn update_price(&mut self, frame: Frame) -> Option<Vec<Order>> {
        self.price_history.history.push_front(frame);

        // Trigger stops
        let orders: Vec<Order> = self
            .trade_log(frame.close)
            .into_iter()
            .filter_map(|t| match t.direction {
                Direction::Buy if t.status == TradeStatus::Open && frame.low.bid < t.stop => {
                    Some(Order::Stop(Exit {
                        position_id: t.id,
                        price: frame.low.bid,
                        time: frame.open_time + self.price_history.resolution,
                    }))
                }
                Direction::Sell if t.status == TradeStatus::Open && frame.high.ask > t.stop => {
                    Some(Order::Stop(Exit {
                        position_id: t.id,
                        price: frame.high.ask,
                        time: frame.open_time + self.price_history.resolution,
                    }))
                }
                _ => None,
            })
            .collect();

        if orders.len() > 0 {
            return Some(orders);
        }

        None
    }

    // Log an order that has been placed
    pub fn log_order(&mut self, order: Order) -> Result<(), AccountError> {
        match &order {
            Order::Close(exit) | Order::Stop(exit) => {
                for o in &self.orders {
                    match o {
                        Order::Open(entry) if exit.position_id == entry.position_id => {
                            return Ok(self.orders.push(order))
                        }
                        _ => continue,
                    }
                }

                return Err(AccountError::NoMatchingEntry(exit.position_id.clone()));
            }
            Order::Open(_) => return Ok(self.orders.push(order)),
        };
    }
}

#[cfg(test)]
mod test {

    use crate::core::price::Price;
    use crate::core::strategy::{Donchian, MACD};
    use crate::core::trade::{Direction, Entry, Exit, TradeOutcome, TradeStatus};

    use super::*;

    use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
    use iso_currency::Currency::GBP;
    use rust_decimal_macros::dec;

    // Trading

    #[test]
    fn logs_a_price_update() {
        let mut account = account();
        let expected = Frame {
            open: Price::new_mid(dec!(100), dec!(1)),
            close: Price::new_mid(dec!(200), dec!(1)),
            low: Price::new_mid(dec!(50), dec!(1)),
            high: Price::new_mid(dec!(150), dec!(1)),
            open_time: date(),
        };
        account.update_price(expected);

        let actual = account.price_history.history[0];

        assert_eq!(actual, expected);
    }

    #[test]
    fn triggers_a_stop() -> Result<(), AccountError> {
        let mut account = account();

        let open_1 = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(2), GBP),
            time: date(),
        };
        let close_1 = Exit {
            position_id: "1".to_string(),
            price: dec!(89), // slippage
            time: date() + Duration::minutes(10),
        };
        account.log_order(Order::Open(open_1))?;
        account.log_order(Order::Close(close_1))?;

        let open = Entry {
            position_id: "2".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        account.log_order(Order::Open(open.clone()))?;

        let price = Frame {
            open: Price::new_mid(dec!(100), dec!(1)),
            close: Price::new_mid(dec!(200), dec!(1)),
            low: Price::new_mid(dec!(50), dec!(1)),
            high: Price::new_mid(dec!(150), dec!(1)),
            open_time: date() + Duration::minutes(10),
        };

        let actual = account.update_price(price);
        let expected = Some(vec![Order::Stop(Exit {
            position_id: "2".to_string(),
            price: dec!(49.5),
            time: date() + Duration::minutes(20),
        })]);

        Ok(assert_eq!(actual, expected))
    }

    // Trade log

    #[test]
    fn gives_an_empty_trade_log_for_no_orders() {
        let account = account();
        let latest_price = Price {
            bid: dec!(110),
            ask: dec!(110),
        };

        let expected = vec![];
        let actual = account.trade_log(latest_price);

        assert_eq!(actual, expected);
    }

    #[test]
    fn logs_an_open_trade_for_a_single_order() -> Result<(), AccountError> {
        let mut account = account();
        let latest_price = Price {
            bid: dec!(110),
            ask: dec!(112),
        };

        let open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        account.log_order(Order::Open(open.clone()))?;

        let expected = vec![Trade {
            id: "1".to_string(),
            status: TradeStatus::Open,
            direction: Direction::Buy,
            entry_time: open.time,
            entry_price: open.price,
            exit_time: None,
            exit_price: None,
            stop: dec!(90),
            size: open.size,
            risk: CurrencyAmount::new(dec!(10), GBP),
            outcome: TradeOutcome::Profit,
            price_diff: dec!(10),
            balance: CurrencyAmount::new(dec!(10), GBP),
            risk_reward: dec!(1.0),
        }];
        let actual = account.trade_log(latest_price);

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn logs_a_closed_trade_for_a_pair_of_orders() -> Result<(), AccountError> {
        let mut account = account();
        let latest_price = Price {
            bid: dec!(110),
            ask: dec!(112),
        };

        let open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        account.log_order(Order::Open(open.clone()))?;

        let close = Exit {
            position_id: "1".to_string(),
            price: dec!(150),
            time: date().with_hour(14).unwrap(),
        };
        account.log_order(Order::Close(close.clone()))?;

        let expected = vec![Trade {
            id: "1".to_string(),
            status: TradeStatus::Closed,
            direction: Direction::Buy,
            entry_time: open.time,
            entry_price: dec!(100),
            exit_time: Some(close.time),
            exit_price: Some(close.price),
            stop: open.stop,
            size: open.size,
            risk: CurrencyAmount::new(dec!(10), GBP),
            outcome: TradeOutcome::Profit,
            price_diff: dec!(50),
            balance: CurrencyAmount::new(dec!(50), GBP),
            risk_reward: dec!(5.0),
        }];
        let actual = account.trade_log(latest_price);

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn logs_three_trades_for_five_orders() -> Result<(), AccountError> {
        let mut account = account();
        let latest_price = Price {
            bid: dec!(64),
            ask: dec!(66),
        };

        // Closed long Stop, Closed short Win, Open long Loss
        let open_1 = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(2), GBP),
            time: date(),
        };
        let close_1 = Exit {
            position_id: "1".to_string(),
            price: dec!(89), // slippage
            time: date() + Duration::minutes(10),
        };
        let open_2 = Entry {
            position_id: "2".to_string(),
            direction: Direction::Sell,
            price: dec!(80),
            stop: dec!(85),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date() + Duration::minutes(20),
        };
        let close_2 = Exit {
            position_id: "2".to_string(),
            price: dec!(60),
            time: date() + Duration::minutes(30),
        };
        let open_3 = Entry {
            position_id: "3".to_string(),
            direction: Direction::Buy,
            price: dec!(70),
            stop: dec!(60),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date() + Duration::minutes(40),
        };

        let expected = vec![
            Trade::closed(&open_1, &close_1),
            Trade::closed(&open_2, &close_2),
            Trade::open(&open_3, latest_price),
        ];

        account.log_order(Order::Open(open_1))?;
        account.log_order(Order::Stop(close_1))?;
        account.log_order(Order::Open(open_3))?; // out of order should not matter
        account.log_order(Order::Open(open_2))?;
        account.log_order(Order::Stop(close_2))?;

        let actual = account.trade_log(latest_price);

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn does_not_allow_to_log_a_close_order_without_matching_open() -> Result<(), AccountError> {
        let mut account = account();

        let open_1 = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(2), GBP),
            time: date(),
        };
        let close_1 = Exit {
            position_id: "1".to_string(),
            price: dec!(89), // slippage
            time: date() + Duration::minutes(10),
        };
        account.log_order(Order::Open(open_1))?;
        account.log_order(Order::Stop(close_1))?;

        let close = Exit {
            position_id: "3".to_string(),
            price: dec!(89), // slippage
            time: date() + Duration::minutes(10),
        };

        assert_eq!(
            Err(AccountError::NoMatchingEntry("3".to_string())),
            account.log_order(Order::Close(close.clone()))
        );

        assert_eq!(
            Err(AccountError::NoMatchingEntry("3".to_string())),
            account.log_order(Order::Stop(close))
        );

        Ok(())
    }

    // Fixtures

    fn account() -> Account<MACD, Donchian> {
        Account::new(
            market(),
            trading_strategy(),
            risk_strategy(),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        )
    }

    fn market() -> Market {
        Market {
            code: "UKX".to_string(),
            min_deal_size: CurrencyAmount::new(dec!(0.50), GBP),
            min_stop_distance: dec!(8),
            margin_factor: 20,
        }
    }

    fn trading_strategy() -> MACD {
        MACD {
            short_trend_length: 5,
            long_trend_length: 20,
        }
    }

    fn risk_strategy() -> Donchian {
        Donchian { channel_length: 20 }
    }

    fn date() -> DateTime<Utc> {
        Utc.ymd(2021, 1, 1).and_hms(10, 1, 0)
    }
}
