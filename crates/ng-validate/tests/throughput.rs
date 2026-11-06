use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_record::ObservationRefusal;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const MICROSECOND: u64 = 1_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn wall(ns: u64) -> NonZeroU64 {
    NonZeroU64::new(ns).expect("positive")
}

struct Scripted {
    readings: Vec<u64>,
    reads: Cell<usize>,
}
impl Scripted {
    fn new(readings: &[u64]) -> Self {
        Self {
            readings: readings.to_vec(),
            reads: Cell::new(0),
        }
    }
    fn ticking() -> Self {
        Self::new(&(0..64).map(|tick| tick * SECOND).collect::<Vec<_>>())
    }
}
impl WallClock for Scripted {
    fn elapsed_ns(&self) -> Option<u64> {
        let at = self.reads.get();
        self.reads.set(at + 1);
        self.readings.get(at).copied()
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

fn told(failure: &ArenaFailure) -> String {
    match failure {
        ArenaFailure::Initialization(failure) => format!("{failure:?}"),
        ArenaFailure::Faulted(report) => format!("{:?}", report.fault()),
        ArenaFailure::Checkpoint(error) => format!("{error:?}"),
        ArenaFailure::Observation(refusal) => format!("{refusal:?}"),
        ArenaFailure::MissingThroughput => "missing throughput".into(),
        ArenaFailure::RepeatDiffered { repeat } => format!("repeat {repeat} differed"),
    }
}

fn ran(result: Result<ArenaRun<'_>, ArenaFailure>) -> ArenaRun<'_> {
    result.unwrap_or_else(|failure| panic!("{}", told(&failure)))
}

#[test]
fn a_short_slice_window_reports_throughput_beside_the_record() {
    let body = arena_body().expect("compiled");
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(5 * MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"));
    let store = MemoryStore::default();
    let end = SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND);
    let spec = ArenaSpec {
        window: Interval::new(anchor_moment(), end).expect("an interval"),
        packet: arena_packet(&body).expect("a packet"),
        lens: arena_lens(&body).expect("a lens"),
        boundary: BoundaryPort::new(Vec::new()).expect("no event"),
        seed: Seed::new([1, 2, 3, 4]),
        repeats: NonZeroU32::new(2).expect("positive"),
    };
    let clock = Scripted::new(&[0, 7 * SECOND, 8 * SECOND, 10 * SECOND, 11 * SECOND, 14 * SECOND]);
    let arena = ran(run(&body, &core, spec, &clock, &store));
    let record = arena.session().record();
    let (head, revision) = (record.accepted().time(), record.revision());
    let identity = record.identity().digest();
    let evidence = arena.into_evidence();
    let throughput = evidence.throughput();
    assert_eq!(clock.reads.get(), 6);
    assert_eq!(throughput.clock(), ClockSource::Scripted);
    assert_eq!(throughput.admission_wall_ns(), 7 * SECOND);
    assert_eq!(throughput.runs_wall_ns(), [wall(2 * SECOND), wall(3 * SECOND)]);
    assert!(head >= end);
    assert!((2 * SECOND..2 * SECOND + 5 * MILLISECOND).contains(&throughput.simulated_ns()));
    assert_eq!(throughput.accepted_steps(), revision);
    // Each of the lens's three channels closes a one-second window at admission, which saw only that instant and
    // reads as not collected, and one at each of the two whole seconds after it.
    assert_eq!(throughput.waveform_samples(), 3 * 3);
    assert_eq!(evidence.run(), identity);
}

/// One vessel fed by a water boundary that nothing drives, read by a lens on a 100 ms grid.
struct Small {
    body: CompiledBody,
    amount: RowKind,
    reading: RowKind,
}
impl Small {
    fn new(vessel: &'static str) -> Self {
        let mut map = AnatomyBuilder::new();
        let at = Position::new(measure(0.0), measure(0.0), measure(0.0));
        let site = map.site::<VesselSegment>(vessel, at);
        let mut schema = Schema::new(map.finish().expect("a map")).expect("a schema");
        (schema.boundary::<Water>("in", BoundaryDirection::IntoBody, site.id(), Space::Lumen))
            .expect("a boundary");
        let body = assemble(schema, |mut body| {
            let (mut laws, ports) = body.boundary()?.parts();
            let BoundaryRoute { route, drive } = ports.route::<Water>("in")?;
            let class = RateClass::new(RateBand::Minutes, 60 * SECOND)?;
            let flow = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, flow));
            let reading = laws.algebraic::<Rate<Water>>("reading", Admissible::finite());
            let residual = laws.kernel(|mut k| {
                let (x, input) = (k.algebraic(&reading), k.read(&drive));
                let residual = k.subtract(x, input);
                k.finish(residual, laws.every_mode(), class)
            });
            laws.residual(&reading, residual);
            body.add(laws.finish());
            body.compile()
        });
        let local = LawName {
            owner: "Boundary",
            local: "reading",
        };
        Self {
            body: body.expect("compiled"),
            amount: RowKind::Amount {
                site: site.id(),
                space: Space::Lumen,
                species: Species::Water,
            },
            reading: RowKind::Algebraic(local.into()),
        }
    }
    fn lens(&self) -> ObservationLens {
        let channels = vec![ChannelSpec::new("synthetic gauge", self.reading)];
        (self.body.lens(channels, wall(100 * MILLISECOND))).expect("a lens")
    }
    fn spec(&self, lens: ObservationLens, from_ms: u64, to_ms: u64, repeats: u32) -> ArenaSpec {
        let mut packet = ParameterPacket::new();
        packet.within(admission_tolerance());
        (packet.start(self.amount, measure::<Water>(10.0), "synthetic")).expect("a start");
        (packet.start(self.reading, measure::<Rate<Water>>(0.0), "synthetic")).expect("a start");
        let at = |ms| SimTime::from_ns(ms * MILLISECOND);
        ArenaSpec {
            window: Interval::new(at(from_ms), at(to_ms)).expect("an interval"),
            packet,
            lens,
            boundary: BoundaryPort::new(Vec::new()).expect("no event"),
            seed: Seed::new([1, 2, 3, 4]),
            repeats: NonZeroU32::new(repeats).expect("positive"),
        }
    }
}

fn small_core() -> ReferenceIntegrator {
    ReferenceIntegrator::new(50 * MILLISECOND)
}

struct Counted<'c> {
    inner: ReferenceIntegrator,
    solves: &'c Cell<u64>,
}
impl CoupledSolver for Counted<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.solves.set(self.solves.get() + 1);
        self.inner.solve(problem, ticket)
    }
}

/// Wall time that moves only when the core solves.
struct BySolve<'c> {
    solves: &'c Cell<u64>,
}
impl WallClock for BySolve<'_> {
    fn elapsed_ns(&self) -> Option<u64> {
        Some(self.solves.get() * MICROSECOND)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

#[test]
fn every_solve_of_a_repeat_falls_inside_its_wall_time_and_none_inside_admission() {
    let small = Small::new("synthetic vessel");
    let solves = Cell::new(0);
    let core = Counted {
        inner: small_core(),
        solves: &solves,
    };
    let store = MemoryStore::default();
    let clock = BySolve { solves: &solves };
    let spec = small.spec(small.lens(), 0, 1000, 3);
    let arena = ran(run(&small.body, &core, spec, &clock, &store));
    let throughput = arena.into_evidence().throughput().clone();
    let runs = throughput.runs_wall_ns();
    assert_eq!(throughput.admission_wall_ns(), 0);
    assert_eq!(runs.len(), 3);
    assert!(runs.iter().all(|repeat| *repeat == runs[0]));
    let summed: u64 = runs.iter().map(|repeat| repeat.get()).sum();
    assert_eq!(summed, solves.get() * MICROSECOND);
}

#[test]
fn a_throughput_that_measured_nothing_is_missing() {
    let small = Small::new("synthetic vessel");
    let clocks: [(&str, &[u64]); 5] = [
        ("no reading", &[]),
        ("stops after admission", &[0, 1]),
        ("runs backward over admission", &[2, 1]),
        ("stands still over a repeat", &[0, 1, 2, 2]),
        ("runs backward over a repeat", &[0, 1, 3, 2]),
    ];
    for (case, readings) in clocks {
        let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::new(readings));
        let spec = small.spec(small.lens(), 0, 1000, 1);
        let missing = run(&small.body, &core, spec, &clock, &store);
        assert!(matches!(missing, Err(ArenaFailure::MissingThroughput)), "{case}");
    }
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let between_grid_instants = small.spec(small.lens(), 10, 60, 1);
    let unread = run(&small.body, &core, between_grid_instants, &clock, &store);
    assert!(matches!(unread, Err(ArenaFailure::MissingThroughput)));
}

/// Its resolution stays its inner core's, so every repeat resumes; its span halves once the first repeat's ten
/// steps are taken.
struct Shifting {
    inner: ReferenceIntegrator,
    steps: Cell<u64>,
}
impl CoupledSolver for Shifting {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.steps.set(self.steps.get() + 1);
        let span = self.inner.next_step(accepted);
        if self.steps.get() > 10 { span / 2 } else { span }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn the_first_repeat_on_another_course_than_repeat_zero_is_named() {
    let small = Small::new("synthetic vessel");
    let core = Shifting {
        inner: ReferenceIntegrator::new(100 * MILLISECOND),
        steps: Cell::new(0),
    };
    let (store, clock) = (MemoryStore::default(), Scripted::ticking());
    let spec = small.spec(small.lens(), 0, 1000, 3);
    let differed = run(&small.body, &core, spec, &clock, &store);
    assert!(matches!(
        differed,
        Err(ArenaFailure::RepeatDiffered { repeat: 1 })
    ));
}

#[test]
fn a_lens_compiled_on_another_body_is_refused() {
    let (small, other) = (Small::new("synthetic vessel"), Small::new("another vessel"));
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let spec = small.spec(other.lens(), 0, 1000, 1);
    let refused = run(&small.body, &core, spec, &clock, &store);
    assert!(matches!(
        refused,
        Err(ArenaFailure::Observation(ObservationRefusal::OtherLens))
    ));
}

#[test]
fn the_specs_seed_names_the_run_it_times() {
    let small = Small::new("synthetic vessel");
    let named = |seed| {
        let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
        let mut spec = small.spec(small.lens(), 0, 1000, 1);
        spec.seed = seed;
        ran(run(&small.body, &core, spec, &clock, &store)).into_evidence().run()
    };
    let first = named(Seed::new([1, 2, 3, 4]));
    assert_eq!(first, named(Seed::new([1, 2, 3, 4])));
    assert_ne!(first, named(Seed::new([4, 3, 2, 1])));
}

/// The clock opens before admission and closes after it, so a refused admission has read it once.
#[test]
fn a_refused_admission_reads_the_clock_once() {
    let small = Small::new("synthetic vessel");
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let mut spec = small.spec(small.lens(), 0, 1000, 1);
    spec.packet = ParameterPacket::new();
    let refused = run(&small.body, &core, spec, &clock, &store);
    assert!(matches!(refused, Err(ArenaFailure::Initialization(_))));
    assert_eq!(clock.reads.get(), 1);
}
