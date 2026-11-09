use ng_contract::{AdmissibleSet, Compensated, Digest, Interval, ObservationLens, ObservationSeed, SimTime};
use ng_record::{ObservationRefusal, ObservationView};

/// Its channel in the lens gives its method and its row, and an amount's row its site. A reading held by its
/// delay shows twice, `Pending` when collected and with its value once the head reaches `available`.
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

/// A window filling: a sum of grid samples and their count, or for a time mean twice the chords' integral in
/// value-nanoseconds and the nanoseconds it covers, or a sum that left the finite range.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Filling {
    Mean { sum: Compensated, seen: u64 },
    Integral { twice: Compensated, covered: u64 },
    Invalid,
}
impl Filling {
    pub(crate) const EMPTY: Self = Self::Mean {
        sum: Compensated::ZERO,
        seen: 0,
    };
    pub(crate) const EMPTY_INTEGRAL: Self = Self::Integral {
        twice: Compensated::ZERO,
        covered: 0,
    };
    fn empty(integrated: bool) -> Self {
        if integrated {
            Self::EMPTY_INTEGRAL
        } else {
            Self::EMPTY
        }
    }
    /// One chord's piece from `from` to `to` nanoseconds, its two ends' values given: the trapezoid, exact for a
    /// chord.
    fn integrate(self, from: Compensated, to: Compensated, length: u64) -> Self {
        let Self::Integral { twice, covered } = self else {
            return self;
        };
        let twice = twice.plus(from.plus(to).scaled(length as i64));
        if AdmissibleSet::Finite.holds(twice) {
            Self::Integral {
                twice,
                covered: covered.saturating_add(length),
            }
        } else {
            Self::Invalid
        }
    }
    fn add(self, value: Compensated) -> Self {
        let Self::Mean { sum, seen } = self else {
            return self;
        };
        let sum = sum.plus(value);
        if AdmissibleSet::Finite.holds(sum) {
            Self::Mean {
                sum,
                seen: seen.saturating_add(1),
            }
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
            Self::Integral { twice, covered } if covered == window => {
                ObservedValue::Present(Compensated::between(Compensated::ZERO, twice, 1, 2 * window))
            }
            Self::Integral { .. } => ObservedValue::NotCollected,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Held {
    pub(crate) collected: SimTime,
    pub(crate) available: SimTime,
    pub(crate) value: Compensated,
}

/// A channel's delay is constant, so its queue is in `available` order.
pub(crate) struct Reading {
    pub(crate) filling: Filling,
    pub(crate) held: Vec<Held>,
}

/// Its first read attaches it to whatever revision that view holds.
pub struct Observer<'l> {
    pub(crate) lens: &'l ObservationLens,
    pub(crate) seed: ObservationSeed,
    pub(crate) last: Option<Digest>,
    pub(crate) readings: Vec<Reading>,
}
impl<'l> Observer<'l> {
    pub fn new(lens: &'l ObservationLens, seed: ObservationSeed) -> Self {
        let empty = |channel: &ng_contract::Channel| Reading {
            filling: Filling::empty(channel.integrated()),
            held: Vec::new(),
        };
        Self {
            lens,
            seed,
            last: None,
            readings: lens.channels().iter().map(empty).collect(),
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
        if let Some((step, chords)) = view.chords() {
            for (channel, (start, end)) in chords.into_iter().enumerate() {
                if channels[channel].integrated() {
                    observed.extend(self.integrate(channel, step, start, end));
                }
            }
        }
        for (at, values) in view.samples() {
            for (channel, value) in values.into_iter().enumerate() {
                if channels[channel].integrated() {
                    continue;
                }
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
    /// One accepted step's chord into a time-mean channel: each piece inside one window adds its integral, and a piece
    /// that reaches a window's end closes it there.
    fn integrate(
        &mut self,
        channel: usize,
        step: Interval,
        start: Compensated,
        end: Compensated,
    ) -> Vec<Observation> {
        let window = self.lens.channels()[channel].window();
        let (from, to) = (step.start().nanoseconds(), step.end().nanoseconds());
        let at = |ns: u64| Compensated::between(start, end, ns - from, to - from);
        let mut closed = Vec::new();
        let mut here = from;
        while here < to {
            let boundary = (here / window).saturating_add(1).saturating_mul(window);
            let next = if boundary < to { boundary } else { to };
            let reading = &mut self.readings[channel];
            reading.filling = reading.filling.integrate(at(here), at(next), next - here);
            if next == boundary {
                closed.push(self.close(channel, SimTime::from_ns(next)));
            }
            here = next;
        }
        closed
    }
    /// Only a value waits: an absence is known at once.
    fn close(&mut self, channel: usize, at: SimTime) -> Observation {
        let settings = &self.lens.channels()[channel];
        let reading = &mut self.readings[channel];
        let filled = core::mem::replace(&mut reading.filling, Filling::empty(settings.integrated()));
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
