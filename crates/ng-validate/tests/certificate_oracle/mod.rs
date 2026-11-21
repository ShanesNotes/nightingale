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

fn edge_rate(quantity: QuantityId, settled: f64, settling: Option<f64>) -> EdgeRate {
    fn of<U: Substance>(quantity: QuantityId, settled: f64, settling: Option<f64>) -> EdgeRate {
        let rate = |value| Measure::<Rate<U>>::new(value).expect("finite");
        EdgeRate::new(quantity, rate(settled), settling.map(rate)).expect("its own dimension")
    }
    struct Build;
    impl Visit for Build {
        type Output = fn(QuantityId, f64, Option<f64>) -> EdgeRate;
        fn visit<Q: Substance>(self) -> Self::Output {
            of::<Q>
        }
    }
    let build = match quantity.book() {
        Book::Of(Species::Energy) | Book::Energy => of::<Energy>,
        Book::Of(species) => species.visit(Build),
        Book::Element(_) => {
            let rate = |value| Measure::<Rate<Atoms>>::new(value).expect("finite");
            return EdgeRate::element(quantity, rate(settled), settling.map(rate)).expect("atoms");
        }
        Book::Charge => panic!("charge holds to its closure bound and has no rate between edges"),
    };
    build(quantity, settled, settling)
}

const SECOND: u64 = 1_000_000_000;

struct EdgeLedger {
    clock: Slot,
    settling_ns: u64,
    length_ns: u64,
    widest_ns: u64,
    rates: Vec<(QuantityId, Compensated, Option<Compensated>)>,
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
    edges: Option<EdgeLedger>,
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
            edges: None,
        }
    }
    pub fn between_edges(
        self,
        clock: Slot,
        (settling_ns, length_ns, widest_ns): (u64, u64, u64),
        rates: &[(QuantityId, f64, Option<f64>)],
    ) -> Self {
        let length = NonZeroU64::new(length_ns).expect("positive");
        let windows = EdgeWindows::new(clock, settling_ns, length, widest_ns);
        let typed = (rates.iter())
            .map(|&(quantity, settled, settling)| edge_rate(quantity, settled, settling))
            .collect();
        let policy = self.policy.between_edges(windows, typed).expect("a policy");
        let plain = |value| Compensated::of(&Measure::<Rate<Water>>::new(value).expect("finite"));
        let rates = (rates.iter())
            .map(|&(quantity, settled, settling)| (quantity, plain(settled), settling.map(plain)))
            .collect();
        let edges = Some(EdgeLedger {
            clock,
            settling_ns,
            length_ns,
            widest_ns,
            rates,
        });
        Self {
            policy,
            edges,
            ..self
        }
    }
    pub fn digest(&self) -> Digest {
        self.policy.digest()
    }
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

const EXPONENT_BITS: u64 = 0x7ff << 52;
const TWO_TO_MINUS_105: f64 = f64::from_bits((1023 - 105) << 52);
const SMALLEST_FLOAT: f64 = f64::from_bits(1);

fn last_place(total: Compensated) -> Compensated {
    let larger = f64::from_bits(total.bits()[0]);
    let place = if larger.is_finite() {
        let power = f64::from_bits(larger.to_bits() & EXPONENT_BITS);
        let scaled = power * TWO_TO_MINUS_105;
        if scaled > 0.0 { scaled } else { SMALLEST_FLOAT }
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
    allowance: Option<[u64; 2]>,
}

#[derive(Debug, PartialEq)]
struct Rhythmic {
    clock: Slot,
    settling_ns: u64,
    length_ns: u64,
    widest_ns: u64,
    judged: u64,
    results: Vec<(QuantityId, [u64; 2], [u64; 2], Interval)>,
    widest: Interval,
    last_close: Option<SimTime>,
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
        let rhythm = certificate.rhythm().map(|rhythm| {
            let windows = rhythm.windows();
            let results = (rhythm.results().iter()).map(|result| {
                let (change, allowance) = (result.change().bits(), result.allowance().bits());
                (result.quantity(), change, allowance, result.window())
            });
            Rhythmic {
                clock: windows.clock(),
                settling_ns: windows.settling_ns(),
                length_ns: windows.length_ns().get(),
                widest_ns: windows.widest_ns(),
                judged: rhythm.judged(),
                results: results.collect(),
                widest: rhythm.widest(),
                last_close: rhythm.last_close(),
            }
        });
        (worst.collect(), most.collect(), certificate.pieces(), rhythm)
    });
    let seen = seen.map_err(|failure| Failed {
        check: failure.check(),
        quantity: failure.quantity(),
        window: failure.window(),
        residual: failure.residual().bits(),
        allowance: failure.allowance().map(|allowance| allowance.bits()),
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
        allowance: None,
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

type Walked = (Vec<Worst>, Vec<Most>, u64, Option<Rhythmic>);

/// Every window between edges, found by scanning forward from each opening for its close, then judged in order of
/// close and opening; every stretch with no edge, the tail from the last close.
fn between_edges(
    segments: &[HistorySegment],
    span: Interval,
    edges: &EdgeLedger,
    rhythm: RhythmId,
) -> Result<Rhythmic, Failed> {
    let at = SimTime::from_ns;
    let start = span.start().nanoseconds();
    let marks: Vec<(u64, Vec<Compensated>)> = (segments.iter())
        .filter(|segment| segment.ends_on_edge(rhythm))
        .map(|segment| {
            let interval = segment.interval();
            let values = (edges.rates.iter())
                .map(|&(quantity, _, _)| segment.inventory(quantity).on(interval).expect("its own")[1])
                .collect();
            (interval.end().nanoseconds(), values)
        })
        .collect();
    let early = edges.rates.iter().any(|(_, _, settling)| settling.is_some());
    let mut windows = Vec::new();
    for (open, &(opened, _)) in marks.iter().enumerate() {
        if opened - start < edges.settling_ns && !early {
            continue;
        }
        let close = (open + 1..marks.len()).find(|&close| marks[close].0 - opened >= edges.length_ns);
        windows.extend(close.map(|close| (close, open)));
    }
    windows.sort_unstable();
    let allowed = |per_second: Compensated, ns: u64| {
        let whole = i64::try_from(ns / SECOND).expect("seconds");
        let part = Compensated::between(Compensated::ZERO, per_second, ns % SECOND, SECOND);
        per_second.scaled(whole).plus(part)
    };
    let mut drifted = None;
    let mut worst: Vec<Option<(Compensated, Compensated, Interval)>> = vec![None; edges.rates.len()];
    for &(close, open) in &windows {
        let ((opened, before), (closed, after)) = (&marks[open], &marks[close]);
        let window = Interval::new(at(*opened), at(*closed)).expect("a later edge");
        let settled = opened - start >= edges.settling_ns;
        for (index, &(quantity, rate, settling)) in edges.rates.iter().enumerate() {
            let per_second = if settled {
                rate
            } else {
                settling.expect("every rate states one")
            };
            let allowance = allowed(per_second, closed - opened);
            let change = after[index].minus(before[index]).abs();
            if !change.at_most(allowance) && drifted.is_none() {
                drifted = Some(Failed {
                    check: CertificateCheck::EdgeDrift,
                    quantity: Some(quantity),
                    window: Some(window),
                    residual: change.bits(),
                    allowance: Some(allowance.bits()),
                });
            }
            let held = worst[index].get_or_insert((change, allowance, window));
            if !change.minus(allowance).at_most(held.0.minus(held.1)) {
                *held = (change, allowance, window);
            }
        }
    }
    let last_close = windows.last().map(|&(close, _)| marks[close].0);
    let ends = (marks.iter().map(|&(edge, _)| edge)).chain([span.end().nanoseconds()]);
    let starts = [start].into_iter().chain(marks.iter().map(|&(edge, _)| edge));
    let mut stretches: Vec<(u64, u64)> = starts.zip(ends).collect();
    stretches.pop();
    stretches.push((last_close.unwrap_or(start), span.end().nanoseconds()));
    let stretches: Vec<Interval> = (stretches.into_iter())
        .filter_map(|(from, to)| Interval::new(at(from), at(to)))
        .collect();
    if let Some(&stopped) = (stretches.iter()).find(|stretch| stretch.length_ns() > edges.widest_ns) {
        return Err(Failed {
            check: CertificateCheck::RhythmStopped,
            quantity: None,
            window: Some(stopped),
            residual: Compensated::ZERO.bits(),
            allowance: None,
        });
    }
    if let Some(failure) = drifted {
        return Err(failure);
    }
    let widest = stretches
        .iter()
        .fold(None, |held: Option<Interval>, &stretch| match held {
            Some(held) if stretch.length_ns() <= held.length_ns() => Some(held),
            _ => Some(stretch),
        });
    let results = (edges.rates.iter().zip(worst))
        .filter_map(|(&(quantity, _, _), held)| {
            let (change, allowance, window) = held?;
            Some((quantity, change.bits(), allowance.bits(), window))
        })
        .collect();
    Ok(Rhythmic {
        clock: edges.clock,
        settling_ns: edges.settling_ns,
        length_ns: edges.length_ns,
        widest_ns: edges.widest_ns,
        judged: windows.len() as u64,
        results,
        widest: widest.ok_or_else(|| failed(CertificateCheck::Coverage))?,
        last_close: last_close.map(at),
    })
}

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
    let rhythm = match &ledger.edges {
        Some(edges) => {
            let declared = view.rhythms().find(|&(_, phase)| phase == edges.clock);
            let undeclared = || failed(CertificateCheck::UndeclaredRhythm);
            Some((edges, declared.ok_or_else(undeclared)?.0))
        }
        None => None,
    };
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
                        allowance: None,
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
                        allowance: None,
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
    let rhythm = (rhythm.map(|(edges, rhythm)| between_edges(&segments, span, edges, rhythm))).transpose()?;
    Ok((results, support, pieces, rhythm))
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

#[test]
fn the_forward_oracle_refuses_a_clock_the_body_declares_no_rhythm_of_before_any_step_as_certify_does() {
    let start = [600.0, 424.0].map(|amount| Measure::new(amount).expect("a start"));
    let case = TwoReservoirs::delivering(start, Vec::new()).expect("a body");
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(100_000_000));
    let session = case.admit(&core, &store).expect("admitted");
    let view = session.conservation();
    assert_eq!(view.span(), None, "no step, so no span to check");
    let clock = Slot::from(LawName {
        owner: "NS",
        local: "phase",
    });
    let water = view.basis().ids().next().expect("a quantity");
    let ledger = Ledger::sliding(view.basis(), SECOND, SECOND, 0.0, 1e-12, 1e9).between_edges(
        clock,
        (0, SECOND, SECOND),
        &[(water, 1.0, None)],
    );
    let refused = certified(&view, &ledger).expect_err("no rhythm of that clock");
    assert_eq!(refused.check(), CertificateCheck::UndeclaredRhythm);
}
