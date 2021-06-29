use super::price::{CurrencyAmount, Points};
use super::trade::Entry;

// Market holds information about a particular market and the trading rules that apply
pub struct Market {
    pub code: String,
    pub margin_factor: u64,            // 1:X
    pub min_deal_size: CurrencyAmount, // per point
    pub min_stop_distance: Points,
}

pub enum MarketError {
    DealTooSmall,        // size below min_deal_size
    StopTooClose,        // stop-loss is not far enough
    InsufficientBalance, // would result in margin call
}

impl Market {
    pub fn validate_entry(order: Entry, balance: CurrencyAmount) -> Result<(), MarketError> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn todo() {}
}
