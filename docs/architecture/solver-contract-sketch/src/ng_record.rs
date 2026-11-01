//! Proposed ng-record: the sole accepted-state writer, its session latch, one bound history and
//! the checkpoint envelope.
//!
//! A step is validate, then publish. `PatientRecord::attempt(&self, ..)` is immutable and returns the
//! ENTIRE next image or a failure; `publish(&mut self, ..)` installs that image and cannot fail.
//! `&mut PatientRecord` is never lent outside this module, so only the session publishes. A session
//! that met a refusal becomes `Session<Retrying>`, which can only retry, and one that met an
//! `EngineFault` becomes `Session<Faulted>`, which has no step.
use crate::ng_contract::*;

/// The chained seal of one accepted revision: the previous seal, the image's digests and the ledger
/// chunk. It covers the past and nothing that waits, so a ticket's digest built on it reads no later
/// input.
#[derive(Clone, Copy)]
pub struct Seal {
    _digest: Digest,
}
impl Seal {
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
/// No Clone, no public constructor and no setter. One body state per branch.
pub struct PatientRecord<'m> {
    _body: &'m CompiledBody,
    _identity: RunIdentity,
    _parameters: BoundParameters,
    _branch: BranchAncestry,
    _revision: u64,
    _time: SimTime,
    _state: Candidate,
    _continuation: MethodHistory,
    _dense: DenseSegment,
    _pending: PendingInputs,
    _history: History,
}
impl PatientRecord<'_> {
    pub fn identity(&self) -> &RunIdentity {
        todo!()
    }
    pub fn revision(&self) -> u64 {
        todo!()
    }
    pub fn seal(&self) -> Seal {
        todo!()
    }
    pub fn accepted(&self) -> AcceptedStateView<'_> {
        todo!()
    }
    /// Lends the problem for one attempt, built from this record's own fields. Only `step` publishes.
    pub fn problem(&self) -> CoupledProblem<'_> {
        todo!()
    }
    /// The earliest stamp `Session::enqueue` takes now.
    pub fn earliest_stamp(&self) -> SimTime {
        todo!()
    }
    /// Oldest first. Empty on the run's own branch.
    pub fn ancestry(&self) -> &[BranchFork] {
        todo!()
    }
    /// Runs one attempt of the next step to the image it would publish, or its failure: the contract
    /// answers whether the core followed its declared method (`StepProposal::verify`, on this record's
    /// own problem), and the record whether the result may become the next accepted image. It builds
    /// the whole next image, with any validity excursions recorded rather than refused. Immutable, so
    /// a host can drive a core by hand. Only a session publishes. A refusal is retryable; an
    /// `EngineFault` is not, and the session latches on it.
    pub fn attempt(&self, _solver: &dyn CoupledSolver, _attempt: Attempt<'_>) -> Result<NextImage, TrialFailure> {
        todo!()
    }
    /// The one path that changes accepted state. Installs every part of the image together; cannot
    /// fail.
    pub fn publish(&mut self, _next: NextImage) -> StepReceipt {
        todo!()
    }
    /// The one history object the certificate reads.
    pub fn conservation(&self) -> ConservationView<'_> {
        todo!()
    }
    /// One digest per part, recomputed from the live fields on every call. It reads no seal, because
    /// a seal would still match if a field changed underneath it.
    pub fn digests(&self) -> RecordDigests {
        todo!()
    }
    /// Four-family samples of this branch on a grid, for split measurement only.
    pub fn split_trace(&self, _grid: TraceGrid) -> Result<SplitTrace, AdmissionError> {
        todo!()
    }
}

/// What must stay byte-identical across a refusal.
pub struct RecordDigests {
    pub revision: u64,
    pub time: SimTime,
    pub state: Digest,
    pub continuation: Digest,
    pub segment: Digest,
    pub streams: Digest,
    pub applied: Digest,
    pub waiting: Digest,
    pub ledger: Digest,
}

/// The entire next accepted image, validated and not yet visible.
pub struct NextImage {
    _revision: u64,
    _time: SimTime,
    _state: Candidate,
    _fired: Vec<Fired>,
    _continuation: MethodHistory,
    _ledger: LedgerChunk,
    _pending: AppliedInputs,
    _dense: DenseSegment,
    _seal: Seal,
    _applied: Vec<InputId>,
    _excursions: Vec<ValidityExcursion>,
}
pub struct StepReceipt {
    _revision: u64,
    _seal: Seal,
    _applied: Vec<InputId>,
    _after_breakpoint: bool,
    _excursions: Vec<ValidityExcursion>,
}
impl StepReceipt {
    pub fn revision(&self) -> u64 {
        todo!()
    }
    pub fn seal(&self) -> Seal {
        todo!()
    }
    /// Each input this step applied, once, in `(time, id)` order.
    pub fn applied(&self) -> &[InputId] {
        todo!()
    }
    pub fn after_breakpoint(&self) -> bool {
        todo!()
    }
    pub fn excursions(&self) -> &[ValidityExcursion] {
        todo!()
    }
}
/// A durable input reference from an authorized command's receipt, never a speculative request.
pub struct AuthorizedInput {
    _command: Digest,
    _event: BoundaryEvent,
}
impl AuthorizedInput {
    pub fn from_command_receipt(_receipt: Digest, _event: BoundaryEvent) -> Self {
        todo!()
    }
    /// No seal covers it, so an input with a command and the same input known at admission give the same
    /// bits. A host's own log pairs the two.
    pub fn command(&self) -> Digest {
        todo!()
    }
}
/// What a host keeps of an admitted input, derived from the input alone, so one request has one
/// receipt whichever command repeats it.
pub struct InputReceipt {
    _id: InputId,
    _payload: Digest,
    _at: SimTime,
}

/// One accepted interval of the history, each value at the interval's ends in the segment's basis:
/// for every conserved combination its inventory and its cumulative gross inflow and outflow, for
/// every column its cumulative gross forward and reverse extents, plus right-continuous impulses
/// for boluses. All compensated. Private, so fixed-time chunks and a history store change no reader.
struct LedgerChunk {
    _interval: Interval,
    _basis: DenseBasis,
    _quantities: Vec<QuantityStep>,
    _columns: Vec<ColumnStep>,
    _impulses: Vec<Impulse>,
}
struct QuantityStep {
    _inventory: [Compensated; 2],
    _inflow: [Compensated; 2],
    _outflow: [Compensated; 2],
}
struct ColumnStep {
    _forward: [Compensated; 2],
    _reverse: [Compensated; 2],
}
/// A bolus, right-continuous: its amount is in the combination's inventory from `at` on.
pub struct Impulse {
    _at: SimTime,
    _quantity: QuantityId,
    _amount: f64,
}
impl Impulse {
    pub fn at(&self) -> SimTime {
        todo!()
    }
    pub fn quantity(&self) -> QuantityId {
        todo!()
    }
    pub fn amount(&self) -> f64 {
        todo!()
    }
}
/// One sealed chunk per accepted step, with prefix extents. Each seal chains the seal before it, the
/// image's digest and its chunk's digest, so a seal never rehashes all history per beat.
struct History {
    _basis: Digest,
    _opening: Vec<Compensated>,
    _chunks: Vec<LedgerChunk>,
}
/// Run identity, branch and the accepted base this view was lent at.
#[derive(Clone, Copy)]
pub struct HistorySeal {
    pub run: Digest,
    pub branch: Digest,
    pub base: Digest,
}
/// One immutable, sealed view of one record's history over one conserved basis. There is no way
/// to pair one record's inventories with another's exchanges.
pub struct ConservationView<'r> {
    _history: &'r History,
    _basis: &'r ConservedBasis,
    _seal: HistorySeal,
}
impl<'r> ConservationView<'r> {
    pub fn seal(&self) -> HistorySeal {
        todo!()
    }
    pub fn basis(&self) -> &'r ConservedBasis {
        todo!()
    }
    /// Admission to the accepted head; `None` before the first accepted step.
    pub fn span(&self) -> Option<Interval> {
        todo!()
    }
    /// Every accepted segment in time order, each ending where the next starts.
    pub fn segments(&self) -> Segments<'r> {
        todo!()
    }
}
pub struct Segments<'r> {
    _chunks: core::slice::Iter<'r, LedgerChunk>,
}
impl<'r> Iterator for Segments<'r> {
    type Item = HistorySegment<'r>;
    fn next(&mut self) -> Option<HistorySegment<'r>> {
        todo!()
    }
}
impl<'r> DoubleEndedIterator for Segments<'r> {
    fn next_back(&mut self) -> Option<HistorySegment<'r>> {
        todo!()
    }
}
pub struct HistorySegment<'r> {
    _chunk: &'r LedgerChunk,
}
impl<'r> HistorySegment<'r> {
    pub fn interval(&self) -> Interval {
        todo!()
    }
    /// Polynomial family and degree of every curve in this segment.
    pub fn basis(&self) -> DenseBasis {
        todo!()
    }
    pub fn inventory(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    /// Cumulative gross inflow since admission.
    pub fn inflow(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    /// Cumulative gross outflow since admission.
    pub fn outflow(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    /// One column's cumulative gross forward and reverse extents since admission.
    pub fn exchange(&self, _column: ColumnId) -> (Curve<'r>, Curve<'r>) {
        todo!()
    }
    /// Gross exchange on one held route (`CoverageManifest::held_routes`), summed from its columns,
    /// apart from its combination's totals.
    pub fn held_inflow(&self, _route: HeldRouteId) -> Curve<'r> {
        todo!()
    }
    pub fn held_outflow(&self, _route: HeldRouteId) -> Curve<'r> {
        todo!()
    }
    pub fn impulses(&self) -> &'r [Impulse] {
        todo!()
    }
}
/// One quantity's curve over one segment, compensated, in Bernstein form.
pub struct Curve<'r> {
    _interval: Interval,
    _coefficients: &'r [Compensated],
}
impl Curve<'_> {
    /// The Bernstein coefficients of this curve restricted to `piece`; `None` outside the segment.
    /// Same degree, so two curves on one piece subtract coefficient by coefficient, and the hull of
    /// the result bounds the difference.
    pub fn on(&self, _piece: Interval) -> Option<Vec<Compensated>> {
        todo!()
    }
}

/// The host's session over one record, bound at admission to one compiled body and one solver.
pub struct Session<'m, S> {
    _solver: &'m dyn CoupledSolver,
    _record: Box<PatientRecord<'m>>,
    _status: S,
}
pub struct Running {
    _private: (),
}
/// A refusal stands: the record is untouched, and the only way on is the smaller retry.
pub struct Retrying {
    _refused: Refused,
    _earlier: Vec<Refused>,
}
pub struct Faulted {
    _fault: EngineFault,
    _refusals: Vec<Refused>,
}
pub enum Step<'m> {
    Accepted(Session<'m, Running>, StepReceipt),
    /// Nothing changed; `Session::retry` tries the same base over a shorter interval.
    Refused(Session<'m, Retrying>),
    /// Latched: stop and show it, never death.
    Faulted(Session<'m, Faulted>),
}
impl<'m, S> Session<'m, S> {
    pub fn record(&self) -> &PatientRecord<'m> {
        todo!()
    }
    /// Reads the last accepted image through a lens, also after a fault. A lens compiled for
    /// another body is refused. The session never sees observers: physiology is the same with
    /// any lens or none.
    pub fn observation<'r>(&'r self, _lens: &'r ObservationLens) -> Result<ObservationView<'r>, ObservationRefusal> {
        todo!()
    }
}
/// One accepted revision through a lens. It has no write path: nothing read here reaches a law, the
/// record or the certificate, and it borrows the session, so no step runs while it is alive.
pub struct ObservationView<'r> {
    _lens: &'r ObservationLens,
    _accepted: AcceptedStateView<'r>,
    _seal: Digest,
    _previous: Option<Digest>,
}
impl<'r> ObservationView<'r> {
    pub fn lens(&self) -> &'r ObservationLens {
        todo!()
    }
    pub fn time(&self) -> SimTime {
        todo!()
    }
    pub fn seal(&self) -> Digest {
        todo!()
    }
    /// The seal this revision follows; `None` at admission.
    pub fn previous(&self) -> Option<Digest> {
        todo!()
    }
    /// Each grid instant this revision holds, oldest first, with one value per channel. A step holds
    /// its end and not its start, so an instant falls in one step, and revision 0 holds the admission
    /// instant when it is on the grid. At the head a channel reads the accepted state, which is what
    /// a firing left, so one instant has one value. A read never forces a cut.
    pub fn samples(&self) -> Vec<(SimTime, Vec<Compensated>)> {
        todo!()
    }
}
pub enum ObservationRefusal {
    /// A lens compiled for another body, a view through another lens, or a checkpoint of another
    /// lens or seed.
    OtherLens,
    /// A view that does not follow the last revision read: skipped, repeated or of another course.
    Gap { expected: Digest, found: Option<Digest> },
    /// Checkpoint bytes that are not an observer's, or that changed.
    Malformed,
}
impl<'m> Session<'m, Running> {
    /// Checks model, packet and resolution identity against the body, origin and solver, and
    /// refuses a zero floor.
    pub fn admit(
        _body: &'m CompiledBody,
        _solver: &'m dyn CoupledSolver,
        _origin: InitialRevision,
        _identity: RunIdentity,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
    /// Continues the run `saved` holds, on its branch, in `solver`, which may be freshly built.
    /// Everything is verified before a record exists.
    pub fn resume(
        _saved: &Checkpoint,
        _expected: &RunIdentity,
        _body: &'m CompiledBody,
        _solver: &'m dyn CoupledSolver,
        _store: &dyn HistoryStore,
    ) -> Result<Self, CheckpointError> {
        todo!()
    }
    /// Adds a late input. No accepted part moves, so no revision and no seal, and an observer sees no
    /// gap. Stamped before the accepted head or the fence, it is refused.
    pub fn enqueue(&mut self, _input: AuthorizedInput) -> Result<InputReceipt, AdmissionError> {
        todo!()
    }
    /// `next_step`, `solve`, `validate`, then `publish`. A fault from any of them latches.
    pub fn step(self) -> Step<'m> {
        todo!()
    }
    /// Saves the latest accepted boundary; never cuts an interval. `store` takes the links it lacks
    /// first, so the bytes this returns are complete with that store.
    pub fn checkpoint(&self, _store: &mut dyn HistoryStore) -> Result<Checkpoint, CheckpointError> {
        todo!()
    }
    /// A new branch from a checkpoint on this branch's own chain, with ancestry, through the path a
    /// load takes. The parent is untouched, and the solver is this session's.
    pub fn rewind(&self, _saved: &Checkpoint, _branch: BranchId) -> Result<Session<'m, Running>, CheckpointError> {
        todo!()
    }
}
impl<'m> Session<'m, Retrying> {
    pub fn refused(&self) -> &Refused {
        todo!()
    }
    /// The same base and inputs over the refusal's smaller span; an exhausted budget latches. The
    /// core solves; it does not pick the interval.
    pub fn retry(self) -> Step<'m> {
        todo!()
    }
}
impl<'m> Session<'m, Faulted> {
    pub fn fault(&self) -> &EngineFault {
        todo!()
    }
    /// The refusals before the fault, each naming its row or source.
    pub fn refusals(&self) -> &[Refused] {
        todo!()
    }
    /// The image is marked, so `resume` refuses it: a faulted run goes on only through `recover`.
    pub fn export(&self, _store: &mut dyn HistoryStore) -> Result<FaultReport, CheckpointError> {
        todo!()
    }
    /// The only way on: a NEW branch from a checkpoint on this branch's own chain, with ancestry.
    pub fn recover(&self, _earlier: &Checkpoint, _branch: BranchId) -> Result<Session<'m, Running>, CheckpointError> {
        todo!()
    }
}
pub struct FaultReport {
    _last_accepted: Checkpoint,
    /// The fault itself: kind, time, attempted interval and the refusal that ended it.
    _fault: EngineFault,
    /// The refusals that preceded it, each naming its row or source.
    _refusals: Vec<Refused>,
}
pub struct BranchFork {
    _parent: Digest,
    _revision: u64,
    _at: SimTime,
}
/// A checkpoint carries it and no seal covers it, so a fork and the same course loaded from its
/// checkpoint have equal seals.
pub struct BranchAncestry {
    _id: Digest,
    _forks: Vec<BranchFork>,
}
pub struct BranchId {
    _digest: Digest,
}
impl BranchId {
    pub fn new(_digest: Digest) -> Self {
        todo!()
    }
}
/// A checkpoint's only form: the envelope's bytes. `Session::checkpoint` writes them, and
/// `Session::resume`, `Session::rewind` and `Session::recover` read them through one private path, so
/// an in-process retry and a loaded save are one path. A host persists `as_bytes` and reloads with
/// `from_bytes`, which checks nothing. The envelope holds state, bound parameters, solver continuation,
/// stream positions, the timeline, the fence, branch ancestry, the head's seal and the last dense
/// segment. An observer's own state is checkpointed beside it, bound to its seal
/// (`ng_observe::ObserverCheckpoint`), so a restore brings back outputs too (nightingale-pulse#273).
pub struct Checkpoint {
    _bytes: Vec<u8>,
}
impl Checkpoint {
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
    pub fn from_bytes(_bytes: Vec<u8>) -> Self {
        todo!()
    }
}
pub enum CheckpointError {
    Truncated,
    UnknownFormat,
    TrailingContent,
    Corrupt,
    IdentityMismatch,
    Latched,
    Malformed,
    SealMismatch,
    HistoryMissing { seal: Digest },
    HistoryAltered { seal: Digest },
    StoreFailed { seal: Digest },
    BranchInUse,
    NotOnThisBranch,
}
/// Durable storage for the chain's links, addressed by seal. It is host code and trusted for nothing:
/// a walk chains what comes back to the seal it asked for. A checkpoint is complete only with a store
/// that keeps what it took.
pub trait HistoryStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>>;
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError>;
}
