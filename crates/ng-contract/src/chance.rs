use crate::body::HazardLayout;
use crate::identity::{Canonical, Seal};
use crate::{Candidate, CompiledBody, Digest, Fired, LawName, ModeValue, Seed, SimTime};

/// Admission holds no seed, so a start state is a function of the packet alone: a draw reads its mean and a
/// forcing its factor's.
pub(crate) const HELD_DRAW: f64 = 0.5;
pub(crate) const HELD_FACTOR: f64 = 1.0;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// A stable word from owner and name alone, so adding a declaration moves no other key. `check_names` keeps a
/// stream and a forcing of one owner apart, so a name is one key.
pub(crate) fn key_word(name: LawName) -> u64 {
    let mut out = Canonical::new("ng-contract chance key 1");
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
    /// How far a guard's integral stands past its threshold.
    pub(crate) fn gap(&self, state: &Candidate, hazard: &HazardLayout) -> f64 {
        state.value(hazard.integral).rounded() - self.thresholds[hazard.stream]
    }
    /// The first armed guard past its threshold, in stable order. A landing reads this guard alone, so a guard
    /// that crossed nothing moves no other stream's landing, not even inside the floor.
    pub(crate) fn crossed<'b>(&self, body: &'b CompiledBody, state: &Candidate) -> Option<&'b HazardLayout> {
        (body.layout.hazards.iter())
            .find(|hazard| hazard.armed(&state.modes) && self.gap(state, hazard) >= 0.0)
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
            let crossed = self.gap(&state, hazard) >= 0.0;
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

/// Whether the clock holds this period exactly, so a run reads the period the packet gave.
pub(crate) fn holds_a_period(seconds: f64) -> bool {
    let ns = seconds * NANOSECONDS_PER_SECOND;
    (1.0..18_446_744_073_709_551_616.0).contains(&ns) && ns == ns.round()
}

/// `bind` and `parse` refuse a period the clock cannot hold, and a problem built on other values reaches nothing
/// but must not divide by zero.
fn period_ns(seconds: f64) -> u64 {
    ((seconds * NANOSECONDS_PER_SECOND) as u64).max(1)
}

#[cfg(test)]
mod tests {
    use super::{knot, open_unit, seed_word, spline, word};
    use crate::SimTime;

    const KEYED: u64 = 0x0123_4567_89AB_CDEF;
    const SECOND: u64 = 1_000_000_000;

    // Each from exact rational arithmetic in Python, never from the code under test.
    const WORDS: [(u64, u64, u64); 5] = [
        (0, 0xE32D_D5E6_7F3C_0F09, 0x3FEC_65BA_BCCF_E781),
        (1, 0x09F9_FAF5_939B_AE68, 0x3FA3_F3F5_EB27_3750),
        (2, 0x4FFD_4892_7355_E500, 0x3FD3_FF52_249C_D57A),
        (1_000, 0xF10E_C48F_E4CF_37C2, 0x3FEE_21D8_91FC_99E7),
        (u64::MAX, 0xCCDD_C115_EE93_DCD1, 0x3FE9_9BB8_22BD_D27B),
    ];
    const SPLINE: [(u64, u64); 5] = [
        (0, 0xBFE1_89DF_9207_F54F),
        (250_000_000, 0xBFE4_29BF_0F53_113B),
        (999_999_999, 0xBFE1_3B65_1C6C_AE81),
        (1_000_000_000, 0xBFE1_3B65_1C64_E2BA),
        (7_300_000_000, 0xBFD4_D559_314E_AC24),
    ];

    #[test]
    fn a_word_and_its_draw_match_an_outside_implementation() {
        for (index, expected, draw) in WORDS {
            assert_eq!(word(KEYED, index), expected);
            assert_eq!(open_unit(expected).to_bits(), draw);
        }
    }

    #[test]
    fn a_draw_stays_strictly_inside_the_unit_interval_at_both_ends_of_a_word() {
        assert_eq!(open_unit(0).to_bits(), 0x3CA0_0000_0000_0000);
        assert_eq!(open_unit(u64::MAX).to_bits(), 0x3FEF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn a_spline_value_matches_exact_arithmetic_to_its_rounding() {
        for (at, expected) in SPLINE {
            let value = spline(KEYED, SECOND, SimTime::from_ns(at));
            let apart = value.to_bits().abs_diff(expected);
            assert!(apart <= 8, "at {at}: {apart} units in the last place");
        }
    }

    #[test]
    fn a_spline_is_continuous_across_a_knot() {
        let (before, after) = (
            spline(KEYED, SECOND, SimTime::from_ns(SECOND - 1)),
            spline(KEYED, SECOND, SimTime::from_ns(SECOND)),
        );
        assert!((before - after).abs() < 1e-8, "{before} and {after}");
    }

    #[test]
    fn knots_are_uniform_on_the_open_interval_with_mean_zero() {
        const COUNT: u32 = 200_000;
        let (mut sum, mut square) = (0.0, 0.0);
        for index in 0..COUNT {
            let value = knot(KEYED, u64::from(index));
            assert!(-1.0 < value && value < 1.0);
            sum += value;
            square += value * value;
        }
        let count = f64::from(COUNT);
        let spread = (1.0 / 3.0 / count).sqrt();
        assert!((sum / count).abs() < 4.0 * spread, "mean {}", sum / count);
        assert!(
            (square / count - 1.0 / 3.0).abs() < 0.005,
            "variance {}",
            square / count
        );
    }

    #[test]
    fn equal_words_in_two_domains_never_alias() {
        let words = [1, 2, 3, 4];
        assert_ne!(seed_word("run", words), seed_word("observation", words));
        assert_ne!(seed_word("run", words), seed_word("run", [1, 2, 3, 5]));
    }
}
