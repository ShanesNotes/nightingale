use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_record::ConservationView;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000_000_000;
const HOUR: u64 = 3_600 * SECOND;
const INTERVAL_NS: u64 = 16 * SECOND;
const END_NS: u64 = 7_000 * SECOND;

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
    case.until(SimTime::from_ns(END_NS))
}

/// Scales this loose never refuse, so on the slice the core takes the steps of the window's coarsest reference.
fn reference(interval_ns: u64) -> ReferenceIntegrator {
    let loose = ErrorScales::uniform(measure(1e9), measure(1.0)).expect("positive");
    let floor = NonZeroU64::new(100_000).expect("a floor");
    ReferenceIntegrator::new(interval_ns)
        .with_scales(loose)
        .with_floor(floor)
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
        ArenaFailure::MissingEvidence { part } => format!("missing {part:?} evidence"),
    }
}

fn arena<'m>(
    case: &'m TwoReservoirs,
    core: &'m ReferenceIntegrator,
    store: &'m MemoryStore,
    seed: Seed,
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
        seed,
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
    let windows = SlidingWindows::new(NonZeroU64::new(HOUR).expect("an hour"), END_NS).expect("a coverage");
    let policy =
        CertificatePolicy::new(view.basis(), quantities, ExpectedDrift::Zero, windows).expect("a policy");
    certify(view, &policy).expect("the ledger closes and the body holds")
}

fn without_the_window(interval_ns: u64) -> AccuracyEvidence {
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

fn missing(scored: Result<ArenaScore, ArenaFailure>) -> Option<ScorePart> {
    match scored {
        Err(ArenaFailure::MissingEvidence { part }) => Some(part),
        Err(failure) => panic!("{}", told(&failure)),
        Ok(_) => None,
    }
}

#[test]
fn a_score_holds_one_runs_parts_under_the_arena_policy_and_refuses_another_cores_accuracy() {
    let policy = AccuracyPolicy::arena().expect("the reference runs the slice window");
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, case.seed(), END_NS);
    let [certificate, again] = [(); 2].map(|()| certified(&ran.session().conservation()));
    let (evidence, replay) = (ran.into_evidence(), replayed(&case, INTERVAL_NS));
    let another = accuracy(&reference(INTERVAL_NS / 2), &policy).expect("within the arena policy");
    let refused = score(again, another, evidence.clone(), replay.clone());
    assert_eq!(missing(refused), Some(ScorePart::Accuracy));
    let checked = accuracy(&core, &policy).expect("within the arena policy");
    let scored = score(certificate, checked, evidence, replay);
    let scored = scored.unwrap_or_else(|failure| panic!("{}", told(&failure)));
    let evidence = scored.evidence();
    assert_eq!(scored.certificate().seal(), evidence.seal());
    assert_eq!(scored.replay().run(), evidence.run());
    assert_eq!(scored.accuracy().resolution(), evidence.resolution().digest());
    assert_eq!(scored.accuracy().policy(), policy.digest());
    assert_eq!(scored.accuracy().worst().len(), 6);

    let bits = |value: Compensated| value.bits();
    let (floor, relative, _) = limits();
    let windows = scored.certificate().windows();
    assert_eq!((windows.window_ns().get(), windows.coverage_ns()), (HOUR, END_NS));
    assert!(!scored.certificate().results().is_empty());
    for result in scored.certificate().results() {
        assert_eq!(bits(result.closure_floor()), bits(Compensated::of(&floor)));
        assert_eq!(bits(result.closure_relative()), bits(Compensated::of(&relative)));
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
    let loose = ErrorScales::uniform(measure(1e9), measure(1.0)).expect("positive");
    let [window] = checked.windows() else {
        panic!("the arena runs one slice window");
    };
    assert_eq!(window.margin(), 2);
    assert_eq!(
        bits(window.distance_floor()),
        bits(Compensated::of(&measure::<Dimensionless>(1e-6)))
    );
    assert_eq!(window.landing_ns().get(), 100_000);
    assert_eq!(window.grid_ns(), 10 * MILLISECOND);
    assert_eq!(window.past_end_ns(), 500 * MILLISECOND);
    let longest = arena_body().expect("the slice compiles").longest_step_ns();
    assert_eq!(window.refinements_ns(), [longest, longest / 4, longest / 16]);
    assert_eq!(window.refinement_scales(), loose);
    assert_eq!(evidence.resolution().scales(), &loose);
    assert_eq!(evidence.resolution().minimum_interval_ns().get(), 100_000);
    assert_eq!(scored.replay().of(ReplayGate::StepPolicy).tolerance_ns(), None);
}

#[test]
fn a_certificate_of_another_run_or_of_the_same_run_at_another_head_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let evidence = arena(&case, &core, &store, case.seed(), END_NS).into_evidence();
    let replay = replayed(&case, INTERVAL_NS);
    let reseeded = arena(&case, &core, &store, Seed::new([5, 6, 7, 8]), END_NS);
    let longer = arena(&case, &core, &store, case.seed(), END_NS + HOUR);
    for other in [reseeded, longer] {
        let certificate = certified(&other.session().conservation());
        let scored = score(
            certificate,
            without_the_window(INTERVAL_NS),
            evidence.clone(),
            replay.clone(),
        );
        assert_eq!(missing(scored), Some(ScorePart::Certificate));
    }
}

#[test]
fn run_evidence_of_another_run_is_missing_every_part_of_this_one() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, case.seed(), END_NS);
    let certificate = certified(&ran.session().conservation());
    let replay = replayed(&case, INTERVAL_NS);
    let other = arena(&case, &core, &store, Seed::new([5, 6, 7, 8]), END_NS);
    let its_certificate = certified(&other.session().conservation());
    let other = other.into_evidence();
    let scored = score(
        certificate,
        without_the_window(INTERVAL_NS),
        other.clone(),
        replay.clone(),
    );
    assert_eq!(missing(scored), Some(ScorePart::Certificate));
    let scored = score(its_certificate, without_the_window(INTERVAL_NS), other, replay);
    assert_eq!(missing(scored), Some(ScorePart::Replay));
}

#[test]
fn accuracy_evidence_that_never_ran_the_slice_window_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, case.seed(), END_NS);
    let certificate = certified(&ran.session().conservation());
    let scored = score(
        certificate,
        without_the_window(INTERVAL_NS),
        ran.into_evidence(),
        replayed(&case, INTERVAL_NS),
    );
    assert_eq!(missing(scored), Some(ScorePart::Accuracy));
}

#[test]
fn replay_evidence_of_another_run_is_missing() {
    let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
    let ran = arena(&case, &core, &store, case.seed(), END_NS);
    let certificate = certified(&ran.session().conservation());
    let other = replayed(&case, INTERVAL_NS / 2);
    let scored = score(
        certificate,
        without_the_window(INTERVAL_NS),
        ran.into_evidence(),
        other,
    );
    assert_eq!(missing(scored), Some(ScorePart::Replay));
}
