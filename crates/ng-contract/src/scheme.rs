use crate::identity::{Canonical, Seal};
use crate::{AdmissionError, Digest, Dimensionless, Measure, ModelHash, PacketHash};

const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

fn greatest_common_divisor(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// An exact nonnegative fraction in lowest terms.
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
        let common = greatest_common_divisor(u128::from(numerator), u128::from(denominator)) as u64;
        Ok(Self {
            numerator: numerator / common,
            denominator: denominator / common,
        })
    }
    fn reduced(numerator: u128, denominator: u128) -> Option<Self> {
        let common = greatest_common_divisor(numerator, denominator);
        Some(Self {
            numerator: u64::try_from(numerator / common).ok()?,
            denominator: u64::try_from(denominator / common).ok()?,
        })
    }
    pub(crate) fn plus(self, other: Self) -> Option<Self> {
        let (a, b) = (u128::from(self.numerator), u128::from(self.denominator));
        let (c, d) = (u128::from(other.numerator), u128::from(other.denominator));
        Self::reduced(
            a.checked_mul(d)?.checked_add(c.checked_mul(b)?)?,
            b.checked_mul(d)?,
        )
    }
    pub(crate) fn seconds_of(self, length_ns: u64) -> f64 {
        let numerator = u128::from(self.numerator) * u128::from(length_ns);
        let denominator = u128::from(self.denominator) * NANOSECONDS_PER_SECOND;
        numerator as f64 / denominator as f64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    ForwardEuler,
    Heun,
    BackwardEulerDoubled,
}

/// Its value is `from` (the accepted state when `None`) plus `span_ns` times the tangent of stage `slope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stage {
    pub(crate) offset_ns: u64,
    pub(crate) weight: Ratio,
    pub(crate) from: Option<usize>,
    pub(crate) span_ns: u64,
    pub(crate) slope: usize,
}

const HALF: Ratio = Ratio {
    numerator: 1,
    denominator: 2,
};

fn stage(offset_ns: u64, weight: Ratio, from: Option<usize>, span_ns: u64, slope: usize) -> Stage {
    Stage {
        offset_ns,
        weight,
        from,
        span_ns,
        slope,
    }
}

/// How a core's stages make its increment: one of a closed set.
#[derive(Clone, Copy)]
pub struct StageScheme {
    method: Method,
}
impl StageScheme {
    /// One stage at the interval's start. It reads no estimate.
    pub fn forward_euler() -> Self {
        Self {
            method: Method::ForwardEuler,
        }
    }
    /// The start and the Euler-predicted end, at weight one half each. It reads no estimate.
    pub fn heun() -> Self {
        Self { method: Method::Heun }
    }
    /// The whole interval at weight zero, then its first `floor(h / 2)` and the rest, which carry the
    /// increment. The estimate is the difference of the two ends. A length of 1 ns has one whole stage and
    /// no estimate.
    pub fn backward_euler_doubled() -> Self {
        Self {
            method: Method::BackwardEulerDoubled,
        }
    }
    pub(crate) fn stages(self, length_ns: u64) -> Vec<Stage> {
        let whole = length_ns;
        match self.method {
            Method::ForwardEuler => vec![stage(0, Ratio::ONE, None, 0, 0)],
            Method::Heun => vec![stage(0, HALF, None, 0, 0), stage(whole, HALF, None, whole, 0)],
            Method::BackwardEulerDoubled if whole < 2 => {
                vec![stage(whole, Ratio::ONE, None, whole, 0)]
            }
            Method::BackwardEulerDoubled => {
                let (first, second) = (whole / 2, whole - whole / 2);
                let part = |span| Ratio::new(span, whole).expect("a ticket's length is not zero");
                vec![
                    stage(whole, Ratio::ZERO, None, whole, 0),
                    stage(first, part(first), None, first, 1),
                    stage(whole, part(second), Some(1), second, 2),
                ]
            }
        }
    }
    pub(crate) fn estimate(self, length_ns: u64) -> Option<(usize, usize)> {
        (self.method == Method::BackwardEulerDoubled && length_ns >= 2).then_some((2, 0))
    }
}

/// The absolute and relative scales a stage defect and an error estimate are normalized by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorScales {
    absolute: f64,
    relative: f64,
}
impl ErrorScales {
    /// Refuses an absolute scale that is not positive and a relative scale below zero.
    pub fn uniform(
        absolute: Measure<Dimensionless>,
        relative: Measure<Dimensionless>,
    ) -> Result<Self, AdmissionError> {
        if absolute.value() <= 0.0 || relative.value() < 0.0 {
            return Err(AdmissionError::NonPositiveScale);
        }
        Ok(Self {
            absolute: absolute.value(),
            relative: relative.value(),
        })
    }
    pub(crate) fn at(self, value: f64) -> f64 {
        self.absolute + self.relative * value.abs()
    }
}

const ROUTINE_BITS_SHA256: &str = "d42e4b1ce0ae507032fbce98d4721abd6a41f9f8a7766f0ee5a19ce7625d4dcf";

/// The floating-point operations a run may use. Bitwise replay is promised within one profile.
#[derive(Clone, Copy)]
pub struct ArithmeticProfile {
    digest: Digest,
}
impl ArithmeticProfile {
    pub fn contract_v1() -> Self {
        Self {
            digest: Digest::of_canonical(ROUTINE_BITS_SHA256.as_bytes()),
        }
    }
}

/// A solver core's declared settings; part of the run identity. Pace is not in it.
pub struct DeclaredResolution {
    policy: Digest,
    minimum_interval_ns: u64,
    retry_budget: u32,
    arithmetic: ArithmeticProfile,
    scheme: StageScheme,
    scales: ErrorScales,
}
impl DeclaredResolution {
    /// `policy` hashes everything else a core is configured by, such as its interval, so two settings of
    /// one core never share an identity. `minimum_interval_ns` is the floor and `retry_budget` the refusals
    /// one step may spend. `Session::admit` refuses a zero floor.
    pub fn new(
        policy: Digest,
        minimum_interval_ns: u64,
        retry_budget: u32,
        arithmetic: ArithmeticProfile,
        scheme: StageScheme,
        scales: ErrorScales,
    ) -> Self {
        Self {
            policy,
            minimum_interval_ns,
            retry_budget,
            arithmetic,
            scheme,
            scales,
        }
    }
    pub fn digest(&self) -> Digest {
        let Self {
            policy,
            minimum_interval_ns,
            retry_budget,
            arithmetic,
            scheme,
            scales,
        } = self;
        let mut out = Canonical::new("ng-contract resolution 1");
        policy.seal(&mut out);
        out.number(*minimum_interval_ns);
        out.number(u64::from(*retry_budget));
        let ArithmeticProfile { digest } = arithmetic;
        digest.seal(&mut out);
        let StageScheme { method } = scheme;
        out.number(*method as u64);
        let ErrorScales { absolute, relative } = scales;
        absolute.seal(&mut out);
        relative.seal(&mut out);
        out.digest()
    }
    pub fn policy(&self) -> Digest {
        self.policy
    }
    pub fn minimum_interval_ns(&self) -> u64 {
        self.minimum_interval_ns
    }
    pub(crate) fn retry_budget(&self) -> u32 {
        self.retry_budget
    }
    pub(crate) fn scheme(&self) -> StageScheme {
        self.scheme
    }
    pub(crate) fn scales(&self) -> ErrorScales {
        self.scales
    }
}

#[derive(Clone, Copy)]
pub struct Seed {
    words: [u64; 4],
}
impl Seed {
    pub fn new(words: [u64; 4]) -> Self {
        Self { words }
    }
}

/// The body model's hash, the packet's hash, the run seed and the declared resolution. Nothing else.
pub struct RunIdentity {
    model: ModelHash,
    packet: PacketHash,
    seed: Seed,
    resolution: DeclaredResolution,
}
impl RunIdentity {
    pub fn new(model: ModelHash, packet: PacketHash, seed: Seed, resolution: DeclaredResolution) -> Self {
        Self {
            model,
            packet,
            seed,
            resolution,
        }
    }
    pub fn digest(&self) -> Digest {
        let Self {
            model,
            packet,
            seed,
            resolution,
        } = self;
        let mut out = Canonical::new("ng-contract run identity 1");
        model.digest().seal(&mut out);
        packet.digest().seal(&mut out);
        let Seed { words } = seed;
        for word in words {
            out.number(*word);
        }
        resolution.digest().seal(&mut out);
        out.digest()
    }
    pub fn model(&self) -> ModelHash {
        self.model
    }
    pub fn packet(&self) -> PacketHash {
        self.packet
    }
    pub fn resolution(&self) -> &DeclaredResolution {
        &self.resolution
    }
}

#[cfg(test)]
mod tests {
    use super::{ROUTINE_BITS_SHA256, Ratio, StageScheme};
    use crate::pinned::{exp, ln};
    use crate::{AdmissionError, Digest, Dimensionless, ErrorScales, Measure};

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
    fn the_routines_bits_are_the_profiles() {
        let mut bits = Vec::new();
        for step in -5_962..=5_678 {
            bits.extend_from_slice(&exp(f64::from(step) / 8.0).to_bits().to_be_bytes());
        }
        for exponent in 1..2_047_u64 {
            let x = f64::from_bits((exponent << 52) | 0x0005_5555_5555_5555);
            let value = ln(x).unwrap_or(f64::NAN);
            bits.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        let digest = Digest::of_canonical(&bits);
        let hex: String = digest
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex, ROUTINE_BITS_SHA256,
            "a changed bit is a new ArithmeticProfile"
        );
    }

    #[test]
    fn a_scale_must_be_positive() {
        let measure = |value| Measure::<Dimensionless>::new(value);
        let scales = |absolute, relative| -> Result<ErrorScales, AdmissionError> {
            ErrorScales::uniform(measure(absolute)?, measure(relative)?)
        };
        assert_eq!(scales(0.0, 1.0), Err(AdmissionError::NonPositiveScale));
        assert_eq!(scales(-1.0, 1.0), Err(AdmissionError::NonPositiveScale));
        assert_eq!(scales(1.0, -1.0), Err(AdmissionError::NonPositiveScale));
        assert!(scales(1.0, 0.0).is_ok());
    }

    #[test]
    fn backward_euler_doubled_halves_an_odd_interval_without_losing_a_nanosecond() {
        let stages = StageScheme::backward_euler_doubled().stages(7);
        let offsets: Vec<u64> = stages.iter().map(|stage| stage.offset_ns).collect();
        assert_eq!(offsets, [7, 3, 7]);
        let spans: Vec<u64> = stages.iter().map(|stage| stage.span_ns).collect();
        assert_eq!(spans, [7, 3, 4]);
        let sum = stages[1].weight.plus(stages[2].weight);
        assert_eq!(sum, Some(Ratio::ONE));
        assert_eq!(stages[0].weight, Ratio::ZERO);
    }

    #[test]
    fn one_nanosecond_has_one_whole_stage_and_no_estimate() {
        let scheme = StageScheme::backward_euler_doubled();
        assert_eq!(scheme.stages(1).len(), 1);
        assert_eq!(scheme.estimate(1), None);
        assert_eq!(scheme.estimate(2), Some((2, 0)));
    }

    #[test]
    fn seconds_are_exact_where_the_integers_are() -> Result<(), AdmissionError> {
        assert_eq!(Ratio::ONE.seconds_of(2_000_000_000).to_bits(), 2.0_f64.to_bits());
        assert_eq!(
            Ratio::new(1, 4)?.seconds_of(2_000_000_000).to_bits(),
            0.5_f64.to_bits()
        );
        assert_eq!(Ratio::ZERO.seconds_of(1).to_bits(), 0.0_f64.to_bits());
        Ok(())
    }
}
