//! The observation layer: what can be read from the body, how, and with what error. A lens is
//! compiled against one body but hashed apart from it, so it is never part of a run's identity and
//! never an input to a law. Reading never writes back: a chart entry that documents a drug links to
//! the command that delivered it, and only that command moved the body.
use core::num::NonZeroU64;

use super::{Compensated, Digest, Measure, ModelHash, ObservationSeed, RowId, RowKind, SimTime};

/// One channel to compile: how it is taken and the row it reads, then a window of whole grid
/// periods, a delay and an error. With none of those it reads every grid instant, as it is, at once.
pub struct ChannelSpec {
    _method: &'static str,
    _row: RowKind,
    _window: NonZeroU64,
    _delay_ns: u64,
    _sd: Option<Compensated>,
}
impl ChannelSpec {
    pub fn new(_method: &'static str, _row: RowKind) -> Self {
        todo!()
    }
    /// One reading per `samples` grid instants, their mean. A count, so a window always fits the grid.
    pub fn averaged(self, _samples: NonZeroU64) -> Self {
        todo!()
    }
    /// Each value is held this long past its collection.
    pub fn delayed(self, _delay_ns: u64) -> Self {
        todo!()
    }
    /// The standard deviation of a reading, in its row's dimension. Measurement error draws from the
    /// observation seed, keyed by channel and instant, so adding a channel moves no draw.
    pub fn erring<U>(self, _sd: Measure<U>) -> Self {
        todo!()
    }
}
/// A compiled set of channels and a grid, hashed apart from the body. No run reads it.
pub struct ObservationLens {
    _model: ModelHash,
    _period_ns: u64,
    _channels: Vec<Channel>,
    _digest: Digest,
}
impl ObservationLens {
    pub fn digest(&self) -> Digest {
        todo!()
    }
    pub fn model(&self) -> ModelHash {
        todo!()
    }
    pub fn channels(&self) -> &[Channel] {
        todo!()
    }
    /// The output grid: every whole multiple of this. It belongs to the lens, so it never cuts a step.
    pub fn period_ns(&self) -> u64 {
        todo!()
    }
    /// What an observer's checkpoint is bound to: this lens with this seed.
    pub fn observer_digest(&self, _seed: ObservationSeed) -> Digest {
        todo!()
    }
}
/// One compiled channel: a row of accepted state, never a stored value and never an input to a law.
pub struct Channel {
    _method: &'static str,
    _row: RowId,
    _window: u64,
    _period_ns: u64,
    _delay_ns: u64,
    _sd: Option<Compensated>,
    _key: u64,
}
impl Channel {
    pub fn method(&self) -> &'static str {
        todo!()
    }
    pub fn row(&self) -> RowId {
        todo!()
    }
    pub fn window(&self) -> u64 {
        todo!()
    }
    /// Whether a window closes at this grid instant, a function of the instant alone.
    pub fn closes(&self, _at: SimTime) -> bool {
        todo!()
    }
    /// When a reading collected at `collected` results; `None` past the clock's end.
    pub fn available(&self, _collected: SimTime) -> Option<SimTime> {
        todo!()
    }
    /// This reading's error, zero for a channel with none. It reads no state, so it moves none.
    pub fn error(&self, _seed: ObservationSeed, _at: SimTime) -> Compensated {
        todo!()
    }
}
