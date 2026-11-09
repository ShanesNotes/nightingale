use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_solver::Core;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const SLICES_LONGEST_STEP: u64 = 10 * MILLISECOND;
const PAST_THE_SLICES_LONGEST_STEP: u64 = 16 * SECOND;
const UNDER_THE_SLICES_LONGEST_STEP: u64 = SLICES_LONGEST_STEP / 2;
const A_DAY_EARLY: u64 = 86_400 * SECOND;

const GATES: [ReplayGate; 7] = [
    ReplayGate::Pace,
    ReplayGate::Resume,
    ReplayGate::LateAdmission,
    ReplayGate::Rewind,
    ReplayGate::ExtraEvaluation,
    ReplayGate::StepPolicy,
    ReplayGate::NonPerturbation,
];

struct Ticks(Cell<u64>);
impl WallClock for Ticks {
    fn elapsed_ns(&self) -> Option<u64> {
        self.0.set(self.0.get() + 90 * MILLISECOND);
        Some(self.0.get())
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

struct Stopped;
impl WallClock for Stopped {
    fn elapsed_ns(&self) -> Option<u64> {
        None
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    let pick = ArenaProtocol::pick();
    ReferenceIntegrator::new(interval_ns)
        .with_scales(pick.scales)
        .with_floor(pick.minimum_interval_ns)
}

fn short() -> ArenaProtocol {
    let window = NonZeroU64::new(SECOND).expect("a second");
    ArenaProtocol {
        windows: SlidingWindows::new(window, 2 * SECOND).expect("two seconds cover one"),
        repeats: NonZeroU32::MIN,
        ..ArenaProtocol::pick()
    }
}

fn holds_every_part_apart_and_of_one_run(score: &ArenaScore) {
    let protocol = score.protocol();
    let evidence = score.evidence().expect("the timed run");
    let throughput = evidence.throughput();
    assert_eq!(throughput.clock(), ClockSource::Scripted);
    assert_eq!(throughput.runs_wall_ns().len(), 1);
    assert!(throughput.simulated_ns() >= protocol.windows.coverage_ns());
    assert!(throughput.accepted_steps() > 0 && throughput.waveform_samples() > 0);
    assert_eq!(evidence.health().floor_faults(), 0);
    assert_ne!(score.second_policy().digest(), evidence.resolution().digest());

    let certificate = score.certificate().expect("the slice closes and holds");
    assert_eq!(certificate.seal(), evidence.seal());
    assert_eq!(certificate.windows(), protocol.windows);
    assert_eq!(certificate.span().start(), anchor_moment());
    assert!(!certificate.results().is_empty());

    let accuracy = score.accuracy().expect("within the arena's policy");
    assert_eq!(accuracy.resolution(), evidence.resolution().digest());
    assert_eq!((accuracy.worst().len(), accuracy.windows().len()), (6, 1));

    let replay = score.replay().expect("every replay gate");
    assert_eq!(replay.run(), evidence.run());
    let fence = anchor_moment().nanoseconds() + SECOND;
    for gate in GATES {
        let of = replay.of(gate);
        assert_eq!(of.run(), evidence.run(), "{gate:?}");
        let [stir] = of.fired() else {
            panic!("{gate:?} fired {:?}", of.fired());
        };
        assert!(stir.at().nanoseconds() < fence, "{gate:?}");
    }
    assert_eq!(replay.of(ReplayGate::Pace).clock(), Some(ClockSource::Scripted));
    assert_eq!(replay.of(ReplayGate::StepPolicy).tolerance_ns(), Some(100_000));

    let day = score.day().expect("the day checks ran");
    assert_eq!((day.run(), day.seal()), (evidence.run(), evidence.seal().base));
    let mut rows: Vec<u8> = day.checks().iter().map(DayCheck::row).collect();
    rows.dedup();
    assert_eq!(rows, (1..=17).collect::<Vec<u8>>());
    let scored = |check: &DayCheck| !matches!(check.verdict(), Verdict::NotAvailableYet(_));
    assert!(
        !day.checks().iter().any(scored),
        "two seconds from 06:55 reach no event\n{day}"
    );
}

#[test]
fn one_arena_scores_the_reference_and_ng_solver_with_one_call_each() {
    let arena = Arena::new(short()).expect("the arena builds");
    assert_eq!(arena.protocol(), short());
    let clock = |started| Ticks(Cell::new(started));
    let stores = [MemoryStore::default(), MemoryStore::default()];
    let scores = [
        arena.score(
            || reference(PAST_THE_SLICES_LONGEST_STEP),
            || reference(UNDER_THE_SLICES_LONGEST_STEP),
            &clock(0),
            &stores[0],
        ),
        arena.score(
            || Core::new(PAST_THE_SLICES_LONGEST_STEP),
            || Core::new(UNDER_THE_SLICES_LONGEST_STEP),
            &clock(A_DAY_EARLY),
            &stores[1],
        ),
    ]
    .map(|score| score.expect("both cores declare the protocol's scales and interval"));
    scores.iter().for_each(holds_every_part_apart_and_of_one_run);
    let [first, second] = scores
        .each_ref()
        .map(|score| score.evidence().map(RunEvidence::run).ok());
    assert_ne!(first, second, "two cores are two runs");

    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let looser = ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive");
    let coarser = NonZeroU64::new(MILLISECOND).expect("a floor");
    let refused = [
        arena.score(
            || reference(PAST_THE_SLICES_LONGEST_STEP).with_scales(looser),
            || reference(UNDER_THE_SLICES_LONGEST_STEP),
            &Ticks(Cell::new(0)),
            &MemoryStore::default(),
        ),
        arena.score(
            || reference(PAST_THE_SLICES_LONGEST_STEP),
            || reference(UNDER_THE_SLICES_LONGEST_STEP).with_floor(coarser),
            &Ticks(Cell::new(0)),
            &MemoryStore::default(),
        ),
    ];
    for score in refused {
        assert!(
            matches!(score, Err(ArenaFailure::OffProtocol(_))),
            "refused whole"
        );
    }
}

#[test]
fn past_the_sit_up_the_day_checks_end_on_the_timed_runs_seal() {
    let span = 311 * SECOND;
    let window = NonZeroU64::new(span / 2).expect("half the span");
    let protocol = ArenaProtocol {
        windows: SlidingWindows::new(window, span).expect("the span covers two windows"),
        repeats: NonZeroU32::MIN,
        ..ArenaProtocol::pick()
    };
    let arena = Arena::new(protocol).expect("the arena builds");
    let score = arena.score(
        || Core::new(PAST_THE_SLICES_LONGEST_STEP),
        || Core::new(UNDER_THE_SLICES_LONGEST_STEP),
        &Ticks(Cell::new(0)),
        &MemoryStore::default(),
    );
    let score = score.expect("the core declares the protocol's scales and interval");
    let day = score.day().expect("the fed run ended on the timed run's seal");
    let seven = SimTime::from_ns(anchor_moment().nanoseconds() + 300 * SECOND);
    let sat_up = |check: &&DayCheck| check.row() == 1;
    let at_seven = day
        .checks()
        .iter()
        .filter(sat_up)
        .all(|check| check.at() == Some(seven));
    assert!(at_seven, "{day}");
    score.replay().expect("every replay gate");
    score.accuracy().expect("within the arena's policy");
}

#[test]
fn a_clock_with_no_reading_leaves_the_day_checks_and_accuracy_standing() {
    let arena = Arena::new(short()).expect("the arena builds");
    let score = arena.score(
        || reference(PAST_THE_SLICES_LONGEST_STEP),
        || reference(UNDER_THE_SLICES_LONGEST_STEP),
        &Stopped,
        &MemoryStore::default(),
    );
    let score = score.expect("the core declares the protocol's scales and interval");
    assert!(matches!(score.evidence(), Err(ArenaFailure::MissingThroughput)));
    let certificate = score.certificate();
    let part = ScorePart::Certificate;
    assert!(matches!(certificate, Err(ArenaFailure::MissingEvidence { part: missing }) if *missing == part));
    let replay = score.replay();
    let gate = ReplayGate::Pace;
    let pace = ReplayFailure::MissingEvidence { gate };
    assert!(matches!(replay, Err(ArenaFailure::Replay(failure)) if *failure == pace));
    let day = score
        .day()
        .expect("the day checks need neither the clock nor the store");
    let mut rows: Vec<u8> = day.checks().iter().map(DayCheck::row).collect();
    rows.dedup();
    assert_eq!(rows, (1..=17).collect::<Vec<u8>>());
    score.accuracy().expect("within the arena's policy");
}

const DAY: u64 = 24 * 3_600 * SECOND;

#[test]
fn the_pick_protocol_is_one_24_hour_window_timed_twice_and_the_milestone_every_one_over_48_hours() {
    let (pick, milestone) = (ArenaProtocol::pick(), ArenaProtocol::milestone());
    let shape = |protocol: ArenaProtocol| {
        let windows = protocol.windows;
        (
            windows.window_ns().get(),
            windows.coverage_ns(),
            protocol.repeats.get(),
        )
    };
    assert_eq!(shape(pick), (DAY, DAY, 2));
    assert_eq!(shape(milestone), (DAY, 2 * DAY, 2));
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let model_defaults = ErrorScales::uniform(scale(0.1), scale(1e-3)).expect("positive");
    assert_eq!(
        (pick.scales, pick.minimum_interval_ns.get()),
        (model_defaults, 100_000)
    );
    assert_eq!(
        ArenaProtocol {
            windows: pick.windows,
            ..milestone
        },
        pick,
        "the two differ only in what they cover"
    );
}

struct OnlyDeclares(DeclaredResolution);
impl CoupledSolver for OnlyDeclares {
    fn resolution(&self) -> DeclaredResolution {
        self.0.clone()
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        panic!("a core off the protocol never steps")
    }
    fn solve(
        &self,
        _problem: &CoupledProblem<'_>,
        _ticket: &StepTicket,
    ) -> Result<StepProposal, TrialFailure> {
        panic!("a core off the protocol never steps")
    }
}

#[test]
fn a_core_off_either_protocol_is_refused_whole_before_it_steps() {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let looser = ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive");
    let coarser = NonZeroU64::new(MILLISECOND).expect("a floor");
    let declared = |core: ReferenceIntegrator| move || OnlyDeclares(core.resolution().clone());
    let on = || reference(PAST_THE_SLICES_LONGEST_STEP);
    for protocol in [ArenaProtocol::pick(), ArenaProtocol::milestone()] {
        let arena = Arena::new(protocol).expect("the arena builds");
        let refused = [
            arena.score(
                declared(on().with_scales(looser)),
                declared(on()),
                &Ticks(Cell::new(0)),
                &MemoryStore::default(),
            ),
            arena.score(
                declared(on()),
                declared(on().with_floor(coarser)),
                &Ticks(Cell::new(0)),
                &MemoryStore::default(),
            ),
        ];
        for score in refused {
            assert!(
                matches!(score, Err(ArenaFailure::OffProtocol(_))),
                "refused whole under {protocol:?}"
            );
        }
    }
}
