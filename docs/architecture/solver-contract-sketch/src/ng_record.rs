//! Proposed ng-record: the sole accepted-state writer, its session latch, one bound history and
//! the checkpoint envelope.
//!
//! A step is validate, then publish. `PatientRecord::validate(&self, ..)` is immutable and returns
//! the ENTIRE next image or a failure; `publish(&mut self, ..)` installs that image and cannot fail.
//! `&mut PatientRecord` is never lent outside this module, so only the session publishes. A session
//! that met a refusal becomes `Session<Retrying>`, which can only retry, and one that met an
//! `EngineFault` becomes `Session<Faulted>`, which has no step.
use crate::ng_contract::*;

/// The chained seal of one accepted revision: the previous seal, the image's digests and the ledger
/// chunk.
#[derive(Clone, Copy)]
pub struct Seal {
    _digest: Digest,
}
impl Seal {
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
/// Each law stream's event ordinal, kept once, in the accepted image (never in the solver's
/// continuation): a stream's value is a pure function of seed, stream key and ordinal.
struct StreamPositions {
    _ordinals: Vec<(Digest, u64)>,
}

/// No Clone, no public constructor and no setter. One body state per branch.
pub struct PatientRecord<'m> {
    _body: &'m CompiledBody,
    _identity: RunIdentity,
    _parameters: BoundParameters,
    _branch: BranchAncestry,
    _revision: u64,
    _seal: Seal,
    _time: SimTime,
    _state: Candidate,
    _continuation: MethodHistory,
    _dense: DenseSegment,
    _streams: StreamPositions,
    _pending: PendingInputs,
    _commands: CommandCursor,
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
    /// The contract answers whether the core followed its declared method (`StepProposal::verify`,
    /// on this record's own problem); this answers whether the result may become the next accepted
    /// image. `expected` is the ticket the session issued, so a proposal for any other attempt, or
    /// an issued ticket this record would not issue, is `BindingMismatch`. Then it checks admissible
    /// sets and builds the whole next image, with any validity excursions recorded rather than
    /// refused. Never mutates. A refusal is retryable; an `EngineFault` is not, and the session
    /// latches on it.
    pub fn validate(&self, _expected: &StepTicket, _proposal: StepProposal) -> Result<NextImage, TrialFailure> {
        todo!()
    }
    /// Same id and payload returns the prior receipt; a changed payload is refused; a new input
    /// becomes a next image that only extends the pending timeline. An input stamped before the
    /// accepted head is refused: the host stamps a click at the head or later, never in the past.
    pub fn admit_input(&self, _input: AuthorizedInput) -> Result<InputAdmission, AdmissionError> {
        todo!()
    }
    /// The only `&mut` path. Installs every part of the image together; cannot fail.
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
    pub pending: Digest,
    pub ledger: Digest,
}

/// The entire next accepted image, validated and not yet visible.
pub struct NextImage {
    _revision: u64,
    _time: SimTime,
    _state: Candidate,
    _fired: ModeChanges,
    _continuation: MethodHistory,
    _ledger: LedgerChunk,
    _inputs: CommandCursor,
    _pending: PendingInputs,
    _dense: DenseSegment,
    _streams: StreamPositions,
    _seal: Seal,
    _applied: Vec<InputId>,
    _excursions: Vec<ValidityExcursion>,
}
pub enum InputAdmission {
    AlreadyPresent(InputReceipt),
    Staged(NextImage),
}
/// What one published step did: its revision and seal, each input it applied, and any excursion.
pub struct StepReceipt {
    _revision: u64,
    _seal: Seal,
    _applied: Vec<InputId>,
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
/// Fixed-time sealed chunks with prefix extents; a seal never rehashes all history per beat.
struct History {
    _basis: Digest,
    _chunks: Vec<LedgerChunk>,
    _roots: Vec<Digest>,
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
    pub fn enqueue(&mut self, _input: AuthorizedInput) -> Result<InputReceipt, AdmissionError> {
        todo!()
    }
    /// `next_step`, `solve`, `validate`, then `publish`. A fault from any of them latches.
    pub fn step(self) -> Step<'m> {
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
    /// The fault itself: kind, time, attempted interval and the refusal that ended it.
    _fault: EngineFault,
    /// The refusals that preceded it, each naming its row or source.
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
    _continuation: MethodHistory,
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
