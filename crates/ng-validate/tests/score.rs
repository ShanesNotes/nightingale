use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_record::ConservationView;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const HOUR: u64 = 3_600 * SECOND;
const INTERVAL_NS: u64 = 16 * SECOND;
const SEED: [u64; 4] = [1, 2, 3, 4];
/// The certificate slides 24 h windows, so a run it certifies covers more than one.
const WINDOW_NS: u64 = 25 * HOUR;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn case() -> TwoReservoirs {
    let delivery = Delivery {
        id: b"input",
        from: at(2_000),
        to: at(6_000),
        amount: measure(100.0),
    };
    let start = [measure(500_500.0), measure(499_500.0)];
    let case = TwoReservoirs::delivering(start, vec![delivery]).expect("a body");
    case.until(at(7_000))
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    ReferenceIntegrator::new(interval_ns)
}

struct Ticks(Cell<u64>);
impl WallClock for Ticks {
    fn elapsed_ns(&self) -> Option<u64> {
        self.0.set(self.0.get() + SECOND);
        Some(self.0.get())
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
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

fn told(failure: &ArenaFailure) -> String {
    match failure {
        ArenaFailure::Initialization(failure) => format!("{failure:?}"),
        ArenaFailure::Faulted { report, .. } => format!("{:?}", report.fault()),
        ArenaFailure::Checkpoint(error) => format!("{error:?}"),
        ArenaFailure::Observation(refusal) => format!("{refusal:?}"),
        ArenaFailure::MissingThroughput => "missing throughput".into(),
        ArenaFailure::RepeatDiffered { repeat } => format!("repeat {repeat} differed"),
        ArenaFailure::MissingEvidence { evidence } => format!("missing {evidence:?} evidence"),
    }
}

fn arena<'m>(
    case: &'m TwoReservoirs,
    core: &'m ReferenceIntegrator,
    store: &'m MemoryStore,
    seed: [u64; 4],
    window_ns: u64,
) -> ArenaRun<'m> {
    let body = case.body();
    let channels = (body.rows().iter()).map(|row| ChannelSpec::new("every row", row.kind()));
    let lens = (body.lens(channels.collect(), NonZeroU64::new(HOUR).expect("an hour"))).expect("a lens");
    let spec = ArenaSpec {
        window: Interval::new(at(0), SimTime::from_ns(window_ns)).expect("a window"),
        packet: case.packet().expect("the case's packet"),
        lens,
        boundary: BoundaryPort::new(case.events().expect("its inputs")).expect("a port"),
        seed: Seed::new(seed),
        repeats: NonZeroU32::MIN,
    };
    run(body, core, spec, &Ticks(Cell::new(0)), store).unwrap_or_else(|failure| panic!("{}", told(&failure)))
}

fn limits() -> (Measure<Water>, Measure<Dimensionless>, Measure<Water>) {
    (measure(1e-12), measure(1e-9), measure(1e3))
}

fn certified(view: &ConservationView<'_>) -> Certificate {
    let quantities = (view.basis().ids()).map(|quantity| {
        let (floor, relative, drift) = limits();
        QuantityPolicy::new(quantity, floor, relative, drift)
    });
    let quantities = quantities
        .collect::<Result<Vec<_>, _>>()
        .expect("a policy of water");
    let windows = Sliding24Hours::covering(WINDOW_NS).expect("a coverage");
    let policy =
        CertificatePolicy::new(view.basis(), quantities, ExpectedDrift::Zero, windows).expect("a policy");
    certify(view, &policy).expect("the ledger closes and the body holds")
}

fn checked(interval_ns: u64) -> AccuracyEvidence {
    let policy = AccuracyPolicy::manufactured().expect("the model default");
    accuracy(&reference(interval_ns), &policy).expect("within the policy")
}

fn replayed(case: &TwoReservoirs, interval_ns: u64) -> ReplayEvidence {
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
    replay_gates(case, &move || reference(interval_ns), &plan).expect("every gate")
}

fn missing(scored: Result<ArenaScore, ArenaFailure>) -> Option<Evidence> {
    match scored {
        Err(ArenaFailure::MissingEvidence { evidence }) => Some(evidence),
        Err(failure) => panic!("{}", told(&failure)),
        Ok(_) => None,
    }
}

#[test]
fn a_score_holds_one_runs_four_parts_and_reports_every_policy_value_they_rest_on() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, SEED, WINDOW_NS);
    let certificate = certified(&ran.session().conservation());
    let scored = score(
        certificate,
        checked(INTERVAL_NS),
        ran.into_evidence(),
        replayed(&case, INTERVAL_NS),
    );
    let scored = scored.unwrap_or_else(|failure| panic!("{}", told(&failure)));
    let evidence = scored.evidence();
    assert_eq!(scored.certificate().seal(), evidence.seal());
    assert_eq!(scored.replay().run(), evidence.run());
    assert_eq!(scored.accuracy().resolution(), evidence.resolution().digest());

    let bits = |value: Compensated| value.bits();
    let (floor, relative, drift) = limits();
    assert_eq!(scored.certificate().windows().coverage_ns(), WINDOW_NS);
    assert!(!scored.certificate().results().is_empty());
    for result in scored.certificate().results() {
        assert_eq!(bits(result.closure_floor()), bits(Compensated::of(&floor)));
        assert_eq!(bits(result.closure_relative()), bits(Compensated::of(&relative)));
        assert_eq!(bits(result.drift_limit()), bits(Compensated::of(&drift)));
    }
    let checked = scored.accuracy();
    assert_eq!(
        bits(checked.absolute()),
        bits(Compensated::of(&measure::<Water>(5.0)))
    );
    assert_eq!(
        bits(checked.relative()),
        bits(Compensated::of(&measure::<Dimensionless>(0.0)))
    );
    assert_eq!(checked.grid_ns(), 100 * SECOND);
    let reference_scales = ErrorScales::uniform(measure(1.0), measure(1e-5)).expect("positive");
    assert_eq!(evidence.resolution().scales(), &reference_scales);
    assert_eq!(evidence.resolution().minimum_interval_ns().get(), 1_000_000);
    assert_eq!(scored.replay().of(ReplayGate::StepPolicy).tolerance_ns(), None);
}

#[test]
fn a_certificate_of_another_run_or_of_the_same_run_at_another_head_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let evidence = arena(&case, &core, &store, SEED, WINDOW_NS).into_evidence();
    let replay = replayed(&case, INTERVAL_NS);
    let reseeded = arena(&case, &core, &store, [5, 6, 7, 8], WINDOW_NS);
    let longer = arena(&case, &core, &store, SEED, WINDOW_NS + HOUR);
    for other in [reseeded, longer] {
        let certificate = certified(&other.session().conservation());
        let scored = score(
            certificate,
            checked(INTERVAL_NS),
            evidence.clone(),
            replay.clone(),
        );
        assert_eq!(missing(scored), Some(Evidence::Certificate));
    }
}

#[test]
fn run_evidence_of_another_run_is_missing_every_part_of_this_one() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, SEED, WINDOW_NS);
    let certificate = certified(&ran.session().conservation());
    let replay = replayed(&case, INTERVAL_NS);
    let other = arena(&case, &core, &store, [5, 6, 7, 8], WINDOW_NS);
    let its_certificate = certified(&other.session().conservation());
    let other = other.into_evidence();
    let scored = score(certificate, checked(INTERVAL_NS), other.clone(), replay.clone());
    assert_eq!(missing(scored), Some(Evidence::Certificate));
    let scored = score(its_certificate, checked(INTERVAL_NS), other, replay);
    assert_eq!(missing(scored), Some(Evidence::Replay));
}

#[test]
fn accuracy_evidence_of_another_core_or_another_policy_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, SEED, WINDOW_NS);
    let certificates = [(); 3].map(|()| certified(&ran.session().conservation()));
    let (evidence, replay) = (ran.into_evidence(), replayed(&case, INTERVAL_NS));
    let mut without_independence = AccuracyCase::manufactured().expect("every case builds");
    without_independence.pop();
    let fewer = AccuracyPolicy::new(without_independence, measure(5.0), measure(0.0), 100 * SECOND);
    let every = AccuracyCase::manufactured().expect("every case builds");
    let looser = AccuracyPolicy::new(every, measure(6.0), measure(0.0), 100 * SECOND);
    let others = [
        checked(INTERVAL_NS / 2),
        accuracy(&core, &fewer.expect("a policy")).expect("within the policy"),
        accuracy(&core, &looser.expect("a policy")).expect("within the policy"),
    ];
    for (certificate, other) in certificates.into_iter().zip(others) {
        let scored = score(certificate, other, evidence.clone(), replay.clone());
        assert_eq!(missing(scored), Some(Evidence::Accuracy));
    }
}

#[test]
fn replay_evidence_of_another_run_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, SEED, WINDOW_NS);
    let certificate = certified(&ran.session().conservation());
    let other = replayed(&case, INTERVAL_NS / 2);
    let scored = score(certificate, checked(INTERVAL_NS), ran.into_evidence(), other);
    assert_eq!(missing(scored), Some(Evidence::Replay));
}
