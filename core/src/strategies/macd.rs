use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

use crate::core::maths::EMAIterator;
use crate::core::price::{Points, PriceHistory};
use crate::core::strategy::{Signal, TradingStrategy};
use crate::core::trade::Direction;

// Moving Average Convergence/Divergence

const EMA_ERROR: Decimal = dec!(0.1);

pub struct MACD {
    pub short: usize,
    pub long: usize,
    pub signal: usize,
    pub entry_lim: Decimal, // enter above this value
    pub exit_lim: Decimal,  // exit below this value
}

struct MACDValue {
    pub short_ema: Decimal,
    pub long_ema: Decimal,
    pub macd: Decimal,
    pub macd_signal: Decimal,
    pub macd_trend: Decimal,
    pub trade_signal: Option<Signal>,
}

impl MACD {
    fn macd(
        values: &[Decimal],
        short: usize,
        long: usize,
        signal: usize,
        entry_lim: Decimal,
        exit_lim: Decimal,
    ) -> Vec<MACDValue> {
        let mut short = values.into_iter().ema(short);
        let mut long = values.into_iter().ema(long);

        let mut macd = short.clone().zip(long.clone()).map(|(s, l)| s - l).clone();
        let mut macd_sig = macd.clone().ema(signal);

        let mut macd_trend = macd.clone().zip(macd_sig.clone()).map(|(m, s)| m - s);

        // All the clones above are only copying the iterator structs so that we can
        // iterate each of the five streams independently below

        let mut prev: Option<&MACDValue> = None;
        let mut output = Vec::with_capacity(values.len());

        loop {
            match (
                short.next(),
                long.next(),
                macd.next(),
                macd_sig.next(),
                macd_trend.next(),
            ) {
                (Some(s), Some(l), Some(m), Some(ms), Some(msd)) => {
                    let signal = if let Some(pr) = prev {
                        Self::signal(pr.macd, pr.macd_trend, ms, msd, entry_lim, exit_lim)
                    } else {
                        None
                    };

                    let value = MACDValue {
                        short_ema: s,
                        long_ema: l,
                        macd: m,
                        macd_signal: ms,
                        macd_trend: msd,
                        trade_signal: signal,
                    };

                    output.push(value);
                    prev = output.last();
                }
                _ => break,
            }
        }

        output
    }

    fn signal(
        prev_macd: Decimal,
        prev_macd_trend: Decimal,
        last_macd: Decimal,
        last_macd_trend: Decimal,
        entry_lim: Decimal,
        exit_lim: Decimal,
    ) -> Option<Signal> {
        // estimate next MACD value from the current MACD trend
        let est_macd_prev = (prev_macd + prev_macd_trend).abs();
        let est_macd_last = (last_macd + last_macd_trend).abs();

        // enter when estimated forward MACD crosses outside entry limit
        let enter = est_macd_prev <= entry_lim && est_macd_last > entry_lim;
        // exit when estimated forward MACD crosses inside exit limit
        let exit = est_macd_prev > exit_lim && est_macd_last <= exit_lim;

        let long = last_macd >= prev_macd; // moving averages diverging
        let short = last_macd <= prev_macd; // moving averages converging

        if enter && long {
            return Some(Signal::Enter(Direction::Buy));
        }

        if enter && short {
            return Some(Signal::Enter(Direction::Sell));
        }

        if exit && !long {
            return Some(Signal::Exit(Direction::Buy));
        }

        if exit && !short {
            return Some(Signal::Exit(Direction::Sell));
        }

        None
    }

    pub fn samples_needed(length: usize, error: Decimal) -> usize {
        let alpha = dec!(2.0) / Decimal::from(length + 1);
        (error.ln() / -alpha).round().to_isize().unwrap() as usize
    }
}

impl TradingStrategy for MACD {
    fn signal(&self, history: &PriceHistory) -> Option<Signal> {
        let length = [self.short, self.long, self.signal]
            .iter()
            .max()
            .unwrap()
            .clone();
        let take = Self::samples_needed(length, EMA_ERROR) + 1; // need at least 2 valid samples

        if take > history.history.len() {
            // not enough history to make safe judgement
            return None;
        }

        let price: Vec<Points> = history
            .history
            .iter()
            .take(take) // only need this much history for signal
            .rev()
            .map(|it| it.close.mid_price())
            .collect();

        let macd = Self::macd(
            &price,
            self.short,
            self.long,
            self.signal,
            self.entry_lim,
            self.exit_lim,
        );

        macd.last().unwrap().trade_signal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_samples_needed() {
        let actual = MACD::samples_needed(40, dec!(0.1));
        let expected = 47;

        assert_eq!(actual, expected);
    }

    #[test]
    fn produces_correct_signals() {
        let buy = Some(Signal::Enter(Direction::Buy));
        let sell = Some(Signal::Enter(Direction::Sell));
        let exit_buy = Some(Signal::Exit(Direction::Buy));
        let exit_sell = Some(Signal::Exit(Direction::Sell));

        #[rustfmt::skip]
        let cases = [
            (dec!(0), dec!(0), dec!(0), dec!(0), dec!(30), dec!(30), None),
            (dec!(40), dec!(0), dec!(45), dec!(0), dec!(30), dec!(30), None),
            (dec!(-40), dec!(0), dec!(-45), dec!(0), dec!(30), dec!(30), None),
            (dec!(10), dec!(0), dec!(15), dec!(0), dec!(30), dec!(30), None),
            (dec!(-10), dec!(0), dec!(-15), dec!(0), dec!(30), dec!(30), None),

            (dec!(-10), dec!(35), dec!(-5), dec!(40), dec!(30), dec!(30), buy),
            (dec!(10), dec!(10), dec!(21), dec!(10), dec!(30), dec!(30), buy),
            (dec!(40), dec!(-5), dec!(35), dec!(-10), dec!(30), dec!(30), exit_buy),
            (dec!(-20), dec!(-5), dec!(-25), dec!(-10), dec!(50), dec!(30), None),

            (dec!(10), dec!(-35), dec!(5), dec!(-40), dec!(30), dec!(30), sell),
            (dec!(-10), dec!(-10), dec!(-21), dec!(-10), dec!(30), dec!(30), sell),
            (dec!(-40), dec!(5), dec!(-35), dec!(10), dec!(30), dec!(30), exit_sell),
            (dec!(20), dec!(5), dec!(25), dec!(10), dec!(50), dec!(30), None),
        ];

        for case in cases {
            let (p_macd, p_macd_trend, l_macd, l_macd_trend, el, exl, expected) = case;
            let actual = MACD::signal(p_macd, p_macd_trend, l_macd, l_macd_trend, el, exl);

            assert_eq!(
                actual, expected,
                "{} {} -> {} {} expected {:?} got {:?}",
                p_macd, p_macd_trend, l_macd, l_macd_trend, expected, actual
            );
        }
    }
}
