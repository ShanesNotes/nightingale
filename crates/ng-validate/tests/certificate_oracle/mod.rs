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
    struct Build;
    impl Visit for Build {
        type Output = fn(QuantityId, f64, f64, f64) -> QuantityPolicy;
        fn visit<Q: Substance>(self) -> Self::Output {
            of::<Q>
        }
    }
    let build = match quantity.book() {
        Book::Of(Species::Energy) | Book::Energy => of::<Energy>,
        Book::Of(species) => species.visit(Build),
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
    pub policy: CertificatePolicy,
    basis: Digest,
    window_ns: u64,
    coverage_ns: u64,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    drift: Compensated,
    per_update: u64,
    updates_a_step: Vec<u64>,
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
            per_update: 0,
            updates_a_step: vec![0; basis.combinations().len()],
        }
    }
    pub fn digest(&self) -> Digest {
        self.policy.digest()
    }
    /// Each column term on a row of a combination is one update of that combination's rows a step.
    pub fn admit_row_rounding(&mut self, body: &CompiledBody, per_update: u64) {
        let combinations = body.conserved_basis().combinations();
        let column_terms = || body.columns().iter().flat_map(|column| column.terms());
        let updates = |combination: &Combination| {
            let member = |row: &RowId| combination.terms().iter().any(|(held, _)| held == row);
            column_terms().filter(|(row, _)| member(row)).count() as u64
        };
        self.updates_a_step = combinations.iter().map(updates).collect();
        self.per_update = per_update;
    }
}

/// 2^-105 of the power of two at or below the larger part, never below the smallest float, and zero when that part
/// is not finite.
fn last_place(total: Compensated) -> Compensated {
    let larger = f64::from_bits(total.bits()[0]);
    let place = if larger.is_finite() {
        let power = f64::from_bits(larger.to_bits() & (0x7ff << 52));
        let scaled = power * f64::from_bits((1023 - 105) << 52);
        if scaled > 0.0 { scaled } else { f64::from_bits(1) }
    } else {
        0.0
    };
    Compensated::of(&Measure::<Dimensionless>::new(place).expect("finite"))
}

#[derive(Debug, PartialEq)]
struct Worst {
    quantity: QuantityId,
    closure: [u64; 2],
    closure_window: Interval,
    closure_last_places: u64,
    closure_place: [u64; 2],
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
            closure_last_places: result.closure_last_places(),
            closure_place: result.closure_place().bits(),
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

#[derive(Clone)]
struct Held {
    closure: Compensated,
    closure_window: Interval,
    last_places: u64,
    place: Compensated,
    drift: Compensated,
    drift_window: Interval,
}

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
    let mut worst: Vec<Option<Held>> = vec![None; quantities.len()];
    let mut most: Vec<Option<(Compensated, Interval, Compensated, Interval)>> = vec![None; routes.len()];
    let mut drifted: Option<Failed> = None;
    let (mut trail, mut lead) = (0, 0);
    let mut pieces = 0;
    let stretches: Vec<&[u64]> = match starts.len() {
        1 => vec![&starts[..]],
        _ => starts.windows(2).collect(),
    };
    for pair in stretches {
        let (trailing, leading, pieces_of, steps) = match *pair {
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
                let steps = (lead - trail + 1) as u64;
                (&segments[trail], &segments[lead], Some((early, late)), steps)
            }
            _ => {
                let end = SimTime::from_ns(first + length);
                let leading = segments
                    .iter()
                    .position(|segment| segment.interval().end() >= end);
                let leading = leading.ok_or_else(coverage)?;
                (&segments[0], &segments[leading], None, leading as u64 + 1)
            }
        };
        let closing = |later: &Curve<'_>| match pieces_of {
            Some((_, late)) => later.on(late),
            None => {
                let end = SimTime::from_ns(first + length);
                let end = later.on(Interval::new(leading.interval().start(), end)?)?;
                Some(vec![*end.last()?])
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
        for ((&quantity, held), updates) in quantities.iter().zip(&mut worst).zip(&ledger.updates_a_step) {
            let pick = |curve: fn(&HistorySegment, QuantityId) -> Curve<'_>| {
                differences(&curve(leading, quantity), &curve(trailing, quantity))
            };
            let change = pick(HistorySegment::inventory).ok_or_else(coverage)?;
            let inflow = pick(HistorySegment::inflow).ok_or_else(coverage)?;
            let outflow = pick(HistorySegment::outflow).ok_or_else(coverage)?;
            let closing = closing(&leading.inventory(quantity)).ok_or_else(coverage)?;
            let last_places = [*updates, steps]
                .into_iter()
                .try_fold(ledger.per_update, u64::checked_mul)
                .unwrap_or(u64::MAX);
            let rounding = i64::try_from(last_places).unwrap_or(i64::MAX);
            for (index, &start) in pair.iter().enumerate() {
                let window = Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + length))
                    .ok_or_else(coverage)?;
                let closure = change[index].minus(inflow[index].minus(outflow[index])).abs();
                let gross = inflow[index].plus(outflow[index]);
                let place = last_place(closing[index]);
                let allowed =
                    (ledger.absolute.plus(gross.times(&ledger.relative))).plus(place.scaled(rounding));
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
                let held = held.get_or_insert(Held {
                    closure,
                    closure_window: window,
                    last_places,
                    place,
                    drift,
                    drift_window: window,
                });
                if !closure.at_most(held.closure) {
                    (held.closure, held.closure_window) = (closure, window);
                    (held.last_places, held.place) = (last_places, place);
                }
                if !drift.at_most(held.drift) {
                    (held.drift, held.drift_window) = (drift, window);
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
            let held = held?;
            Some(Worst {
                quantity,
                closure: held.closure.bits(),
                closure_window: held.closure_window,
                closure_last_places: held.last_places,
                closure_place: held.place.bits(),
                drift: held.drift.bits(),
                drift_window: held.drift_window,
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

#[test]
fn the_forward_oracle_accepts_the_same_row_rounding_as_certify() {
    let end = SimTime::from_ns(6 * 3_600 * 1_000_000_000);
    let start = [600.0, 424.0].map(|amount| Measure::new(amount).expect("a start"));
    let case = TwoReservoirs::delivering(start, Vec::new())
        .expect("a body")
        .until(end);
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(100_000_000));
    let session = case.admit(&core, &store).expect("admitted");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    let view = session.conservation();
    let span = end.nanoseconds();
    let mut ledger = Ledger::sliding(view.basis(), span / 2, span, 0.0, 1e-12, 1e9);
    ledger.policy = ledger
        .policy
        .admitting_row_rounding(case.body(), 7)
        .expect("a policy");
    ledger.admit_row_rounding(case.body(), 7);
    let passed = certify(&view, &ledger.policy).expect("within seven last places per row update");
    assert_eq!(passed.results()[0].closure_last_places(), 2_268_021);
    let oracle = in_time_order(&view, &ledger)
        .map(|_| ())
        .map_err(|failure| failure.check);
    assert_eq!(oracle, Ok(()), "the forward oracle must accept the same policy");
}
