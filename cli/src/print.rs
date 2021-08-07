use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use term_table::{row::Row, table_cell::TableCell, Table, TableStyle};
use termion::{color, style};

use betty::price::{CurrencyAmount, Price};
use betty::trade::{Direction, Trade, TradeOutcome};

pub fn format_trade_log(
    trade_log: &Vec<Trade>,
    opening_balance: CurrencyAmount,
    latest_price: Price,
) -> String {
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
        .map(|it| TableCell::new(format!("{}{}{}", style::Bold, it, style::Reset))),
    ));

    let mut balance = opening_balance;

    for trade in trade_log {
        balance += trade.profit;

        table.add_row(Row::new(
            vec![
                trade.id.clone(),
                trade.status.to_string(),
                trade.entry_time.format("%e-%b-%Y %k:%M").to_string(),
                trade.entry_price.to_string(),
                trade.direction.to_string(),
                trade
                    .exit_time
                    .map(|t| t.format("%e-%b-%Y %k:%M").to_string())
                    .unwrap_or("-".to_string()),
                trade
                    .exit_price
                    .map(|p| p.to_string())
                    .unwrap_or("-".to_string()),
                format!(
                    "{}{}{}",
                    stop_colour(&trade, latest_price),
                    trade.stop,
                    color::Fg(color::Reset)
                ),
                trade.price_diff.to_string(),
                trade.size.to_string(),
                trade.risk.to_string(),
                format!(
                    "{}{}{}",
                    outcome_color(trade.outcome),
                    trade.outcome,
                    color::Fg(color::Reset)
                ),
                format!(
                    "{}{}{}",
                    outcome_color(trade.outcome),
                    trade.profit,
                    color::Fg(color::Reset)
                ),
                format!(
                    "{}{}{}",
                    risk_colour(trade.risk_reward),
                    trade.risk_reward.round_dp(2),
                    color::Fg(color::Reset)
                ),
                balance.to_string(),
            ]
            .into_iter()
            .map(|it| TableCell::new(it)),
        ));
    }

    format!("{}", table.render())
}

fn outcome_color(outcome: TradeOutcome) -> String {
    match outcome {
        TradeOutcome::Profit => format!("{}", color::Fg(color::Green)),
        TradeOutcome::Loss => format!("{}", color::Fg(color::Red)),
    }
}

fn stop_colour(trade: &Trade, latest_price: Price) -> String {
    match trade.direction {
        Direction::Buy if trade.stop >= trade.exit_price.unwrap_or(latest_price.bid) => {
            format!("{}", color::Fg(color::Red))
        }
        Direction::Sell if trade.stop <= trade.exit_price.unwrap_or(latest_price.ask) => {
            format!("{}", color::Fg(color::Red))
        }
        _ => String::new(),
    }
}

fn risk_colour(risk: Decimal) -> String {
    if risk < dec!(-0.5) {
        return format!("{}", color::Fg(color::Red));
    }

    if risk > dec!(1.0) {
        return format!("{}", color::Fg(color::Green));
    }

    String::new()
}
