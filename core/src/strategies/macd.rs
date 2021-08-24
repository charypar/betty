use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

use crate::core::maths::EMAIterator;
use crate::core::price::PriceHistory;
use crate::core::strategy::{Signal, TradingStrategy};
use crate::core::trade::Direction;
use crate::price::Frame;

// Moving Average Convergence/Divergence

const EMA_ERROR: Decimal = dec!(0.1);

pub struct MACD {
    pub short: usize,
    pub long: usize,
    pub signal: usize,
    pub entry_lim: Decimal, // enter above this value
    pub exit_lim: Decimal,  // exit below this value
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Neutral,
    Bullish,
    Bearish,
}

#[derive(Debug)]
struct Indicators {
    short_ema: Decimal,
    long_ema: Decimal,
    macd: Decimal,
    macd_signal: Decimal,
    macd_trend: Decimal,
}

pub struct MACDValue {
    pub short_ema: Decimal,
    pub long_ema: Decimal,
    pub macd: Decimal,
    pub macd_signal: Decimal,
    pub macd_trend: Decimal,
    pub sentiment: Sentiment,
    pub trade_signal: Option<Signal>,
}

impl MACD {
    pub fn macd(
        history: &[Frame],
        short: usize,
        long: usize,
        signal: usize,
        entry_lim: Decimal,
        exit_lim: Decimal,
    ) -> Vec<MACDValue> {
        let points = history.into_iter().map(|it| it.close.mid_price());

        let short_ema = points.clone().into_iter().ema(short);
        let long_ema = points.clone().into_iter().ema(long);

        let macd = short_ema
            .clone()
            .zip(long_ema.clone())
            .map(|(s, l)| s - l)
            .clone();
        let macd_sig = macd.clone().ema(signal);

        let macd_trend = macd.clone().zip(macd_sig.clone()).map(|(m, s)| m - s);

        let all = short_ema
            .zip(long_ema)
            .zip(macd)
            .zip(macd_sig)
            .zip(macd_trend)
            .map(|((((s, l), m), ms), mt)| Indicators {
                short_ema: s,
                long_ema: l,
                macd: m,
                macd_signal: ms,
                macd_trend: mt,
            });

        let length = [short, long, signal].iter().max().unwrap().clone();
        let needed = Self::samples_needed(length, EMA_ERROR) + 1;

        let mut output: Vec<MACDValue> = Vec::with_capacity(history.len());

        for (i, indicators) in all.enumerate() {
            let (sentiment, signal) = if let Some(last) = output.last() {
                let sentiment = Self::sentiment(last.sentiment, &indicators, entry_lim, exit_lim);

                if i < needed {
                    (sentiment, None) // No signal until we have enough data for the EMAs
                } else {
                    (sentiment, Self::signal(last.sentiment, sentiment))
                }
            } else {
                (Sentiment::Neutral, None)
            };

            let value = MACDValue {
                short_ema: indicators.short_ema,
                long_ema: indicators.long_ema,
                macd: indicators.macd,
                macd_signal: indicators.macd_signal,
                macd_trend: indicators.macd_trend,
                sentiment: sentiment,
                trade_signal: signal,
            };

            output.push(value);
        }

        output
    }

    fn sentiment(
        sentiment: Sentiment,
        iv: &Indicators,
        entry_lim: Decimal,
        exit_lim: Decimal,
    ) -> Sentiment {
        match sentiment {
            // TODO these rules need more work
            Sentiment::Bearish | Sentiment::Neutral if iv.macd > entry_lim => Sentiment::Bullish,
            Sentiment::Bullish | Sentiment::Neutral if iv.macd < -entry_lim => Sentiment::Bearish,
            Sentiment::Bullish if iv.macd <= exit_lim => Sentiment::Neutral,
            Sentiment::Bearish if iv.macd >= -exit_lim => Sentiment::Neutral,
            _ => sentiment,
        }
    }

    fn signal(last_sentiment: Sentiment, current_sentiment: Sentiment) -> Option<Signal> {
        if last_sentiment == current_sentiment {
            return None;
        }

        match (last_sentiment, current_sentiment) {
            (Sentiment::Bullish, Sentiment::Neutral) => Some(Signal::Exit(Direction::Buy)),
            (Sentiment::Bearish, Sentiment::Neutral) => Some(Signal::Exit(Direction::Sell)),
            (_, Sentiment::Bullish) => Some(Signal::Enter(Direction::Buy)),
            (_, Sentiment::Bearish) => Some(Signal::Enter(Direction::Sell)),
            _ => panic!(
                "Unexpected sentiment change {:?} -> {:?}",
                last_sentiment, current_sentiment
            ),
        }
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

        let price: Vec<Frame> = history
            .history
            .iter()
            .take(take + 1) // only need this much history for signal
            .rev()
            .cloned()
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
}
