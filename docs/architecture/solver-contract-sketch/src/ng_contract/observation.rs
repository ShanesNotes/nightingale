//! The observation layer: what can be read from the body, how, and with what error. A lens is
//! compiled against one body but versioned and hashed apart from it, so it is never part of a
//! run's identity and never an input to a law. Reading never writes back: a chart entry that
//! documents a drug links to the command that delivered it, and only that command moved the body.
use super::law::Instruction;
use super::{AdmissionError, Digest, EvidenceId, SiteId};

/// One channel to declare: the quantity and site it reads, its transformation, how it errs and
/// who may see it. The quantity names a published output or a state by key, never a layout slot.
pub struct ChannelSpec {
    _key: &'static str,
    _quantity: &'static str,
    _site: Option<SiteId>,
    _transform: Transform,
    _error: Option<ErrorModel>,
    _access: Access,
    _reference: Option<ReferenceInterval>,
}
impl ChannelSpec {
    pub fn new(_key: &'static str, _quantity: &'static str, _site: Option<SiteId>, _access: Access) -> Self {
        todo!()
    }
    pub fn transformed(self, _transform: Transform) -> Self {
        todo!()
    }
    /// Measurement error drawn from the observation seed namespace, keyed by channel and sample
    /// index, so adding or removing a channel never moves a physiological draw.
    pub fn erring(self, _error: ErrorModel) -> Self {
        todo!()
    }
    pub fn flagged(self, _reference: ReferenceInterval) -> Self {
        todo!()
    }
}
/// Averaging window and the delay before a result is available (a lab's turnaround).
pub struct Transform {
    _window_ns: u64,
    _delay_ns: u64,
}
impl Transform {
    pub fn new(_window_ns: u64, _delay_ns: u64) -> Self {
        todo!()
    }
}
pub struct ErrorModel {
    _sd: f64,
    _evidence: Option<EvidenceId>,
}
impl ErrorModel {
    pub fn new(_sd: f64, _evidence: Option<EvidenceId>) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Who reads the channel: a device, an examination, the laboratory, imaging, or a teaching lens
/// that shows what no instrument can.
pub enum Access {
    Device,
    Examination,
    Laboratory,
    Imaging,
    Teaching,
}
/// A clinical reference interval for flags (high, low). An annotation on an observation with its
/// evidence: physiology never reads it, and it is none of the other three ranges (`Admissible`,
/// `ValidityRange`, a `PopulationRecord`).
pub struct ReferenceInterval {
    _lower: f64,
    _upper: f64,
    _evidence: EvidenceId,
}
impl ReferenceInterval {
    pub fn new(_lower: f64, _upper: f64, _evidence: EvidenceId) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The declared channels and output grid of one lens.
pub struct ObservationCatalog {
    _channels: Vec<ChannelSpec>,
    _grid: OutputGrid,
}
impl ObservationCatalog {
    pub fn new(_channels: Vec<ChannelSpec>, _grid: OutputGrid) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The simulation-time grid waveforms are sampled on. It belongs to the lens, not the run: no law
/// reads it and it never cuts a step.
pub struct OutputGrid {
    _period_ns: u64,
}
impl OutputGrid {
    pub fn new(_period_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// A compiled catalog: channel programs over one body's layout, with its own digest.
pub struct ObservationLens {
    _body: Digest,
    _program: Vec<Instruction>,
    _channels: Vec<Channel>,
    _grid: OutputGrid,
    _digest: Digest,
}
impl ObservationLens {
    pub fn digest(&self) -> Digest {
        todo!()
    }
    pub fn channels(&self) -> &[Channel] {
        todo!()
    }
}
/// One compiled channel: a transformation of accepted state and bound parameters, never a stored
/// value and never an input to a law.
pub struct Channel {
    _id: ChannelId,
    _key: Digest,
    _dimension: Digest,
    _access: Access,
}
impl Channel {
    pub fn id(&self) -> ChannelId {
        todo!()
    }
    /// Names the channel stably across runs, bodies and lens versions.
    pub fn key(&self) -> Digest {
        todo!()
    }
    pub fn dimension(&self) -> Digest {
        todo!()
    }
}
/// A channel of one lens; it carries the lens digest, so a read through another lens is refused.
#[derive(Clone, Copy)]
pub struct ChannelId {
    _lens: Digest,
    _index: u32,
}
