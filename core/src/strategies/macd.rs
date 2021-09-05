use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;

use crate::core::maths::EMAIterator;
use crate::core::price::PriceHistory;
use crate::core::strategy::TradingStrategy;
use crate::price::Frame;
use crate::strategy::Trend;

// Moving Average Convergence/Divergence

const EMA_ERROR: Decimal = dec!(0.1);

pub struct MACD {
    pub short: usize,
    pub long: usize,
    pub signal: usize,
    pub entry_lim: Decimal, // enter above this value
    pub exit_lim: Decimal,  // exit below this value
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
    pub trend: Trend,
}

impl MACD {
    pub fn macd(&self, history: &[Frame]) -> Vec<MACDValue> {
        let points = history.into_iter().map(|it| it.close.mid_price());

        let short_ema = points.clone().into_iter().ema(self.short);
        let long_ema = points.clone().into_iter().ema(self.long);
        let macd = short_ema
            .clone()
            .zip(long_ema.clone())
            .map(|(s, l)| s - l)
            .clone();
        let macd_sig = macd.clone().ema(self.signal);
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

        let mut output: Vec<MACDValue> = Vec::with_capacity(history.len());

        for indicators in all {
            let trend = if let Some(last) = output.last() {
                // Note we're not worried about having enough history in here,
                // this is the raw indicators, the TradingStrategy implementation
                // further down is used for actual decision making
                Self::trend(last.trend, &indicators, self.entry_lim, self.exit_lim)
            } else {
                Trend::Neutral
            };

            let value = MACDValue {
                short_ema: indicators.short_ema,
                long_ema: indicators.long_ema,
                macd: indicators.macd,
                macd_signal: indicators.macd_signal,
                macd_trend: indicators.macd_trend,
                trend,
            };

            output.push(value);
        }

        output
    }

    fn trend(trend: Trend, iv: &Indicators, entry_lim: Decimal, exit_lim: Decimal) -> Trend {
        match trend {
            // TODO these rules need more work
            Trend::Bearish | Trend::Neutral if iv.macd > entry_lim => Trend::Bullish,
            Trend::Bullish | Trend::Neutral if iv.macd < -entry_lim => Trend::Bearish,
            Trend::Bullish if iv.macd <= exit_lim => Trend::Neutral,
            Trend::Bearish if iv.macd >= -exit_lim => Trend::Neutral,
            _ => trend,
        }
    }

    pub fn samples_needed(length: usize, error: Decimal) -> usize {
        let alpha = dec!(2.0) / Decimal::from(length + 1);
        (error.ln() / -alpha).round().to_isize().unwrap() as usize
    }
}

impl TradingStrategy for MACD {
    fn trend(&self, history: &PriceHistory) -> Trend {
        let length = [self.short, self.long, self.signal]
            .iter()
            .max()
            .unwrap()
            .clone();
        let take = Self::samples_needed(length, EMA_ERROR) + 1; // need at least 2 valid samples

        if take > history.history.len() {
            // not enough history to make safe judgement
            return Trend::Neutral;
        }

        let price: Vec<Frame> = history
            .history
            .iter()
            .take(take + 1) // only need this much history for signal
            .rev()
            .cloned()
            .collect();

        let macd = self.macd(&price);

        macd.last().unwrap().trend
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
