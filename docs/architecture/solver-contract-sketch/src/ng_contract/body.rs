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
    /// Compiles channels and a grid against this body into a lens, hashed apart from the model, so a
    /// new lens applies to any replay of this body. Refuses a row the body does not hold, an error in
    /// another dimension, a negative error and two channels that read one row one way.
    pub fn lens(
        &self,
        _channels: Vec<ChannelSpec>,
        _period_ns: core::num::NonZeroU64,
    ) -> Result<ObservationLens, AdmissionError> {
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
    /// Every place a trial can be refused for leaving a set: each row's `Admissible` set and each
    /// operator's domain, with its law. A precondition can still encode a threshold; listing them
    /// makes that reviewable, and `NumericalHealth` counts refusals per source.
    pub fn refusal_sources(&self) -> &[RefusalSource] {
        todo!()
    }
    /// The source that names one row's admissible set.
    pub fn row_source(&self, _row: RowId) -> SourceId {
        todo!()
    }
}
/// A position in `CompiledBody::refusal_sources`.
#[derive(Clone, Copy)]
pub struct SourceId {
    _index: usize,
}
pub enum RefusalSource {
    /// A row's admissible set: an amount is nonnegative, and a slot takes its declared set.
    Row { row: RowId },
    /// A `divide` whose divisor must stay positive, so a step cannot jump a pole unseen.
    Divisor { law: LawOrigin },
    /// A `ln` whose argument must stay positive.
    LogarithmArgument { law: LawOrigin },
    /// An evaluation at a finite point that came back non-finite, such as an `exp` overflow.
    NonFinite { law: LawOrigin },
}
/// An accepted value outside a law's `ValidityRange`: reported on the step, never refused. Abnormal
/// physiology publishes; only an `Admissible` set can refuse a trial.
pub struct ValidityExcursion {
    _law: LawOrigin,
    _at: SimTime,
    _side: ExcursionSide,
    _value: Compensated,
}
pub enum ExcursionSide {
    Below,
    Above,
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

/// Stable column identity in the compiled incidence.
#[derive(Clone, Copy)]
pub struct ColumnId {
    _index: u32,
}
/// What a row holds, by key. A channel names a row this way, never by its slot.
#[derive(Clone, Copy)]
pub enum RowKind {
    Amount { site: SiteId, space: Space },
    Memory { owner: &'static str, local: &'static str },
    Algebraic { owner: &'static str, local: &'static str },
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
    /// The slot this unknown occupies in the solve workspace, so a core can read sparse
    /// columns and scatter a block's solution. Read-only: only the compiler makes unknown ids.
    pub fn index(self) -> usize {
        todo!()
    }
}

/// A full state in one compiled layout: amounts, memory, algebraic values and modes. No public
/// constructor: admission makes the first and `verify` every other.
pub struct Candidate {
    _layout: Digest,
    /// Base plus paired extents, kept compensated (see `Compensated`).
    _amounts: Vec<Compensated>,
    _memory: Vec<f64>,
    _algebraic: Vec<f64>,
    _modes: Vec<u32>,
}
impl Candidate {
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
/// What a problem reads of the accepted image; no setter, no organ object. The record builds one per
/// attempt, and a core reads the head's time, values and continuation here.
#[derive(Clone, Copy)]
pub struct AcceptedStateView<'a> {
    _time: SimTime,
    _state: &'a Candidate,
    _history: &'a MethodHistory,
    _dense: &'a DenseSegment,
}
impl<'a> AcceptedStateView<'a> {
    pub fn new(
        _time: SimTime,
        _state: &'a Candidate,
        _history: &'a MethodHistory,
        _dense: &'a DenseSegment,
    ) -> Self {
        todo!()
    }
    pub fn time(&self) -> SimTime {
        todo!()
    }
    /// The accepted value of one row, compensated for an amount and exact for memory and algebraic
    /// values.
    pub fn value(&self, _row: RowId) -> Compensated {
        todo!()
    }
    pub fn method_history(&self) -> &'a MethodHistory {
        todo!()
    }
    /// The last accepted dense segment, so consecutive segments join and observers sample it.
    pub fn last_segment(&self) -> &'a DenseSegment {
        todo!()
    }
}

/// What a solver core needs to continue exactly, tagged by its schema, advanced only through
/// accepted images. It contains no authoritative physiology. Integers only: a float travels as its
/// `to_bits`.
pub struct MethodHistory {
    _schema: Digest,
    _started_at: SimTime,
    _counters: Vec<u64>,
}
impl MethodHistory {
    /// `started_at` is where this history began; after a breakpoint it must be that breakpoint.
    pub fn new(_schema: Digest, _started_at: SimTime, _counters: Vec<u64>) -> Self {
        todo!()
    }
    pub fn schema(&self) -> Digest {
        todo!()
    }
    pub fn started_at(&self) -> SimTime {
        todo!()
    }
    pub fn counters(&self) -> &[u64] {
        todo!()
    }
}
