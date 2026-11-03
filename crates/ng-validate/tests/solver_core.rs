use core::cell::Cell;
use ng_contract::*;
use ng_record::*;
use ng_solver::Core;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3600 * SECOND;
/// A model default, as in tests/chance.rs: the landing tolerance plus what a quadrature books wrong on this case.
const EVENT_TOLERANCE_NS: u64 = 500 * MILLISECOND;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

struct Readings {
    readings: Vec<u64>,
    next: Cell<usize>,
}
impl WallClock for Readings {
    fn elapsed_ns(&self) -> Option<u64> {
        let reading = *self.readings.get(self.next.get())?;
        self.next.set(self.next.get() + 1);
        Some(reading)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

fn to<'m>(session: Session<'m, Running>, end: SimTime) -> Session<'m, Running> {
    match run_until(session, end) {
        Ok(session) => session,
        Err(stopped) => panic!("{:?}", stopped.fault()),
    }
}

#[test]
fn the_stepped_inflow_meets_its_closed_form_at_first_order_over_three_caps() {
    let policy = || {
        let cases = vec![AccuracyCase::TwoReservoir(
            TwoReservoirs::stepped_inflow().expect("a body"),
        )];
        AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy")
    };
    let error = |cap| accuracy(&Core::new(cap), &policy()).expect("within the policy");
    let (coarse, medium, fine) = (error(16 * SECOND), error(8 * SECOND), error(4 * SECOND));
    observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1)).expect("first order");
}

fn ledger_policy(basis: &ConservedBasis, drift: f64) -> CertificatePolicy {
    let quantities = (basis.ids())
        .map(|quantity| QuantityPolicy::new::<Water>(quantity, measure(1e-24), measure(1e-9), measure(drift)))
        .collect::<Result<Vec<_>, _>>()
        .expect("a policy of water");
    let windows = Sliding24Hours::covering(48 * HOUR).expect("a coverage");
    CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows).expect("a policy")
}

#[test]
fn over_48_hours_a_closed_body_holds_still_and_an_open_one_closes() {
    let core = Core::new(60 * SECOND);
    let closed = TwoReservoirs::closed().expect("a body");
    let session = to(
        closed.admit(&core).expect("admitted"),
        SimTime::from_ns(48 * HOUR),
    );
    let view = session.record().conservation();
    certify(&view, &ledger_policy(view.basis(), 1e-6)).expect("a closed body holds still");
    let open = TwoReservoirs::stepped_inflow().expect("a body");
    let session = to(open.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    certify(&view, &ledger_policy(view.basis(), 1e12)).expect("an open body's ledger still closes");
}

fn delivery(id: &'static [u8], from: u64, to: u64) -> Delivery {
    Delivery {
        id,
        from: at(from),
        to: at(to),
        amount: measure(100.0),
    }
}

fn delivering(deliveries: Vec<Delivery>, end: u64) -> TwoReservoirs {
    TwoReservoirs::delivering([measure(500_500.0), measure(499_500.0)], deliveries)
        .expect("a body")
        .until(at(end))
}

#[test]
fn every_replay_gate_passes_on_the_two_reservoir_cases() {
    let off_the_grid = Delivery {
        id: b"off the grid",
        from: SimTime::from_ns(45 * SECOND + 500_000_000),
        to: at(70),
        amount: measure(40.0),
    };
    let busy = vec![
        delivery(b"first", 30, 90),
        delivery(b"beside it", 30, 60),
        off_the_grid,
    ];
    for case in [
        delivering(vec![delivery(b"input", 2_000, 6_000)], 7_000),
        delivering(busy, 120),
    ] {
        let clock = Readings {
            readings: vec![
                0,
                0,
                5 * SECOND,
                5 * SECOND,
                7 * SECOND,
                900 * SECOND,
                90_000 * SECOND,
            ],
            next: Cell::new(0),
        };
        let plan = ReplayPlan {
            clock: &clock,
            simulated_per_wall: 1,
            chunks: vec![3 * SECOND, 11 * SECOND + 1, SECOND / 2],
            resume_every: 100,
            chance: None,
        };
        let evidence = replay_gates(&case, &|| Core::new(8 * SECOND), &plan).expect("every gate");
        let core = Core::new(8 * SECOND);
        let run = case.admit(&core).expect("admitted").record().identity().digest();
        assert_eq!(evidence.run(), run);
    }
}

#[test]
fn every_replay_gate_passes_with_a_stream_and_a_forcing_on() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = Readings {
        readings: vec![0, 40, 90, 170, 900, 90_000],
        next: Cell::new(0),
    };
    let fine = || Core::new(SECOND / 4);
    let plan = ReplayPlan {
        clock: &clock,
        simulated_per_wall: SECOND,
        chunks: vec![7 * SECOND, 13 * SECOND + 1],
        resume_every: 60,
        chance: Some(ChancePlan {
            second_policy: &fine,
            reference: &|| ReferenceIntegrator::new(SECOND / 16),
            event_tolerance_ns: EVENT_TOLERANCE_NS,
        }),
    };
    let evidence = replay_gates(&case, &|| Core::new(SECOND), &plan).expect("every gate");
    assert!(!evidence.of(ReplayGate::Resume).fired().is_empty());
    assert!(!evidence.of(ReplayGate::NonPerturbation).fired().is_empty());
}
