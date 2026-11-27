mod certificate_oracle;

use certificate_oracle::{Ledger, certified};
use core::cell::RefCell;
use ng_contract::*;
use ng_record::{CheckpointError, HistoryStore, Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3_600 * SECOND;
const STEP: u64 = 100 * MILLISECOND;
const PAIR_PRECISION: f64 = f64::EPSILON * f64::EPSILON / 2.0;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn to<'m>(session: Session<'m, Running>, end: SimTime) -> Session<'m, Running> {
    match run_until(session, end) {
        Ok(session) => session,
        Err(stopped) => panic!("{:?}", stopped.fault()),
    }
}

fn start_of(failure: &CertificateFailure) -> u64 {
    failure.window().expect("a window").start().nanoseconds()
}

fn drifts_then_opens() -> TwoReservoirs {
    let held = 1e18;
    let bulk = Delivery {
        id: b"bulk",
        from: SimTime::from_ns(0),
        to: SimTime::from_ns(600 * SECOND),
        amount: measure(600.0),
    };
    let tiny = Delivery {
        id: b"below the pair's precision",
        from: SimTime::from_ns(2 * HOUR),
        to: SimTime::from_ns(4 * HOUR),
        amount: measure(held * PAIR_PRECISION / 4.0),
    };
    TwoReservoirs::delivering([measure(held), measure(held)], vec![bulk, tiny]).expect("a body")
}

#[test]
fn a_closure_failure_outranks_an_earlier_drift_failure() {
    let (case, store) = (drifts_then_opens(), MemoryStore::default());
    let core = ReferenceIntegrator::new(60 * SECOND);
    let session = to(
        case.admit(&core, &store).expect("admitted"),
        SimTime::from_ns(4 * HOUR),
    );
    let view = session.conservation();
    let ledger = |absolute| Ledger::sliding(view.basis(), HOUR, 4 * HOUR, absolute, 1e-9, 1.0);
    let drifted = certified(&view, &ledger(1e300)).expect_err("the bulk moves the body");
    assert_eq!(
        (drifted.check(), start_of(&drifted)),
        (CertificateCheck::Drift, 0),
        "with closure held loosely, the first window drifts"
    );
    let failure = certified(&view, &ledger(1e-24)).expect_err("it does not close");
    assert_eq!(
        (failure.check(), start_of(&failure)),
        (CertificateCheck::Closure, HOUR + 60 * SECOND),
        "the earliest window that reaches past two hours, later than the drift"
    );
}

#[test]
fn of_equal_worst_windows_the_earliest_is_kept() {
    let rested = [measure(1_000.0), measure(1_000.0)];
    let (case, store) = (
        TwoReservoirs::delivering(rested, Vec::new()).expect("a body"),
        MemoryStore::default(),
    );
    let core = ReferenceIntegrator::new(60 * SECOND);
    let session = to(
        case.admit(&core, &store).expect("admitted"),
        SimTime::from_ns(4 * HOUR),
    );
    let view = session.conservation();
    let ledger = Ledger::sliding(view.basis(), HOUR, 4 * HOUR, 1e-24, 1e-9, 1e-6);
    let certificate = certified(&view, &ledger).expect("a body at rest holds still");
    let result = certificate.results()[0];
    let zero = Compensated::ZERO.bits();
    assert_eq!(
        (result.worst_closure().bits(), result.worst_drift().bits()),
        (zero, zero),
        "every window ties at zero"
    );
    let first = Interval::new(SimTime::from_ns(0), SimTime::from_ns(HOUR)).expect("an hour");
    assert_eq!((result.closure_window(), result.drift_window()), (first, first));
}

#[derive(Default)]
struct Damaged {
    kept: MemoryStore,
    lost: RefCell<Vec<Digest>>,
    altered: RefCell<Vec<Digest>>,
}
impl HistoryStore for Damaged {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        if self.lost.borrow().contains(seal) {
            return None;
        }
        let mut link = self.kept.resolve(seal)?;
        if self.altered.borrow().contains(seal) {
            let last = link.len() - 1;
            link[last] ^= 1;
        }
        Some(link)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.kept.keep(seal, link)
    }
}

#[test]
fn a_history_error_outranks_a_closure_failure_and_the_newer_of_two_bad_links_is_reported() {
    let (case, store) = (drifts_then_opens(), Damaged::default());
    let core = ReferenceIntegrator::new(60 * SECOND);
    let session = to(
        case.admit(&core, &store).expect("admitted"),
        SimTime::from_ns(HOUR),
    );
    let early = session.record().seal().digest();
    let session = to(session, SimTime::from_ns(3 * HOUR));
    let late = session.record().seal().digest();
    let session = to(session, SimTime::from_ns(4 * HOUR));
    let view = session.conservation();
    let ledger = Ledger::sliding(view.basis(), HOUR, 4 * HOUR, 1e-24, 1e-9, 1.0);
    let opened = certified(&view, &ledger).expect_err("it does not close");
    assert_eq!(opened.check(), CertificateCheck::Closure);
    let damaged = |lost: Digest, altered: Option<Digest>| {
        *store.lost.borrow_mut() = vec![lost];
        *store.altered.borrow_mut() = altered.into_iter().collect();
        certified(&view, &ledger).expect_err("a bad link").check()
    };
    let missing = |seal| CertificateCheck::History(CheckpointError::HistoryMissing { seal });
    let altered = |seal| CertificateCheck::History(CheckpointError::HistoryAltered { seal });
    assert_eq!(
        damaged(late, None),
        missing(late),
        "a link newer than the window that fails closure"
    );
    assert_eq!(damaged(late, Some(early)), missing(late), "the newer of two");
    assert_eq!(damaged(early, Some(late)), altered(late), "the newer of two");
}

#[test]
fn a_history_shorter_than_its_coverage_fails_coverage_before_a_bad_link_is_read() {
    let (case, store) = (drifts_then_opens(), Damaged::default());
    let core = ReferenceIntegrator::new(60 * SECOND);
    let session = to(
        case.admit(&core, &store).expect("admitted"),
        SimTime::from_ns(HOUR),
    );
    let lost = session.record().seal().digest();
    let session = to(session, SimTime::from_ns(4 * HOUR));
    let view = session.conservation();
    *store.lost.borrow_mut() = vec![lost];
    let covering = |coverage| Ledger::sliding(view.basis(), HOUR, coverage, 1e-24, 1e-9, 1.0);
    assert_eq!(
        certified(&view, &covering(4 * HOUR))
            .expect_err("a bad link")
            .check(),
        CertificateCheck::History(CheckpointError::HistoryMissing { seal: lost }),
        "a coverage the history spans reads the lost link"
    );
    assert_eq!(
        certified(&view, &covering(5 * HOUR))
            .expect_err("too short")
            .check(),
        CertificateCheck::Coverage,
        "a coverage longer than the history fails before any link is read"
    );
}

#[test]
fn a_shared_start_is_judged_by_both_of_its_pieces_and_the_earlier_ones_steps_are_reported() {
    let (span, length) = (6 * HOUR, 3 * HOUR + STEP / 2);
    let start = [measure(600.0), measure(424.0)];
    let case = TwoReservoirs::delivering(start, Vec::new())
        .expect("a body")
        .until(SimTime::from_ns(span));
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(STEP));
    let session = to(case.admit(&core, &store).expect("admitted"), case.end());
    let view = session.conservation();
    let mut ledger = Ledger::sliding(view.basis(), length, span, 0.0, 1e-12, 1e9);
    ledger.policy = (ledger.policy)
        .admitting_row_rounding(case.body(), 7)
        .expect("a policy");
    ledger.admit_row_rounding(case.body(), 7);
    let certificate = certified(&view, &ledger).expect("within seven last places per row update");
    let water = certificate.results()[0];
    let worst = water.closure_window();

    let mut segments: Vec<Interval> = (view.newest_first())
        .map(|segment| segment.expect("a whole chain").interval())
        .collect();
    segments.reverse();
    let holding = |instant: SimTime, ending_there: bool| {
        let holds = |segment: &Interval| match ending_there {
            true => instant <= segment.end(),
            false => instant < segment.end(),
        };
        segments.iter().position(holds).expect("inside the history") as u64
    };
    let steps = |ending_there| holding(worst.end(), ending_there) - holding(worst.start(), ending_there) + 1;
    let (earlier, later) = (steps(true), steps(false));
    assert_ne!(
        earlier, later,
        "the worst window starts or ends on a step boundary"
    );
    assert_eq!(
        water.closure_last_places(),
        7 * 3 * earlier,
        "seven last places for each of water's three row updates in each step of the earlier piece"
    );
}

#[test]
fn the_worst_closure_reports_the_place_of_its_own_windows_closing_total() {
    let (span, length, from) = (6 * HOUR, 2 * HOUR, 10_200 * SECOND);
    let inflow = Delivery {
        id: b"inflow",
        from: SimTime::from_ns(from),
        to: SimTime::from_ns(span),
        amount: measure(f64::from(1 << 20)),
    };
    let start = [600.0, 424.0];
    let case = TwoReservoirs::delivering(start.map(measure), vec![inflow])
        .expect("a body")
        .until(SimTime::from_ns(span));
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(STEP));
    let session = to(case.admit(&core, &store).expect("admitted"), case.end());
    let view = session.conservation();
    let mut ledger = Ledger::sliding(view.basis(), length, span, 0.0, 1e-12, 1e9);
    ledger.policy = (ledger.policy)
        .admitting_row_rounding(case.body(), 7)
        .expect("a policy");
    ledger.admit_row_rounding(case.body(), 7);
    let certificate = certified(&view, &ledger).expect("within seven last places per row update");
    let water = certificate.results()[0];
    let end = water.closure_window().end().nanoseconds();
    let total =
        start[0] + start[1] + f64::from(1 << 20) * (end.saturating_sub(from) as f64 / (span - from) as f64);
    let place = ((total.to_bits() >> 52) - 105) << 52;
    assert!(
        total >= 2_048.0,
        "the worst window closes on {total}, past the first window's binade"
    );
    assert_eq!(
        water.closure_place().bits()[0],
        place,
        "the place of {total} at {end} ns"
    );
}

fn two_pool_body() -> (CompiledBody, [SiteId; 2]) {
    let origin = || Position::new(measure(0.0), measure(0.0), measure(0.0));
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let first = map.site::<VesselSegment>("first", origin());
    let second = map.site::<VesselSegment>("second", origin());
    map.contain(root.id(), first.id());
    map.contain(root.id(), second.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema
        .boundary::<Water>("first-in", BoundaryDirection::IntoBody, first.id(), Space::Lumen)
        .expect("a boundary");
    schema
        .boundary::<Water>(
            "second-in",
            BoundaryDirection::IntoBody,
            second.id(),
            Space::Lumen,
        )
        .expect("a boundary");
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Wave, 3_600 * SECOND)?;
        for name in ["first-in", "second-in"] {
            let BoundaryRoute { route, drive } = ports.route::<Water>(name)?;
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, rate));
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    (body, [first.id(), second.id()])
}

fn water_event(boundary: &'static str, id: &[u8], from: u64, to: u64, amount: f64) -> BoundaryEvent {
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(id)),
        SimTime::from_ns(from),
        BoundaryInput::Water(WaterDelivery {
            boundary,
            amount: measure(amount),
            ends: SimTime::from_ns(to),
        }),
    )
}

#[test]
fn among_windows_judged_together_a_later_closure_outranks_an_earlier_drift_and_an_earlier_closure_wins() {
    let (body, [first, second]) = two_pool_body();
    let held = 1e18;
    let tiny = held * PAIR_PRECISION / 4.0;
    let step = 60 * SECOND;
    let span = HOUR + step;
    let at = SimTime::from_ns;
    let window = |from, to| Some(Interval::new(at(from), at(to)).expect("a window"));
    let lumen = |site| RowKind::Amount {
        site,
        space: Space::Lumen,
        species: Species::Water,
    };
    let quantity_of = |site: SiteId| {
        let wanted = lumen(site);
        body.conserved_basis()
            .combinations()
            .iter()
            .find(|combination| {
                combination
                    .terms()
                    .iter()
                    .any(|&(row, _)| body.rows()[row.index()].kind() == wanted)
            })
            .expect("in basis")
            .id()
    };
    let (first_qty, second_qty) = (quantity_of(first), quantity_of(second));
    assert!(
        first_qty.index() < second_qty.index(),
        "basis order is first then second"
    );

    for first_opens in [true, false] {
        let mut packet = ParameterPacket::new();
        let (first_start, second_start) = if first_opens {
            (held, 1_000.0)
        } else {
            (1_000.0, held)
        };
        packet
            .start(lumen(first), measure::<Water>(first_start), "synthetic")
            .expect("once");
        packet
            .start(lumen(second), measure::<Water>(second_start), "synthetic")
            .expect("once");
        let (open_in, drift_in, open_qty, drift_qty) = if first_opens {
            ("first-in", "second-in", first_qty, second_qty)
        } else {
            ("second-in", "first-in", second_qty, first_qty)
        };
        let events = vec![
            water_event(open_in, b"fill", 0, step, 10.0),
            water_event(open_in, b"tiny", HOUR, span, tiny),
            water_event(drift_in, b"drift", 0, HOUR, 600.0),
        ];
        let (core, store) = (ReferenceIntegrator::new(step), MemoryStore::default());
        let port = BoundaryPort::new(events).expect("a port");
        let session = to(
            admit_fixture(&body, packet, port, &core, &store).expect("admitted"),
            at(span),
        );
        let view = session.conservation();
        let drifted = certified(
            &view,
            &Ledger::sliding(view.basis(), HOUR, span, 1e300, 1e-9, 100.0),
        )
        .expect_err("drifts at admission");
        assert_eq!(
            (drifted.check(), drifted.quantity(), drifted.window()),
            (CertificateCheck::Drift, Some(drift_qty), window(0, HOUR)),
            "first_opens={first_opens}: the admission window drifts when closure holds"
        );
        let opened = certified(
            &view,
            &Ledger::sliding(view.basis(), HOUR, span, 1e-24, 1e-9, 100.0),
        )
        .expect_err("closure fails one step later");
        assert_eq!(
            (opened.check(), opened.quantity(), opened.window()),
            (
                CertificateCheck::Closure,
                Some(open_qty),
                window(step, HOUR + step)
            ),
            "first_opens={first_opens}: later closure in the same pair outranks earlier drift"
        );
    }

    let mut packet = ParameterPacket::new();
    packet
        .start(lumen(first), measure::<Water>(held), "synthetic")
        .expect("once");
    packet
        .start(lumen(second), measure::<Water>(held), "synthetic")
        .expect("once");
    let events = vec![
        water_event("first-in", b"f-fill", 0, step, 10.0),
        water_event("first-in", b"f-tiny", HOUR, span, tiny),
        water_event("second-in", b"s-fill", 0, step, 10.0),
        water_event("second-in", b"s-tiny", step, HOUR, tiny),
    ];
    let (core, store) = (ReferenceIntegrator::new(step), MemoryStore::default());
    let session = to(
        admit_fixture(
            &body,
            packet,
            BoundaryPort::new(events).expect("a port"),
            &core,
            &store,
        )
        .expect("admitted"),
        at(span),
    );
    let view = session.conservation();
    let opened = certified(&view, &Ledger::sliding(view.basis(), HOUR, span, 1e-24, 0.0, 1e9))
        .expect_err("both open in the first pair");
    assert_eq!(
        (opened.check(), opened.quantity(), opened.window()),
        (CertificateCheck::Closure, Some(second_qty), window(0, HOUR)),
        "earliest closure wins over a later closure on an earlier quantity"
    );
}
