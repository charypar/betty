mod print;
mod read;

use std::io;

use iso_currency::Currency;
use rust_decimal_macros::dec;

use betty::account::Account;
use betty::market::Market;
use betty::price::Frame;
use betty::price::{CurrencyAmount, Resolution};
use betty::strategies::{Donchian, MACD};
use betty::strategy::{RiskStrategy, TradingStrategy};
use betty::trade::{Entry, Exit, Order};

use crate::print::format_trade_log;
use crate::read::read_prices_csv;

fn main() {
    let prices = read_prices_csv(io::stdin());

    let market = Market {
        code: "GDAXI".to_string(),
        margin_factor: dec!(0.05),
        min_deal_size: CurrencyAmount::new(dec!(0.50), Currency::GBP),
        min_stop_distance: dec!(12),
    };

    let ts = MACD {
        short: 16,
        long: 42,
        signal: 20,
        entry_lim: dec!(20),
        exit_lim: dec!(10),
    };
    let rs = Donchian { channel_length: 50 };

    let opening_balance = CurrencyAmount::new(dec!(20000.00), Currency::GBP);

    let mut account = Account::new(market, ts, rs, dec!(0.03), opening_balance, Resolution::Day);

    run_test(&mut account, &prices);

    let latest_price = prices.last().unwrap().close;
    let trade_log = account.trade_log(latest_price);

    let log = format_trade_log(&trade_log, opening_balance, latest_price);
    println!("{}", log);
}

// TODO turn this into a back-test optimiser

fn run_test<TS, RS>(account: &mut Account<TS, RS>, prices: &Vec<Frame>)
where
    TS: TradingStrategy,
    RS: RiskStrategy,
{
    // Run the test
    let mut p_id = 0;

    for price in prices {
        for order in account.update_price(*price) {
            match order {
                Order::Open(entry) => {
                    if let Err(_) = account.market.validate_entry(&entry, account.balance) {
                        continue;
                    }

                    let o = Order::Open(Entry {
                        position_id: p_id.to_string(),
                        ..entry
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("!! Position already open");
                    }
                }
                Order::Close(exit) => {
                    let o = Order::Close(Exit {
                        position_id: p_id.to_string(),
                        ..exit
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("Position already closed");
                    }

                    p_id += 1;
                }
                Order::Stop(exit) => {
                    let o = Order::Stop(Exit {
                        position_id: p_id.to_string(),
                        ..exit
                    });

                    let r = account.log_order(o);
                    if let Err(_) = r {
                        println!("Position already closed");
                    }

                    p_id += 1;
                }
            }
        }
    }
}
