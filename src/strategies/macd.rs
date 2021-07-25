use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

use crate::core::price::PriceHistory;
use crate::core::strategy::{Signal, TradingStrategy};

const EMA_ERROR: Decimal = dec!(0.1);
pub struct MACD {
    pub short_trend_length: usize,
    pub long_trend_length: usize,
    pub macd_signal_length: usize,
    pub entry_signal_diff_limit: Decimal,
    pub exit_signal_diff_limit: Decimal,
}

#[derive(Clone)]
pub struct EMA<I, T> {
    iter: I,
    prev: Option<T>,
    alpha: Decimal,
}

impl<I, T> EMA<I, T> {
    pub fn new(iter: I, length: usize) -> Self {
        Self {
            iter,
            prev: None,
            alpha: dec!(2.0) / Decimal::from(length + 1),
        }
    }
}

impl<I, T> Iterator for EMA<I, T>
where
    I: Iterator<Item = T>,
    T: std::ops::Mul<Decimal, Output = Decimal> + Copy,
{
    type Item = Decimal;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.prev, self.iter.next()) {
            (Some(prev), Some(current)) => {
                self.prev = Some(current);

                Some(current * self.alpha + prev * (dec!(1.0) - self.alpha))
            }
            (None, Some(current)) => {
                self.prev = Some(current);

                // multiply to avoid T != Decimal type error.
                // I'm sure there's a way to constrain T to not need this
                Some(current * dec!(1.0))
            }
            _ => None,
        }
    }
}

pub trait EMAIterator<T>: Iterator<Item = T> + Sized {
    fn ema(self, length: usize) -> EMA<Self, T> {
        EMA::new(self, length)
    }
}

impl<T, I: Iterator<Item = T>> EMAIterator<T> for I {}

struct MACDValue {
    short_ema: Decimal,
    long_ema: Decimal,
    macd: Decimal,
    macd_signal: Decimal,
    macd_signal_diff: Decimal,
}

impl MACD {
    fn macd(values: &[Decimal], short: usize, long: usize, signal: usize) -> Vec<MACDValue> {
        let mut short = values.into_iter().ema(short);
        let mut long = values.into_iter().ema(long);

        let mut macd = short.clone().zip(long.clone()).map(|(s, l)| s - l).clone();
        let mut macd_sig = macd.clone().ema(signal);

        let mut macd_sig_diff = macd.clone().zip(macd_sig.clone()).map(|(m, s)| m - s);

        // All the clones above are only copying the iterator structs so that we can
        // iterate each of the five streams independently below

        let mut output = Vec::with_capacity(values.len());
        loop {
            match (
                short.next(),
                long.next(),
                macd.next(),
                macd_sig.next(),
                macd_sig_diff.next(),
            ) {
                (Some(s), Some(l), Some(m), Some(ms), Some(msd)) => output.push(MACDValue {
                    short_ema: s,
                    long_ema: l,
                    macd: m,
                    macd_signal: ms,
                    macd_signal_diff: msd,
                }),
                _ => break,
            }
        }

        output
    }

    pub fn samples_needed(length: usize, error: Decimal) -> usize {
        let alpha = dec!(2.0) / Decimal::from(length + 1);
        println!("{} {} {}", error.ln(), alpha, error.ln() / -alpha);
        (error.ln() / -alpha).round().to_isize().unwrap() as usize
    }
}

impl TradingStrategy for MACD {
    fn signal(&self, history: &PriceHistory) -> Option<Signal> {
        todo!()
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
    fn empty_value_ema() {
        let actual: Vec<_> = vec![].iter().ema(40).collect();
        let expected = vec![];

        assert_eq!(actual, expected);
    }

    #[test]
    fn single_value_ema() {
        let actual: Vec<_> = vec![dec!(5.0)].into_iter().ema(40).collect();
        let expected = vec![dec!(5.0)];

        assert_eq!(actual, expected);
    }

    #[test]
    fn ema_of_a_constant() {
        let values = vec![dec!(3.0); 50];
        let actual: Vec<_> = values.iter().ema(40).collect();
        let expected = values.clone();

        assert_eq!(actual, expected);
    }

    #[test]
    fn ema_of_a_step_change() {
        let values = [vec![dec!(0.0); 3], vec![dec!(5.0); 87]].concat();

        let actual_short: Vec<_> = values.iter().ema(20).collect();
        let actual_long: Vec<_> = values.iter().ema(40).collect();

        // ema converges to 5.0
        assert!(dec!(5.0) - actual_short.last().unwrap() < dec!(0.001));
        // short converges to 5.0 faster
        assert!(actual_short.iter().zip(&actual_long).all(|(s, l)| s >= l));
    }
}
