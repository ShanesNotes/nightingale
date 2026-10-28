//! Proposed ng-validate: the synthetic composition root, the conserved-quantity certificate, the
//! accuracy gate and the arena measurement. The certificate reads one `ConservationView`; it has no
//! vital, observation, waveform or record parameter. It proves conservation and drift, not accuracy:
//! a core can close every ledger and still compute the wrong trajectory, which the accuracy gate
//! checks.
use crate::ng_contract::*;
use crate::ng_record::{ConservationView, FaultReport, HistorySeal, PatientRecord, Running, Session};

pub struct QuantityTolerance {
    _quantity: QuantityId,
    _absolute: f64,
    _relative: f64,
}
impl QuantityTolerance {
    pub fn new(_quantity: QuantityId, _absolute: f64, _relative: f64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Tolerance per conserved combination is data, fixed at admission against one basis.
pub struct CertificatePolicy {
    _basis: Digest,
    _tolerances: Vec<QuantityTolerance>,
    _expected: ExpectedDrift,
    _windows: Sliding24Hours,
}
impl CertificatePolicy {
    /// Refuses unless every combination of the basis has exactly one tolerance.
    pub fn new(
        _basis: &ConservedBasis,
        _tolerances: Vec<QuantityTolerance>,
        _expected: ExpectedDrift,
        _windows: Sliding24Hours,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub enum ExpectedDrift {
    /// Meals and drinks: a stable body returns to the same inventories.
    Zero,
    /// A later fasting protocol: a provenance-bearing curve, never a fitted baseline.
    Stated { per_quantity: Vec<DriftCurve>, evidence: Digest },
}
pub struct DriftCurve {
    _quantity: QuantityId,
    _coefficients: Vec<f64>,
}
/// Every sliding 24 h window over at least 48 h, from integral segments, never vital samples.
pub struct Sliding24Hours {
    _coverage_ns: u64,
}
impl Sliding24Hours {
    pub fn covering(_coverage_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct Certificate {
    _seal: HistorySeal,
    _basis: Digest,
    _windows: Vec<WindowResult>,
}
pub struct WindowResult {
    _from: SimTime,
    _to: SimTime,
    _closure: Vec<f64>,
    _drift: Vec<f64>,
    /// Gross inflow and outflow on each held route in this window, from the route's own curves
    /// (`HistorySegment::held_inflow`, `held_outflow`), in `CoverageManifest::held_routes` order.
    _held_support: Vec<(HeldRouteId, f64, f64)>,
}
pub enum CertificateCheck {
    /// inventory change minus net boundary exchange
    Closure,
    /// inventory change minus expected change
    Drift,
    /// segments do not cover the window, or the policy's basis is not the view's
    Coverage,
}
pub struct CertificateFailure {
    _check: CertificateCheck,
    _quantity: QuantityId,
    _window: Interval,
    _residual: f64,
}
/// Pure by construction of its inputs: one sealed history view and policy data. Window starts
/// include every accepted segment boundary and its 24 h shift; inside a segment the Bernstein
/// coefficients bound interior extrema of the closure and drift differences.
pub fn certify(_view: &ConservationView<'_>, _policy: &CertificatePolicy) -> Result<Certificate, CertificateFailure> {
    todo!()
}

/// The manufactured cases every core runs besides the slice. Each is synthetic and has a closed
/// form or a refined reference; none carries a physiological value.
pub enum AccuracyCase {
    /// Two compartments joined by one conductance, with an exterior inflow that steps at a
    /// breakpoint. The closed form is known, so a step that conserves but diverges fails here.
    TwoReservoir,
    /// An amount drained toward zero and refilled. The core keeps publishing until the admissible
    /// bound, refuses only there, and never refuses or faults on a validity range.
    DrainAndRefill,
    /// Flow through a route stopped and restored, with no pool emptied: laws stay defined at
    /// zero flow and accumulated state carries through.
    ZeroFlowRestore,
    /// A declared exterior removal route: every removed amount appears in its receipt, and no held
    /// stand-in replenishes the body outside its declared supplies.
    ControlledRemoval,
    /// The same stage evaluated before and after refused evaluations returns identical residuals
    /// and Jacobian bits.
    EvaluationIndependence,
    /// Windows of the slice against `ReferenceIntegrator` at three refinements (`factor` apart),
    /// which give the reference's own error and the core's observed order. The windows cover
    /// admission and the first posture change and meal, sleep onset, and the second day's waking.
    /// Never against the core itself: a lagged or biased core converges to its own wrong limit.
    SliceReference { factor: u32 },
}
/// The contract's slow reference: fully coupled backward Euler with Newton to convergence on the
/// compiled body, at a fixed small interval. It serves the accuracy gate and split measurement only
/// and is never scored.
pub struct ReferenceIntegrator {
    _interval_ns: u64,
}
impl ReferenceIntegrator {
    pub fn new(_interval_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Error bounds per case. The values are policy: model defaults set before the arena and reported
/// with every result.
pub struct AccuracyPolicy {
    _cases: Vec<AccuracyCase>,
    _reference: ReferenceIntegrator,
    _absolute: f64,
    _relative: f64,
}
impl AccuracyPolicy {
    pub fn new(
        _cases: Vec<AccuracyCase>,
        _reference: ReferenceIntegrator,
        _absolute: f64,
        _relative: f64,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct AccuracyEvidence {
    _errors: Vec<f64>,
    _observed_order: Vec<f64>,
}
pub struct AccuracyFailure {
    _case: AccuracyCase,
    _at: SimTime,
    _error: f64,
}
/// Runs each case on its own manufactured body (the slice case on `slice`) with the same core,
/// and compares accepted dense output, never trial states, with the closed form or the refined run.
pub fn accuracy(
    _slice: &CompiledBody,
    _solver: &dyn CoupledSolver,
    _initializer: &dyn Initializer,
    _policy: &AccuracyPolicy,
) -> Result<AccuracyEvidence, AccuracyFailure> {
    todo!()
}

/// External scheduling only. It cannot enter a compiled body, a ticket or a run identity.
pub enum Pace {
    Unbounded,
    WallClock,
    Chunked,
}
/// The synthetic composition root's anatomy for the CV, FL, RN, NS and receptor slice.
pub fn arena_schema() -> Schema {
    todo!()
}
/// Boundary routes: intake staging and absorption, urine exit, evaporative and heat exchange.
pub fn boundary_routes<'b>(_scope: BoundaryScope<'b>) -> Fragment<'b, Boundary> {
    todo!()
}
/// A held card with only its catalog part and receipt pools.
pub fn stand_in<'b, S: OutsideSlice>(_scope: HeldScope<'b, S>) -> Fragment<'b, S> {
    todo!()
}
/// The held endocrine card: its catalog part plus synthetic AVP, angiotensin II, aldosterone and
/// ANP surrogate secretion and clearance, with declared clock and osmotic gains. It acts only
/// through receptors and never sets an output; it does not masquerade as an EN crate.
pub fn endocrine_stand_in<'b>(_scope: HeldScope<'b, Endocrine>) -> Fragment<'b, Endocrine> {
    todo!()
}
/// The reference timeline from the 06:55 admission phase: posture at 07:00 and 23:00, meals and
/// drinks at 07:30, 12:30 and 18:30, sleep from 23:00 to 06:30, for every simulated day.
pub fn reference_days(_days: u32) -> BoundaryPort {
    todo!()
}
/// Only a synthetic fixture packet is accepted: the arena is nonclinical by type.
pub struct ArenaSpec {
    _duration_ns: u64,
    _packet: ParameterPacket<SyntheticFixture>,
    _boundary: BoundaryPort,
    _seed: Seed,
}
impl ArenaSpec {
    pub fn new(
        _duration_ns: u64,
        _packet: ParameterPacket<SyntheticFixture>,
        _boundary: BoundaryPort,
        _seed: Seed,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Digests of accepted state, ledger, events and observer findings under each replay gate. Every
/// gate compares against the same uninterrupted run:
/// - pace: unbounded, live wall clock and irregular chunks;
/// - resume: each checkpoint decoded into a freshly built core;
/// - late admission: each timeline input admitted mid-run through `Session::enqueue` instead of
///   at admission (the causal step rule);
/// - rewind: a branch rewound in the same process after the parent ran ahead, against the same
///   branch decoded into a fresh core;
/// - non-perturbation: physiology bits with observers attached, detached and changed.
pub struct ReplayEvidence {
    _run: Digest,
    _pace_digests: Vec<Digest>,
    _resume_digests: Vec<Digest>,
    _late_admission_digests: Vec<Digest>,
    _rewind_digests: Vec<Digest>,
    _observer_variant_digests: Vec<Digest>,
}
pub struct ThroughputEvidence {
    _elapsed_wall_ns: u64,
    _waveform_samples: u64,
}
/// What the run cost numerically, beside the certificate: refusals by reason and by refusal
/// source, faults at the minimum interval, and validity excursions.
pub struct NumericalHealth {
    _refusals: Vec<(NumericalRefusal, u64)>,
    _by_source: Vec<(RowId, u64)>,
    _floor_faults: u64,
    _excursions: u64,
}
/// Builds a fresh core with no state from earlier runs, for the resume and rewind gates.
pub trait CoreFactory {
    fn fresh(&self) -> Box<dyn CoupledSolver>;
}
/// The run's evidence, bound to the run identity it measured, so evidence from another run
/// cannot be scored with this one.
pub struct RunEvidence {
    _run: Digest,
    _throughput: ThroughputEvidence,
    _health: NumericalHealth,
}
pub struct ArenaRun<'m> {
    _session: Session<'m, Running>,
    _evidence: RunEvidence,
}
impl ArenaRun<'_> {
    pub fn record(&self) -> &PatientRecord {
        todo!()
    }
    pub fn into_evidence(self) -> RunEvidence {
        todo!()
    }
}
pub struct ArenaScore {
    _certificate: Certificate,
    _accuracy: AccuracyEvidence,
    _evidence: RunEvidence,
    _replay: ReplayEvidence,
}
pub enum ArenaFailure {
    Admission(AdmissionError),
    Initialization(InitializationFailure),
    /// A latched session, exported for diagnosis.
    Faulted(FaultReport),
    Certificate(CertificateFailure),
    Accuracy(AccuracyFailure),
    Replay { gate: ReplayGate, first_divergence: SimTime },
    /// A gate that did not run, or evidence from another run: never scored as a pass.
    MissingEvidence { gate: ReplayGate },
}
pub enum ReplayGate {
    Pace,
    Resume,
    LateAdmission,
    Rewind,
    NonPerturbation,
}
/// Compiles nothing: binds the fixture, initializes once, admits a session, runs the declared span
/// with full waveform under `pace`, and collects replay and wall-time evidence beside the record.
pub fn run<'m>(
    _body: &'m CompiledBody,
    _solver: &'m dyn CoupledSolver,
    _initializer: &dyn Initializer,
    _spec: ArenaSpec,
    _pace: Pace,
) -> Result<ArenaRun<'m>, ArenaFailure> {
    todo!()
}
/// A score needs all three: the certificate, the accuracy evidence and the run's evidence.
/// Runs every replay gate against the uninterrupted run, building each core it resumes or
/// rewinds into from `cores`.
pub fn replay_gates(
    _body: &CompiledBody,
    _cores: &dyn CoreFactory,
    _initializer: &dyn Initializer,
    _spec: &ArenaSpec,
) -> Result<ReplayEvidence, ArenaFailure> {
    todo!()
}
/// A score needs all four: the certificate, the accuracy evidence, the run's evidence and the
/// replay gates' evidence for the same run. Replay evidence from another run is `MissingEvidence`.
pub fn score(
    _certificate: Certificate,
    _accuracy: AccuracyEvidence,
    _evidence: RunEvidence,
    _replay: ReplayEvidence,
) -> Result<ArenaScore, ArenaFailure> {
    todo!()
}
