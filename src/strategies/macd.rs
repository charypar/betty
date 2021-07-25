use rust_decimal::Decimal;

use crate::core::price::PriceHistory;
use crate::core::strategy::{Signal, TradingStrategy};

pub struct MACD {
    pub short_trend_length: usize,
    pub long_trend_length: usize,
    pub macd_signal_length: usize,
    pub entry_signal_diff_limit: Decimal,
    pub exit_signal_diff_limit: Decimal,
}

struct MACDValue {
    short_ema: Decimal,
    long_ema: Decimal,
    macd: Decimal,
    macd_signal: Decimal,
    macd_signal_diff: Decimal,
}

impl MACD {
    fn macd(values: &[Decimal], short: usize, long: usize, signal: usize) -> MACDValue {
        todo!()
    }

    fn ema(values: &[Decimal], length: usize) -> Decimal {
        todo!()
    }

    fn ema_samples_needed(lenght: usize) -> usize {
        todo!()
    }
}

impl TradingStrategy for MACD {
    fn signal(&self, history: &PriceHistory) -> Option<Signal> {
        todo!()
    }
}
