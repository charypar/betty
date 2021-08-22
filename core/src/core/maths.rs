use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Clone, Debug)]
pub struct EMA<I> {
    iter: I,
    prev: Option<Decimal>,
    alpha: Decimal,
}

impl<I> EMA<I> {
    pub fn new(iter: I, length: usize) -> Self {
        Self {
            iter,
            prev: None,
            alpha: dec!(2.0) / Decimal::from(length + 1),
        }
    }
}

impl<I, T> Iterator for EMA<I>
where
    I: Iterator<Item = T>,
    T: std::ops::Mul<Decimal, Output = Decimal> + Copy,
{
    type Item = Decimal;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.prev, self.iter.next()) {
            (Some(prev), Some(current)) => {
                let v = Some(current * self.alpha + prev * (dec!(1.0) - self.alpha));
                self.prev = v;

                v
            }
            (None, Some(current)) => {
                // multiply to avoid T != Decimal type error.
                // I'm sure there's a way to constrain T to not need this
                let v = Some(current * dec!(1.0));
                self.prev = v;

                v
            }
            _ => None,
        }
    }
}

pub trait EMAIterator<T>: Iterator<Item = T> + Sized {
    fn ema(self, length: usize) -> EMA<Self> {
        EMA::new(self, length)
    }
}

impl<T, I: Iterator<Item = T>> EMAIterator<T> for I {}

#[cfg(test)]
mod tests {
    use super::*;

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
