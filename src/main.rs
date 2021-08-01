mod core;
mod strategies;

use std::error::Error;
use std::io;

use chrono::{DateTime, TimeZone, Utc};
use iso_currency::Currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Deserializer};
use term_table::{row::Row, table_cell::TableCell, Table, TableStyle};
use termion::color;

use crate::core::market::Market;
use crate::core::price::Frame;
use crate::core::price::{CurrencyAmount, Price, Resolution};
use crate::core::trade::Direction;
use crate::core::trade::Entry;
use crate::core::trade::Exit;
use crate::core::trade::Order;
use crate::core::trade::TradeOutcome;
use crate::core::trade::TradeStatus;
use crate::core::Account;
use crate::strategies::{Donchian, MACD};

// CSV processing
// FIXME move this somewhere else

#[derive(Deserialize, Debug)]
struct PriceRecord {
    #[serde(rename = "Date", deserialize_with = "parse_date")]
    date: DateTime<Utc>,
    #[serde(rename = "Open")]
    open: Decimal,
    #[serde(rename = "High")]
    high: Decimal,
    #[serde(rename = "Low")]
    low: Decimal,
    #[serde(rename = "Close")]
    close: Decimal,
}

const DATE_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S";

fn parse_date<'de, D>(de: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(de)?;

    Utc.datetime_from_str(&s, DATE_FORMAT)
        .map_err(serde::de::Error::custom)
}

fn frame_from(price_record: PriceRecord, spread: Decimal) -> Frame {
    Frame {
        close_time: price_record.date,
        open: Price::new_mid(price_record.open, spread),
        high: Price::new_mid(price_record.high, spread),
        low: Price::new_mid(price_record.low, spread),
        close: Price::new_mid(price_record.close, spread),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Read the price

    let mut reader = csv::Reader::from_reader(io::stdin());
    let prices: Vec<_> = reader
        .deserialize()
        .flat_map(|line| -> Result<Frame, csv::Error> { Ok(frame_from(line?, dec!(5))) })
        .collect();

    // Set up a test run

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

    let mut account = Account::new(
        market,
        ts,
        rs,
        dec!(0.025),
        CurrencyAmount::new(dec!(20000.00), Currency::GBP),
        Resolution::Day,
    );

    // Run the test
    let mut p_id = 0;

    for price in &prices {
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

    // TODO feed in a price history and log resulting orders
    let latest_price = prices.last().unwrap().close;

    // Pretty print a trade log
    let mut table = Table::new();
    table.max_column_width = 40;
    table.style = TableStyle::simple();
    table.add_row(Row::new(
        vec![
            "ID", "Status", "Entry", "Price", "Dir", "Exit", "Price", "Stop", "Change", "£ PP",
            "Risk", "Outcome", "Profit", "RR", "Balance",
        ]
        .into_iter()
        .map(|it| TableCell::new(it)),
    ));

    let trade_log = account.trade_log(latest_price);
    let mut balance = CurrencyAmount::new(dec!(20000), Currency::GBP);

    for trade in trade_log {
        balance += trade.profit;

        table.add_row(Row::new(
            vec![
                trade.id,
                match trade.status {
                    TradeStatus::Open => "Open".to_string(),
                    TradeStatus::Closed => "Closed".to_string(),
                },
                trade.entry_time.format("%e-%b-%Y %k:%M").to_string(),
                trade.entry_price.to_string(),
                match trade.direction {
                    Direction::Buy => "Buy".to_string(),
                    Direction::Sell => "Sell".to_string(),
                },
                trade
                    .exit_time
                    .map(|t| t.format("%e-%b-%Y %k:%M").to_string())
                    .unwrap_or("-".to_string()),
                trade
                    .exit_price
                    .map(|p| p.to_string())
                    .unwrap_or("-".to_string()),
                trade.stop.to_string(),
                trade.price_diff.to_string(),
                trade.size.to_string(),
                trade.risk.to_string(),
                match trade.outcome {
                    TradeOutcome::Profit => format!(
                        "{}Profit{}",
                        color::Fg(color::Green),
                        color::Fg(color::Reset)
                    ),
                    TradeOutcome::Loss => {
                        format!("{}Loss{}", color::Fg(color::Red), color::Fg(color::Reset))
                    }
                },
                match trade.outcome {
                    TradeOutcome::Profit => format!(
                        "{}{}{}",
                        color::Fg(color::Green),
                        trade.profit,
                        color::Fg(color::Reset)
                    ),
                    TradeOutcome::Loss => format!(
                        "{}{}{}",
                        color::Fg(color::Red),
                        trade.profit,
                        color::Fg(color::Reset)
                    ),
                },
                match trade.outcome {
                    TradeOutcome::Profit => trade.risk_reward.round_dp(2).to_string(),
                    TradeOutcome::Loss => trade.risk_reward.round_dp(2).to_string(),
                },
                balance.to_string(),
            ]
            .into_iter()
            .map(|it| TableCell::new(it)),
        ));
    }

    println!("{}", table.render());
    Ok(())
}
