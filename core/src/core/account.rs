use std::collections::VecDeque;
use std::convert::TryInto;
use std::error::Error;
use std::fmt::Display;

use rust_decimal::Decimal;

use crate::core::market::Market;
use crate::core::price::{CurrencyAmount, Frame, Price, PriceHistory, Resolution};
use crate::core::strategy::{RiskStrategy, TradingStrategy, Trend};
use crate::core::trade::{Direction, Entry, Order, Trade};

// Account holds the state of the trading account and history of all the orders placed
// in response to price updates.
pub struct Account<TS, RS>
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    pub balance: CurrencyAmount,
    pub market: Market,
    pub price_history: PriceHistory,
    pub trading_strategy: TS,
    pub risk_strategy: RS,
    pub risk_per_trade: Decimal,
    closed_trades: Vec<Trade>,
    live_trade: Option<Entry>,
}

#[derive(Debug, PartialEq)]
pub enum AccountError {
    DuplicateEntry(String),
    NoMatchingEntry(String),
    PositionAlreadyClosed(String),
}

impl Error for AccountError {}

impl Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountError::DuplicateEntry(s) => writeln!(f, "Duplicate position {}", s),
            AccountError::NoMatchingEntry(s) => writeln!(f, "No matching entry {}", s),
            AccountError::PositionAlreadyClosed(s) => writeln!(f, "Position {} alerady closed", s),
        }
    }
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
            balance: opening_balance,
            market,
            trading_strategy,
            risk_strategy,
            risk_per_trade,
            price_history: PriceHistory {
                resolution,
                history: VecDeque::new(),
            },
            closed_trades: vec![],
            live_trade: None,
        }
    }

    pub fn trade_log(&self, latest_price: Price) -> Vec<Trade> {
        let mut trades: Vec<Trade> = self
            .closed_trades
            .iter()
            .cloned()
            .chain(
                self.live_trade
                    .as_ref()
                    .map(|e| Trade::open(&e, latest_price)),
            )
            .collect();

        trades.sort_by(|a, b| a.entry_time.cmp(&b.entry_time));

        trades
    }

    // Add new price information
    // This potentially results in new orders to be executed
    pub fn update_price(&mut self, frame: Frame) -> Vec<Order> {
        self.price_history.history.push_front(frame);

        let time = frame.close_time;
        let trend = self.trading_strategy.trend(&self.price_history);

        let mut orders = vec![];

        // Handle exits first
        if let Some(lt) = &self.live_trade {
            match trend {
                // Stop - thes are only in the match so we don't generate both stop and close at the same time
                _ if lt.direction == Direction::Buy && frame.low.bid < lt.stop => {
                    orders.push(Order::Stop(lt.exit(frame.close, time)));
                }
                _ if lt.direction == Direction::Sell && frame.high.ask > lt.stop => {
                    orders.push(Order::Stop(lt.exit(frame.close, time)));
                }
                // Exit
                Trend::Neutral => {
                    orders.push(Order::Close(lt.exit(frame.close, time)));
                }
                // Reverse
                Trend::Bullish if lt.direction == Direction::Sell => {
                    orders.push(Order::Close(lt.exit(frame.close, time)));
                }
                Trend::Bearish if lt.direction == Direction::Buy => {
                    orders.push(Order::Close(lt.exit(frame.close, time)));
                }
                // Stay
                _ => (),
            }
        }

        if self.live_trade.is_none() || orders.len() > 0 {
            match trend {
                Trend::Bullish | Trend::Bearish => {
                    let risk = self.balance * self.risk_per_trade;
                    let dir = trend
                        .try_into()
                        .expect("Trend could not convert to direction");

                    if let Ok(entry) = self.risk_strategy.entry(dir, &self.price_history, risk) {
                        orders.push(Order::Open(entry));
                    }
                }
                _ => (),
            }
        }

        orders
    }

    // Log an order that has been placed
    pub fn log_order(&mut self, order: Order) -> Result<(), AccountError> {
        match (order, &self.live_trade) {
            (Order::Open(entry), None) => {
                self.live_trade = Some(entry);

                return Ok(());
            }
            (Order::Open(_), Some(entry)) => {
                return Err(AccountError::DuplicateEntry(entry.position_id.clone()));
            }
            (Order::Close(exit) | Order::Stop(exit), None) => {
                if self.closed_trades.iter().any(|t| t.id == exit.position_id) {
                    return Err(AccountError::PositionAlreadyClosed(
                        exit.position_id.clone(),
                    ));
                } else {
                    return Err(AccountError::NoMatchingEntry(exit.position_id.clone()));
                }
            }
            (Order::Close(exit) | Order::Stop(exit), Some(entry)) => {
                let trade = Trade::closed(&entry, &exit);
                self.balance += trade.profit;
                self.live_trade = None;

                return Ok(self.closed_trades.push(trade));
            }
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::core::price::{Points, Price};
    use crate::core::strategy::RiskStrategyError;
    use crate::core::trade::{Direction, Entry, Exit, TradeOutcome, TradeStatus};
    use crate::strategy::Trend;

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
            close_time: date(),
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
            time: date(),
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
            close_time: date() + Duration::minutes(10),
        };

        let actual = account.update_price(price);
        let expected = vec![Order::Stop(Exit {
            position_id: "2".to_string(),
            price: dec!(199.5),
            time: date() + Duration::minutes(10),
        })];

        Ok(assert_eq!(actual, expected))
    }

    #[test]
    fn opens_a_position_based_on_a_trend() -> Result<(), RiskStrategyError> {
        let bullish_strategy = Bullish {};
        let mut long_account = Account::new(
            market(),
            bullish_strategy,
            risk_strategy(),
            dec!(0.01),
            CurrencyAmount::new(dec!(1000), GBP),
            Resolution::Minute(10),
        );
        let bearish_strategy = Bearish {};
        let mut short_account = Account::new(
            market(),
            bearish_strategy,
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
    fn closes_a_position_based_on_a_trend_ending() -> Result<(), AccountError> {
        let neutral_strategy = Neutral {};
        let mut long_account = Account::new(
            market(),
            neutral_strategy,
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

        let neutral_strategy = Neutral {};
        let mut short_account = Account::new(
            market(),
            neutral_strategy,
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
            time: date(),
        })];
        let actual_long = long_account.update_price(frame());

        assert_eq!(actual_long, expected_long);

        let expected_short = vec![Order::Close(Exit {
            position_id: "1".to_string(),
            price: dec!(200.5),
            time: date(),
        })];
        let actual_short = short_account.update_price(frame());

        assert_eq!(actual_short, expected_short);

        Ok(())
    }

    #[test]
    fn reverses_a_positon_based_on_tred_reversal() -> Result<(), ()> {
        let bearish_strategy = Bearish {};
        let mut long_account = Account::new(
            market(),
            bearish_strategy,
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
            .log_order(Order::Open(long_open))
            .map_err(|_| ())?;

        let bullish_strategy = Bullish {};
        let mut short_account = Account::new(
            market(),
            bullish_strategy,
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
            .log_order(Order::Open(short_open))
            .map_err(|_| ())?;

        let expected_long = vec![
            Order::Close(Exit {
                position_id: "1".to_string(),
                price: dec!(199.5),
                time: date(),
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
                time: date(),
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
            profit: CurrencyAmount::new(dec!(10), GBP),
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
            profit: CurrencyAmount::new(dec!(50), GBP),
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
        account.log_order(Order::Open(open_2))?;
        account.log_order(Order::Stop(close_2))?;
        account.log_order(Order::Open(open_3))?;

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
            Err(AccountError::DuplicateEntry("1".to_string())),
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

    struct Neutral {}
    impl TradingStrategy for Neutral {
        fn trend(&self, _history: &PriceHistory) -> crate::strategy::Trend {
            Trend::Neutral
        }
    }

    struct Bullish {}
    impl TradingStrategy for Bullish {
        fn trend(&self, _history: &PriceHistory) -> crate::strategy::Trend {
            Trend::Bullish
        }
    }

    struct Bearish {}
    impl TradingStrategy for Bearish {
        fn trend(&self, _history: &PriceHistory) -> crate::strategy::Trend {
            Trend::Bearish
        }
    }

    struct NoRisk {}

    impl RiskStrategy for NoRisk {
        fn stop(
            &self,
            _direction: Direction,
            history: &PriceHistory,
        ) -> Result<Points, RiskStrategyError> {
            Ok(history.history[0].close.mid_price())
        }
    }

    fn account() -> Account<Neutral, NoRisk> {
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

    fn trading_strategy() -> Neutral {
        Neutral {}
    }

    fn risk_strategy() -> NoRisk {
        NoRisk {}
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
            close_time: date(),
        }
    }

    fn history() -> PriceHistory {
        PriceHistory {
            resolution: Resolution::Minute(10),
            history: vec![frame()].into(),
        }
    }
}
