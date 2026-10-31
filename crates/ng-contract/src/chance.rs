use crate::body::HazardLayout;
use crate::identity::{Canonical, Seal};
use crate::{
    Candidate, CompiledBody, Digest, DomainSeed, Fired, LawName, ModeValue, ObservationSeed, Seed, SimTime,
};

/// Admission holds no seed, so a start state is a function of the packet alone: a draw reads its mean and a
/// forcing its factor's.
pub(crate) const HELD_DRAW: f64 = 0.5;
pub(crate) const HELD_FACTOR: f64 = 1.0;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// `check_names` keeps a stream and a forcing of one owner apart, so a name is one key.
pub(crate) fn key_word(name: LawName) -> u64 {
    let mut out = Canonical::new("ng-contract chance key 1");
    name.seal(&mut out);
    first_word(out.digest())
}

pub(crate) fn seed_word(domain: &str, words: [u64; 4]) -> u64 {
    let mut out = Canonical::new("ng-contract seed word 1");
    out.text(domain);
    for word in words {
        out.number(word);
    }
    first_word(out.digest())
}

pub(crate) fn first_word(digest: Digest) -> u64 {
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

fn keyed<D: Copy>(seed: DomainSeed<D>, key: u64) -> u64 {
    mix(seed.word() ^ key)
}

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

/// Twelve uniform words less six: mean zero and variance one, inside (-6, 6), from sums alone. A pure function
/// of seed, channel and instant, so an observer keeps no cursor.
pub(crate) fn measurement_error(seed: ObservationSeed, channel: u64, at: SimTime) -> f64 {
    let instant = word(keyed(seed, channel), at.nanoseconds());
    let sum: f64 = (0..12).map(|index| open_unit(word(instant, index))).sum();
    sum - 6.0
}

struct Knots {
    keyed: u64,
    period: usize,
    amplitude: usize,
}

/// Modes and ordinals move only when an image publishes, so a draw is constant across a ticket, as a
/// drive is.
pub(crate) struct Chance {
    draws: Vec<f64>,
    thresholds: Vec<f64>,
    forcings: Vec<Knots>,
}
impl Chance {
    pub(crate) fn at(body: &CompiledBody, seed: Seed, state: &Candidate) -> Self {
        let draws: Vec<f64> = (body.layout.streams.iter().zip(&state.ordinals))
            .map(|(stream, &ordinal)| open_unit(word(keyed(seed, stream.key), ordinal)))
            .collect();
        let thresholds = (draws.iter())
            .map(|&draw| -crate::pinned::ln(draw).expect("a draw is positive"))
            .collect();
        let forcings = (body.layout.forcings.iter())
            .map(|forcing| Knots {
                keyed: keyed(seed, forcing.key),
                period: forcing.period.index(),
                amplitude: forcing.amplitude.index(),
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
    pub(crate) fn factors(&self, parameters: &[f64], at: SimTime) -> Vec<f64> {
        (self.forcings.iter())
            .map(|forcing| {
                let period_ns = period_ns(parameters[forcing.period]);
                1.0 + parameters[forcing.amplitude] * spline(forcing.keyed, period_ns, at)
            })
            .collect()
    }
    pub(crate) fn gap(&self, state: &Candidate, hazard: &HazardLayout) -> f64 {
        state.value(hazard.integral).rounded() - self.thresholds[hazard.stream]
    }
    /// A landing's gaps read only the guard this returns, so a guard that crossed nothing moves no other
    /// stream's landing, not even inside the floor.
    pub(crate) fn crossed<'b>(&self, body: &'b CompiledBody, state: &Candidate) -> Option<&'b HazardLayout> {
        (body.layout.hazards.iter())
            .find(|hazard| hazard.armed(&state.modes) && self.gap(state, hazard) >= 0.0)
    }
    /// Only a guard armed before any guard fires can fire, so walking in stable order decides a race on one
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
            state.fired = true;
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
        (state, fired)
    }
}

/// Whether a period is a count of the clock's nanoseconds, so a run reads the period the packet gave. The count
/// is read back because the product is inexact: 1.001 s multiplies to an ulp past 1,001,000,000.
pub(crate) fn holds_a_period(seconds: f64) -> bool {
    let ns = period_ns(seconds);
    ns > 0 && ns as f64 / NANOSECONDS_PER_SECOND == seconds
}

fn period_ns(seconds: f64) -> u64 {
    (seconds * NANOSECONDS_PER_SECOND).round() as u64
}

#[cfg(test)]
mod tests {
    use super::{holds_a_period, knot, measurement_error, open_unit, period_ns, seed_word, spline, word};
    use crate::{ObservationSeed, PacketSeed, Seed, SimTime};

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

    const ERRORS: [(u64, u64); 5] = [
        (0, 0x3FE7_FC4F_3669_13C8),
        (1, 0xBFE1_52C2_35F0_CE60),
        (4_000_000, 0xBFD7_C504_0505_C6D0),
        (7_300_000_000, 0x3FE3_9AE4_12A1_D518),
        (u64::MAX, 0x3FFB_E9F3_0202_DB48),
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

    #[test]
    fn a_measurement_error_matches_an_outside_implementation() {
        let seed = ObservationSeed::new([1, 2, 3, 4]);
        for (at, expected) in ERRORS {
            let error = measurement_error(seed, KEYED, SimTime::from_ns(at));
            assert_eq!(error.to_bits(), expected, "at {at}");
        }
    }

    #[test]
    fn measurement_errors_have_mean_zero_and_variance_one() {
        const COUNT: u32 = 200_000;
        let seed = ObservationSeed::new([1, 2, 3, 4]);
        let (mut sum, mut square) = (0.0, 0.0);
        for at in 0..COUNT {
            let error = measurement_error(seed, KEYED, SimTime::from_ns(u64::from(at)));
            assert!(-6.0 < error && error < 6.0);
            sum += error;
            square += error * error;
        }
        let count = f64::from(COUNT);
        assert!((sum / count).abs() < 4.0 / count.sqrt(), "mean {}", sum / count);
        assert!((square / count - 1.0).abs() < 0.02, "variance {}", square / count);
    }

    #[test]
    fn the_run_domain_keeps_its_word_and_each_other_domain_has_its_own() {
        let words = [1, 2, 3, 4];
        let (run, packet, observation) = (
            Seed::new(words).word(),
            PacketSeed::new(words).word(),
            ObservationSeed::new(words).word(),
        );
        assert_eq!(run, seed_word("run", words));
        assert_eq!(packet, seed_word("packet", words));
        assert_eq!(observation, seed_word("observation", words));
        assert!(run != packet && run != observation && packet != observation);
    }

    #[test]
    fn a_period_written_in_whole_nanoseconds_is_held_exactly() {
        for per_second in [1e3, 1e6, 1e9] {
            for count in 1..100_000_u64 {
                let seconds = count as f64 / per_second;
                assert!(holds_a_period(seconds), "{seconds}");
                assert_eq!(period_ns(seconds), (seconds * 1e9).round() as u64);
            }
        }
        let mut count = 1_u64;
        for _ in 0..1_000_000 {
            count = word(count, count) >> 13;
            let seconds = count as f64 / 1e9;
            assert!(holds_a_period(seconds), "{count}");
            assert_eq!(period_ns(seconds), count);
        }
    }

    #[test]
    fn a_period_the_clock_cannot_hold_is_not_held() {
        for seconds in [1.5e-9, 1e-12, 1e100, 0.0, -1.0, 0.1 + 0.2] {
            assert!(!holds_a_period(seconds), "{seconds}");
        }
    }
}
