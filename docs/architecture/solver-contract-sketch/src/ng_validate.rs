//! Proposed ng-validate: the synthetic composition root, the conserved-quantity certificate, the
//! accuracy gate and the arena measurement. The certificate reads one `ConservationView`; it has no
//! vital, observation, waveform or record parameter. It proves conservation and drift, not accuracy:
//! a core can close every ledger and still compute the wrong trajectory, which the accuracy gate
//! checks.
use crate::ng_contract::*;
use crate::ng_record::{ConservationView, FaultReport, HistorySeal, PatientRecord, RecordDigests, Running, Session};

/// One combination's limits. `absolute + relative * gross exchange in the window` bounds closure, so
/// a tiny exchange needs a tiny residual. `drift` is 1.96 times the combination's within-subject
/// variation from the healthy-range set, or the closure floor for charge (record 0004); a fixture
/// states a synthetic limit in its place.
pub struct QuantityPolicy {
    _quantity: QuantityId,
    _absolute: Compensated,
    _relative: Measure<Dimensionless>,
    _drift: Compensated,
}
impl QuantityPolicy {
    /// Refuses a dimension that is not what the combination balances.
    pub fn new<U>(
        _quantity: QuantityId,
        _absolute: Measure<U>,
        _relative: Measure<Dimensionless>,
        _drift: Measure<U>,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Limits per conserved combination are data, fixed at admission against one basis.
pub struct CertificatePolicy {
    _basis: Digest,
    _digest: Digest,
    _quantities: Vec<QuantityPolicy>,
    _expected: ExpectedDrift,
    _windows: Sliding24Hours,
}
impl CertificatePolicy {
    /// Refuses unless every combination of the basis has exactly one entry, in basis order.
    pub fn new(
        _basis: &ConservedBasis,
        _quantities: Vec<QuantityPolicy>,
        _expected: ExpectedDrift,
        _windows: Sliding24Hours,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
    pub fn digest(&self) -> Digest {
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
    /// Refuses a coverage that does not exceed one window, which could slide nowhere.
    pub fn covering(_coverage_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct Certificate {
    _seal: HistorySeal,
    _basis: Digest,
    _policy: Digest,
    _results: Vec<QuantityResult>,
    _pieces: u64,
    /// Gross inflow and outflow on each held route per window, from the route's own curves
    /// (`HistorySegment::held_inflow`, `held_outflow`), in `CoverageManifest::held_routes` order.
    _held_support: Vec<(HeldRouteId, f64, f64)>,
}
impl Certificate {
    pub fn seal(&self) -> HistorySeal {
        todo!()
    }
    pub fn basis(&self) -> Digest {
        todo!()
    }
    pub fn policy(&self) -> Digest {
        todo!()
    }
    pub fn results(&self) -> &[QuantityResult] {
        todo!()
    }
    /// How many stretches of window starts the walk covered, so a run that checked nothing shows zero.
    pub fn pieces(&self) -> u64 {
        todo!()
    }
}
/// The worst window of one combination, named by instant. Every window was checked; keeping each
/// would add rows and no fact.
pub struct QuantityResult {
    _quantity: QuantityId,
    _worst_closure: Compensated,
    _closure_window: Interval,
    _worst_drift: Compensated,
    _drift_window: Interval,
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
    _quantity: Option<QuantityId>,
    _window: Option<Interval>,
    _residual: Compensated,
}
/// Pure by construction of its inputs: one sealed history view and policy data. Window starts
/// include every accepted segment boundary and its 24 h shift; inside a segment the Bernstein
/// coefficients bound interior extrema of the closure and drift differences. Closure failures win
/// over drift, so a `Drift` failure says the ledger closed everywhere and the body still moved.
pub fn certify(_view: &ConservationView<'_>, _policy: &CertificatePolicy) -> Result<Certificate, CertificateFailure> {
    todo!()
}

/// Two reservoirs joined by one conductance, with an exterior inflow that steps at breakpoints,
/// declared through the public API with obviously synthetic numbers. Its closed form is known.
pub struct TwoReservoirs {
    _body: CompiledBody,
}
impl TwoReservoirs {
    pub fn stepped_inflow() -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The manufactured cases every core runs besides the slice. Each is synthetic and has a closed
/// form or a refined reference; none carries a physiological value.
pub enum AccuracyCase {
    /// A step that conserves but diverges fails here.
    TwoReservoir(TwoReservoirs),
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
    SliceReference { reference: ReferenceIntegrator, factor: u32 },
}
/// The contract's slow reference: fully coupled backward Euler, step doubled, with Newton to
/// convergence on the compiled body, on the public seam and nothing else. It serves the accuracy
/// gate and split measurement only and is never scored.
pub struct ReferenceIntegrator {
    _interval_ns: u64,
    _minimum_interval_ns: u64,
    _scales: ErrorScales,
}
impl ReferenceIntegrator {
    pub fn new(_interval_ns: u64) -> Self {
        todo!()
    }
    /// Tighter or looser scales, which make the estimate refuse sooner or later.
    pub fn with_scales(self, _scales: ErrorScales) -> Self {
        todo!()
    }
    pub fn with_floor(self, _minimum_interval_ns: u64) -> Self {
        todo!()
    }
}
impl CoupledSolver for ReferenceIntegrator {
    fn resolution(&self) -> DeclaredResolution {
        todo!()
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        todo!()
    }
    fn solve(&self, _problem: &CoupledProblem<'_>, _ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        todo!()
    }
}
/// Error bounds per case. The values are policy: model defaults set before the arena and reported
/// with every result. `absolute + relative * exact` bounds the error at each compared instant, and
/// `grid_ns` spaces the instants compared inside a segment.
pub struct AccuracyPolicy {
    _cases: Vec<AccuracyCase>,
    _absolute: Compensated,
    _relative: Measure<Dimensionless>,
    _grid_ns: u64,
    _digest: Digest,
}
impl AccuracyPolicy {
    /// Refuses a zero grid and no case.
    pub fn new(
        _cases: Vec<AccuracyCase>,
        _absolute: Measure<Water>,
        _relative: Measure<Dimensionless>,
        _grid_ns: u64,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
/// The worst error per case, under one policy. Accepted values only: a trial state is never read.
pub struct AccuracyEvidence {
    _policy: Digest,
    _worst: Vec<Compensated>,
}
impl AccuracyEvidence {
    pub fn policy(&self) -> Digest {
        todo!()
    }
    pub fn worst(&self) -> &[Compensated] {
        todo!()
    }
}
pub enum AccuracyFailure {
    /// The first compared instant above the policy, in time order.
    Exceeded { case: usize, at: SimTime, error: Compensated },
    /// The core could not finish the case. A must-fail core that reaches this has not failed on
    /// accuracy.
    Faulted(EngineFault),
    Admission(InitializationFailure),
    /// Adjacent refinements' errors are outside the band.
    Order { case: usize, refinement: usize },
    /// Adjacent refinements ran under different policies, so they compare no one case.
    MixedEvidence { refinement: usize },
}
/// Runs each case with the same core. After every accepted step it compares the accepted dense
/// segment, never a trial state, with the closed form or the refined run: amounts, exchanges and
/// the gross inflow, at each grid instant and each segment's end.
pub fn accuracy(_solver: &dyn CoupledSolver, _policy: &AccuracyPolicy) -> Result<AccuracyEvidence, AccuracyFailure> {
    todo!()
}
/// The ratio test for a core's observed order: for a refinement by two and order `p`, each error
/// over the next must lie in `[low, high]` around `2^p`. A ratio states the order with no logarithm.
pub fn observed_order(
    _ladder: [&AccuracyEvidence; 3],
    _low: Measure<Dimensionless>,
    _high: Measure<Dimensionless>,
) -> Result<(), AccuracyFailure> {
    todo!()
}

/// External scheduling only. It cannot enter a compiled body, a ticket or a run identity.
pub enum Pace {
    Unbounded,
    WallClock,
    Chunked,
}
#[derive(Clone, Copy)]
pub enum ClockSource {
    /// A list of readings, which proves pace cannot reach a run and is no live clock.
    Scripted,
    Host,
}
/// No crate here reads a clock, so a host outside the workspace passes one in. A host's clock may wait
/// for its next tick before it answers.
pub trait WallClock {
    /// Wall ns since the run began, or `None` when this clock has no more to say.
    fn elapsed_ns(&self) -> Option<u64>;
    fn source(&self) -> ClockSource;
}
/// The slice's map, grouped: the regions the anchors and stage 1 checks name, and nothing finer.
pub fn arena_anatomy() -> Result<Anatomy, AdmissionError> {
    todo!()
}
/// Boundary routes: intake staging and absorption, urine exit, evaporative and heat exchange.
pub fn boundary_routes<'b>(_scope: BoundaryScope<'b>) -> Fragment<'b, Boundary> {
    todo!()
}
/// Holds all nine cards outside the slice. EN gains its synthetic AVP, angiotensin II, aldosterone
/// and ANP surrogate secretion and clearance with the ligands, with declared clock and osmotic
/// gains. It acts only through receptors and never sets an output.
pub fn hold_outside_slice(_body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
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
/// Digests of the reference run first, then each run the gate compared.
pub struct GateEvidence {
    _gate: ReplayGate,
    _run: Digest,
    _runs: Vec<RecordDigests>,
    _clock: Option<ClockSource>,
    _observed: Vec<Digest>,
}
/// Digests of accepted state, ledger, events and observations under each replay gate. Every
/// gate compares against the same uninterrupted run:
/// - pace: unbounded, live wall clock and irregular chunks;
/// - resume: each checkpoint loaded into a freshly built core;
/// - late admission: each timeline input admitted through `Session::enqueue` at the last head a run
///   can still take it, instead of at admission (the causal step rule);
/// - rewind: a branch rewound in the same process after the parent ran ahead, against the same
///   branch loaded into a fresh core;
/// - non-perturbation: physiology bits with observers attached, detached and changed, each observer
///   reading a value.
pub struct ReplayEvidence {
    _run: Digest,
    _pace: GateEvidence,
    _resume: GateEvidence,
    _late_admission: GateEvidence,
    _rewind: GateEvidence,
    _non_perturbation: GateEvidence,
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
/// Builds a fresh core with no state from earlier runs. A gate calls it once per run, its reference
/// run first, so a core that must differ between two runs can differ only through what its factory
/// gives it.
pub trait CoreFactory {
    fn fresh(&self) -> Box<dyn CoupledSolver + '_>;
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
impl<'m> ArenaRun<'m> {
    pub fn record(&self) -> &PatientRecord<'m> {
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
    /// An input with no head after admission to arrive at, or no step left to apply it: the gate
    /// would test nothing.
    NoLateBoundary { id: InputId },
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
