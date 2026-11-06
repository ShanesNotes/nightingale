//! The certificate's walk in time order over a history collected whole, kept as the oracle the newest-first
//! walk must agree with on every field.
use ng_contract::*;
use ng_record::{ConservationView, Curve, HistorySegment};
use ng_validate::*;

const DAY_NS: u64 = 86_400_000_000_000;
const COVERAGE_NS: u64 = 2 * DAY_NS;

/// One water policy for every combination of a basis, with the numbers the oracle reads.
pub struct Ledger {
    policy: CertificatePolicy,
    basis: Digest,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    drift: Compensated,
}
impl Ledger {
    pub fn new(basis: &ConservedBasis, absolute: f64, relative: f64, drift: f64) -> Self {
        let water = |value| Measure::<Water>::new(value).expect("finite");
        let ratio = || Measure::<Dimensionless>::new(relative).expect("finite");
        let quantities = (basis.ids())
            .map(|quantity| QuantityPolicy::new(quantity, water(absolute), ratio(), water(drift)))
            .collect::<Result<Vec<_>, _>>()
            .expect("a policy of water");
        let windows = Sliding24Hours::covering(COVERAGE_NS).expect("a coverage");
        Self {
            policy: CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
                .expect("a policy"),
            basis: basis.digest(),
            absolute: Compensated::of(&water(absolute)),
            relative: ratio(),
            drift: Compensated::of(&water(drift)),
        }
    }
    pub fn digest(&self) -> Digest {
        self.policy.digest()
    }
}

#[derive(Debug, PartialEq)]
struct Worst {
    quantity: QuantityId,
    closure: [u64; 2],
    closure_window: Interval,
    drift: [u64; 2],
    drift_window: Interval,
}

#[derive(Debug, PartialEq)]
struct Failed {
    check: CertificateCheck,
    quantity: Option<QuantityId>,
    window: Option<Interval>,
    residual: [u64; 2],
}

/// `certify`, after asserting that the walk in time order gives the same certificate or the same failure.
pub fn certified(view: &ConservationView<'_>, ledger: &Ledger) -> Result<Certificate, CertificateFailure> {
    let walked = certify(view, &ledger.policy);
    let seen = walked.as_ref().map(|certificate| {
        assert_eq!(certificate.seal(), view.seal());
        assert_eq!(certificate.basis(), ledger.basis);
        assert_eq!(certificate.policy(), ledger.digest());
        let worst = (certificate.results().iter()).map(|result| Worst {
            quantity: result.quantity(),
            closure: result.worst_closure().bits(),
            closure_window: result.closure_window(),
            drift: result.worst_drift().bits(),
            drift_window: result.drift_window(),
        });
        (worst.collect(), certificate.pieces())
    });
    let seen = seen.map_err(|failure| Failed {
        check: failure.check(),
        quantity: failure.quantity(),
        window: failure.window(),
        residual: failure.residual().bits(),
    });
    assert_eq!(
        seen,
        in_time_order(view, ledger),
        "newest first, then in time order"
    );
    walked
}

fn failed(check: CertificateCheck) -> Failed {
    Failed {
        check,
        quantity: None,
        window: None,
        residual: Compensated::ZERO.bits(),
    }
}

fn window_starts(segments: &[HistorySegment], first: u64, last: u64) -> Vec<u64> {
    let mut boundaries = vec![first];
    boundaries.extend(
        segments
            .iter()
            .map(|segment| segment.interval().end().nanoseconds()),
    );
    let (mut direct, mut shifted) = (0, 0);
    let mut starts: Vec<u64> = Vec::with_capacity(boundaries.len());
    loop {
        let plain = boundaries.get(direct).copied();
        let moved = (boundaries.get(shifted)).map(|boundary| boundary.saturating_sub(DAY_NS));
        let next = match (plain, moved) {
            (Some(plain), Some(moved)) if plain <= moved => {
                direct += 1;
                plain
            }
            (_, Some(moved)) => {
                shifted += 1;
                moved
            }
            (Some(plain), None) => {
                direct += 1;
                plain
            }
            (None, None) => break,
        };
        if (first..=last).contains(&next) && starts.last() != Some(&next) {
            starts.push(next);
        }
    }
    starts
}

fn differences(
    later: &Curve<'_>,
    late: Interval,
    earlier: &Curve<'_>,
    early: Interval,
) -> Option<Vec<Compensated>> {
    let (later, earlier) = (later.on(late)?, earlier.on(early)?);
    Some(
        later
            .iter()
            .zip(&earlier)
            .map(|(later, earlier)| later.minus(*earlier))
            .collect(),
    )
}

fn in_time_order(view: &ConservationView<'_>, ledger: &Ledger) -> Result<(Vec<Worst>, u64), Failed> {
    let coverage = || failed(CertificateCheck::Coverage);
    if ledger.basis != view.basis().digest() {
        return Err(coverage());
    }
    let span = view.span().ok_or_else(coverage)?;
    if span.length_ns() < COVERAGE_NS {
        return Err(coverage());
    }
    let mut segments = (view.newest_first().collect::<Result<Vec<_>, _>>())
        .map_err(|error| failed(CertificateCheck::History(error)))?;
    segments.reverse();
    let first = span.start().nanoseconds();
    let last = span.end().nanoseconds() - DAY_NS;
    let starts = window_starts(&segments, first, last);
    let quantities: Vec<QuantityId> = view.basis().ids().collect();
    let mut worst: Vec<Option<(Compensated, Interval, Compensated, Interval)>> = vec![None; quantities.len()];
    let mut drifted: Option<Failed> = None;
    let (mut trail, mut lead) = (0, 0);
    let mut pieces = 0;
    for pair in starts.windows(2) {
        let [from, to] = [pair[0], pair[1]];
        let early = Interval::new(SimTime::from_ns(from), SimTime::from_ns(to)).ok_or_else(coverage)?;
        let late = Interval::new(SimTime::from_ns(from + DAY_NS), SimTime::from_ns(to + DAY_NS))
            .ok_or_else(coverage)?;
        while segments[trail].interval().end() <= early.start() {
            trail += 1;
        }
        while segments[lead].interval().end() <= late.start() {
            lead += 1;
        }
        let (trailing, leading) = (&segments[trail], &segments[lead]);
        pieces += 1;
        for (&quantity, held) in quantities.iter().zip(&mut worst) {
            let pick = |curve: fn(&HistorySegment, QuantityId) -> Curve<'_>| {
                differences(&curve(leading, quantity), late, &curve(trailing, quantity), early)
            };
            let change = pick(HistorySegment::inventory).ok_or_else(coverage)?;
            let inflow = pick(HistorySegment::inflow).ok_or_else(coverage)?;
            let outflow = pick(HistorySegment::outflow).ok_or_else(coverage)?;
            for (index, &start) in pair.iter().enumerate() {
                let window = Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + DAY_NS))
                    .ok_or_else(coverage)?;
                let closure = change[index].minus(inflow[index].minus(outflow[index])).abs();
                let gross = inflow[index].plus(outflow[index]);
                let allowed = ledger.absolute.plus(gross.times(&ledger.relative));
                if !closure.at_most(allowed) {
                    return Err(Failed {
                        check: CertificateCheck::Closure,
                        quantity: Some(quantity),
                        window: Some(window),
                        residual: closure.bits(),
                    });
                }
                let drift = change[index].abs();
                if !drift.at_most(ledger.drift) && drifted.is_none() {
                    drifted = Some(Failed {
                        check: CertificateCheck::Drift,
                        quantity: Some(quantity),
                        window: Some(window),
                        residual: drift.bits(),
                    });
                }
                let (worst_closure, closure_window, worst_drift, drift_window) =
                    held.get_or_insert((closure, window, drift, window));
                if !closure.at_most(*worst_closure) {
                    (*worst_closure, *closure_window) = (closure, window);
                }
                if !drift.at_most(*worst_drift) {
                    (*worst_drift, *drift_window) = (drift, window);
                }
            }
        }
    }
    if let Some(failure) = drifted {
        return Err(failure);
    }
    let results = (quantities.iter().zip(worst))
        .map(|(&quantity, held)| {
            let (closure, closure_window, drift, drift_window) = held?;
            Some(Worst {
                quantity,
                closure: closure.bits(),
                closure_window,
                drift: drift.bits(),
                drift_window,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(coverage)?;
    Ok((results, pieces))
}
