use crate::AdmissionError;

const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

fn greatest_common_divisor(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// An exact nonnegative fraction in lowest terms. Stage weights are ratios, so summing them to one is an
/// integer equality with no float tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    numerator: u64,
    denominator: u64,
}
impl Ratio {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// Refuses a zero denominator.
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, AdmissionError> {
        if denominator == 0 {
            return Err(AdmissionError::ZeroDenominator);
        }
        Self::reduced(u128::from(numerator), u128::from(denominator)).ok_or(AdmissionError::ZeroDenominator)
    }
    fn reduced(numerator: u128, denominator: u128) -> Option<Self> {
        let common = greatest_common_divisor(numerator, denominator);
        Some(Self {
            numerator: u64::try_from(numerator / common).ok()?,
            denominator: u64::try_from(denominator / common).ok()?,
        })
    }
    /// The exact sum, or `None` when it does not fit in lowest terms.
    pub(crate) fn plus(self, other: Self) -> Option<Self> {
        let (a, b) = (u128::from(self.numerator), u128::from(self.denominator));
        let (c, d) = (u128::from(other.numerator), u128::from(other.denominator));
        Self::reduced(
            a.checked_mul(d)?.checked_add(c.checked_mul(b)?)?,
            b.checked_mul(d)?,
        )
    }
    /// This fraction of an interval, in seconds, rounded once.
    pub(crate) fn seconds_of(self, length_ns: u64) -> f64 {
        let numerator = u128::from(self.numerator) * u128::from(length_ns);
        let denominator = u128::from(self.denominator) * NANOSECONDS_PER_SECOND;
        numerator as f64 / denominator as f64
    }
}

#[cfg(test)]
mod tests {
    use super::Ratio;
    use crate::AdmissionError;

    #[test]
    fn a_ratio_is_kept_in_lowest_terms() {
        assert_eq!(Ratio::new(2, 4), Ratio::new(1, 2));
        assert_eq!(Ratio::new(0, 7), Ok(Ratio::ZERO));
        assert_eq!(Ratio::new(7, 7), Ok(Ratio::ONE));
    }

    #[test]
    fn a_zero_denominator_is_refused() {
        assert_eq!(Ratio::new(1, 0), Err(AdmissionError::ZeroDenominator));
    }

    #[test]
    fn weights_of_an_odd_interval_sum_to_one_exactly() -> Result<(), AdmissionError> {
        let length = 1_000_000_007;
        let half = length / 2;
        let sum = Ratio::new(half, length)?.plus(Ratio::new(length - half, length)?);
        assert_eq!(sum, Some(Ratio::ONE));
        Ok(())
    }

    #[test]
    fn a_sum_too_large_for_lowest_terms_is_none() -> Result<(), AdmissionError> {
        let (first, second) = (Ratio::new(1, u64::MAX)?, Ratio::new(1, u64::MAX - 1)?);
        assert_eq!(first.plus(second), None);
        Ok(())
    }

    #[test]
    fn seconds_round_once_from_exact_integers() -> Result<(), AdmissionError> {
        assert_eq!(Ratio::ONE.seconds_of(2_000_000_000).to_bits(), 2.0_f64.to_bits());
        assert_eq!(
            Ratio::new(1, 4)?.seconds_of(2_000_000_000).to_bits(),
            0.5_f64.to_bits()
        );
        assert_eq!(Ratio::ZERO.seconds_of(1).to_bits(), 0.0_f64.to_bits());
        Ok(())
    }
}
