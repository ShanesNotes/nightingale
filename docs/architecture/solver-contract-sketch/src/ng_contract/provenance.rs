//! Where every number comes from. `Specified`, `Derived` and `Calibrated` never convert into each
//! other. A packet's kind decides what its anchors are: a `Reviewed` packet holds cited
//! `Specified` anchors; a `SyntheticFixture` holds `Synthetic` ones, which have no citation and no
//! conversion to `Specified`. The packet keeps real values and their provenance; parameters bind to
//! the compiled body by key (`CompiledBody::bind`), so evaluation sees the values.
use super::{AdmissionError, Digest, Measure, PacketHash, PacketSeed, Unit};
use core::marker::PhantomData;

/// Reading status records evidence actually inspected; there is no FromMemory variant.
pub enum Reading {
    FullText,
    SecondaryReproduction { reproduction: String },
}
pub struct Citation {
    _source: String,
    _page_or_table: String,
    _reading: Reading,
}
impl Citation {
    /// Validates nonempty locators, but cannot prove that a person read a source.
    pub fn new(_source: String, _page_or_table: String, _reading: Reading) -> Result<Self, AdmissionError> {
        todo!()
    }
}
#[derive(Clone, Copy)]
pub enum AnchorId {
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    P15,
    P16,
    P17,
    P18,
    P19,
    P20,
    P21,
    P22,
    P23,
    P24,
    P25,
    P26,
}
/// What an anchor's value summarizes. An anchor on a fluctuating output (a pressure that moves
/// with every beat) states its statistic, and its admission row matches it.
pub enum AnchorStatistic {
    Instant,
    CycleMean,
    WindowMean { window_ns: u64 },
}
/// A physiological anchor: value, stated unit and the citation it was read from.
pub struct Specified<U> {
    _anchor: AnchorId,
    _value: Measure<U>,
    _unit: Unit<U>,
    _citation: Citation,
    _statistic: AnchorStatistic,
}
impl<U> Specified<U> {
    /// An instant anchor; `over` states another statistic.
    pub fn new(_anchor: AnchorId, _value: Measure<U>, _unit: Unit<U>, _citation: Citation) -> Self {
        todo!()
    }
    /// What the value summarizes. `bind` refuses an instant anchor on an output whose kernel has
    /// a wave or beat rate class.
    pub fn over(self, _statistic: AnchorStatistic) -> Self {
        todo!()
    }
}
/// An arena stand-in for an anchor: explicitly nonclinical and uncited. No conversion to
/// `Specified`, `Citation` or clinical admission exists.
pub struct Synthetic<U> {
    _anchor: AnchorId,
    _value: Measure<U>,
    _unit: Unit<U>,
    _note: &'static str,
    _statistic: AnchorStatistic,
}
impl<U> Synthetic<U> {
    pub fn new(_anchor: AnchorId, _value: Measure<U>, _unit: Unit<U>, _note: &'static str) -> Self {
        todo!()
    }
    pub fn over(self, _statistic: AnchorStatistic) -> Self {
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
/// A parameter the admission solve fits so the named anchors of the SAME packet hold.
pub struct Calibrated<U> {
    _initial_guess: Measure<U>,
    _targets: Vec<AnchorId>,
}
impl<U> Calibrated<U> {
    /// Refuses an empty target set.
    pub fn new(_initial_guess: Measure<U>, _targets: Vec<AnchorId>) -> Result<Self, AdmissionError> {
        todo!()
    }
}

mod sealed {
    pub trait PacketKind {}
}
/// What a packet admits as an anchor. Sealed: exactly two kinds.
pub trait PacketKind: sealed::PacketKind {
    type Anchor<U>;
}
/// Clinical numbers after source review: anchors carry citations.
pub enum Reviewed {}
/// The arena's nonclinical fixture: anchors are synthetic.
pub enum SyntheticFixture {}
impl sealed::PacketKind for Reviewed {}
impl sealed::PacketKind for SyntheticFixture {}
impl PacketKind for Reviewed {
    type Anchor<U> = Specified<U>;
}
impl PacketKind for SyntheticFixture {
    type Anchor<U> = Synthetic<U>;
}

/// Stable key a fragment declares a parameter under; the packet binds values by it.
pub struct ParameterKey {
    _key: &'static str,
}
impl ParameterKey {
    pub const fn new(_key: &'static str) -> Self {
        Self { _key }
    }
}
/// How one bound value was obtained, as typed data for the admission record.
pub enum ValueProvenance {
    Derived,
    Calibrated(Vec<AnchorId>),
    /// Drawn from a population record when the packet was built, so the patient's draw is part of
    /// the packet hash and never of the run seed.
    Sampled { population: PopulationId, packet_seed: PacketSeed, draw: u64 },
    /// Set by hand for one patient, outside or inside the record's range, with the reason (a
    /// disease overlay, a teaching case).
    Authored { population: PopulationId, reason: &'static str },
    /// Fixture packets only; never a clinical value.
    Fixture,
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
    _rights: Digest,
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
        _rights: Digest,
    ) -> Result<Self, AdmissionError> {
        // Refuses a `Textbook` record whose certainty is above `Moderate`.
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
/// What a record supports. Any tier may support any claim; a `Textbook` record caps its
/// `Certainty` at `Moderate` and names the edition and page.
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
/// textbook, then inference (record 0003). A non-primary record states whether a primary
/// source would upgrade it.
pub enum Tier {
    PrimaryMeasured,
    PrimaryFitted,
    CompiledReference,
    Review,
    Textbook { primary_would_upgrade: bool },
    Standard,
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
/// A parameter's range in a population: its distribution, the evidence behind it, and the
/// correlation group it varies with (body size scales several values together).
pub struct PopulationRecord {
    _parameter: ParameterKey,
    _distribution: Statistic,
    _evidence: Vec<EvidenceId>,
    _correlation_group: Option<&'static str>,
}
impl PopulationRecord {
    pub fn new(
        _parameter: ParameterKey,
        _distribution: Statistic,
        _evidence: Vec<EvidenceId>,
        _correlation_group: Option<&'static str>,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
#[derive(Clone, Copy)]
pub struct PopulationId {
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
/// Every parameter value with its provenance, plus the anchors the admission solve must meet.
/// Its hash is part of the run identity; it is data bound to a model, not part of the model.
pub struct ParameterPacket<K: PacketKind> {
    _anchors: Vec<PacketEntry>,
    _values: Vec<PacketEntry>,
    _evidence: Vec<EvidenceRecord>,
    _populations: Vec<PopulationRecord>,
    _kind: PhantomData<K>,
}
impl<K: PacketKind> ParameterPacket<K> {
    pub fn new() -> Self {
        todo!()
    }
    /// An anchor of this packet's kind; a second value for one anchor is refused.
    pub fn anchor<U>(&mut self, _anchor: K::Anchor<U>) -> Result<(), AdmissionError> {
        todo!()
    }
    /// A value drawn for this patient from one of this packet's population records, with the
    /// packet seed, when the packet is built. The record's distribution and correlation group come
    /// from this packet's manifest; the draw is then inside the packet hash.
    pub fn sampled(&mut self, _key: ParameterKey, _population: PopulationId, _seed: &PacketSeed, _draw: u64)
        -> Result<(), AdmissionError> {
        todo!()
    }
    /// A value set by hand for this patient, inside or outside the record's range, with the reason.
    pub fn authored<U>(&mut self, _key: ParameterKey, _value: Measure<U>, _population: PopulationId,
        _reason: &'static str) -> Result<(), AdmissionError> {
        todo!()
    }
    pub fn derived<U>(&mut self, _key: ParameterKey, _value: Derived<U>) -> Result<(), AdmissionError> {
        todo!()
    }
    pub fn calibrated<U>(&mut self, _key: ParameterKey, _value: Calibrated<U>) -> Result<(), AdmissionError> {
        todo!()
    }
    /// Adds a record to the packet's evidence manifest, which is hashed into the packet.
    pub fn evidence(&mut self, _record: EvidenceRecord) -> EvidenceId {
        todo!()
    }
    pub fn population(&mut self, _record: PopulationRecord) -> Result<PopulationId, AdmissionError> {
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
