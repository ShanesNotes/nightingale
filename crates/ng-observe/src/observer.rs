use ng_contract::{AdmissibleSet, Compensated, Digest, ObservationLens, ObservationSeed, SimTime};
use ng_record::{ObservationRefusal, ObservationView};

/// A snapshot an observer took. Its channel in the lens gives its site and method. A reading held by its delay
/// shows twice, `Pending` when collected and with its value once the head reaches `available`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    channel: usize,
    collected: SimTime,
    available: SimTime,
    value: ObservedValue,
}
impl Observation {
    /// Its place in the lens's channels.
    pub fn channel(&self) -> usize {
        self.channel
    }
    /// The instant taken: for a window, its last instant.
    pub fn collected(&self) -> SimTime {
        self.collected
    }
    /// The instant resulted, exact whatever step reached it.
    pub fn available(&self) -> SimTime {
        self.available
    }
    pub fn value(&self) -> ObservedValue {
        self.value
    }
}

/// A value or why there is none, never a zero. Every close gives one, so a lost reading cannot pass for an
/// absent one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObservedValue {
    /// The pair, untouched by rounding. A host's one rounding is its `hi`, the pair's nearest float.
    Present(Compensated),
    /// A window that did not see every instant in it.
    NotCollected,
    /// Collected, and held until `available`.
    Pending,
    /// The arithmetic left the finite range, of amounts or of the clock.
    InvalidSignal,
}

/// A non-finite pair has no bytes, because `Compensated::from_bits` refuses it, so one never stays in a window.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Filling {
    Mean { sum: Compensated, seen: u64 },
    Invalid,
}
impl Filling {
    pub(crate) const EMPTY: Self = Self::Mean {
        sum: Compensated::ZERO,
        seen: 0,
    };
    fn add(self, value: Compensated) -> Self {
        let Self::Mean { sum, seen } = self else {
            return self;
        };
        let sum = sum.plus(value);
        if AdmissibleSet::Finite.holds(sum) {
            Self::Mean { sum, seen: seen + 1 }
        } else {
            Self::Invalid
        }
    }
    fn closed(self, window: u64) -> ObservedValue {
        match self {
            Self::Invalid => ObservedValue::InvalidSignal,
            Self::Mean { sum, seen } if seen == window => {
                ObservedValue::Present(Compensated::between(Compensated::ZERO, sum, 1, window))
            }
            Self::Mean { .. } => ObservedValue::NotCollected,
        }
    }
}

/// A reading its delay holds.
#[derive(Clone, Copy)]
pub(crate) struct Held {
    pub(crate) collected: SimTime,
    pub(crate) available: SimTime,
    pub(crate) value: Compensated,
}

/// One per channel. A channel's delay is constant, so its queue is in `available` order.
pub(crate) struct Reading {
    pub(crate) filling: Filling,
    pub(crate) held: Vec<Held>,
}

/// Holds its own state, so physiology never depends on it. `last` is the seal it last read and `None` only
/// before its first read, which attaches it to whatever revision that view holds.
pub struct Observer<'l> {
    pub(crate) lens: &'l ObservationLens,
    pub(crate) seed: ObservationSeed,
    pub(crate) last: Option<Digest>,
    pub(crate) readings: Vec<Reading>,
}
impl<'l> Observer<'l> {
    pub fn new(lens: &'l ObservationLens, seed: ObservationSeed) -> Self {
        let empty = || Reading {
            filling: Filling::EMPTY,
            held: Vec::new(),
        };
        Self {
            lens,
            seed,
            last: None,
            readings: lens.channels().iter().map(|_| empty()).collect(),
        }
    }
    /// Reads the next revision and returns what it collected, then what it released. A view that does not
    /// follow the last one read is refused and changes nothing.
    pub fn advance(&mut self, view: &ObservationView<'_>) -> Result<Vec<Observation>, ObservationRefusal> {
        if view.lens().digest() != self.lens.digest() {
            return Err(ObservationRefusal::OtherLens);
        }
        if let Some(expected) = self.last.filter(|last| view.previous() != Some(*last)) {
            let found = view.previous();
            return Err(ObservationRefusal::Gap { expected, found });
        }
        let channels = self.lens.channels();
        let mut observed = Vec::new();
        for (at, values) in view.samples() {
            for (channel, value) in values.into_iter().enumerate() {
                let reading = &mut self.readings[channel];
                reading.filling = reading.filling.add(value);
                if channels[channel].closes(at) {
                    observed.push(self.close(channel, at));
                }
            }
        }
        for (channel, reading) in self.readings.iter_mut().enumerate() {
            let due = |held: &&Held| held.available <= view.time();
            let released = reading.held.iter().take_while(due).count();
            observed.extend(reading.held.drain(..released).map(|held| Observation {
                channel,
                collected: held.collected,
                available: held.available,
                value: ObservedValue::Present(held.value),
            }));
        }
        self.last = Some(view.seal());
        Ok(observed)
    }
    /// Closes one window. Only a value waits: an absence is known at once.
    fn close(&mut self, channel: usize, at: SimTime) -> Observation {
        let settings = &self.lens.channels()[channel];
        let reading = &mut self.readings[channel];
        let filled = core::mem::replace(&mut reading.filling, Filling::EMPTY);
        let absent = |value| Observation {
            channel,
            collected: at,
            available: at,
            value,
        };
        let mean = match filled.closed(settings.window()) {
            ObservedValue::Present(mean) => mean,
            absence => return absent(absence),
        };
        let value = mean.plus(settings.error(self.seed, at));
        let (true, Some(available)) = (AdmissibleSet::Finite.holds(value), settings.available(at)) else {
            return absent(ObservedValue::InvalidSignal);
        };
        if available == at {
            return absent(ObservedValue::Present(value));
        }
        reading.held.push(Held {
            collected: at,
            available,
            value,
        });
        Observation {
            available,
            ..absent(ObservedValue::Pending)
        }
    }
}

/// One step's observations chained onto `previous`, so a run's stream is one digest and a gate keeps nothing
/// else.
pub fn chained(previous: Digest, step: &[Observation]) -> Digest {
    if step.is_empty() {
        return previous;
    }
    let mut bytes = previous.as_bytes().to_vec();
    for observation in step {
        let Observation {
            channel,
            collected,
            available,
            value,
        } = observation;
        let (tag, pair) = match value {
            ObservedValue::Present(value) => (0, value.bits()),
            ObservedValue::NotCollected => (1, [0, 0]),
            ObservedValue::Pending => (2, [0, 0]),
            ObservedValue::InvalidSignal => (3, [0, 0]),
        };
        let words = [
            *channel as u64,
            collected.nanoseconds(),
            available.nanoseconds(),
            tag,
            pair[0],
            pair[1],
        ];
        for word in words {
            bytes.extend_from_slice(&word.to_be_bytes());
        }
    }
    Digest::of_canonical(&bytes)
}

#[cfg(test)]
mod tests {
    use super::{Observation, ObservedValue, chained};
    use ng_contract::{Compensated, Digest, SimTime};

    #[test]
    fn a_stream_digest_covers_every_field_of_an_observation() {
        let start = Digest::of_canonical(b"a stream");
        let base = Observation {
            channel: 0,
            collected: SimTime::from_ns(1),
            available: SimTime::from_ns(2),
            value: ObservedValue::Present(Compensated::ZERO),
        };
        assert_eq!(chained(start, &[]), start);
        let digest = chained(start, &[base]);
        let changed = [
            Observation { channel: 1, ..base },
            Observation {
                collected: SimTime::from_ns(3),
                ..base
            },
            Observation {
                available: SimTime::from_ns(3),
                ..base
            },
            Observation {
                value: ObservedValue::Pending,
                ..base
            },
            Observation {
                value: ObservedValue::NotCollected,
                ..base
            },
            Observation {
                value: ObservedValue::InvalidSignal,
                ..base
            },
        ];
        for observation in changed {
            assert_ne!(chained(start, &[observation]), digest, "{observation:?}");
        }
        let pair = |low: f64| Compensated::from_bits([1.0_f64.to_bits(), low.to_bits()]).expect("a pair");
        let present = |value| Observation {
            value: ObservedValue::Present(value),
            ..base
        };
        assert_ne!(
            chained(start, &[present(pair(0.0))]),
            chained(start, &[present(pair(1e-30))]),
            "a pair's low part"
        );
        assert_ne!(chained(digest, &[base]), digest);
    }
}
