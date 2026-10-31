use crate::identity::{Canonical, Seal};
use crate::landing::GuardGap;
use crate::{Candidate, CompiledBody, Digest, Fired, LawName, ModeValue, Seed, SimTime};

/// Admission holds no seed, so a start state is a function of the packet alone: a draw reads its mean and a
/// forcing its factor's.
pub(crate) const HELD_DRAW: f64 = 0.5;
pub(crate) const HELD_FACTOR: f64 = 1.0;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// The kind enters a key, so a stream and a forcing never share one word sequence.
#[derive(Clone, Copy)]
pub(crate) enum ChanceKind {
    Stream,
    Forcing,
}

/// A stable word from owner and name alone, so adding a declaration moves no other key.
pub(crate) fn key_word(kind: ChanceKind, name: LawName) -> u64 {
    let mut out = Canonical::new("ng-contract chance key 1");
    out.number(kind as u64);
    name.seal(&mut out);
    first_word(out.digest())
}

/// The domain enters here, so equal words in two seed domains never alias.
pub(crate) fn seed_word(domain: &str, words: [u64; 4]) -> u64 {
    let mut out = Canonical::new("ng-contract seed word 1");
    out.text(domain);
    for word in words {
        out.number(word);
    }
    first_word(out.digest())
}

fn first_word(digest: Digest) -> u64 {
    let bytes = digest.as_bytes().first_chunk().expect("thirty-two bytes");
    u64::from_be_bytes(*bytes)
}

/// The SplitMix64 finalizer. Measured at about 15 ns for a spline value from four knots, against about
/// 1,400 ns from SHA-256 knots, on a step of about 13 evaluations.
fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn keyed(seed: Seed, key: u64) -> u64 {
    mix(seed.word() ^ key)
}

/// A draw and a knot are the word at one index of one key. It takes no step, attempt or evaluation count, so
/// it cannot depend on one (I8).
fn word(keyed: u64, index: u64) -> u64 {
    mix(keyed ^ mix(index.wrapping_add(0x9E37_79B9_7F4A_7C15)))
}

/// `(2w + 1) / 2^53` for the word's top 52 bits `w`. Every operation is exact, so the value lies in
/// `[2^-53, 1 - 2^-53]`, strictly inside (0, 1), and `u` and `1 - u` are both values, so its mean is one half.
fn open_unit(word: u64) -> f64 {
    ((word >> 12) as f64 + 0.5) / 4_503_599_627_370_496.0
}

fn knot(keyed: u64, index: u64) -> f64 {
    2.0 * open_unit(word(keyed, index)) - 1.0
}

/// The uniform cubic B-spline over the four knots from `at / period`. Its weights sum to one, so it stays in
/// the knots' hull, and it is C2 across a knot, which is why a knot is no breakpoint.
fn spline(keyed: u64, period_ns: u64, at: SimTime) -> f64 {
    let (span, into) = (at.nanoseconds() / period_ns, at.nanoseconds() % period_ns);
    let c = [0, 1, 2, 3].map(|place| knot(keyed, span.wrapping_add(place)));
    let u = into as f64 / period_ns as f64;
    let v = 1.0 - u;
    (v * v * v * c[0]
        + (3.0 * u * u * u - 6.0 * u * u + 4.0) * c[1]
        + (-3.0 * u * u * u + 3.0 * u * u + 3.0 * u + 1.0) * c[2]
        + u * u * u * c[3])
        / 6.0
}

struct Knots {
    keyed: u64,
    period_ns: u64,
    amplitude: f64,
}

/// What one attempt reads of chance, built once per problem from the accepted state. Modes and ordinals move
/// only when an image publishes, so a draw is constant across a ticket, as a drive is.
pub(crate) struct Chance {
    draws: Vec<f64>,
    thresholds: Vec<f64>,
    forcings: Vec<Knots>,
}
impl Chance {
    pub(crate) fn at(body: &CompiledBody, parameters: &[f64], seed: Seed, state: &Candidate) -> Self {
        let draws: Vec<f64> = (body.layout.streams.iter().zip(&state.ordinals))
            .map(|(stream, &ordinal)| open_unit(word(keyed(seed, stream.key), ordinal)))
            .collect();
        let thresholds = (draws.iter())
            .map(|&draw| -crate::pinned::ln(draw).expect("a draw is positive"))
            .collect();
        let forcings = (body.layout.forcings.iter())
            .map(|forcing| Knots {
                keyed: keyed(seed, forcing.key),
                period_ns: period_ns(parameters[forcing.period.index()]),
                amplitude: parameters[forcing.amplitude.index()],
            })
            .collect();
        Self {
            draws,
            thresholds,
            forcings,
        }
    }
    pub(crate) fn draws(&self) -> &[f64] {
        &self.draws
    }
    /// `1 + a s(t)` per forcing at a stage's instant. The contract reads it, so an author has no literal one
    /// to want, and it is exactly 1 at admission.
    pub(crate) fn factors(&self, at: SimTime) -> Vec<f64> {
        (self.forcings.iter())
            .map(|forcing| 1.0 + forcing.amplitude * spline(forcing.keyed, forcing.period_ns, at))
            .collect()
    }
    fn past_threshold(&self, state: &Candidate, integral: crate::RowId, stream: usize) -> f64 {
        state.value(integral).rounded() - self.thresholds[stream]
    }
    /// How far the most-crossed armed guard stands past its threshold.
    pub(crate) fn gap(&self, body: &CompiledBody, state: &Candidate) -> GuardGap {
        let armed = (body.layout.hazards.iter()).filter(|hazard| hazard.armed(&state.modes));
        armed.fold(GuardGap::NONE, |worst, hazard| {
            worst.or(GuardGap::of(self.past_threshold(
                state,
                hazard.integral,
                hazard.stream,
            )))
        })
    }
    /// Applies a crossing to a value no record holds yet, so a refusal or a dropped trial discards every part.
    /// Only a guard armed before any of them fires can fire, so walking in stable order decides a race on one
    /// mode variable: the loser did not happen, and it keeps its integral and its draw.
    pub(crate) fn fired(
        &self,
        body: &CompiledBody,
        mut state: Candidate,
        at: SimTime,
    ) -> (Candidate, Vec<Fired>) {
        let armed: Vec<bool> = (body.layout.hazards.iter())
            .map(|hazard| hazard.armed(&state.modes))
            .collect();
        let first_memory = state.amounts.len();
        let mut fired = Vec::new();
        for (hazard, armed) in body.layout.hazards.iter().zip(armed) {
            let crossed = self.past_threshold(&state, hazard.integral, hazard.stream) >= 0.0;
            if !armed || !hazard.armed(&state.modes) || !crossed {
                continue;
            }
            let mode = &body.layout.modes[hazard.variable];
            state.memory[hazard.integral.index() - first_memory] = 0.0;
            state.modes[hazard.variable] = hazard.enters;
            let ordinal = state.ordinals[hazard.stream];
            state.ordinals[hazard.stream] = ordinal.wrapping_add(1);
            fired.push(Fired {
                stream: body.layout.streams[hazard.stream].name,
                at,
                ordinal,
                threshold_bits: self.thresholds[hazard.stream].to_bits(),
                entered: ModeValue {
                    variable: mode.variable.local,
                    value: mode.values[hazard.enters],
                },
            });
        }
        state.fired = !fired.is_empty();
        (state, fired)
    }
}

/// Finer than the clock is the clock's own nanosecond, and a positive period below it must not divide by zero.
fn period_ns(seconds: f64) -> u64 {
    ((seconds * NANOSECONDS_PER_SECOND).round() as u64).max(1)
}
