//! Identity and time. Identity carries schema and version, never process addresses or `Debug`
//! text. All physical and event boundaries are integer nanoseconds.
use super::AdmissionError;

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
/// Chained seal of one accepted revision: previous seal plus the digest of the published image.
pub struct Seal {
    _digest: Digest,
}

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
pub struct Interval {
    _start: SimTime,
    _end: SimTime,
}
impl Interval {
    pub fn start(&self) -> SimTime {
        todo!()
    }
    pub fn end(&self) -> SimTime {
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

/// Algorithm, version, arithmetic profile, tolerances, event rules, partition policy and the
/// minimum interval. Pace is not in it; a reduced-detail run declares a different resolution.
pub struct DeclaredResolution {
    _policy: Digest,
    _minimum_interval_ns: u64,
    _arithmetic: ArithmeticProfile,
    _scheme: StageScheme,
    _scales: ErrorScales,
}
impl DeclaredResolution {
    /// `minimum_interval_ns` is the floor: a refusal whose retry would go below it is the fault
    /// `NoAdmissibleStep`, and validation refuses an estimated error above tolerance at the floor
    /// as well, so a core cannot accept excess error there. A breakpoint may still end an interval
    /// shorter than the floor. Session admission refuses a zero floor. The value is policy, chosen
    /// per core before the arena. The scheme and scales are readable, not only a digest, because
    /// `PatientRecord::validate` rechecks the proposal against them.
    pub fn new(
        _policy: Digest,
        _minimum_interval_ns: u64,
        _arithmetic: ArithmeticProfile,
        _scheme: StageScheme,
        _scales: ErrorScales,
    ) -> Self {
        todo!()
    }
    pub fn minimum_interval_ns(&self) -> u64 {
        todo!()
    }
    pub fn scheme(&self) -> &StageScheme {
        todo!()
    }
}
/// How a core's stages make its increment: stage coefficients, nonnegative weights per column, and
/// the embedded error estimate. A step-doubled scheme or backward-Euler substeps (a beat core's
/// bins) are schemes too. Validation re-evaluates the witnessed stages, checks the stage equations
/// and recomputes the error estimate from them.
pub struct StageScheme {
    _coefficients: Vec<f64>,
    _weights: Vec<f64>,
    _nodes: Vec<f64>,
    _error_weights: Vec<f64>,
    _substeps: u32,
}
impl StageScheme {
    /// One implicit Runge-Kutta tableau (backward Euler is `[1], [1], [1]` with no embedded pair).
    pub fn tableau(_coefficients: Vec<f64>, _weights: Vec<f64>, _nodes: Vec<f64>, _error_weights: Vec<f64>)
        -> Result<Self, AdmissionError> {
        todo!()
    }
    /// The same tableau once over the interval and twice over its halves; the difference is the
    /// error estimate (solver-A's step doubling).
    pub fn step_doubled(_base: StageScheme) -> Self {
        todo!()
    }
    /// Backward-Euler substeps inside the interval, each with a predictor-corrector error estimate
    /// (solver-B's bins).
    pub fn backward_euler_substeps() -> Self {
        todo!()
    }
}
/// Per-unknown absolute and relative scales for local error and residual norms.
pub struct ErrorScales {
    _absolute: Vec<f64>,
    _relative: Vec<f64>,
}
impl ErrorScales {
    pub fn uniform(_absolute: f64, _relative: f64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The arithmetic a run is bitwise reproducible under: correctly rounded IEEE 754 binary64 add,
/// subtract, multiply, divide and sqrt, no host libm and no `mul_add` (both banned by the
/// repository's clippy.toml), and the contract's own versioned transcendental routines. Replay is promised bitwise within one
/// profile at any pace and chunking, from any checkpoint, and between native and WASM where both
/// run; across profiles or other targets it is not promised.
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
}
