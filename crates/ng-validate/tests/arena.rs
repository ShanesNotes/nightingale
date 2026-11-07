use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_solver::Core;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;

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

/// Scales this loose never refuse, and the floor is the slice window's landing tolerance.
fn loose(interval_ns: u64) -> ReferenceIntegrator {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    ReferenceIntegrator::new(interval_ns)
        .with_scales(ErrorScales::uniform(scale(1e9), scale(1.0)).expect("positive"))
        .with_floor(NonZeroU64::new(100_000).expect("a floor"))
}

fn short() -> Protocol {
    let window = NonZeroU64::new(SECOND).expect("a second");
    Protocol {
        windows: SlidingWindows::new(window, 2 * SECOND).expect("two seconds cover one"),
        repeats: NonZeroU32::MIN,
    }
}

/// Every part comes from the one call, and each is held against the run the arena timed.
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

/// The manufactured cases run at a core's own step, so both cores cap at 16 s; on the slice the body's longest step
/// caps every ticket at 10 ms, and each second policy at 5 ms runs a grid of its own. The second core's clock started
/// a day before the arena, as a host's may.
#[test]
fn one_arena_scores_the_reference_and_ng_solver_with_one_call_each() {
    let arena = Arena::new(short()).expect("the arena builds");
    assert_eq!(arena.protocol(), short());
    let reference: [&dyn CoreFactory; 2] = [&|| loose(16 * SECOND), &|| loose(5 * MILLISECOND)];
    let solver: [&dyn CoreFactory; 2] = [&|| Core::new(16 * SECOND), &|| Core::new(5 * MILLISECOND)];
    let started = [0, 86_400 * SECOND];
    let scores = [(reference, started[0]), (solver, started[1])].map(|([cores, second_policy], started)| {
        let (clock, store) = (Ticks(Cell::new(started)), MemoryStore::default());
        arena.score(cores, second_policy, &clock, &store)
    });
    scores.iter().for_each(holds_every_part_apart_and_of_one_run);
    let [first, second] = scores
        .each_ref()
        .map(|score| score.evidence().map(RunEvidence::run).ok());
    assert_ne!(first, second, "two cores are two runs");
}
