use crate::identity::{Canonical, Seal};
use crate::{Dimensionless, Measure};

/// An amount as a compensated pair: `hi` the rounded sum and `lo` what rounding left out, normalized so
/// `at_most` reads the order of the exact values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Compensated {
    hi: f64,
    lo: f64,
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let part = sum - a;
    (sum, (a - (sum - part)) + (b - part))
}

/// Needs `|a| >= |b|`.
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    (sum, b - (sum - a))
}

/// Past this, the splitter's own product overflows though `a` and the product asked for are finite.
const SPLIT_LIMIT: f64 = f64::from_bits(0x7E30_0000_0000_0000);
const SPLIT_SCALE: f64 = 268_435_456.0;

fn split(a: f64) -> (f64, f64) {
    let scale = if a.abs() > SPLIT_LIMIT { SPLIT_SCALE } else { 1.0 };
    let reduced = a / scale;
    let scaled = reduced * 134_217_729.0;
    let high = scaled - (scaled - reduced);
    (high * scale, (reduced - high) * scale)
}

fn two_product(a: f64, b: f64) -> (f64, f64) {
    let product = a * b;
    let (a_high, a_low) = split(a);
    let (b_high, b_low) = split(b);
    let error = ((a_high * b_high - product) + a_high * b_low + a_low * b_high) + a_low * b_low;
    (product, error)
}

impl Compensated {
    pub const ZERO: Self = Self { hi: 0.0, lo: 0.0 };

    pub fn of<U>(value: &Measure<U>) -> Self {
        Self::exact(value.value())
    }
    pub(crate) fn exact(value: f64) -> Self {
        Self { hi: value, lo: 0.0 }
    }
    /// Whether this is no more than `limit`, and false when either side is not finite. It is the one
    /// comparison: `<` and `>` read false on a NaN, so a check written with them passes what overflowed.
    pub fn at_most(self, limit: Self) -> bool {
        self.is_finite()
            && limit.is_finite()
            && (self.hi < limit.hi || (self.hi == limit.hi && self.lo <= limit.lo))
    }
    pub fn plus(self, other: Self) -> Self {
        let (sum, error) = two_sum(self.hi, other.hi);
        let (low, low_error) = two_sum(self.lo, other.lo);
        let (sum, error) = two_sum(sum, error + low);
        let (hi, lo) = two_sum(sum, error + low_error);
        Self { hi, lo }
    }
    pub fn minus(self, other: Self) -> Self {
        self.plus(Self {
            hi: -other.hi,
            lo: -other.lo,
        })
    }
    pub fn abs(self) -> Self {
        if self.hi < 0.0 || (self.hi == 0.0 && self.lo < 0.0) {
            Self::ZERO.minus(self)
        } else {
            self
        }
    }
    pub fn scaled(self, coefficient: i64) -> Self {
        self.product(coefficient as f64)
    }
    pub fn times(self, factor: &Measure<Dimensionless>) -> Self {
        self.product(factor.value())
    }
    fn product(self, factor: f64) -> Self {
        let (product, error) = two_product(self.hi, factor);
        let (hi, lo) = fast_two_sum(product, error + self.lo * factor);
        Self { hi, lo }
    }
    fn quotient(self, divisor: f64) -> Self {
        let first = self.hi / divisor;
        let (product, error) = two_product(first, divisor);
        let remainder = ((self.hi - product) - error) + self.lo;
        let (hi, lo) = fast_two_sum(first, remainder / divisor);
        Self { hi, lo }
    }
    /// `start + (end - start) * numerator / denominator`, the point an integer-ns fraction of a chord
    /// reaches. Exact at both ends.
    pub fn between(start: Self, end: Self, numerator: u64, denominator: u64) -> Self {
        if numerator == 0 {
            return start;
        }
        if numerator == denominator {
            return end;
        }
        let change = end.minus(start).product(numerator as f64);
        start.plus(change.quotient(denominator as f64))
    }
    pub fn bits(self) -> [u64; 2] {
        [self.hi.to_bits(), self.lo.to_bits()]
    }
    pub fn from_bits(bits: [u64; 2]) -> Self {
        let [hi, lo] = bits;
        Self {
            hi: f64::from_bits(hi),
            lo: f64::from_bits(lo),
        }
    }
    pub(crate) fn rounded(self) -> f64 {
        self.hi + self.lo
    }
    pub(crate) fn is_finite(self) -> bool {
        self.hi.is_finite() && self.lo.is_finite()
    }
}

impl Seal for Compensated {
    fn seal(&self, out: &mut Canonical) {
        let Self { hi, lo } = self;
        hi.seal(out);
        lo.seal(out);
    }
}

#[cfg(test)]
mod tests {
    use super::Compensated;
    use crate::{Dimensionless, Measure};

    const TWO_53: f64 = 9_007_199_254_740_992.0;

    fn exact(value: f64) -> Compensated {
        Compensated::exact(value)
    }

    #[test]
    fn a_sum_keeps_what_a_plain_add_rounds_away() {
        assert_eq!(TWO_53 + 1.0, TWO_53);
        let sum = exact(TWO_53).plus(exact(1.0));
        assert_eq!(sum.minus(exact(TWO_53)), exact(1.0));
        let tiny = exact(1.0).plus(exact(1e-30)).plus(exact(1e-30));
        assert_eq!(tiny.minus(exact(1.0)).bits()[0], 2e-30_f64.to_bits());
    }

    #[test]
    fn subtraction_cancels_exactly() {
        let value = exact(1e16).plus(exact(0.125));
        assert_eq!(value.minus(value).bits(), Compensated::ZERO.bits());
    }

    #[test]
    fn the_order_is_the_order_of_the_exact_values() {
        let above = exact(1.0).plus(exact(1e-30));
        let below = exact(1.0).minus(exact(1e-30));
        assert!(below.at_most(exact(1.0)) && exact(1.0).at_most(above));
        assert!(!exact(1.0).at_most(below) && !above.at_most(exact(1.0)));
        assert!(above.at_most(above));
    }

    #[test]
    fn no_comparison_passes_what_is_not_finite() {
        let nan = exact(f64::NAN);
        assert!(!nan.at_most(exact(1.0)) && !exact(1.0).at_most(nan) && !nan.at_most(nan));
        assert!(!exact(1.0).at_most(exact(f64::INFINITY)));
        let overflowed = exact(f64::MAX).scaled(1);
        assert!(!overflowed.is_finite() && !exact(1.0).at_most(overflowed));
    }

    #[test]
    fn abs_reads_the_sign_of_the_pair() {
        let negative = exact(0.0).minus(exact(1.0).plus(exact(1e-30)));
        assert_eq!(negative.abs(), exact(1.0).plus(exact(1e-30)));
        assert_eq!(exact(2.0).abs(), exact(2.0));
    }

    #[test]
    fn a_product_keeps_its_low_part() {
        let third = exact(1.0).quotient(3.0);
        let one = third.scaled(3);
        assert!(one.minus(exact(1.0)).abs().at_most(exact(1e-31)));
        let half = Measure::<Dimensionless>::new(0.5).map(|factor| third.times(&factor));
        assert_eq!(half.map(|value| value.bits()), Ok(third.quotient(2.0).bits()));
    }

    #[test]
    fn a_finite_product_of_a_huge_value_stays_exact() {
        let huge = exact(1e301);
        assert_eq!(huge.scaled(1), huge);
        assert_eq!(huge.scaled(-1), exact(-1e301));
        assert_eq!(exact(1e300).scaled(4), exact(4e300));
        let half = Measure::<Dimensionless>::new(0.5).map(|factor| huge.times(&factor));
        assert_eq!(half, Ok(exact(5e300)));
    }

    #[test]
    fn a_product_past_the_finite_range_says_so() {
        assert!(!exact(1e301).scaled(1_000_000_000).is_finite());
    }

    #[test]
    fn between_is_exact_at_both_ends_and_true_inside() {
        let (start, end) = (exact(1e6).plus(exact(1e-12)), exact(2e6).minus(exact(1e-12)));
        assert_eq!(Compensated::between(start, end, 0, 7), start);
        assert_eq!(Compensated::between(start, end, 7, 7), end);
        let (start, end) = (
            exact(1.0).quotient(3.0),
            exact(2e6).plus(exact(1.0).quotient(7.0)),
        );
        for denominator in 1..1000 {
            assert_eq!(Compensated::between(start, end, 0, denominator), start);
            assert_eq!(Compensated::between(start, end, denominator, denominator), end);
        }
        let inside = Compensated::between(exact(0.0), exact(3.0), 1, 3);
        assert_eq!(inside, exact(1.0));
        let tiny = Compensated::between(exact(1.0), exact(1.0).plus(exact(3e-30)), 2, 3);
        assert!(
            tiny.minus(exact(1.0))
                .minus(exact(2e-30))
                .abs()
                .at_most(exact(1e-45))
        );
    }

    #[test]
    fn stored_bits_read_back_as_the_same_pair() {
        let value = exact(1.0).plus(exact(1e-30));
        assert_eq!(Compensated::from_bits(value.bits()).bits(), value.bits());
    }

    #[test]
    fn a_non_finite_pair_says_so() {
        assert!(exact(1.0).is_finite());
        assert!(!exact(f64::INFINITY).is_finite());
        assert!(!exact(f64::MAX).plus(exact(f64::MAX)).is_finite());
    }
}
