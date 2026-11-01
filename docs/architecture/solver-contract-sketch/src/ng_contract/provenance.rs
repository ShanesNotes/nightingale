//! Where every number comes from. A packet's kind decides what backs a value: a `Reviewed` packet holds
//! `Citation`s, and a `SyntheticFixture` holds notes, with no conversion between them. The packet keeps real
//! values and their provenance; parameters bind to the compiled body by key (`CompiledBody::bind`), so
//! evaluation sees the values.
use super::{
    AdmissionError, Compensated, Digest, Dimensionless, ErrorScales, Measure, PacketHash, PacketSeed, RowKind, Unit,
};
use core::marker::PhantomData;
use core::num::{NonZeroU32, NonZeroU64};

/// Reading status records evidence actually inspected; there is no FromMemory variant.
pub enum Reading {
    FullText,
    /// Only the abstract was read; the full text upgrades the record when it turns up.
    Abstract,
    SecondaryReproduction { reproduction: String },
}
/// A source and the page or table in it. It cannot prove anyone read the source.
pub struct Citation {
    _source: &'static str,
    _locator: &'static str,
}
impl Citation {
    pub fn new(_source: &'static str, _locator: &'static str) -> Self {
        todo!()
    }
}
/// One anchor's stable name in its packet, the id its source record carries in `data/`.
#[derive(Clone, Copy)]
pub struct AnchorId {
    _id: &'static str,
}
impl AnchorId {
    pub const fn new(_id: &'static str) -> Self {
        Self { _id }
    }
}
/// What an anchor's value summarizes. A cycle mean names no length: the cycle is the packet's, so a cycle a
/// heart sets for itself moves no anchor.
pub enum AnchorStatistic {
    Instant,
    CycleMean,
}
/// What an anchor states (rule 17): the row it reads, the method that reads it, the window and the value.
pub struct Snapshot<U> {
    _row: RowKind,
    _method: &'static str,
    _statistic: AnchorStatistic,
    _value: Measure<U>,
}
impl<U> Snapshot<U> {
    pub fn new(_row: RowKind, _method: &'static str, _statistic: AnchorStatistic, _value: Measure<U>) -> Self {
        todo!()
    }
}
/// A target a produced quantity must meet at admission. Its trace is what its packet's kind takes.
pub struct Anchor<K: PacketKind, U> {
    _id: AnchorId,
    _snapshot: Snapshot<U>,
    _trace: K::Trace,
}
impl<K: PacketKind, U> Anchor<K, U> {
    pub fn new(_id: AnchorId, _snapshot: Snapshot<U>, _trace: K::Trace) -> Self {
        todo!()
    }
}
/// A cited anchor, the one kind a reviewed packet takes.
pub type Specified<U> = Anchor<Reviewed, U>;
/// A nonclinical anchor with a note where a citation would be.
pub type Synthetic<U> = Anchor<SyntheticFixture, U>;
/// One pass of the periodic goal from the anchor moment, in equal intervals. The packet holds it, because it
/// shapes the start state and the packet's hash is what a run's identity covers.
#[derive(Clone, Copy)]
pub struct Cycle {
    _period_ns: u64,
    _intervals: u32,
}
impl Cycle {
    /// Refuses a period `intervals` does not divide.
    pub fn new(_period_ns: NonZeroU64, _intervals: NonZeroU32) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// A named identity (for example stroke volume from cardiac output and heart rate).
pub struct IdentityLaw<U> {
    _definition: Digest,
    _inputs: Vec<ParameterKey>,
    _unit: PhantomData<U>,
}
impl<U> IdentityLaw<U> {
    pub fn new(_definition: Digest, _inputs: Vec<ParameterKey>) -> Self {
        todo!()
    }
}
pub struct Derived<U> {
    _identity: IdentityLaw<U>,
}
impl<U> Derived<U> {
    pub fn by(_identity: IdentityLaw<U>) -> Self {
        todo!()
    }
}
mod sealed {
    pub trait PacketKind {}
}
/// What a packet admits. Sealed: exactly two kinds. The one difference between them is what backs a value.
pub trait PacketKind: sealed::PacketKind {
    type Trace;
}
/// Clinical numbers after source review: values carry citations.
pub enum Reviewed {}
/// The arena's nonclinical fixture: values carry notes.
pub enum SyntheticFixture {}
impl sealed::PacketKind for Reviewed {}
impl sealed::PacketKind for SyntheticFixture {}
impl PacketKind for Reviewed {
    type Trace = Citation;
}
impl PacketKind for SyntheticFixture {
    type Trace = &'static str;
}

/// Stable key a fragment declares a parameter under; the packet binds values by it.
#[derive(Clone, Copy)]
pub struct ParameterKey {
    _key: &'static str,
}
impl ParameterKey {
    pub const fn new(_key: &'static str) -> Self {
        Self { _key }
    }
}
/// How a value was set, as typed data for the admission record. A fixture's records are synthetic, so every
/// kind can stand in one, and `Fixture` is a synthetic value set by hand.
pub enum ValueProvenance {
    Fixture,
    /// A named identity: one other entry of this packet, scaled. `bind` computes it.
    Derived { identity: &'static str, source: ParameterKey },
    /// Drawn from its key's population record with the packet seed when the packet was built, so the patient's
    /// draw is part of the packet hash and never of the run seed.
    Sampled,
    /// Chosen inside its key's population record, with a reason.
    Authored { reason: &'static str },
    /// Fitted at admission so this anchor holds.
    Calibrated { anchor: AnchorId },
    /// A start value admission moved off the packet's guess.
    Solved,
}
/// Which packet value a record speaks for: a parameter entry, a row's start or a first mode.
pub enum ValueName {
    Parameter { key: ParameterKey },
    Start(RowKind),
    Mode { value: &'static str },
}
/// One value as the run starts from it.
pub struct ValueRecord {
    pub name: ValueName,
    pub provenance: ValueProvenance,
    pub value: Compensated,
}

/// One located claim in an admitted source: the repository's source registry span, how it was
/// read, what it claims and about whom. Packet data; laws never read it.
pub struct EvidenceRecord {
    _span: SourceSpan,
    _reading: Reading,
    _claim: Claim,
    _tier: Tier,
    _statistic: Statistic,
    _population: Population,
    _method: &'static str,
    _certainty: Certainty,
}
impl EvidenceRecord {
    pub fn new(
        _span: SourceSpan,
        _reading: Reading,
        _claim: Claim,
        _tier: Tier,
        _statistic: Statistic,
        _population: Population,
        _method: &'static str,
        _certainty: Certainty,
    ) -> Result<Self, AdmissionError> {
        // Refuses a `Compiled` or `Textbook` record whose certainty is above `Moderate`.
        todo!()
    }
}
/// A span of an admitted source snapshot (the content package's SourceWork, Snapshot and Span).
pub struct SourceSpan {
    _snapshot: Digest,
    _locator: String,
}
impl SourceSpan {
    pub fn new(_snapshot: Digest, _locator: String) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// What a record supports. Any tier may support any claim; a `Compiled` or `Textbook` record
/// caps its `Certainty` at `Moderate`, and a `Textbook` record names the edition and page.
pub enum Claim {
    Magnitude,
    Range,
    Shape,
    Validity,
}
pub enum Statistic {
    MeanSd { mean: f64, sd: f64 },
    Quantiles(Vec<(f64, f64)>),
    Interval { lower: f64, upper: f64, coverage: f64 },
}
/// Who was measured: the description, the count, and the covariates the values depend on.
pub struct Population {
    _description: &'static str,
    _count: Option<u32>,
    _covariates: Vec<&'static str>,
}
impl Population {
    pub fn new(_description: &'static str, _count: Option<u32>, _covariates: Vec<&'static str>) -> Self {
        todo!()
    }
}
/// How strong a source is for one claim: population-specific primary data first, then a
/// compiled source (a reference compilation, a review or a consensus standard), then a
/// textbook, then inference (record 0003). A compiled or textbook record states whether a
/// primary source would upgrade it.
pub enum Tier {
    PrimaryMeasured,
    PrimaryFitted,
    Compiled { primary_would_upgrade: bool },
    Textbook { primary_would_upgrade: bool },
}
pub enum Certainty {
    High,
    Moderate,
    Low,
    VeryLow,
}
/// An evidence record's place in a packet's evidence manifest.
#[derive(Clone, Copy)]
pub struct EvidenceId {
    _index: u32,
}
/// What a validity range (or any non-fixture range) rests on.
pub enum Basis {
    Evidence(EvidenceId),
    Fixture(&'static str),
}
struct PacketEntry {
    _key: &'static str,
    _canonical: f64,
    _unit: &'static str,
    _provenance: ValueProvenance,
}
/// A parameter's range in a stated population: an interval and what backs it.
struct PopulationEntry {
    _key: &'static str,
    _lower: f64,
    _upper: f64,
}
/// Every parameter value with its provenance, the anchors the admission solve must meet, its goal and its tolerance.
/// Its hash is part of the run identity; it is data bound to a model, not part of the model.
pub struct ParameterPacket<K: PacketKind> {
    _cycle: Option<Cycle>,
    _tolerance: Option<ErrorScales>,
    _anchors: Vec<PacketEntry>,
    _values: Vec<PacketEntry>,
    _evidence: Vec<EvidenceRecord>,
    _populations: Vec<PopulationEntry>,
    _kind: PhantomData<K>,
}
impl<K: PacketKind> ParameterPacket<K> {
    /// A packet whose start state stands at one instant.
    pub fn new() -> Self {
        todo!()
    }
    /// A packet whose start state returns after `cycle`: the periodic goal.
    pub fn periodic(_cycle: Cycle) -> Self {
        todo!()
    }
    /// How near every row of the admission system must come. The hash covers it, and a packet that states none is
    /// held exactly.
    pub fn within(&mut self, _tolerance: ErrorScales) {
        todo!()
    }
    /// An anchor of this packet's kind; a second anchor under one id is refused.
    pub fn anchor<U>(&mut self, _anchor: Anchor<K, U>) -> Result<(), AdmissionError> {
        todo!()
    }
    /// `key`'s range in the patient's population.
    pub fn population<U>(&mut self, _key: ParameterKey, _lower: Measure<U>, _upper: Measure<U>, _trace: K::Trace)
        -> Result<(), AdmissionError> {
        todo!()
    }
    /// A value drawn from `key`'s population record with the packet seed, when the packet is built. The draw is
    /// then inside the packet hash.
    pub fn sampled(&mut self, _key: ParameterKey, _seed: PacketSeed) -> Result<(), AdmissionError> {
        todo!()
    }
    /// A value chosen inside `key`'s population record, with the reason. A value outside it is refused.
    pub fn authored<U>(&mut self, _key: ParameterKey, _value: Measure<U>, _reason: &'static str)
        -> Result<(), AdmissionError> {
        todo!()
    }
    /// A value an identity gives: `scale` times the entry at `source`, computed at `bind`. A source the packet does
    /// not hold, or one derived or fitted, is refused.
    pub fn derived(&mut self, _key: ParameterKey, _identity: &'static str, _source: ParameterKey,
        _scale: Measure<Dimensionless>) -> Result<(), AdmissionError> {
        todo!()
    }
    /// A parameter admission fits so `anchor` holds. One fit names one anchor.
    pub fn calibrated<U>(&mut self, _key: ParameterKey, _guess: Measure<U>, _anchor: AnchorId)
        -> Result<(), AdmissionError> {
        todo!()
    }
    /// Adds a record to the packet's evidence manifest, which is hashed into the packet.
    pub fn evidence(&mut self, _record: EvidenceRecord) -> EvidenceId {
        todo!()
    }
    pub fn hash(&self) -> PacketHash {
        todo!()
    }
}
impl ParameterPacket<SyntheticFixture> {
    /// Fixture-only: a dimensioned synthetic design value with no anchor and no citation.
    pub fn synthetic<U>(
        &mut self,
        _key: ParameterKey,
        _value: Measure<U>,
        _unit: Unit<U>,
        _note: &'static str,
    ) -> Result<(), AdmissionError> {
        todo!()
    }
}
