//! Proposed ng-record: the sole accepted-state writer, its session latch, one bound history and
//! the checkpoint envelope.
//!
//! A step is validate, then publish. `PatientRecord::validate(&self, ..)` is immutable and returns
//! the ENTIRE next image or a refusal; `publish(&mut self, ..)` installs that image and cannot fail.
//! `&mut PatientRecord` is never lent outside this module, so only `Session<Running>` publishes,
//! and a session that met an `EngineFault` becomes `Session<Faulted>`, which has no step.
use crate::ng_contract::*;

/// No Clone, no public constructor and no setter. One body state per branch.
pub struct PatientRecord {
    _identity: RunIdentity,
    _parameters: BoundParameters,
    _basis: ConservedBasis,
    _branch: BranchAncestry,
    _revision: u64,
    _seal: Seal,
    _time: SimTime,
    _state: Candidate,
    _continuation: SolverContinuation,
    _dense: DenseSegment,
    _streams: StreamPositions,
    _pending: PendingInputs,
    _commands: CommandCursor,
    _history: History,
}
impl PatientRecord {
    pub fn identity(&self) -> &RunIdentity {
        todo!()
    }
    pub fn accepted(&self) -> AcceptedStateView<'_> {
        todo!()
    }
    /// Rechecks, with the proposal's own compiled body and parameter values: identity, branch and
    /// base seal, exact ticket start AND end, due input ids, complete mode, state and closure
    /// coverage, admissible sets, amounts = base + paired extents, nonlinear energy closure,
    /// continuation schema and dense-segment endpoints. It re-evaluates every witnessed stage on
    /// the compiled body in the scope the stage names, the whole or a block of the witnessed plan
    /// (`MethodMismatch` if a recorded rate or the weighted sum differs), checks the declared
    /// scheme's stage equations (`StageResidual`), recomputes the scheme's error estimate against
    /// the resolution's scales (refused above one, and a fault at the floor), and after a
    /// breakpoint checks that the method history restarted there. Then it builds the whole next
    /// image, with any validity excursions recorded rather than refused. Never mutates. A refusal
    /// is retryable; an `EngineFault` (a method mismatch, excess error at the floor) is not, and
    /// the session latches on it.
    pub fn validate(&self, _proposal: StepProposal<'_>) -> Result<NextImage, TrialFailure> {
        todo!()
    }
    /// Same id and payload returns the prior receipt; a changed payload is refused; a new input
    /// becomes a next image that only extends the pending timeline. An input stamped before the
    /// accepted head is refused: the host stamps a click at the head or later, never in the past.
    pub fn admit_input(&self, _input: AuthorizedInput) -> Result<InputAdmission, AdmissionError> {
        todo!()
    }
    /// The only `&mut` path. Installs every part of the image together; cannot fail.
    pub fn publish(&mut self, _next: NextImage) -> Receipt {
        todo!()
    }
    /// The one history object the certificate reads.
    pub fn conservation(&self) -> ConservationView<'_> {
        todo!()
    }
    /// Four-family samples of this branch on a grid, for split measurement only.
    pub fn split_trace(&self, _grid: TraceGrid) -> Result<SplitTrace, AdmissionError> {
        todo!()
    }
}

/// The entire next accepted image, validated and not yet visible.
pub struct NextImage {
    _base: Seal,
    _revision: u64,
    _time: SimTime,
    _state: Candidate,
    _fired: ModeChanges,
    _continuation: SolverContinuation,
    _ledger: LedgerChunk,
    _inputs: CommandCursor,
    _pending: PendingInputs,
    _dense: DenseSegment,
    _streams: StreamPositions,
    _seal: Seal,
}
pub enum InputAdmission {
    AlreadyPresent(InputReceipt),
    Staged(NextImage),
}
pub struct Receipt {
    _revision: u64,
    _seal: Seal,
    _excursions: Vec<ValidityExcursion>,
}
/// An accepted value outside its law's `ValidityRange`: reported, never refused. Abnormal
/// physiology publishes; only an `Admissible` set can refuse a trial.
pub struct ValidityExcursion {
    _row: RowId,
    _at: SimTime,
    _value: f64,
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
}
pub struct InputReceipt {
    _id: InputId,
    _revision: u64,
}
/// Applied input ids advance only with a published image.
pub struct CommandCursor {
    _applied: u64,
    _fence: u64,
}

/// One accepted interval in the history: dense inventory of every conserved combination and its
/// cumulative gross inflow and outflow, the same exchange for every held route, all in the segment's
/// declared basis, plus right-continuous impulses for boluses.
pub struct LedgerChunk {
    _interval: Interval,
    _basis: DenseBasis,
    /// Compensated inventory of every combination at the chunk start; coefficients are changes.
    _inventory: Vec<Compensated>,
    /// Cumulative gross exchange since admission, compensated like the amounts themselves.
    _inflow_prefix: Vec<Compensated>,
    _outflow_prefix: Vec<Compensated>,
    _inflow: Vec<f64>,
    _outflow: Vec<f64>,
    /// Per held route, summed from that route's compiled columns; part of the totals above, not
    /// added to them.
    _held: Vec<RouteExchange>,
    _impulses: Vec<Impulse>,
}
/// One held route's gross exchange in a chunk, compensated and in the chunk's basis like the totals.
pub struct RouteExchange {
    _inflow_prefix: Compensated,
    _outflow_prefix: Compensated,
    _inflow: Vec<f64>,
    _outflow: Vec<f64>,
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
/// Fixed-time sealed chunks with prefix extents; a seal never rehashes all history per beat.
pub struct History {
    _basis: Digest,
    _chunks: Vec<LedgerChunk>,
    _roots: Vec<Digest>,
}
/// Run identity, branch and the accepted base this view was lent at.
pub struct HistorySeal {
    _run: Digest,
    _branch: Digest,
    _base: Digest,
}
/// One immutable, sealed view of one record's history over one conserved basis. There is no way
/// to pair one record's inventories with another's exchanges.
pub struct ConservationView<'r> {
    _history: &'r History,
    _basis: &'r ConservedBasis,
    _seal: HistorySeal,
}
impl<'r> ConservationView<'r> {
    pub fn seal(&self) -> &HistorySeal {
        todo!()
    }
    pub fn basis(&self) -> &'r ConservedBasis {
        todo!()
    }
    pub fn span(&self) -> Interval {
        todo!()
    }
    /// Accepted segments overlapping the window, in time order.
    pub fn segments(&self, _window: &Interval) -> Segments<'r> {
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
pub struct HistorySegment<'r> {
    _chunk: &'r LedgerChunk,
}
impl<'r> HistorySegment<'r> {
    pub fn interval(&self) -> &'r Interval {
        todo!()
    }
    /// Polynomial family and degree of every curve in this segment.
    pub fn basis(&self) -> DenseBasis {
        todo!()
    }
    pub fn inventory(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    pub fn inflow(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    pub fn outflow(&self, _quantity: QuantityId) -> Curve<'r> {
        todo!()
    }
    /// Gross exchange on one held route (`CoverageManifest::held_routes`), apart from its
    /// combination's totals.
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
/// Compensated value at the segment start plus f64 coefficients of the change within the segment,
/// in the segment's basis. Window differences are formed in compensated arithmetic.
pub struct Curve<'r> {
    _start: Compensated,
    _coefficients: &'r [f64],
}
impl<'r> Curve<'r> {
    pub fn start(&self) -> &Compensated {
        todo!()
    }
    pub fn coefficients(&self) -> &'r [f64] {
        todo!()
    }
}

/// The host's session over one record, bound at admission to one compiled body and one solver.
pub struct Session<'m, S> {
    _body: &'m CompiledBody,
    _solver: &'m dyn CoupledSolver,
    _record: PatientRecord,
    _status: S,
}
pub struct Running {
    _private: (),
}
pub struct Faulted {
    _fault: EngineFault,
}
pub enum Step<'m> {
    Accepted(Session<'m, Running>, Receipt),
    /// Nothing changed; retry smaller with `Session::retry`.
    Refused(Session<'m, Running>, Refused),
    /// Latched: stop and show it, never death.
    Faulted(Session<'m, Faulted>),
}
impl<S> Session<'_, S> {
    pub fn record(&self) -> &PatientRecord {
        todo!()
    }
    /// Reads the last accepted image through a lens, also after a fault. A lens compiled for
    /// another body is refused. The session never sees observers: physiology is the same with
    /// any lens or none.
    pub fn observation<'r>(&'r self, _lens: &'r ObservationLens) -> Result<ObservationView<'r>, ObservationRefusal> {
        todo!()
    }
}
/// Read-only observation of one accepted image. It has no write path: nothing read here reaches a
/// law, the record or the certificate.
pub struct ObservationView<'r> {
    _lens: &'r ObservationLens,
    _parameters: &'r BoundParameters,
    _revision: u64,
    _time: SimTime,
    _state: &'r Candidate,
    _dense: &'r DenseSegment,
}
impl<'r> ObservationView<'r> {
    pub fn time(&self) -> SimTime {
        todo!()
    }
    /// The accepted revision this view reads; an observer advances one revision at a time.
    pub fn revision(&self) -> u64 {
        todo!()
    }
    /// The lens's channels, in stable order.
    pub fn channels(&self) -> &'r [Channel] {
        todo!()
    }
    /// A channel at the accepted instant: its program over accepted state and bound parameters,
    /// before any window, delay or error the observer applies.
    pub fn value(&self, _channel: ChannelId) -> Result<f64, ObservationRefusal> {
        todo!()
    }
    /// The lens grid's instants in the last accepted segment, start excluded and end included.
    /// Each instant falls in exactly one segment, so reading every step sees every sample once.
    pub fn grid(&self) -> Vec<SimTime> {
        todo!()
    }
    /// A channel on the last accepted dense segment at `at`. A read never forces a cut.
    pub fn sample(&self, _channel: ChannelId, _at: SimTime) -> Result<f64, ObservationRefusal> {
        todo!()
    }
}
pub enum ObservationRefusal {
    /// The lens was compiled for another body, or the channel belongs to another lens.
    OtherLens,
    /// The instant lies outside the last accepted dense segment.
    OutsideSegment,
    /// An observer read a revision other than the one after its last.
    Gap { expected: u64, found: u64 },
}
impl<'m> Session<'m, Running> {
    /// Checks model, packet and resolution identity against the body, origin and solver.
    pub fn admit(
        _body: &'m CompiledBody,
        _solver: &'m dyn CoupledSolver,
        _origin: InitialRevision,
        _identity: RunIdentity,
        _boundary: BoundaryPort,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
    /// Lends the problem for inspection or a dry validation; only `step` publishes.
    pub fn problem(&self) -> CoupledProblem<'_> {
        todo!()
    }
    pub fn enqueue(&mut self, _input: AuthorizedInput) -> Result<InputReceipt, AdmissionError> {
        todo!()
    }
    /// `next_step`, `solve`, `validate`, then `publish`. A fault from any of them latches.
    pub fn step(self) -> Step<'m> {
        todo!()
    }
    /// The same base and inputs over the refusal's smaller ticket; an exhausted budget latches.
    pub fn retry(self, _refused: Refused) -> Step<'m> {
        todo!()
    }
    /// Saves the latest accepted boundary; never cuts an interval.
    pub fn checkpoint(&self) -> Checkpoint {
        todo!()
    }
    /// A new branch from a saved boundary of this run, with ancestry. The parent is untouched.
    pub fn rewind(&self, _saved: &Checkpoint, _branch: BranchId) -> Result<Session<'m, Running>, CheckpointError> {
        todo!()
    }
}
impl<'m> Session<'m, Faulted> {
    pub fn fault(&self) -> &EngineFault {
        todo!()
    }
    /// Last accepted image, the fault and the refused attempt's ticket, for diagnosis.
    pub fn export(&self) -> FaultReport {
        todo!()
    }
    /// The only way on: a NEW branch from an earlier checkpoint of this run, with ancestry.
    pub fn recover(&self, _earlier: &Checkpoint, _branch: BranchId) -> Result<Session<'m, Running>, CheckpointError> {
        todo!()
    }
}
pub struct FaultReport {
    _last_accepted: Checkpoint,
    /// The fault itself: kind, time, attempted interval and per-row diagnosis.
    _fault: EngineFault,
    /// The refusals that preceded it, each with its own diagnosis.
    _refusals: Vec<Refused>,
}
pub struct BranchAncestry {
    _parent: Digest,
    _fork_revision: u64,
    _fork_at: SimTime,
}
pub struct BranchId {
    _digest: Digest,
}
impl BranchId {
    pub fn new(_digest: Digest) -> Self {
        todo!()
    }
}
/// The accepted record at one boundary: state, bound parameters, solver continuation, stream
/// positions, pending and active boundary inputs (nightingale-pulse#238), command cursor and fence, history roots,
/// and the last dense segment. An observer's own state is checkpointed beside it, bound to its
/// seal (`ng_observe::ObserverCheckpoint`), so a restore brings back outputs too (nightingale-pulse#273).
pub struct Checkpoint {
    _identity: RunIdentity,
    _branch: BranchAncestry,
    _revision: u64,
    _seal: Seal,
    _time: SimTime,
    _state: Candidate,
    _parameters: BoundParameters,
    _continuation: SolverContinuation,
    _streams: StreamPositions,
    _pending: PendingInputs,
    _commands: CommandCursor,
    _history_roots: Vec<Digest>,
    _dense: DenseSegment,
}
/// The sealed envelope as stored. A host persists and reloads its bytes; only the codec reads them.
pub struct CanonicalCheckpoint {
    _sealed_bytes: Vec<u8>,
}
impl CanonicalCheckpoint {
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
    /// Wraps stored bytes unchecked: `decode` verifies the raw seal and the identity before any
    /// body byte and refuses bytes that do not verify.
    pub fn from_bytes(_sealed_bytes: Vec<u8>) -> Self {
        todo!()
    }
}
/// Only this codec interprets the envelope. Verify identity and raw seal before body decode.
/// Body is pinned postcard; a raw SHA-256 seals stored bytes; the header is RFC 8785 JCS.
pub trait CheckpointCodec {
    fn codec_digest(&self) -> Digest;
    fn encode(&self, checkpoint: &Checkpoint) -> Result<CanonicalCheckpoint, CheckpointError>;
    /// Resumes the branch head; an earlier checkpoint continues only through `rewind`/`recover`.
    /// The envelope holds history roots, not chunks: `history` resolves each root to sealed chunk
    /// bytes, which are verified against the root before the record is rebuilt.
    fn decode<'m>(
        &self,
        bytes: &CanonicalCheckpoint,
        expected: &RunIdentity,
        body: &'m CompiledBody,
        solver: &'m dyn CoupledSolver,
        history: &dyn HistoryStore,
    ) -> Result<Session<'m, Running>, CheckpointError>;
}
pub struct PostcardCheckpointCodec;
impl CheckpointCodec for PostcardCheckpointCodec {
    fn codec_digest(&self) -> Digest {
        todo!()
    }
    fn encode(&self, _checkpoint: &Checkpoint) -> Result<CanonicalCheckpoint, CheckpointError> {
        todo!()
    }
    fn decode<'m>(
        &self,
        _bytes: &CanonicalCheckpoint,
        _expected: &RunIdentity,
        _body: &'m CompiledBody,
        _solver: &'m dyn CoupledSolver,
        _history: &dyn HistoryStore,
    ) -> Result<Session<'m, Running>, CheckpointError> {
        todo!()
    }
}
/// Durable storage for sealed history chunks, addressed by root. A checkpoint is complete only
/// with a store that resolves every root it names.
pub trait HistoryStore {
    fn resolve(&self, root: &Digest) -> Result<Vec<u8>, CheckpointError>;
}
