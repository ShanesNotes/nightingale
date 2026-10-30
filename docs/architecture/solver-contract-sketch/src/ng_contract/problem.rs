//! One step's problem and every value a solver core produces or consumes. The record builds one
//! per attempt from its own fields: compiled body, bound parameter values, run identity and the
//! accepted view. No dt enters a kernel; stage times are exact integer offsets inside a ticketed
//! interval.
use super::*;

/// Building one on other values is allowed and reaches nothing: `StepProposal::verify` re-evaluates
/// on the record's own.
pub struct CoupledProblem<'r> {
    _body: &'r CompiledBody,
    _parameters: &'r BoundParameters,
    _identity: &'r RunIdentity,
    _accepted: AcceptedStateView<'r>,
    _base: Digest,
}
impl<'r> CoupledProblem<'r> {
    pub fn new(
        _body: &'r CompiledBody,
        _parameters: &'r BoundParameters,
        _identity: &'r RunIdentity,
        _accepted: AcceptedStateView<'r>,
        _base: Digest,
    ) -> Self {
        todo!()
    }
    pub fn accepted(&self) -> AcceptedStateView<'r> {
        todo!()
    }
    /// Numerical scratch seeded from the accepted state, never an accepted patient copy.
    pub fn workspace(&self) -> SolveWorkspace {
        todo!()
    }
    /// Binds the core's intrinsic span to the accepted base, due inputs, frozen drives and identity.
    /// The contract ends the ticket at the earlier of that span's end and the next physical
    /// breakpoint, which no core can see: the causal step rule holds by construction, and a ticket
    /// that starts at a breakpoint says so (`StepTicket::after_breakpoint`).
    pub fn ticket(&self, _intrinsic_span_ns: u64) -> Result<StepTicket, EngineFault> {
        todo!()
    }
    /// The same base over the smaller span a refusal left, through the same clipping as any ticket.
    /// A refusal of another base is `BindingMismatch`.
    pub fn retry(&self, _refused: &Refused) -> Result<StepTicket, EngineFault> {
        todo!()
    }
    /// Rate-class and strongly connected blocks derived from dependencies, never organ names.
    /// `Coupled` gives the exact block-triangular order; a split needs a matching qualification.
    pub fn blocks(&self, _plan: EvaluationPlan<'_>) -> Result<&[EquationBlock], EngineFault> {
        todo!()
    }
    /// Residuals F(t, y, y') of every row and the guards, with the bound parameter values. A drive
    /// reads the ticket's frozen frame, never the stage's instant. The evaluation keeps a copy of
    /// the point it was evaluated at.
    pub fn evaluate(&self, _point: &StagePoint<'_, '_>) -> Result<StageEvaluation, TrialFailure> {
        todo!()
    }
    /// One block's rows, for a split core: added beside `evaluate`, so a whole-scope core never
    /// names a scope.
    pub fn evaluate_block(
        &self,
        _block: &EquationBlock,
        _point: &StagePoint<'_, '_>,
    ) -> Result<StageEvaluation, TrialFailure> {
        todo!()
    }
    /// dF/dy + shift * dF/dy' of the same expression program, by fixed-order forward
    /// differentiation, as a sparse matrix over the compiled pattern.
    pub fn linearize(&self, _point: &StagePoint<'_, '_>, _shift: Shift) -> Result<Jacobian, TrialFailure> {
        todo!()
    }
    pub fn linearize_block(
        &self,
        _block: &EquationBlock,
        _point: &StagePoint<'_, '_>,
        _shift: Shift,
    ) -> Result<Jacobian, TrialFailure> {
        todo!()
    }
    /// Bound to the ticket, so a stage from another attempt is refused.
    pub fn quadrature(&self, _ticket: &StepTicket) -> Quadrature {
        todo!()
    }
    /// Bound also to the plan a split stepper's blocks come from. A split plan is checked as
    /// `blocks` checks it, else `BindingMismatch`; the plan's digest enters the stage witness.
    pub fn quadrature_for(
        &self,
        _ticket: &StepTicket,
        _plan: EvaluationPlan<'_>,
    ) -> Result<Quadrature, EngineFault> {
        todo!()
    }
    /// Guard localization and simultaneous-event ordering by stable guard identity.
    pub fn transitions(
        &self,
        _before: &StageEvaluation,
        _after: &StageEvaluation,
        _ticket: &StepTicket,
    ) -> Result<ModeChanges, TrialFailure> {
        todo!()
    }
    /// Assembles a proposal. Amounts and memory will come from the start plus the increment, never
    /// from the solver's own coordinates, and the dense segment from the published ends; `end`
    /// supplies only algebraic values, which `verify` rechecks at the published state.
    pub fn propose(
        &self,
        _ticket: &StepTicket,
        _increment: Increment,
        _end: &SolveWorkspace,
        _modes: ModeChanges,
        _history: MethodHistory,
    ) -> StepProposal {
        todo!()
    }
    /// Eligible only when the compiled body has no algebraic residual rows.
    pub fn explicit_bench(&self) -> Result<ExplicitBench<'_>, AdmissionError> {
        todo!()
    }
}

/// Solver-writable trial coordinates in one fixed layout; organ code never touches them.
pub struct SolveWorkspace {
    _layout: Digest,
    _unknowns: Vec<f64>,
    _derivative: Vec<f64>,
}
impl SolveWorkspace {
    pub fn unknowns_mut(&mut self) -> &mut [f64] {
        todo!()
    }
    /// The tangent y' a DAE stage evaluates against, written by the method (backward Euler,
    /// SDIRK or Radau stages; see `Quadrature` for why multistep methods are outside the seam).
    pub fn derivative_mut(&mut self) -> &mut [f64] {
        todo!()
    }
    /// The current (y, y') at a stage time, bound to this layout.
    pub fn point<'w, 't>(&'w self, _time: StageTime<'t>) -> StagePoint<'w, 't> {
        todo!()
    }
}
pub struct StagePoint<'w, 't> {
    _layout: Digest,
    _time: StageTime<'t>,
    _unknowns: &'w [f64],
    _derivative: &'w [f64],
}
/// An exact integer offset inside one ticket, minted by it, never from f64 time. It borrows the
/// ticket, so `evaluate` can refuse with only a point in hand, and an evaluation from an earlier
/// attempt cannot enter this one.
#[derive(Clone, Copy)]
pub struct StageTime<'t> {
    _ticket: &'t StepTicket,
    _offset_ns: u64,
}
/// The derivative coefficient in dF/dy + shift * dF/dy' (1/h for backward Euler).
#[derive(Clone, Copy)]
pub struct Shift {
    _per_second: f64,
}
/// One stage's evaluation. It carries the point (y, y') it was evaluated at, copied in, so the stage
/// outlives the workspace moving on and the record can re-evaluate it, and, for a block evaluation,
/// the block's plan digest and ordinal. Only `CoupledProblem::evaluate` makes one.
pub struct StageEvaluation {
    _layout: Digest,
    _ticket: Digest,
    _offset_ns: u64,
    _block: Option<(Digest, u32)>,
    _unknowns: Vec<f64>,
    _derivative: Vec<f64>,
    _columns: Vec<f64>,
    _rows: Vec<f64>,
    _residuals: Vec<f64>,
    _guards: Vec<f64>,
}
impl StageEvaluation {
    /// F per row: what a Newton core drives to zero. With the tangent at zero, a differential row's
    /// value is minus its rate, so an explicit core reads its rates here too.
    pub fn residuals(&self) -> &[f64] {
        todo!()
    }
    pub fn guards(&self) -> &[f64] {
        todo!()
    }
}
pub struct Jacobian {
    _layout: Digest,
    _row_offsets: Vec<usize>,
    _columns: Vec<UnknownId>,
    _values: Vec<f64>,
}
impl Jacobian {
    pub fn view(&self) -> SparseView<'_> {
        todo!()
    }
}
/// Compressed sparse rows; the pattern is the compiled dependency envelope. View row `r` is
/// `residuals()[r]` of an evaluation at the same point, its entries are
/// `row_offsets()[r]..row_offsets()[r + 1]`, and a column's `UnknownId::index` is its workspace slot.
pub struct SparseView<'a> {
    _row_offsets: &'a [usize],
    _columns: &'a [UnknownId],
    _values: &'a [f64],
}
impl<'a> SparseView<'a> {
    pub fn row_offsets(&self) -> &'a [usize] {
        todo!()
    }
    pub fn columns(&self) -> &'a [UnknownId] {
        todo!()
    }
    pub fn values(&self) -> &'a [f64] {
        todo!()
    }
}
/// One block of the partition a plan issued; it carries that plan's digest and its ordinal there.
pub struct EquationBlock {
    _plan: Digest,
    _ordinal: u32,
    _rows: Vec<RowId>,
    _unknowns: Vec<UnknownId>,
    _rate: RateClass,
}
impl EquationBlock {
    pub fn rows(&self) -> &[RowId] {
        todo!()
    }
    pub fn unknowns(&self) -> &[UnknownId] {
        todo!()
    }
    pub fn rate_class(&self) -> &RateClass {
        todo!()
    }
}
/// Which blocks a stepper evaluates apart. `blocks` and `quadrature` take the same plan, so the
/// witness says which partition its stages claim; a split's digest is its qualification's.
#[derive(Clone, Copy)]
pub enum EvaluationPlan<'a> {
    Coupled,
    QualifiedSplit(&'a SplitQualification),
    /// Only in a run whose declared resolution is a split measurement; such records feed
    /// `SplitMeasurement` and are never scored by the arena.
    UnderMeasurement,
}

/// Accumulates weighted stage evaluations over one ticketed interval: the ONLY producer of
/// `Extents` and the memory increment. Backward Euler is one evaluation at weight 1; a beat
/// stepper's bins are many block evaluations whose weights sum to 1 per column; SDIRK uses its
/// stage weights. Forward and reverse parts of each column stay apart, so the ledger keeps gross
/// in and out. The seam therefore admits one-step methods only: a linear multistep (BDF) or
/// Rosenbrock update is not a weighted sum of rates inside the interval, and such a core needs a
/// contract version with a history term. No ledger term crosses an interval boundary.
pub struct Quadrature {
    _layout: Digest,
    _ticket: Digest,
    _plan: Digest,
    _length_ns: u64,
    _total: Ratio,
    _forward: Vec<Compensated>,
    _reverse: Vec<Compensated>,
    _memory: Vec<f64>,
    _stages: Vec<WitnessStage>,
}
impl Quadrature {
    /// Adds the weighted rates and keeps the evaluation's point, weight and block as a witness
    /// stage. `weight` is an exact fraction of the ticketed interval, so summing to one is an
    /// integer equality. Refuses an evaluation from another layout, one another ticket minted (an
    /// earlier attempt), or one of a block another plan issued: `BindingMismatch`.
    pub fn add(&mut self, _evaluation: &StageEvaluation, _weight: Ratio) -> Result<(), EngineFault> {
        todo!()
    }
    /// Refuses unless the weights sum to one exactly.
    pub fn finish(self) -> Result<Increment, EngineFault> {
        todo!()
    }
}
pub struct Increment {
    _ticket: Digest,
    _extents: Extents,
    _memory: Vec<f64>,
    _witness: StageWitness,
}
/// The stages an increment was built from: each stage's offset, weight, point (y, y') and block,
/// and the digest of the evaluation plan the quadrature was bound to. Made only by `Quadrature`,
/// and private: a rate is not kept, because `StepProposal::verify` re-evaluates every point and
/// re-adds it through the same quadrature, so one code path decides that a recorded rate differs
/// and that the sum is not the scheme's. A ledger cannot come from one sample while the state came
/// from other stages, a block cannot be lagged without a qualification, and the declared scheme's
/// error estimate is recomputed, not trusted.
struct StageWitness {
    _plan: Digest,
    _stages: Vec<WitnessStage>,
}
struct WitnessStage {
    _offset_ns: u64,
    _weight: Ratio,
    /// The block's ordinal under the witnessed plan; `None` for a whole-scope stage.
    _block: Option<u32>,
    _unknowns: Vec<f64>,
    _derivative: Vec<f64>,
}
/// Integrated gross extents per compiled column. No public constructor.
pub struct Extents {
    _layout: Digest,
    _forward: Vec<Compensated>,
    _reverse: Vec<Compensated>,
}

/// One accepted interval of every row: today the chord between the base and the published values,
/// degree 1, derived, so no core can publish a curve its stages did not make. Waveform samples on
/// the output grid and inventory curves all read this form. The admission segment has no interval.
pub struct DenseSegment {
    _interval: Option<Interval>,
    _basis: DenseBasis,
    _start: Vec<Compensated>,
    _end: Vec<Compensated>,
}
impl DenseSegment {
    pub fn interval(&self) -> Option<Interval> {
        todo!()
    }
    pub fn basis(&self) -> DenseBasis {
        todo!()
    }
    /// One row at `at`; `None` outside the interval.
    pub fn sample(&self, _row: RowId, _at: SimTime) -> Option<Compensated> {
        todo!()
    }
}
#[derive(Clone, Copy)]
pub struct DenseBasis {
    _family: BasisFamily,
    _degree: u8,
}
impl DenseBasis {
    pub fn family(&self) -> BasisFamily {
        todo!()
    }
    pub fn degree(&self) -> u8 {
        todo!()
    }
}
/// Bernstein form: the coefficient hull bounds every interior value, which makes the
/// certificate's interior-extremum check conservative without sampling.
#[derive(Clone, Copy)]
pub enum BasisFamily {
    Bernstein,
}
pub struct ModeChanges {
    _events: Vec<EventWitness>,
}
pub struct EventWitness {
    _guard: u32,
    _at: SimTime,
    _ordinal: u64,
}

/// Binds one attempt: its base, the identity, the exact interval, the due inputs, the frozen drives
/// and the attempt number, all in its digest.
#[derive(Clone)]
pub struct StepTicket {
    _base: Digest,
    _interval: Interval,
    _due: Vec<InputId>,
    _frame: DriveFrame,
    _after_breakpoint: bool,
    _attempt: u32,
    _floor_ns: u64,
    _budget: u32,
    _digest: Digest,
}
impl StepTicket {
    pub fn interval(&self) -> Interval {
        todo!()
    }
    pub fn length_ns(&self) -> u64 {
        todo!()
    }
    /// How many refusals of this base came before this ticket.
    pub fn attempt(&self) -> u32 {
        todo!()
    }
    pub fn digest(&self) -> Digest {
        todo!()
    }
    /// A stage time inside this interval; an offset past the end is `BindingMismatch`.
    pub fn at(&self, _offset_ns: u64) -> Result<StageTime<'_>, EngineFault> {
        todo!()
    }
    /// `span_ns` as a fraction of this interval: an exact stage weight.
    pub fn fraction(&self, _span_ns: u64) -> Ratio {
        todo!()
    }
    /// `1 / span` for a stage that spans `span_ns` of this interval. Zero is `BindingMismatch`.
    pub fn shift(&self, _span_ns: u64) -> Result<Shift, EngineFault> {
        todo!()
    }
    /// A refusal of this attempt, or the fault `NoAdmissibleStep` when its retry would go below the
    /// floor or past the budget. One path, so nothing can accept excess error at the floor.
    pub fn refuse(&self, _reason: NumericalRefusal) -> TrialFailure {
        todo!()
    }
    /// A `BindingMismatch` naming this attempt.
    pub fn mismatch(&self) -> EngineFault {
        todo!()
    }
    /// Whether this is the ticket `problem` issues for its length and attempt.
    pub fn binds(&self, _problem: &CoupledProblem<'_>) -> bool {
        todo!()
    }
    /// True when this interval starts at a physical breakpoint (an input applied, a delivery ended,
    /// or a guard fired and changed mode). A core restarts its numerical caches there (method
    /// history, Jacobian, predictor, step controller) and `verify` checks that the proposal's
    /// `MethodHistory` starts at this instant. Physiological state and the cumulative ledger never
    /// restart (pulse 0006).
    pub fn after_breakpoint(&self) -> bool {
        todo!()
    }
}
/// A candidate step. Owned, holding no problem, and proving nothing until `verify` runs on a problem
/// the record built from its own fields.
pub struct StepProposal {
    _ticket: StepTicket,
    _layout: Digest,
    _increment: Increment,
    _algebraic: Vec<f64>,
    _modes: ModeChanges,
    _history: MethodHistory,
}
impl StepProposal {
    pub fn ticket(&self) -> &StepTicket {
        todo!()
    }
    /// Whether the core followed its declared method on this body. In order: the stage structure
    /// (`MethodMismatch`), the replay of each witnessed point compared by bits (`MethodMismatch`),
    /// the stage equations (`StageResidual`), the scheme's error estimate (`Accuracy`, or
    /// `NoAdmissibleStep` at the floor), the restart after a breakpoint (`MethodMismatch`) and the
    /// continuation schema (`BindingMismatch`), then the published state, evaluated once: algebraic
    /// residuals (`StageResidual`), non-finite values (`TrialDomain`) and validity excursions.
    pub fn verify(self, _problem: &CoupledProblem<'_>) -> Result<VerifiedStep, TrialFailure> {
        todo!()
    }
}
/// A step that followed its declared method. Everything in it derives from the record's own
/// problem; the record still decides whether it may publish.
pub struct VerifiedStep {
    _ticket: StepTicket,
    _state: Candidate,
    _continuation: MethodHistory,
    _dense: DenseSegment,
    _forward: Vec<Compensated>,
    _reverse: Vec<Compensated>,
    _excursions: Vec<ValidityExcursion>,
}
impl VerifiedStep {
    pub fn ticket(&self) -> &StepTicket {
        todo!()
    }
    /// The published value of one row.
    pub fn value(&self, _row: RowId) -> Compensated {
        todo!()
    }
    /// Each column's gross forward and reverse extent, in compiled column order.
    pub fn gross(&self) -> impl Iterator<Item = (Compensated, Compensated)> + '_ {
        core::iter::empty()
    }
    /// The inputs this step applies, each once, in `(time, id)` order.
    pub fn applied(&self) -> &[InputId] {
        todo!()
    }
    pub fn into_parts(self) -> VerifiedParts {
        todo!()
    }
}
/// What the record takes from a verified step when it builds the next image.
pub struct VerifiedParts {
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub excursions: Vec<ValidityExcursion>,
}
