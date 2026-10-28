//! The compiled body and the accepted-state shapes it defines. The body is immutable dense arrays
//! indexed once at compilation; parameter VALUES are bound to it separately, by key.
use super::law::Instruction;
use super::*;

/// Brand-erased output of `Assembly::compile`. No organ object survives into it.
pub struct CompiledBody {
    _schema: Schema,
    _program: Vec<Instruction>,
    _layout: Digest,
    _incidence: Vec<(u32, u32)>,
    _parameter_layout: Vec<ParameterKey>,
    _blocks: Vec<EquationBlock>,
    _basis: ConservedBasis,
    _pk: PharmacokineticsRegistry,
    _hash: ModelHash,
}
impl CompiledBody {
    pub fn model_hash(&self) -> ModelHash {
        todo!()
    }
    /// Parses a packet against the compiled parameter layout: every declared key present once,
    /// units converted, domains checked, calibration targets resolved to the packet's own anchors.
    pub fn bind<K: PacketKind>(&self, _packet: ParameterPacket<K>) -> Result<BoundParameters, AdmissionError> {
        todo!()
    }
    /// Compiles an observation catalog against this body into a lens. The lens is versioned and
    /// hashed apart from the model, so a new lens applies to any replay of this body.
    pub fn lens(&self, _catalog: ObservationCatalog) -> Result<ObservationLens, AdmissionError> {
        todo!()
    }
    /// Conserved combinations derived from incidence and balanced reactions (see `ConservedBasis`).
    pub fn conserved_basis(&self) -> &ConservedBasis {
        todo!()
    }
    /// The law a compiled row came from, for diagnosis.
    pub fn origin(&self, _row: RowId) -> LawOrigin {
        todo!()
    }
    /// Every place a trial can be refused for leaving a set: each slot's `Admissible` set and each
    /// operator precondition (a logarithm's positive argument), with its law. A precondition can
    /// still encode a threshold; listing them makes that reviewable, and `NumericalHealth` counts
    /// refusals per source.
    pub fn refusal_sources(&self) -> &[RefusalSource] {
        todo!()
    }
}
pub struct RefusalSource {
    _row: RowId,
    _origin: LawOrigin,
    _kind: RefusalKind,
}
pub enum RefusalKind {
    /// A slot's closed `Admissible` set, or a pool's nonnegative amount.
    Slot,
    /// An operator's own domain, such as a logarithm's positive argument.
    OperatorPrecondition,
}
/// The owning card and the physical process of one compiled row.
pub struct LawOrigin {
    _card: &'static str,
    _process: Digest,
}
/// Packet values in compiled parameter order, with provenance and packet hash. Evaluation reads
/// these values through `CoupledProblem`; the packet kind survives as provenance data.
pub struct BoundParameters {
    _model: Digest,
    _packet: Digest,
    _values: Vec<f64>,
    _provenance: Vec<ValueProvenance>,
}
impl BoundParameters {
    pub fn packet_hash(&self) -> PacketHash {
        todo!()
    }
}

/// Stable row identity in the compiled equation layout.
#[derive(Clone, Copy)]
pub struct RowId {
    _index: u32,
}
impl RowId {
    /// Position in the compiled row order. Read-only: only the compiler makes row ids.
    pub fn index(self) -> usize {
        todo!()
    }
}
/// Stable unknown identity (amount, memory, algebraic or calibrated parameter) in a solve layout.
#[derive(Clone, Copy)]
pub struct UnknownId {
    _index: u32,
}
impl UnknownId {
    /// The slot this unknown occupies in `SolveWorkspace::unknowns()`, so a core can read sparse
    /// columns and scatter a block's solution. Read-only: only the compiler makes unknown ids.
    pub fn index(self) -> usize {
        todo!()
    }
}

/// A full candidate state in one compiled layout: amounts, memory, algebraic values and modes.
pub struct Candidate {
    _layout: Digest,
    /// Base plus paired extents, kept compensated (see `Compensated`).
    _amounts: Vec<Compensated>,
    _memory: Vec<f64>,
    _algebraic: Vec<f64>,
    _modes: Vec<u32>,
}
/// Immutable snapshot of the accepted image; no setter, no organ object.
#[derive(Clone, Copy)]
pub struct AcceptedStateView<'a> {
    _time: SimTime,
    _state: &'a Candidate,
    _continuation: &'a SolverContinuation,
    _dense: &'a DenseSegment,
}
impl<'a> AcceptedStateView<'a> {
    pub fn time(&self) -> SimTime {
        todo!()
    }
    pub fn state(&self) -> &'a Candidate {
        todo!()
    }
    pub fn continuation(&self) -> &'a SolverContinuation {
        todo!()
    }
    /// The last accepted dense segment, so consecutive segments join and observers sample it.
    pub fn last_segment(&self) -> &'a DenseSegment {
        todo!()
    }
}

/// Solver-owned numerical memory, tagged by schema; contains no authoritative physiology.
pub struct MethodHistory {
    _schema: Digest,
    _started_at: SimTime,
    _scalars: Vec<f64>,
    _integer_counters: Vec<u64>,
}
impl MethodHistory {
    /// `started_at` is where this history began; after a breakpoint it must be that breakpoint.
    pub fn new(_schema: Digest, _started_at: SimTime, _scalars: Vec<f64>, _integer_counters: Vec<u64>) -> Self {
        todo!()
    }
    pub fn scalars(&self) -> &[f64] {
        todo!()
    }
    pub fn integer_counters(&self) -> &[u64] {
        todo!()
    }
}
/// Each law stream's event ordinal, kept once, in the accepted image (never in the solver's
/// continuation): a stream's value is a pure function of seed, stream key and ordinal.
pub struct StreamPositions {
    _ordinals: Vec<(Digest, u64)>,
}
/// Everything a solver needs to continue bit for bit; advanced only through accepted images.
pub struct SolverContinuation {
    _schema: Digest,
    _method: MethodHistory,
    _event_cursors: Vec<u64>,
}
impl SolverContinuation {
    pub fn method_history(&self) -> &MethodHistory {
        todo!()
    }
}
