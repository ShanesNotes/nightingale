use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{ConservationView, Curve, HistorySegment};
use ng_validate::*;

fn limits(quantity: QuantityId, absolute: f64, relative: f64, drift: f64) -> QuantityPolicy {
    fn of<U: Substance>(quantity: QuantityId, absolute: f64, relative: f64, drift: f64) -> QuantityPolicy {
        let measure = |value| Measure::<U>::new(value).expect("finite");
        let ratio = Measure::<Dimensionless>::new(relative).expect("finite");
        QuantityPolicy::new(quantity, measure(absolute), ratio, measure(drift)).expect("its own dimension")
    }
    let build = match quantity.book() {
        Book::Of(Species::Water) => of::<Water>,
        Book::Of(Species::Sodium) => of::<Sodium>,
        Book::Of(Species::Potassium) => of::<Potassium>,
        Book::Of(Species::Chloride) => of::<Chloride>,
        Book::Of(Species::Albumin) => of::<Albumin>,
        Book::Of(Species::Vasopressin) => of::<Vasopressin>,
        Book::Of(Species::Norepinephrine) => of::<Norepinephrine>,
        Book::Of(Species::Acetylcholine) => of::<Acetylcholine>,
        Book::Of(Species::Energy) | Book::Energy => of::<Energy>,
        Book::Of(Species::Oxygen) => of::<Oxygen>,
        Book::Of(Species::CarbonDioxide) => of::<CarbonDioxide>,
        Book::Of(Species::Glucose) => of::<Glucose>,
        Book::Element(_) => {
            let measure = |value| Measure::<Atoms>::new(value).expect("finite");
            let ratio = Measure::<Dimensionless>::new(relative).expect("finite");
            return QuantityPolicy::element(quantity, measure(absolute), ratio, measure(drift))
                .expect("atoms");
        }
        Book::Charge => {
            let floor = Measure::<Charge>::new(absolute).expect("finite");
            let ratio = Measure::<Dimensionless>::new(relative).expect("finite");
            return QuantityPolicy::charge(quantity, floor, ratio).expect("charge");
        }
    };
    build(quantity, absolute, relative, drift)
}

pub struct Ledger {
    policy: CertificatePolicy,
    basis: Digest,
    window_ns: u64,
    coverage_ns: u64,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    drift: Compensated,
}
impl Ledger {
    pub fn sliding(
        basis: &ConservedBasis,
        window_ns: u64,
        coverage_ns: u64,
        absolute: f64,
        relative: f64,
        drift: f64,
    ) -> Self {
        let quantities = (basis.ids())
            .map(|quantity| limits(quantity, absolute, relative, drift))
            .collect();
        let window = NonZeroU64::new(window_ns).expect("positive");
        let windows = SlidingWindows::new(window, coverage_ns).expect("a coverage");
        let plain = |value| Compensated::of(&Measure::<Water>::new(value).expect("finite"));
        Self {
            policy: CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
                .expect("a policy"),
            basis: basis.digest(),
            window_ns,
            coverage_ns,
            absolute: plain(absolute),
            relative: Measure::<Dimensionless>::new(relative).expect("finite"),
            drift: plain(drift),
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
struct Most {
    route: HeldRoute,
    inflow: [u64; 2],
    inflow_window: Interval,
    outflow: [u64; 2],
    outflow_window: Interval,
}

#[derive(Debug, PartialEq)]
struct Failed {
    check: CertificateCheck,
    quantity: Option<QuantityId>,
    window: Option<Interval>,
    residual: [u64; 2],
}

pub fn certified(view: &ConservationView<'_>, ledger: &Ledger) -> Result<Certificate, CertificateFailure> {
    let walked = certify(view, &ledger.policy);
    let seen = walked.as_ref().map(|certificate| {
        assert_eq!(certificate.seal(), view.seal());
        assert_eq!(certificate.basis(), ledger.basis);
        assert_eq!(certificate.policy(), ledger.digest());
        assert_eq!(Some(certificate.span()), view.span());
        assert_eq!(certificate.coverage(), view.basis().coverage());
        let worst = (certificate.results().iter()).map(|result| Worst {
            quantity: result.quantity(),
            closure: result.worst_closure().bits(),
            closure_window: result.closure_window(),
            drift: result.worst_drift().bits(),
            drift_window: result.drift_window(),
        });
        let most = (certificate.support().iter()).map(|support| Most {
            route: support.route(),
            inflow: support.most_inflow().bits(),
            inflow_window: support.inflow_window(),
            outflow: support.most_outflow().bits(),
            outflow_window: support.outflow_window(),
        });
        (worst.collect(), most.collect(), certificate.pieces())
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

fn window_starts(segments: &[HistorySegment], first: u64, last: u64, length: u64) -> Vec<u64> {
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
        let moved = (boundaries.get(shifted)).map(|boundary| boundary.saturating_sub(length));
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

type Walked = (Vec<Worst>, Vec<Most>, u64);

fn in_time_order(view: &ConservationView<'_>, ledger: &Ledger) -> Result<Walked, Failed> {
    let coverage = || failed(CertificateCheck::Coverage);
    if ledger.basis != view.basis().digest() {
        return Err(coverage());
    }
    let span = view.span().ok_or_else(coverage)?;
    if span.length_ns() < ledger.coverage_ns {
        return Err(coverage());
    }
    let length = ledger.window_ns;
    let mut segments = (view.newest_first().collect::<Result<Vec<_>, _>>())
        .map_err(|error| failed(CertificateCheck::History(error)))?;
    segments.reverse();
    let first = span.start().nanoseconds();
    let last = if ledger.coverage_ns == length {
        first
    } else {
        span.end().nanoseconds() - length
    };
    let starts = window_starts(&segments, first, last, length);
    let quantities: Vec<QuantityId> = view.basis().ids().collect();
    let routes = view.basis().coverage().held_routes();
    let mut worst: Vec<Option<(Compensated, Interval, Compensated, Interval)>> = vec![None; quantities.len()];
    let mut most: Vec<Option<(Compensated, Interval, Compensated, Interval)>> = vec![None; routes.len()];
    let mut drifted: Option<Failed> = None;
    let (mut trail, mut lead) = (0, 0);
    let mut pieces = 0;
    let stretches: Vec<&[u64]> = match starts.len() {
        1 => vec![&starts[..]],
        _ => starts.windows(2).collect(),
    };
    for pair in stretches {
        let (trailing, leading, pieces_of) = match *pair {
            [from, to] => {
                let early =
                    Interval::new(SimTime::from_ns(from), SimTime::from_ns(to)).ok_or_else(coverage)?;
                let late = Interval::new(SimTime::from_ns(from + length), SimTime::from_ns(to + length))
                    .ok_or_else(coverage)?;
                while segments[trail].interval().end() <= early.start() {
                    trail += 1;
                }
                while segments[lead].interval().end() <= late.start() {
                    lead += 1;
                }
                (&segments[trail], &segments[lead], Some((early, late)))
            }
            _ => {
                let end = SimTime::from_ns(first + length);
                let leading = segments.iter().find(|segment| segment.interval().end() >= end);
                (&segments[0], leading.ok_or_else(coverage)?, None)
            }
        };
        let differences = |later: &Curve<'_>, earlier: &Curve<'_>| match pieces_of {
            Some((early, late)) => differences(later, late, earlier, early),
            None => {
                let end = SimTime::from_ns(first + length);
                let end = later.on(Interval::new(leading.interval().start(), end)?)?;
                let start = earlier.on(trailing.interval())?;
                Some(vec![end.last()?.minus(*start.first()?)])
            }
        };
        pieces += 1;
        for (&quantity, held) in quantities.iter().zip(&mut worst) {
            let pick = |curve: fn(&HistorySegment, QuantityId) -> Curve<'_>| {
                differences(&curve(leading, quantity), &curve(trailing, quantity))
            };
            let change = pick(HistorySegment::inventory).ok_or_else(coverage)?;
            let inflow = pick(HistorySegment::inflow).ok_or_else(coverage)?;
            let outflow = pick(HistorySegment::outflow).ok_or_else(coverage)?;
            for (index, &start) in pair.iter().enumerate() {
                let window = Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + length))
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
                let bound = match quantity.book() {
                    Book::Charge => allowed,
                    Book::Of(_) | Book::Element(_) | Book::Energy => ledger.drift,
                };
                if !drift.at_most(bound) && drifted.is_none() {
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
        for (route, held) in routes.iter().zip(&mut most) {
            let ((forward, reverse), (forward_before, reverse_before)) = (
                leading.exchange(route.column()),
                trailing.exchange(route.column()),
            );
            let forward = differences(&forward, &forward_before).ok_or_else(coverage)?;
            let reverse = differences(&reverse, &reverse_before).ok_or_else(coverage)?;
            let (inflow, outflow) = match route.forward() {
                BoundaryDirection::IntoBody => (forward, reverse),
                BoundaryDirection::OutOfBody => (reverse, forward),
            };
            for (index, &start) in pair.iter().enumerate() {
                let window = Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + length))
                    .ok_or_else(coverage)?;
                let (into, out_of) = (inflow[index], outflow[index]);
                let (most_in, in_window, most_out, out_window) =
                    held.get_or_insert((into, window, out_of, window));
                if !into.at_most(*most_in) {
                    (*most_in, *in_window) = (into, window);
                }
                if !out_of.at_most(*most_out) {
                    (*most_out, *out_window) = (out_of, window);
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
    let support = (routes.iter().zip(most))
        .map(|(&route, held)| {
            let (inflow, inflow_window, outflow, outflow_window) = held?;
            Some(Most {
                route,
                inflow: inflow.bits(),
                inflow_window,
                outflow: outflow.bits(),
                outflow_window,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(coverage)?;
    Ok((results, support, pieces))
}
