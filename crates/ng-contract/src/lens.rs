use crate::chance::{first_word, measurement_error};
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::{
    AdmissionError, Compensated, CompiledBody, Digest, Dimension, Dimensionless, Measure, ModelHash,
    ObservationSeed, RowId, RowKind, SimTime,
};
use core::num::NonZeroU64;

/// What one reading spans: a count of grid instants whose samples it averages, or a duration in nanoseconds over which
/// it integrates the accepted curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Window {
    Samples(NonZeroU64),
    Time(NonZeroU64),
}

/// With no window, delay or error it reads every grid instant, as it is, at once.
pub struct ChannelSpec {
    method: &'static str,
    row: RowKind,
    window: Window,
    delay_ns: u64,
    error: Option<(String, Compensated)>,
}
impl ChannelSpec {
    pub fn new(method: &'static str, row: RowKind) -> Self {
        Self {
            method,
            row,
            window: Window::Samples(NonZeroU64::MIN),
            delay_ns: 0,
            error: None,
        }
    }
    /// One reading per `window_ns`, the row's integral over the window divided by its length: the accepted curve's
    /// time mean, not a mean of samples. Windows sit on whole multiples of their length.
    pub fn time_averaged(self, window_ns: NonZeroU64) -> Self {
        Self {
            window: Window::Time(window_ns),
            ..self
        }
    }
    /// One reading per `samples` grid instants, their mean. A count, so a window always fits the grid.
    pub fn averaged(self, samples: NonZeroU64) -> Self {
        Self {
            window: Window::Samples(samples),
            ..self
        }
    }
    /// Each value is held this long past its collection.
    pub fn delayed(self, delay_ns: u64) -> Self {
        Self { delay_ns, ..self }
    }
    /// The standard deviation of a reading, in its row's dimension.
    pub fn erring<U: Dimension>(self, sd: Measure<U>) -> Self {
        Self {
            error: Some((dimension::<U>(), Compensated::of(&sd))),
            ..self
        }
    }
}

/// Its key hashes what it reads and how, so adding a channel moves no other's draws.
pub struct Channel {
    method: &'static str,
    row: RowId,
    window: Window,
    period_ns: u64,
    delay_ns: u64,
    sd: Option<Compensated>,
    key: u64,
}
impl Channel {
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn row(&self) -> RowId {
        self.row
    }
    /// How many grid instants one reading holds, or for a time mean how many nanoseconds.
    pub fn window(&self) -> u64 {
        match self.window {
            Window::Samples(count) | Window::Time(count) => count.get(),
        }
    }
    pub fn integrated(&self) -> bool {
        matches!(self.window, Window::Time(_))
    }
    /// How long one reading's window lasts, whichever kind it is.
    pub fn window_ns(&self) -> u64 {
        match self.window {
            Window::Samples(count) => count.get().saturating_mul(self.period_ns),
            Window::Time(ns) => ns.get(),
        }
    }
    /// Whether a window closes at this grid instant. Windows sit on whole multiples of their length, so an
    /// observer keeps no cursor.
    pub fn closes(&self, at: SimTime) -> bool {
        (at.nanoseconds() / self.period_ns).is_multiple_of(self.window())
    }
    /// When a reading collected at `collected` results; `None` past the clock's end.
    pub fn available(&self, collected: SimTime) -> Option<SimTime> {
        collected.plus_ns(self.delay_ns)
    }
    /// This reading's error, zero for a channel with none.
    pub fn error(&self, seed: ObservationSeed, at: SimTime) -> Compensated {
        let Some(sd) = self.sd else {
            return Compensated::ZERO;
        };
        let draw = measurement_error(seed, self.key, at);
        sd.times(&Measure::<Dimensionless>::new(draw).expect("twelve draws stay finite"))
    }
}

pub struct ObservationLens {
    model: ModelHash,
    period_ns: u64,
    channels: Vec<Channel>,
    digest: Digest,
}
impl ObservationLens {
    pub fn digest(&self) -> Digest {
        self.digest
    }
    pub fn model(&self) -> ModelHash {
        self.model
    }
    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }
    /// The output grid: every whole multiple of this. It belongs to the lens, so it never cuts a step.
    pub fn period_ns(&self) -> u64 {
        self.period_ns
    }
    /// Whether a channel reads `row` by `method`, one reading per `window_ns`, each the mean of exactly `points`
    /// grid instants: a finer grid reads a mean admission did not match.
    pub(crate) fn reads(&self, method: &str, row: RowId, window_ns: u64, points: u64) -> bool {
        (self.channels.iter()).any(|channel| {
            channel.method == method
                && channel.row == row
                && matches!(channel.window, Window::Samples(count)
                    if count.get() == points && count.get().checked_mul(self.period_ns) == Some(window_ns))
        })
    }
    pub(crate) fn reads_time_mean(&self, method: &str, row: RowId, window_ns: u64) -> bool {
        (self.channels.iter()).any(|channel| {
            channel.method == method
                && channel.row == row
                && matches!(channel.window, Window::Time(ns) if ns.get() == window_ns)
        })
    }
    /// What an observer's checkpoint is bound to: this lens with this seed.
    pub fn observer_digest(&self, seed: ObservationSeed) -> Digest {
        let mut out = Canonical::new("ng-contract observer 1");
        self.digest.seal(&mut out);
        out.number(seed.word());
        out.digest()
    }
}

impl CompiledBody {
    /// A lens applies to any replay, because no bit of a run depends on it: admission reads one only to
    /// refuse.
    pub fn lens(
        &self,
        channels: Vec<ChannelSpec>,
        period_ns: NonZeroU64,
    ) -> Result<ObservationLens, AdmissionError> {
        let period_ns = period_ns.get();
        let mut out = Canonical::new("ng-contract lens 1");
        self.model_hash().digest().seal(&mut out);
        out.number(period_ns);
        let mut compiled: Vec<Channel> = Vec::with_capacity(channels.len());
        for channel in channels {
            let ChannelSpec {
                method,
                row: kind,
                window,
                delay_ns,
                error,
            } = channel;
            let row = (self.rows().iter())
                .find(|row| row.kind() == kind)
                .ok_or(AdmissionError::UnknownChannel { row: kind })?;
            let sd = match error {
                Some((unit, _)) if unit != row.dimension => {
                    return Err(AdmissionError::ChannelDimension { row: kind });
                }
                Some((_, sd)) if !Compensated::ZERO.at_most(sd) => {
                    return Err(AdmissionError::NegativeChannelError { row: kind });
                }
                Some((_, sd)) => Some(sd),
                None => None,
            };
            // A sampled channel keeps the bytes it had before time means, so no existing lens moves.
            let (version, length) = match window {
                Window::Samples(count) => ("ng-contract channel 1", count),
                Window::Time(ns) => ("ng-contract channel 2", ns),
            };
            let mut reading = Canonical::new(version);
            method.seal(&mut reading);
            kind.seal(&mut reading);
            reading.number(length.get());
            reading.number(delay_ns);
            sd.seal(&mut reading);
            if let Window::Time(_) = window {
                reading.number(1);
            }
            let digest = reading.digest();
            let key = first_word(digest);
            if compiled.iter().any(|known| known.key == key) {
                return Err(AdmissionError::DuplicateChannel { row: kind });
            }
            digest.seal(&mut out);
            compiled.push(Channel {
                method,
                row: row.id(),
                window,
                period_ns,
                delay_ns,
                sd,
                key,
            });
        }
        Ok(ObservationLens {
            model: self.model_hash(),
            period_ns,
            channels: compiled,
            digest: out.digest(),
        })
    }
}
