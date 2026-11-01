//! Proposed ng-observe: an observer applies one lens to accepted images, one revision at a time.
//! It holds its own state (windows, delayed readings, the seal it last read), so physiology never
//! depends on it, and it is not an input to the stability certificate.
use crate::ng_contract::*;
use crate::ng_record::{ObservationRefusal, ObservationView};
/// A snapshot an observer took. Its channel in the lens gives its method and its row, and an amount's
/// row its site. A reading held by its delay shows twice, `Pending` when collected and with its value
/// once the head reaches `available`.
pub struct Observation {
    _channel: usize,
    _collected: SimTime,
    _available: SimTime,
    _value: ObservedValue,
}
/// A value or why there is none, never a zero. The pair is untouched by rounding: a host's one
/// rounding is its `hi`, the pair's nearest float.
pub enum ObservedValue {
    Present(Compensated),
    NotCollected,
    Pending,
    InvalidSignal,
}
/// An observer's only saved form, stored beside the record checkpoint whose seal it last read.
/// `from_bytes` checks nothing.
pub struct ObserverCheckpoint {
    _bytes: Vec<u8>,
}
impl ObserverCheckpoint {
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }
    pub fn from_bytes(_bytes: Vec<u8>) -> Self {
        todo!()
    }
}
pub struct Observer<'l> {
    _lens: &'l ObservationLens,
    _seed: ObservationSeed,
}
impl<'l> Observer<'l> {
    /// Measurement error draws only from `seed`, keyed by channel and instant; a run seed does not
    /// type-check here, so no observer can move a physiological draw.
    pub fn new(_lens: &'l ObservationLens, _seed: ObservationSeed) -> Self {
        todo!()
    }
    /// Reads the next revision and returns what it collected, then what it released. A view that
    /// does not follow the last one read is refused and changes nothing, so a skip, a repeat and a
    /// revision of another course all fail one comparison. The first read attaches anywhere.
    pub fn advance(&mut self, _view: &ObservationView<'_>) -> Result<Vec<Observation>, ObservationRefusal> {
        todo!()
    }
    pub fn checkpoint(&self) -> ObserverCheckpoint {
        todo!()
    }
    /// Any bytes give an observer or a refusal, never a panic. A checkpoint of another lens or seed
    /// is refused here, and one off this record's course at the next `advance`.
    pub fn resume(
        _lens: &'l ObservationLens,
        _seed: ObservationSeed,
        _saved: &ObserverCheckpoint,
    ) -> Result<Self, ObservationRefusal> {
        todo!()
    }
}
