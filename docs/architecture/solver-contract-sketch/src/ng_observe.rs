//! Proposed ng-observe: an observer applies one lens to accepted images, one revision at a time.
//! It holds its own state (windows, delays, the grid cursor), so physiology never depends on it,
//! and it is not an input to the stability certificate.
use crate::ng_contract::*;
use crate::ng_record::{ObservationRefusal, ObservationView};
/// A reading as the chart sees it: when it was collected and when it became available.
pub struct Finding {
    _channel: ChannelId,
    _value: FindingValue,
    _collected: SimTime,
    _available: SimTime,
}
/// Typed absence: "not collected", "pending" and "no valid signal" are values, never zero.
pub enum FindingValue {
    Present(f64),
    NotCollected,
    Pending,
    InvalidSignal,
}
/// Window sums, pending delayed results and the last revision read, for one lens.
pub struct ObserverContinuation {
    _lens: Digest,
    _revision: u64,
    _record_seal: Digest,
    _windows: Vec<f64>,
    _pending: Vec<Finding>,
}
/// An observer's state, checkpointed beside the record checkpoint whose seal it last read.
pub struct ObserverCheckpoint {
    _continuation: ObserverContinuation,
}
pub struct Observer<'l> {
    _lens: &'l ObservationLens,
    _seed: ObservationSeed,
    _continuation: ObserverContinuation,
}
impl<'l> Observer<'l> {
    /// Measurement error draws only from `seed`, keyed by channel and sample index; a run seed
    /// does not type-check here, so no observer can move a physiological draw.
    pub fn new(_lens: &'l ObservationLens, _seed: ObservationSeed) -> Self {
        todo!()
    }
    /// Reads the next revision: every grid instant in its segment for waveform channels, windows
    /// and delays for the rest. A skipped or repeated revision is refused, never guessed.
    pub fn advance(&mut self, _view: &ObservationView<'_>) -> Result<Vec<Finding>, ObservationRefusal> {
        todo!()
    }
    pub fn checkpoint(&self) -> ObserverCheckpoint {
        todo!()
    }
    /// Restores an observer beside a resumed record. A checkpoint from another lens, or one whose
    /// seal is not the resumed record's, is refused.
    pub fn resume(
        _lens: &'l ObservationLens,
        _seed: ObservationSeed,
        _saved: ObserverCheckpoint,
        _view: &ObservationView<'_>,
    ) -> Result<Self, ObservationRefusal> {
        todo!()
    }
}
