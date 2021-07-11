pub mod market;
pub mod price;
pub mod strategy;
pub mod trade;

use std::collections::VecDeque;

use rust_decimal::Decimal;

use self::market::Market;
use self::price::{CurrencyAmount, Frame, Price, PriceHistory, Resolution};
use self::strategy::{RiskStrategy, Signal, TradingStrategy};
use self::trade::{Direction, Entry, Exit, Order, Trade, TradeStatus};

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
    pub risk_per_trade: Decimal,
    orders: Vec<Order>,
}

#[derive(Debug, PartialEq)]
pub enum AccountError {
    NoMatchingEntry(String),
    DuplicatePosition(String),
    PositionAlreadyClosed(String),
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
        risk_per_trade: Decimal,
        opening_balance: CurrencyAmount,
        resolution: Resolution,
    ) -> Self {
        Account {
            opening_balance,
            market,
            trading_strategy,
            risk_strategy,
            risk_per_trade,
            orders: vec![],
            price_history: PriceHistory {
                resolution,
                history: VecDeque::new(),
            },
        }
    }

    pub fn trade_log(&self, latest_price: Price) -> Vec<Trade> {
        if self.orders.len() < 1 {
            return vec![];
        }

        let mut trades: Vec<Trade> = (&self.orders)
            .iter()
            .filter_map(|order| match order {
                Order::Open(entry) => {
                    if let Some(exit) = self.matching_exit(&entry) {
                        Some(Trade::closed(&entry, exit))
                    } else {
                        Some(Trade::open(&entry, latest_price))
                    }
                }
                _ => None,
            })
            .collect();

        trades.sort_by(|a, b| a.entry_time.cmp(&b.entry_time));

        trades
    }

    // Add new price information
    // This potentially results in new orders to be executed
    pub fn update_price(&mut self, frame: Frame) -> Vec<Order> {
        self.price_history.history.push_front(frame);

        let time = frame.open_time + self.price_history.resolution;
        let signal = self.trading_strategy.signal(&self.price_history);
        let open_trades = self
            .trade_log(frame.close)
            .into_iter()
            .filter(|t| t.status == TradeStatus::Open);

        let mut orders = vec![];

        for t in open_trades {
            // Stop
            match t.direction {
                Direction::Buy if frame.low.bid < t.stop => {
                    orders.push(Order::Stop(t.exit(frame.close, time)));
                }
                Direction::Sell if frame.high.ask > t.stop => {
                    orders.push(Order::Stop(t.exit(frame.close, time)));
                }
                _ => (),
            }

            // Exit
            match signal {
                Some(Signal::Exit(direction)) if t.direction == direction => {
                    orders.push(Order::Close(t.exit(frame.close, time)));
                }
                Some(Signal::Enter(direction)) if t.direction != direction => {
                    orders.push(Order::Close(t.exit(frame.close, time)));
                }
                _ => (),
            }
        }

        // Enter a new trade on signal
        if let Some(Signal::Enter(direction)) = &signal {
            let risk = self.opening_balance * self.risk_per_trade;

            if let Ok(entry) = self
                .risk_strategy
                .entry(*direction, &self.price_history, risk)
            {
                orders.push(Order::Open(entry));
            }
        }

        orders
    }

    // Log an order that has been placed
    pub fn log_order(&mut self, order: Order) -> Result<(), AccountError> {
        match &order {
            Order::Open(entry) => {
                self.check_entry(entry)?;

                return Ok(self.orders.push(order));
            }
            Order::Close(exit) | Order::Stop(exit) => {
                let matching_entry = self.matching_entry(exit)?;

                if let Some(_) = matching_entry {
                    return Ok(self.orders.push(order));
                } else {
                    return Err(AccountError::NoMatchingEntry(exit.position_id.clone()));
                }
            }
        };
    }

    // FIXME these are all O(n) in number of orders
    // They can be improved by indexing orders by position_id when logging them.

    fn check_entry(&self, entry: &Entry) -> Result<(), AccountError> {
        for o in &self.orders {
            match o {
                Order::Open(e) if entry.position_id == e.position_id => {
                    // There already is an entry for this position
                    return Err(AccountError::DuplicatePosition(entry.position_id.clone()));
                }
                _ => continue,
            }
        }

        Ok(())
    }

    fn matching_entry(&self, exit: &Exit) -> Result<Option<&Entry>, AccountError> {
        let mut matching_entry = None;

        for o in &self.orders {
            match o {
                Order::Close(e) | Order::Stop(e) if e.position_id == exit.position_id => {
                    return Err(AccountError::PositionAlreadyClosed(e.position_id.clone()));
                }
                Order::Open(entry) if exit.position_id == entry.position_id => {
                    matching_entry = Some(entry);
                }
                _ => continue,
            }
        }

        Ok(matching_entry)
    }

    fn matching_exit(&self, entry: &Entry) -> Option<&Exit> {
        let id = &entry.position_id;

        for o in &self.orders {
            match o {
                Order::Close(e) | Order::Stop(e) if &e.position_id == id => {
                    // Duplicate exit
                    return Some(e);
                }
                _ => continue,
            }
        }

        None
    }
}

#[cfg(test)]
mod test {

    use crate::core::price::Price;
    use crate::core::strategy::Donchian;
    use crate::core::trade::{Direction, Entry, Exit, TradeOutcome, TradeStatus};

    use super::strategy::{RiskStrategyError, Signal};
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
        let expected = vec![Order::Stop(Exit {
            position_id: "2".to_string(),
            price: dec!(199.5),
            time: date() + Duration::minutes(20),
        })];

        Ok(assert_eq!(actual, expected))
    }

    #[test]
    fn opens_a_position_based_on_a_signal() -> Result<(), RiskStrategyError> {
        let long = LongEntry {};
        let mut long_account = Account::new(
            market(),
            long,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let short = ShortEntry {};
        let mut short_account = Account::new(
            market(),
            short,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );

        let expected_long = vec![Order::Open(long_account.risk_strategy.entry(
            Direction::Buy,
            &history(),
            CurrencyAmount::new(dec!(10), GBP),
        )?)];
        let actual_long = long_account.update_price(frame());

        assert_eq!(actual_long, expected_long);

        let expected_long = vec![Order::Open(short_account.risk_strategy.entry(
            Direction::Sell,
            &history(),
            CurrencyAmount::new(dec!(10), GBP),
        )?)];
        let actual_long = short_account.update_price(frame());

        assert_eq!(actual_long, expected_long);

        Ok(())
    }

    #[test]
    fn closes_a_position_based_on_an_exit_signal() -> Result<(), AccountError> {
        let long = LongExit {};
        let mut long_account = Account::new(
            market(),
            long,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let long_open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(40),
            stop: dec!(30),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        long_account.log_order(Order::Open(long_open.clone()))?;

        let short = ShortExit {};
        let mut short_account = Account::new(
            market(),
            short,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let short_open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Sell,
            price: dec!(250),
            stop: dec!(260),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        short_account.log_order(Order::Open(short_open.clone()))?;

        let expected_long = vec![Order::Close(Exit {
            position_id: "1".to_string(),
            price: dec!(199.5),
            time: date() + Duration::minutes(10),
        })];
        let actual_long = long_account.update_price(frame());

        assert_eq!(actual_long, expected_long);

        let expected_short = vec![Order::Close(Exit {
            position_id: "1".to_string(),
            price: dec!(200.5),
            time: date() + Duration::minutes(10),
        })];
        let actual_short = short_account.update_price(frame());

        assert_eq!(actual_short, expected_short);

        Ok(())
    }

    #[test]
    fn reverses_a_positon_based_on_an_entry_signal() -> Result<(), ()> {
        let long = ShortEntry {};
        let mut long_account = Account::new(
            market(),
            long,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let long_open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(40),
            stop: dec!(30),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        long_account
            .log_order(Order::Open(long_open.clone()))
            .map_err(|_| ())?;

        let short = LongEntry {};
        let mut short_account = Account::new(
            market(),
            short,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let short_open = Entry {
            position_id: "1".to_string(),
            direction: Direction::Sell,
            price: dec!(250),
            stop: dec!(260),
            size: CurrencyAmount::new(dec!(1), GBP),
            time: date(),
        };
        short_account
            .log_order(Order::Open(short_open.clone()))
            .map_err(|_| ())?;

        let expected_long = vec![
            Order::Close(Exit {
                position_id: "1".to_string(),
                price: dec!(199.5),
                time: date() + Duration::minutes(10),
            }),
            Order::Open(
                long_account
                    .risk_strategy
                    .entry(
                        Direction::Sell,
                        &history(),
                        CurrencyAmount::new(dec!(10), GBP),
                    )
                    .map_err(|_| ())?,
            ),
        ];
        let actual_long = long_account.update_price(frame());

        assert_eq!(actual_long, expected_long);

        let expected_short = vec![
            Order::Close(Exit {
                position_id: "1".to_string(),
                price: dec!(200.5),
                time: date() + Duration::minutes(10),
            }),
            Order::Open(
                short_account
                    .risk_strategy
                    .entry(
                        Direction::Buy,
                        &history(),
                        CurrencyAmount::new(dec!(10), GBP),
                    )
                    .map_err(|_| ())?,
            ),
        ];
        let actual_short = short_account.update_price(frame());

        assert_eq!(actual_short, expected_short);

        Ok(())
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

    // Order validation

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

    #[test]
    fn rejects_an_order_with_duplicate_position_id() -> Result<(), AccountError> {
        let mut account = account();

        let open_1 = Entry {
            position_id: "1".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(2), GBP),
            time: date(),
        };
        account.log_order(Order::Open(open_1.clone()))?;

        assert_eq!(
            Err(AccountError::DuplicatePosition("1".to_string())),
            account.log_order(Order::Open(open_1))
        );

        Ok(())
    }

    #[test]
    fn rejects_orders_for_closed_positions() -> Result<(), AccountError> {
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
        account.log_order(Order::Close(close_1.clone()))?;

        let open_2 = Entry {
            position_id: "2".to_string(),
            direction: Direction::Buy,
            price: dec!(100),
            stop: dec!(90),
            size: CurrencyAmount::new(dec!(2), GBP),
            time: date(),
        };
        let close_2 = Exit {
            position_id: "2".to_string(),
            price: dec!(89), // slippage
            time: date() + Duration::minutes(10),
        };
        account.log_order(Order::Open(open_2))?;
        account.log_order(Order::Stop(close_2.clone()))?;

        assert_eq!(
            Err(AccountError::PositionAlreadyClosed("1".to_string())),
            account.log_order(Order::Close(close_1.clone()))
        );
        assert_eq!(
            Err(AccountError::PositionAlreadyClosed("1".to_string())),
            account.log_order(Order::Stop(close_1))
        );
        assert_eq!(
            Err(AccountError::PositionAlreadyClosed("2".to_string())),
            account.log_order(Order::Close(close_2.clone()))
        );
        assert_eq!(
            Err(AccountError::PositionAlreadyClosed("2".to_string())),
            account.log_order(Order::Close(close_2))
        );

        Ok(())
    }

    // Fixtures
    struct LongEntry;
    impl TradingStrategy for LongEntry {
        fn signal(&self, _history: &PriceHistory) -> Option<Signal> {
            Some(Signal::Enter(Direction::Buy))
        }
    }

    struct LongExit;
    impl TradingStrategy for LongExit {
        fn signal(&self, _history: &PriceHistory) -> Option<Signal> {
            Some(Signal::Exit(Direction::Buy))
        }
    }
    struct ShortEntry;
    impl TradingStrategy for ShortEntry {
        fn signal(&self, _history: &PriceHistory) -> Option<Signal> {
            Some(Signal::Enter(Direction::Sell))
        }
    }
    struct ShortExit;
    impl TradingStrategy for ShortExit {
        fn signal(&self, _history: &PriceHistory) -> Option<Signal> {
            Some(Signal::Exit(Direction::Sell))
        }
    }

    struct NoSignal;
    impl TradingStrategy for NoSignal {
        fn signal(&self, _history: &PriceHistory) -> Option<Signal> {
            None
        }
    }

    fn account() -> Account<NoSignal, Donchian> {
        Account::new(
            market(),
            trading_strategy(),
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        )
    }

    fn market() -> Market {
        Market {
            code: "UKX".to_string(),
            min_deal_size: CurrencyAmount::new(dec!(0.50), GBP),
            min_stop_distance: dec!(8),
            margin_factor: dec!(0.5),
        }
    }

    fn trading_strategy() -> NoSignal {
        NoSignal {}
    }

    fn risk_strategy() -> Donchian {
        Donchian { channel_length: 1 }
    }

    fn date() -> DateTime<Utc> {
        Utc.ymd(2021, 1, 1).and_hms(10, 1, 0)
    }

    fn frame() -> Frame {
        Frame {
            open: Price::new_mid(dec!(100), dec!(1)),
            close: Price::new_mid(dec!(200), dec!(1)),
            low: Price::new_mid(dec!(50), dec!(1)),
            high: Price::new_mid(dec!(150), dec!(1)),
            open_time: date(),
        }
    }

    fn history() -> PriceHistory {
        PriceHistory {
            resolution: Resolution::Minute(10),
            history: vec![frame()].into(),
        }
    }
}
