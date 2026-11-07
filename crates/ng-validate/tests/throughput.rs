use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_record::{ObservationRefusal, Running, Session, Step};
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
        ArenaFailure::Faulted { report, .. } => format!("{:?}", report.fault()),
        ArenaFailure::Checkpoint(error) => format!("{error:?}"),
        ArenaFailure::Observation(refusal) => format!("{refusal:?}"),
        ArenaFailure::MissingThroughput => "missing throughput".into(),
        ArenaFailure::RepeatDiffered { repeat } => format!("repeat {repeat} differed"),
        ArenaFailure::MissingEvidence { part } => format!("missing {part:?} evidence"),
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
    // Each of the lens's three channels gives a one-second mean at each of the two whole seconds after admission.
    // The window that closes at admission saw only that instant, so it gives an absence, which is no sample.
    assert_eq!(throughput.waveform_samples(), 3 * 2);
    assert_eq!(evidence.run(), identity);
}

struct OneVessel {
    body: CompiledBody,
    amount: RowKind,
    reading: RowKind,
}
impl OneVessel {
    fn new(vessel: &'static str) -> Self {
        Self::built(vessel, false)
    }
    fn distrusting_zero(vessel: &'static str) -> Self {
        Self::built(vessel, true)
    }
    fn built(vessel: &'static str, distrusting_zero: bool) -> Self {
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
            if distrusting_zero {
                let range = ValidityRange::new(measure(1.0), measure(2.0), Basis::Fixture("synthetic"))?;
                laws.validity(&flow, range);
            }
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
    fn averaged_lens(&self, points: u64) -> ObservationLens {
        let channels = vec![ChannelSpec::new("synthetic gauge", self.reading).averaged(wall(points))];
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
    let small = OneVessel::new("synthetic vessel");
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
    let small = OneVessel::new("synthetic vessel");
    let clocks: [(&str, &[u64]); 5] = [
        ("no reading", &[]),
        ("stops after admission", &[0, 1]),
        ("runs backward over admission", &[2, 1, 2, 3]),
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
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let inside_one_second_mean = small.spec(small.averaged_lens(10), 0, 500, 1);
    let absent = run(&small.body, &core, inside_one_second_mean, &clock, &store);
    assert!(matches!(absent, Err(ArenaFailure::MissingThroughput)));
}

const STEPS_IN_REPEAT_ZERO: u64 = 10;

/// Its resolution stays its inner core's, so every repeat resumes instead of failing its checkpoint.
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
        if self.steps.get() > STEPS_IN_REPEAT_ZERO {
            span / 2
        } else {
            span
        }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn the_first_repeat_on_another_course_than_repeat_zero_is_named() {
    let small = OneVessel::new("synthetic vessel");
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
    let (small, other) = (
        OneVessel::new("synthetic vessel"),
        OneVessel::new("another vessel"),
    );
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
    let small = OneVessel::new("synthetic vessel");
    let named = |seed| {
        let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
        let mut spec = small.spec(small.lens(), 0, 1000, 1);
        spec.seed = seed;
        ran(run(&small.body, &core, spec, &clock, &store))
            .into_evidence()
            .run()
    };
    let first = named(Seed::new([1, 2, 3, 4]));
    assert_eq!(first, named(Seed::new([1, 2, 3, 4])));
    assert_ne!(first, named(Seed::new([4, 3, 2, 1])));
}

#[test]
fn the_clock_opens_before_admission_so_a_refused_one_has_read_it_once() {
    let small = OneVessel::new("synthetic vessel");
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let mut spec = small.spec(small.lens(), 0, 1000, 1);
    spec.packet = ParameterPacket::new();
    let refused = run(&small.body, &core, spec, &clock, &store);
    assert!(matches!(refused, Err(ArenaFailure::Initialization(_))));
    assert_eq!(clock.reads.get(), 1);
}

fn slice_core() -> ReferenceIntegrator {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    ReferenceIntegrator::new(5 * MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"))
}

fn slice_spec(body: &CompiledBody, end: SimTime, repeats: u32) -> ArenaSpec {
    ArenaSpec {
        window: Interval::new(anchor_moment(), end).expect("an interval"),
        packet: arena_packet(body).expect("a packet"),
        lens: arena_lens(body).expect("a lens"),
        boundary: BoundaryPort::new(Vec::new()).expect("no event"),
        seed: Seed::new([1, 2, 3, 4]),
        repeats: NonZeroU32::new(repeats).expect("positive"),
    }
}

fn stepped<'m>(
    session: Session<'m, Running>,
    refused: &mut Vec<NumericalRefusal>,
    excursions: &mut u64,
) -> Session<'m, Running> {
    let mut attempt = session.step();
    loop {
        match attempt {
            Step::Accepted(session, receipt) => {
                *excursions += receipt.excursions().len() as u64;
                return session;
            }
            Step::Refused(session) => {
                refused.push(session.refused().reason());
                attempt = session.retry();
            }
            Step::Faulted(session) => panic!("{:?}", session.fault()),
        }
    }
}

fn tallied(reasons: &[NumericalRefusal]) -> Vec<(NumericalRefusal, u64)> {
    let mut tally: Vec<(NumericalRefusal, u64)> = Vec::new();
    for &reason in reasons {
        match tally.iter_mut().find(|(seen, _)| *seen == reason) {
            Some((_, count)) => *count += 1,
            None => tally.push((reason, 1)),
        }
    }
    tally
}

#[test]
fn a_short_slice_window_reports_what_its_session_refused_once_per_repeat() {
    let body = arena_body().expect("compiled");
    let core = slice_core();
    let end = SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND);
    let (store, clock) = (MemoryStore::default(), Scripted::ticking());
    let evidence = ran(run(&body, &core, slice_spec(&body, end, 2), &clock, &store)).into_evidence();
    let health = evidence.health();

    let stepped_store = MemoryStore::default();
    let ArenaSpec {
        packet,
        lens,
        boundary,
        ..
    } = slice_spec(&body, end, 1);
    let admitted = admit_at(
        &body,
        packet,
        boundary,
        &core,
        &lens,
        anchor_moment(),
        &stepped_store,
    );
    let mut session = admitted.expect("admitted").session;
    let (mut refused, mut excursions) = (Vec::new(), 0);
    while session.record().accepted().time() < end {
        session = stepped(session, &mut refused, &mut excursions);
    }
    assert!(
        !refused.is_empty(),
        "the slice refuses at 5 ms, so the report has something to count"
    );
    assert_eq!(health.refusals(), tallied(&refused));
    assert_eq!(health.excursions(), excursions);
    assert_eq!(health.floor_faults(), 0);
}

struct RefusingFrom<'c> {
    inner: &'c dyn CoupledSolver,
    from: SimTime,
    sources: [SourceId; 2],
}
impl CoupledSolver for RefusingFrom<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() < self.from {
            return self.inner.solve(problem, ticket);
        }
        let source = self.sources[ticket.attempt() as usize % 2];
        Err(ticket.refuse(NumericalRefusal::TrialDomain { source }))
    }
}

#[test]
fn a_slice_window_that_faults_at_the_floor_reports_its_health_up_to_the_fault() {
    let body = arena_body().expect("compiled");
    let rows = body.rows();
    let (first, last) = (rows[0].id(), rows[rows.len() - 1].id());
    let (low, high) = (body.row_source(first), body.row_source(last));
    let inner = slice_core();
    let core = RefusingFrom {
        inner: &inner,
        from: SimTime::from_ns(anchor_moment().nanoseconds() + 50 * MILLISECOND),
        sources: [high, low],
    };
    let end = SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND);
    let (store, clock) = (MemoryStore::default(), Scripted::ticking());
    let Err(ArenaFailure::Faulted { report, health }) =
        run(&body, &core, slice_spec(&body, end, 2), &clock, &store)
    else {
        panic!("the planted step faults");
    };
    assert_eq!(report.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    assert_eq!(health.floor_faults(), 1);
    let planted = report.refusals().len() as u64;
    assert!(planted >= 2, "both sources refuse before the floor");
    assert_eq!(
        health.by_source(),
        [(low, planted / 2), (high, planted.div_ceil(2))]
    );
    let counted: u64 = health.refusals().iter().map(|(_, count)| count).sum();
    assert!(counted > planted, "the steps before the planted one refused too");
}

struct MismatchedFrom {
    inner: ReferenceIntegrator,
    from: SimTime,
}
impl CoupledSolver for MismatchedFrom {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() < self.from {
            return self.inner.solve(problem, ticket);
        }
        Err(ticket.mismatch().into())
    }
}

#[test]
fn a_fault_off_the_floor_is_no_floor_fault() {
    let small = OneVessel::new("synthetic vessel");
    let core = MismatchedFrom {
        inner: small_core(),
        from: SimTime::from_ns(500 * MILLISECOND),
    };
    let (store, clock) = (MemoryStore::default(), Scripted::ticking());
    let Err(ArenaFailure::Faulted { report, health }) = run(
        &small.body,
        &core,
        small.spec(small.lens(), 0, 1000, 1),
        &clock,
        &store,
    ) else {
        panic!("the step at 500 ms faults");
    };
    assert_eq!(report.fault().kind(), EngineFaultKind::BindingMismatch);
    assert_eq!(health.floor_faults(), 0);
}

#[test]
fn every_accepted_step_outside_a_validity_range_is_one_excursion() {
    let small = OneVessel::distrusting_zero("synthetic vessel");
    let (core, store, clock) = (small_core(), MemoryStore::default(), Scripted::ticking());
    let evidence = ran(run(
        &small.body,
        &core,
        small.spec(small.lens(), 0, 1000, 3),
        &clock,
        &store,
    ))
    .into_evidence();
    assert!(evidence.throughput().accepted_steps() > 1);
    assert_eq!(
        evidence.health().excursions(),
        evidence.throughput().accepted_steps()
    );
    assert!(evidence.health().refusals().is_empty());
}

struct RefusedAt {
    inner: ReferenceIntegrator,
    at: SimTime,
    refused: Cell<u32>,
    renamed: bool,
}
impl CoupledSolver for RefusedAt {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() != self.at || ticket.attempt() > 0 {
            return self.inner.solve(problem, ticket);
        }
        self.refused.set(self.refused.get() + 1);
        let reason = match (self.renamed, self.refused.get()) {
            (true, 2..) => NumericalRefusal::GuardInside,
            _ => NumericalRefusal::IterationLimit,
        };
        Err(ticket.refuse(reason))
    }
}

#[test]
fn a_repeat_that_refused_for_another_reason_differed() {
    let small = OneVessel::new("synthetic vessel");
    let repeated = |renamed| {
        let core = RefusedAt {
            inner: small_core(),
            at: SimTime::from_ns(500 * MILLISECOND),
            refused: Cell::new(0),
            renamed,
        };
        let (store, clock) = (MemoryStore::default(), Scripted::ticking());
        let outcome = run(
            &small.body,
            &core,
            small.spec(small.lens(), 0, 1000, 2),
            &clock,
            &store,
        );
        assert_eq!(core.refused.get(), 2);
        outcome.map(|arena| arena.into_evidence())
    };
    let same = repeated(false).unwrap_or_else(|failure| panic!("{}", told(&failure)));
    assert_eq!(same.health().refusals(), [(NumericalRefusal::IterationLimit, 1)]);
    assert!(matches!(
        repeated(true),
        Err(ArenaFailure::RepeatDiffered { repeat: 1 })
    ));
}
