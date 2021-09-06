use crate::account::Account;
use crate::price::Frame;
use crate::strategy::{RiskStrategy, TradingStrategy};
use crate::trade::{Entry, Exit, Order};

pub struct Backtest<TS, RS>
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    pub account: Account<TS, RS>,
    pub p_id: usize,
    pub trace: Vec<Result<Order, String>>,
}

impl<TS, RS> Backtest<TS, RS>
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    pub fn new(account: Account<TS, RS>) -> Self {
        Self {
            account,
            p_id: 0,
            trace: Vec::new(),
        }
    }

    pub fn run(&mut self, prices: &Vec<Frame>) {
        for price in prices {
            let orders = self.account.update_price(*price);

            for order in orders {
                let event = self.place_order(&order);
                self.trace.push(event);
            }
        }
    }

    fn place_order(&mut self, order: &Order) -> Result<Order, String> {
        match order {
            Order::Open(entry) => {
                self.account
                    .market
                    .validate_entry(&entry, self.account.balance)
                    .map_err(|e| format!("Market rejected entry: {:?}, {}", entry, e))?;

                let o = Order::Open(Entry {
                    position_id: self.p_id.to_string(),
                    ..entry.clone()
                });

                self.account
                    .log_order(o.clone())
                    .map_err(|e| format!("{}", e))?;

                Ok(o)
            }
            Order::Close(exit) => {
                let o = Order::Close(Exit {
                    position_id: self.p_id.to_string(),
                    ..exit.clone()
                });

                self.account
                    .log_order(o.clone())
                    .map_err(|e| format!("{}", e))?;

                self.p_id += 1;
                Ok(o)
            }
            Order::Stop(exit) => {
                let o = Order::Stop(Exit {
                    position_id: self.p_id.to_string(),
                    ..exit.clone()
                });

                self.account
                    .log_order(o.clone())
                    .map_err(|e| format!("{}", e))?;

                self.p_id += 1;
                Ok(o)
            }
        }
    }
}
