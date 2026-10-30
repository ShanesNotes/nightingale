//! Identity and time. Identity carries schema and version, never process addresses or `Debug`
//! text. All physical and event boundaries are integer nanoseconds.
use super::{AdmissionError, Dimensionless, Measure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Digest {
    _bytes: [u8; 32],
}
impl Digest {
    /// SHA-256 over canonical bytes.
    pub fn of_canonical(_bytes: &[u8]) -> Self {
        todo!()
    }
}
pub struct ModelHash {
    _digest: Digest,
}
pub struct PacketHash {
    _digest: Digest,
}
/// A seed of one domain. The three kinds of chance never share a stream: which patient (the
/// packet seed, used when the packet is built), how the run varies (the run seed: law streams,
/// forcings and hazards) and how observations err (the observation seed, which no law reads). The
/// domain is part of every stream's key derivation, so equal words in two domains never alias.
pub struct DomainSeed<D> {
    _words: [u64; 4],
    _domain: core::marker::PhantomData<D>,
}
impl<D> DomainSeed<D> {
    pub fn new(_words: [u64; 4]) -> Self {
        todo!()
    }
}
pub enum PacketDomain {}
pub enum RunDomain {}
pub enum ObservationDomain {}
pub type PacketSeed = DomainSeed<PacketDomain>;
/// The seed in a run's identity. Only the run domain.
pub type Seed = DomainSeed<RunDomain>;
pub type ObservationSeed = DomainSeed<ObservationDomain>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimTime {
    _ns: u64,
}
impl SimTime {
    pub fn from_ns(_ns: u64) -> Self {
        todo!()
    }
    pub fn nanoseconds(&self) -> u64 {
        todo!()
    }
    pub fn plus_ns(self, _ns: u64) -> Option<SimTime> {
        todo!()
    }
}
/// `[start, end]`, with `start` before `end`.
#[derive(Clone, Copy)]
pub struct Interval {
    _start: SimTime,
    _end: SimTime,
}
impl Interval {
    /// `None` unless `start` is before `end`.
    pub fn new(_start: SimTime, _end: SimTime) -> Option<Self> {
        todo!()
    }
    pub fn start(self) -> SimTime {
        todo!()
    }
    pub fn end(self) -> SimTime {
        todo!()
    }
    pub fn length_ns(self) -> u64 {
        todo!()
    }
}

/// A declared scheduling constraint from wave to hours; never a hardcoded organ lane.
pub struct RateClass {
    _maximum_ns: u64,
    _band: RateBand,
}
impl RateClass {
    pub fn new(_band: RateBand, _maximum_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
    pub fn band(&self) -> RateBand {
        todo!()
    }
    pub fn maximum_ns(&self) -> u64 {
        todo!()
    }
}
#[derive(Clone, Copy)]
pub enum RateBand {
    Wave,
    Beat,
    Reflex,
    Minutes,
    Hours,
}

/// Algorithm, version, arithmetic profile, tolerances, event rules, partition policy, the minimum
/// interval and the retry budget. Pace is not in it; a reduced-detail run declares a different
/// resolution.
pub struct DeclaredResolution {
    _policy: Digest,
    _minimum_interval_ns: u64,
    _retry_budget: u32,
    _arithmetic: ArithmeticProfile,
    _scheme: StageScheme,
    _scales: ErrorScales,
}
impl DeclaredResolution {
    /// `policy` hashes everything else a core is configured by, such as its interval, so two
    /// settings of one core never share an identity. `minimum_interval_ns` is the floor: a refusal
    /// whose retry would go below it, or past `retry_budget`, is the fault `NoAdmissibleStep`, so a
    /// core cannot accept excess error there. A breakpoint may still end an interval shorter than
    /// the floor. Session admission refuses a zero floor. The value is policy, chosen per core
    /// before the arena.
    pub fn new(
        _policy: Digest,
        _minimum_interval_ns: u64,
        _retry_budget: u32,
        _arithmetic: ArithmeticProfile,
        _scheme: StageScheme,
        _scales: ErrorScales,
    ) -> Self {
        todo!()
    }
    pub fn policy(&self) -> Digest {
        todo!()
    }
    pub fn minimum_interval_ns(&self) -> u64 {
        todo!()
    }
}
/// An exact nonnegative fraction in lowest terms. Stage weights are ratios, so summing them to one
/// is an integer equality with no float tolerance. Negative weights are outside the seam, because
/// gross forward and reverse extents could not then be kept apart.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    _numerator: u64,
    _denominator: u64,
}
impl Ratio {
    pub const ZERO: Self = Self {
        _numerator: 0,
        _denominator: 1,
    };
    pub const ONE: Self = Self {
        _numerator: 1,
        _denominator: 1,
    };
    /// Refuses a zero denominator.
    pub fn new(_numerator: u64, _denominator: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// How a core's stages make its increment: one of a closed set over one private table, so no caller
/// depends on a general tableau that only one case would test, and a later general form changes
/// no caller. Validation re-evaluates the witnessed stages, checks the stage equations and
/// recomputes the error estimate from them.
#[derive(Clone, Copy)]
pub struct StageScheme {
    _method: u32,
}
impl StageScheme {
    /// One stage at the interval's start. It reads no estimate.
    pub fn forward_euler() -> Self {
        todo!()
    }
    /// The start and the Euler-predicted end, at weight one half each. It reads no estimate.
    pub fn heun() -> Self {
        todo!()
    }
    /// The whole interval at weight zero, then its first `floor(h / 2)` and the rest, which carry
    /// the increment. The estimate is the difference of the two ends (solver-A's step doubling). A
    /// length of 1 ns has one whole stage and no estimate.
    pub fn backward_euler_doubled() -> Self {
        todo!()
    }
    /// Backward-Euler substeps inside the interval, each with a predictor-corrector error estimate
    /// (solver-B's bins).
    pub fn backward_euler_substeps() -> Self {
        todo!()
    }
}
/// The absolute and relative scales a stage defect and an error estimate are normalized by.
#[derive(Clone, Copy)]
pub struct ErrorScales {
    _absolute: f64,
    _relative: f64,
}
impl ErrorScales {
    /// Refuses an absolute scale that is not positive and a relative scale below zero.
    pub fn uniform(
        _absolute: Measure<Dimensionless>,
        _relative: Measure<Dimensionless>,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The arithmetic a run is bitwise reproducible under: correctly rounded IEEE 754 binary64 add,
/// subtract, multiply, divide and sqrt, no host libm and no `mul_add` (both banned by the
/// repository's clippy.toml), and the contract's own versioned transcendental routines. Replay is promised bitwise within one
/// profile at any pace and chunking and from any checkpoint;
/// across profiles or other targets it is not promised.
#[derive(Clone, Copy)]
pub struct ArithmeticProfile {
    _digest: Digest,
}
impl ArithmeticProfile {
    pub fn contract_v1() -> Self {
        todo!()
    }
}
/// Model hash + parameter packet hash + seed + declared resolution. Nothing else.
pub struct RunIdentity {
    _model: ModelHash,
    _parameters: PacketHash,
    _seed: Seed,
    _resolution: DeclaredResolution,
}
impl RunIdentity {
    pub fn new(
        _model: ModelHash,
        _parameters: PacketHash,
        _seed: Seed,
        _resolution: DeclaredResolution,
    ) -> Self {
        todo!()
    }
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
