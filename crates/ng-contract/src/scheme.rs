use crate::chance::seed_word;
use crate::identity::{Canonical, Seal};
use crate::{AdmissionError, Digest, Dimensionless, Measure, ModelHash, PacketHash, StepTicket};
use core::marker::PhantomData;
use core::num::NonZeroU64;

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
    ForwardEuler = 0,
    Heun = 1,
    BackwardEulerDoubled = 2,
    BackwardEulerHalved = 3,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageSet {
    ForwardEuler,
    Heun,
    Whole,
    Doubled,
    Halved,
}
impl StageSet {
    pub(crate) fn stages(self, length_ns: u64) -> Vec<Stage> {
        let whole = length_ns;
        let (first, second) = (whole / 2, whole - whole / 2);
        let part = |span| Ratio::new(span, whole).expect("a ticket's length is not zero");
        match self {
            StageSet::ForwardEuler => vec![stage(0, Ratio::ONE, None, 0, 0)],
            StageSet::Heun => vec![stage(0, HALF, None, 0, 0), stage(whole, HALF, None, whole, 0)],
            StageSet::Whole => vec![stage(whole, Ratio::ONE, None, whole, 0)],
            StageSet::Doubled => vec![
                stage(whole, Ratio::ZERO, None, whole, 0),
                stage(first, part(first), None, first, 1),
                stage(whole, part(second), Some(1), second, 2),
            ],
            StageSet::Halved => vec![
                stage(first, part(first), None, first, 0),
                stage(whole, part(second), Some(0), second, 1),
            ],
        }
    }
    pub(crate) fn estimate(self) -> Option<Estimate> {
        match self {
            StageSet::ForwardEuler | StageSet::Heun | StageSet::Whole => None,
            StageSet::Doubled => Some(Estimate::Difference { fine: 2, coarse: 0 }),
            StageSet::Halved => Some(Estimate::SecondDifference { half: 0, end: 1 }),
        }
    }
}

/// On a smooth row `|end - 2 half + start|` is `h^2 y'' / 4`, the leading term of `|fine - coarse|`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Estimate {
    Difference { fine: usize, coarse: usize },
    SecondDifference { half: usize, end: usize },
}

/// How a core's stages make its increment: one of a closed set.
#[derive(Clone, Copy, Debug)]
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
    /// The doubled scheme's two halves alone, and the estimate is their second difference from the start. A
    /// ticket after a breakpoint shows the doubled scheme's stages, since a firing or a delivery's start or end
    /// can start a stiff transient the second difference over-reads. A length of 1 ns has one whole stage and no
    /// estimate.
    pub fn backward_euler_halved() -> Self {
        Self {
            method: Method::BackwardEulerHalved,
        }
    }
    pub(crate) fn set(self, ticket: &StepTicket) -> StageSet {
        let whole = ticket.length_ns() < 2;
        match self.method {
            Method::ForwardEuler => StageSet::ForwardEuler,
            Method::Heun => StageSet::Heun,
            Method::BackwardEulerDoubled | Method::BackwardEulerHalved if whole => StageSet::Whole,
            Method::BackwardEulerDoubled => StageSet::Doubled,
            Method::BackwardEulerHalved if ticket.after_breakpoint() => StageSet::Doubled,
            Method::BackwardEulerHalved => StageSet::Halved,
        }
    }
}

/// The absolute and relative scales a stage defect, an error estimate and an admission row are normalized by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorScales {
    absolute: f64,
    relative: f64,
}
impl ErrorScales {
    pub(crate) const EXACT: Self = Self {
        absolute: 0.0,
        relative: 0.0,
    };
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
    /// What these scales allow a row at `value`, in the row's own unit, given its natural size.
    pub fn at(self, value: f64, size: f64) -> f64 {
        self.absolute * size + self.relative * value.abs()
    }
}
impl Seal for ErrorScales {
    fn seal(&self, out: &mut Canonical) {
        let Self { absolute, relative } = self;
        absolute.seal(out);
        relative.seal(out);
    }
}

const ROUTINE_BITS_SHA256: &str = "d42e4b1ce0ae507032fbce98d4721abd6a41f9f8a7766f0ee5a19ce7625d4dcf";

/// The floating-point operations a run may use. Bitwise replay is promised within one profile.
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Debug)]
pub struct DeclaredResolution {
    policy: Digest,
    minimum_interval_ns: NonZeroU64,
    retry_budget: u32,
    arithmetic: ArithmeticProfile,
    scheme: StageScheme,
    scales: ErrorScales,
}
impl DeclaredResolution {
    /// `policy` hashes everything else a core is configured by, such as its interval, so two settings of
    /// one core never share an identity. `minimum_interval_ns` is the floor and `retry_budget` the refusals
    /// one step may spend. The floor is also this core's event rule: a guard's crossing is located to within
    /// it.
    pub fn new(
        policy: Digest,
        minimum_interval_ns: NonZeroU64,
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
        out.number(minimum_interval_ns.get());
        out.number(u64::from(*retry_budget));
        let ArithmeticProfile { digest } = arithmetic;
        digest.seal(&mut out);
        let StageScheme { method } = scheme;
        out.number(*method as u64);
        scales.seal(&mut out);
        out.digest()
    }
    pub fn policy(&self) -> Digest {
        self.policy
    }
    pub fn minimum_interval_ns(&self) -> NonZeroU64 {
        self.minimum_interval_ns
    }
    pub(crate) fn retry_budget(&self) -> u32 {
        self.retry_budget
    }
    pub(crate) fn scheme(&self) -> StageScheme {
        self.scheme
    }
    pub fn scales(&self) -> &ErrorScales {
        &self.scales
    }
}

mod domains {
    pub trait Sealed {
        /// It enters the seed's word, so equal words in two domains never alias.
        const NAME: &'static str;
    }
}
pub trait SeedDomain: domains::Sealed {}
macro_rules! seed_domains {
    ($($domain:ident = $name:literal),* $(,)?) => {$(
        #[derive(Clone, Copy)]
        pub enum $domain {}
        impl domains::Sealed for $domain {
            const NAME: &'static str = $name;
        }
        impl SeedDomain for $domain {}
    )*};
}
seed_domains!(
    PacketDomain = "packet",
    RunDomain = "run",
    ObservationDomain = "observation"
);

#[derive(Clone, Copy)]
pub struct DomainSeed<D> {
    words: [u64; 4],
    word: u64,
    domain: PhantomData<D>,
}
impl<D: SeedDomain> DomainSeed<D> {
    pub fn new(words: [u64; 4]) -> Self {
        Self {
            words,
            word: seed_word(D::NAME, words),
            domain: PhantomData,
        }
    }
}
impl<D: Copy> DomainSeed<D> {
    pub(crate) fn word(self) -> u64 {
        self.word
    }
}
/// Which patient is drawn from a population.
pub type PacketSeed = DomainSeed<PacketDomain>;
/// The seed in a run's identity: how the run varies.
pub type Seed = DomainSeed<RunDomain>;
/// How observations err.
pub type ObservationSeed = DomainSeed<ObservationDomain>;

/// The body model's hash, the packet's hash, the run seed and the declared resolution. Nothing else.
#[derive(Clone)]
pub struct RunIdentity {
    parts: IdentityParts,
    digest: Digest,
}
#[derive(Clone)]
struct IdentityParts {
    model: ModelHash,
    packet: PacketHash,
    seed: Seed,
    resolution: DeclaredResolution,
}
impl IdentityParts {
    fn digest(&self) -> Digest {
        let Self {
            model,
            packet,
            seed,
            resolution,
        } = self;
        let mut out = Canonical::new("ng-contract run identity 1");
        model.digest().seal(&mut out);
        packet.digest().seal(&mut out);
        let Seed {
            words,
            word: _,
            domain: _,
        } = seed;
        for word in words {
            out.number(*word);
        }
        resolution.digest().seal(&mut out);
        out.digest()
    }
}
impl RunIdentity {
    pub fn new(model: ModelHash, packet: PacketHash, seed: Seed, resolution: DeclaredResolution) -> Self {
        let parts = IdentityParts {
            model,
            packet,
            seed,
            resolution,
        };
        Self {
            digest: parts.digest(),
            parts,
        }
    }
    pub fn digest(&self) -> Digest {
        self.digest
    }
    pub fn model(&self) -> ModelHash {
        self.parts.model
    }
    pub fn packet(&self) -> PacketHash {
        self.parts.packet
    }
    pub fn resolution(&self) -> &DeclaredResolution {
        &self.parts.resolution
    }
    pub(crate) fn seed(&self) -> Seed {
        self.parts.seed
    }
    pub(crate) fn same_seed(&self, other: &Self) -> bool {
        self.parts.seed.words == other.parts.seed.words
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArithmeticProfile, DeclaredResolution, Estimate, ROUTINE_BITS_SHA256, Ratio, StageScheme, StageSet,
    };
    use crate::boundary::DriveFrame;
    use crate::pinned::{exp, ln};
    use crate::{AdmissionError, Digest, Dimensionless, ErrorScales, Interval, Measure, SimTime, StepTicket};
    use core::num::NonZeroU64;

    fn ticket(length_ns: u64, after_breakpoint: bool) -> StepTicket {
        let start = SimTime::from_ns(0);
        StepTicket {
            base: Digest::of_canonical(b"a base"),
            interval: Interval::new(start, SimTime::from_ns(length_ns)).expect("a positive length"),
            span_ns: length_ns,
            due: Vec::new(),
            frame: DriveFrame::default(),
            after_breakpoint,
            attempt: 0,
            fence: start,
            floor_ns: NonZeroU64::MIN,
            budget: 0,
            digest: Digest::of_canonical(b"a ticket"),
        }
    }

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
        let stages = StageSet::Doubled.stages(7);
        let offsets: Vec<u64> = stages.iter().map(|stage| stage.offset_ns).collect();
        assert_eq!(offsets, [7, 3, 7]);
        let spans: Vec<u64> = stages.iter().map(|stage| stage.span_ns).collect();
        assert_eq!(spans, [7, 3, 4]);
        let sum = stages[1].weight.plus(stages[2].weight);
        assert_eq!(sum, Some(Ratio::ONE));
        assert_eq!(stages[0].weight, Ratio::ZERO);
    }

    #[test]
    fn the_halved_set_is_the_doubled_sets_two_halves() -> Result<(), AdmissionError> {
        let (halved, doubled) = (StageSet::Halved.stages(7), StageSet::Doubled.stages(7));
        let shape = |stages: &[super::Stage]| -> Vec<(u64, Ratio, u64)> {
            (stages.iter())
                .map(|stage| (stage.offset_ns, stage.weight, stage.span_ns))
                .collect()
        };
        assert_eq!(shape(&halved), shape(&doubled[1..]));
        assert_eq!(
            (halved[0].weight, halved[1].weight),
            (Ratio::new(3, 7)?, Ratio::new(4, 7)?)
        );
        let chain: Vec<(Option<usize>, usize)> =
            halved.iter().map(|stage| (stage.from, stage.slope)).collect();
        assert_eq!(chain, [(None, 0), (Some(0), 1)]);
        Ok(())
    }

    #[test]
    fn one_nanosecond_has_one_whole_stage_and_no_estimate() {
        for scheme in [
            StageScheme::backward_euler_doubled(),
            StageScheme::backward_euler_halved(),
        ] {
            for after_breakpoint in [false, true] {
                let set = scheme.set(&ticket(1, after_breakpoint));
                assert_eq!(set.stages(1).len(), 1);
                assert_eq!(set.estimate(), None);
            }
        }
        let doubled = Some(Estimate::Difference { fine: 2, coarse: 0 });
        assert_eq!(StageSet::Doubled.estimate(), doubled);
        let halved = Some(Estimate::SecondDifference { half: 0, end: 1 });
        assert_eq!(StageSet::Halved.estimate(), halved);
    }

    #[test]
    fn a_tickets_stage_set_follows_its_scheme_breakpoint_and_length() {
        use StageSet::{Doubled, ForwardEuler, Halved, Heun, Whole};
        let schemes = [
            StageScheme::forward_euler(),
            StageScheme::heun(),
            StageScheme::backward_euler_doubled(),
            StageScheme::backward_euler_halved(),
        ];
        for (length, after_breakpoint, sets) in [
            (1, false, [ForwardEuler, Heun, Whole, Whole]),
            (1, true, [ForwardEuler, Heun, Whole, Whole]),
            (2, false, [ForwardEuler, Heun, Doubled, Halved]),
            (2, true, [ForwardEuler, Heun, Doubled, Doubled]),
            (7, false, [ForwardEuler, Heun, Doubled, Halved]),
            (7, true, [ForwardEuler, Heun, Doubled, Doubled]),
        ] {
            let ticket = ticket(length, after_breakpoint);
            let seen = schemes.map(|scheme| scheme.set(&ticket));
            assert_eq!(seen, sets, "{length} ns, after a breakpoint: {after_breakpoint}");
        }
    }

    #[test]
    fn each_scheme_seals_its_own_number_and_the_first_three_stay() -> Result<(), AdmissionError> {
        let measure = |value| Measure::<Dimensionless>::new(value);
        let scales = ErrorScales::uniform(measure(1.0)?, measure(0.0)?)?;
        let sealed = [
            StageScheme::forward_euler(),
            StageScheme::heun(),
            StageScheme::backward_euler_doubled(),
            StageScheme::backward_euler_halved(),
        ]
        .map(|scheme| {
            let (policy, profile) = (
                Digest::of_canonical(b"each scheme"),
                ArithmeticProfile::contract_v1(),
            );
            let resolution = DeclaredResolution::new(policy, NonZeroU64::MIN, 1, profile, scheme, scales);
            (resolution.digest().as_bytes().iter())
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        });
        let before_the_halved_scheme = [
            "cb728da5231a9400e5c16913ddaef980376887c4bf4b73a91479760f12124a6f",
            "a4ef8307d42e51fc289f9e34b8565716ae4de15ad97ddf88831f35dacda3c552",
            "dfa677b154b92b992aca82c98e6efc1bbfe94122e35a7125c9cac4c15c844486",
        ];
        assert_eq!(sealed[..3], before_the_halved_scheme);
        assert!(!sealed[..3].contains(&sealed[3]), "{sealed:?}");
        Ok(())
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
