use core::cell::Cell;
use core::num::NonZeroU64;
use ng_contract::*;
use ng_observe::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const SEED: [u64; 4] = [5, 6, 7, 8];
/// A synthetic spread, in the amount unit.
const SYNTHETIC_SD: f64 = 2.0;

fn seed() -> ObservationSeed {
    ObservationSeed::new(SEED)
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn reference() -> ReferenceIntegrator {
    ReferenceIntegrator::new(8 * SECOND)
}

fn water_case() -> TwoReservoirs {
    let start = [measure(500_500.0), measure(499_500.0)];
    let delivery = Delivery {
        id: b"input",
        from: at(40),
        to: at(60),
        amount: measure(100.0),
    };
    let case = TwoReservoirs::delivering(start, vec![delivery]).expect("a body");
    case.until(at(200))
}

fn kind(body: &CompiledBody, row: RowId) -> RowKind {
    body.rows()[row.index()].kind()
}

fn bare(case: &TwoReservoirs) -> [ChannelSpec; 2] {
    case.rows()
        .map(|row| ChannelSpec::new("a synthetic method", kind(case.body(), row)))
}

fn lens(body: &CompiledBody, channels: Vec<ChannelSpec>, period_ns: u64) -> ObservationLens {
    body.lens(channels, count(period_ns)).expect("a lens")
}

fn step<'m>(session: Session<'m, Running>) -> (Session<'m, Running>, StepReceipt) {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, receipt) => return (session, receipt),
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        };
    }
}

fn read(
    observer: &mut Observer<'_>,
    session: &Session<'_, Running>,
    lens: &ObservationLens,
) -> Vec<Observation> {
    let view = session.observation(lens).expect("this body's lens");
    observer.advance(&view).expect("the next revision")
}

/// What a run gave: its seal per step, its last digests, its firings and every observation.
struct Watched {
    seals: Vec<Seal>,
    digests: RecordDigests,
    fired: Vec<Fired>,
    observations: Vec<Observation>,
}

fn watched(
    case: &dyn ReplayCase,
    core: &dyn CoupledSolver,
    lens: Option<(&ObservationLens, ObservationSeed)>,
) -> Watched {
    let mut session = case.admit(core).expect("admitted");
    let mut observer = lens.map(|(lens, seed)| (lens, Observer::new(lens, seed)));
    let mut seals = vec![session.record().seal()];
    let (mut fired, mut observations) = (Vec::new(), Vec::new());
    if let Some((lens, observer)) = &mut observer {
        observations.extend(read(observer, &session, lens));
    }
    while session.record().accepted().time() < case.end() {
        let (next, receipt) = step(session);
        session = next;
        seals.push(session.record().seal());
        fired.extend_from_slice(receipt.fired());
        if let Some((lens, observer)) = &mut observer {
            observations.extend(read(observer, &session, lens));
        }
    }
    let digests = session.record().digests();
    Watched {
        seals,
        digests,
        fired,
        observations,
    }
}

fn same_run(first: &Watched, second: &Watched) {
    assert_eq!(first.seals, second.seals);
    assert_eq!(first.digests, second.digests);
    assert_eq!(first.fired, second.fired);
}

// Check 1.

#[test]
fn a_new_lens_on_a_finished_replay_gives_the_same_physiology_and_new_observations() {
    let case = water_case();
    let core = reference();
    let [a, b] = bare(&case);
    let first = lens(case.body(), vec![a], 4 * SECOND);
    let second = lens(case.body(), vec![b.averaged(count(3))], 5 * SECOND);
    assert_ne!(first.digest(), second.digest());
    let detached = watched(&case, &core, None);
    let one = watched(&case, &core, Some((&first, seed())));
    let other = watched(&case, &core, Some((&second, seed())));
    same_run(&detached, &one);
    same_run(&detached, &other);
    assert!(!one.observations.is_empty());
    assert!(!other.observations.is_empty());
    assert_ne!(one.observations, other.observations);
}

// Check 2.

struct Clock {
    next: Cell<u64>,
}
impl WallClock for Clock {
    fn elapsed_ns(&self) -> Option<u64> {
        self.next.set(self.next.get() + 1);
        Some(self.next.get() * 16 * SECOND)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

fn plan(clock: &Clock) -> ReplayPlan<'_> {
    ReplayPlan {
        clock,
        simulated_per_wall: 1,
        chunks: vec![11 * SECOND],
        resume_every: 100,
        chance: None,
    }
}

#[test]
fn physiology_is_identical_with_observers_attached_detached_and_changed() {
    let clock = Clock { next: Cell::new(0) };
    let case = water_case();
    let evidence =
        replay_gate(ReplayGate::NonPerturbation, &case, &reference, &plan(&clock)).expect("the gate");
    assert_eq!(evidence.runs().len(), 3);
    assert!(evidence.runs().iter().all(|run| *run == evidence.runs()[0]));
    assert_eq!(evidence.observed().len(), 2);
    assert_ne!(evidence.observed()[0], evidence.observed()[1]);
    let chance = ChanceCase::ectopy().expect("a body");
    let evidence =
        replay_gate(ReplayGate::NonPerturbation, &chance, &reference, &plan(&clock)).expect("the gate");
    assert!(!evidence.fired().is_empty());
    assert!(evidence.runs().iter().all(|run| *run == evidence.runs()[0]));
}

/// The bug a lens could cause: a caller hands its core the output times, so reading cuts a step.
struct CutAtTheGrid {
    inner: ReferenceIntegrator,
    period_ns: u64,
}
impl CoupledSolver for CutAtTheGrid {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let reached = accepted.time().nanoseconds() % self.period_ns;
        self.inner.next_step(accepted).min(self.period_ns - reached)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn a_core_told_the_grid_only_in_a_watched_run_fails_the_gate() {
    let clock = Clock { next: Cell::new(0) };
    let case = water_case();
    let calls = Cell::new(0);
    let cores = || {
        calls.set(calls.get() + 1);
        CutAtTheGrid {
            inner: reference(),
            period_ns: if calls.get() == 1 { u64::MAX } else { 3 * SECOND },
        }
    };
    let failure = replay_gate(ReplayGate::NonPerturbation, &case, &cores, &plan(&clock));
    assert!(
        matches!(
            failure,
            Err(ReplayFailure::Diverged {
                gate: ReplayGate::NonPerturbation,
                ..
            })
        ),
        "{failure:?}"
    );
    let same = || CutAtTheGrid {
        inner: reference(),
        period_ns: 3 * SECOND,
    };
    replay_gate(ReplayGate::NonPerturbation, &case, &same, &plan(&clock))
        .expect("one caller's times in every run");
}

/// One step ends before the changed lens's first window results, so that run shows an observer read nothing.
#[test]
fn a_run_too_short_for_a_reading_to_result_is_missing_evidence() {
    let clock = Clock { next: Cell::new(0) };
    let case = water_case().until(at(8));
    let failure = replay_gate(ReplayGate::NonPerturbation, &case, &reference, &plan(&clock));
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::NonPerturbation
        })
    );
}

// Check 3.

#[test]
fn an_observer_refuses_a_skipped_revision() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a], SECOND);
    let session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let expected = session.observation(&lens).expect("a view").seal();
    read(&mut observer, &session, &lens);
    let (session, _) = step(session);
    let (session, _) = step(session);
    let view = session.observation(&lens).expect("a view");
    assert_eq!(
        observer.advance(&view).err(),
        Some(ObservationRefusal::Gap {
            expected,
            found: view.previous()
        })
    );
}

#[test]
fn an_observer_refuses_a_repeated_revision_and_a_refused_read_changes_nothing() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a], SECOND);
    let session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    let (session, _) = step(session);
    read(&mut observer, &session, &lens);
    let before = observer.checkpoint();
    let view = session.observation(&lens).expect("a view");
    assert!(matches!(
        observer.advance(&view),
        Err(ObservationRefusal::Gap { .. })
    ));
    assert_eq!(observer.checkpoint().as_bytes(), before.as_bytes());
}

#[derive(Default)]
struct Store {
    links: Vec<(Digest, Vec<u8>)>,
}
impl HistoryStore for Store {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        (self.links.iter())
            .find(|link| link.0 == *seal)
            .map(|link| link.1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        self.links.push((seal, link));
        Ok(())
    }
}

fn water(id: &[u8], from: SimTime) -> AuthorizedInput {
    let event = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(id)),
        from,
        BoundaryInput::Water(WaterDelivery {
            boundary: "exterior",
            amount: measure(100.0),
            ends: SimTime::from_ns(from.nanoseconds() + 5 * SECOND),
        }),
    );
    AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a command"), event)
}

/// Two forks of one parent have a revision 2 each, with different seals.
#[test]
fn an_observer_refuses_a_same_numbered_revision_of_another_course() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a], SECOND);
    let mut store = Store::default();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    (session, _) = step(session);
    let saved = session.checkpoint(&mut store).expect("a checkpoint");
    let fork = |name: &[u8]| {
        session
            .rewind(&saved, BranchId::new(Digest::of_canonical(name)))
            .expect("a fork")
    };
    let (mut first, mut second) = (fork(b"first"), fork(b"second"));
    second
        .enqueue(water(b"only here", session.record().earliest_stamp()))
        .expect("taken");
    read(&mut observer, &first, &lens);
    (first, _) = step(first);
    (second, _) = step(second);
    (second, _) = step(second);
    read(&mut observer, &first, &lens);
    let view = second.observation(&lens).expect("a view");
    assert_eq!(first.record().revision(), 2);
    assert_eq!(second.record().revision(), 3);
    assert!(matches!(
        observer.advance(&view),
        Err(ObservationRefusal::Gap { .. })
    ));
}

#[test]
fn each_grid_instant_appears_in_exactly_one_step() {
    let case = water_case();
    let core = reference();
    let [a, b] = bare(&case);
    let period_ns = 3 * SECOND;
    let lens = lens(case.body(), vec![a, b], period_ns);
    let mut session = case.admit(&core).expect("admitted");
    let mut instants = Vec::new();
    loop {
        let view = session.observation(&lens).expect("a view");
        instants.extend(view.samples().into_iter().map(|(at, values)| {
            assert_eq!(values.len(), 2);
            at.nanoseconds()
        }));
        if session.record().accepted().time() >= case.end() {
            break;
        }
        (session, _) = step(session);
    }
    let head = session.record().accepted().time().nanoseconds();
    let expected: Vec<u64> = (0..=head / period_ns).map(|index| index * period_ns).collect();
    assert_eq!(
        instants, expected,
        "from the admission instant to the end, each once"
    );
}

struct RefusesSome {
    inner: ReferenceIntegrator,
    refused: Cell<u64>,
}
impl CoupledSolver for RefusesSome {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns().to_be_bytes();
        if Digest::of_canonical(&length).as_bytes()[0].is_multiple_of(3) {
            self.refused.set(self.refused.get() + 1);
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// The grid lands on the firing, so the instant a reset could show twice or not at all is one a window reads.
#[test]
fn each_grid_instant_reaches_an_observer_once_across_refusals_and_a_firing() {
    let case = ChanceCase::ectopy().expect("a body").until(at(120));
    let core = RefusesSome {
        inner: reference(),
        refused: Cell::new(0),
    };
    let plain = watched(&case, &core, None);
    assert!(core.refused.get() > 0, "a run with no refusal presses nothing");
    let firing = plain.fired.first().expect("a firing");
    let period_ns = firing.at().nanoseconds();
    let row = case.memory("onset-hazard").expect("the integral");
    let channel = ChannelSpec::new("a synthetic method", kind(case.body(), row));
    let lens = lens(case.body(), vec![channel], period_ns);
    let seen = watched(&case, &core, Some((&lens, seed())));
    same_run(&plain, &seen);
    let collected: Vec<u64> = (seen.observations.iter())
        .map(|observation| observation.collected().nanoseconds())
        .collect();
    let head = seen.digests.time.nanoseconds();
    let expected: Vec<u64> = (0..=head / period_ns).map(|index| index * period_ns).collect();
    assert!(expected.len() > 2, "the grid holds more than the firing");
    assert_eq!(collected, expected);
    let reset = (seen.observations.iter()).find(|observation| observation.collected() == firing.at());
    assert_eq!(
        reset.map(|observation| observation.value()),
        Some(ObservedValue::Present(Compensated::ZERO))
    );
}

#[test]
fn an_input_admitted_between_reads_leaves_no_gap() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a], SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    let seal = session.observation(&lens).expect("a view").seal();
    let stamp = session.record().earliest_stamp();
    session.enqueue(water(b"between reads", stamp)).expect("taken");
    assert_eq!(session.observation(&lens).expect("a view").seal(), seal);
    (session, _) = step(session);
    assert_eq!(session.observation(&lens).expect("a view").previous(), Some(seal));
    read(&mut observer, &session, &lens);
}

#[test]
fn an_observer_may_attach_to_a_run_under_way_and_reads_from_there() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.averaged(count(12))], SECOND);
    let mut session = case.admit(&core).expect("admitted");
    (session, _) = step(session);
    (session, _) = step(session);
    (session, _) = step(session);
    let mut observer = Observer::new(&lens, seed());
    let observed = read(&mut observer, &session, &lens);
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].value(),
        ObservedValue::NotCollected,
        "it saw 8 of the 12 instants"
    );
    assert_eq!(observed[0].collected(), at(24));
    (session, _) = step(session);
    assert!(read(&mut observer, &session, &lens).is_empty());
    (session, _) = step(session);
    let observed = read(&mut observer, &session, &lens);
    assert_eq!(observed[0].collected(), at(36));
    assert!(matches!(observed[0].value(), ObservedValue::Present(_)));
}

// Check 4.

fn lab(case: &TwoReservoirs) -> ObservationLens {
    let [a, b] = bare(case);
    let sd = measure::<Water>(SYNTHETIC_SD);
    lens(
        case.body(),
        vec![a.averaged(count(5)).erring(sd), b.delayed(30 * SECOND)],
        3 * SECOND,
    )
}

#[test]
fn an_observer_checkpoint_restores_its_observations_after_a_resume() {
    let case = water_case();
    let lens = lab(&case);
    let core = reference();
    let whole = watched(&case, &core, Some((&lens, seed())));
    let mut store = Store::default();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let mut observed = read(&mut observer, &session, &lens);
    let mut pending = 0;
    while session.record().accepted().time() < at(100) {
        (session, _) = step(session);
        observed.extend(read(&mut observer, &session, &lens));
        pending += 1;
        let record =
            Checkpoint::from_bytes(session.checkpoint(&mut store).expect("saved").as_bytes().to_vec());
        let bytes = observer.checkpoint().as_bytes().to_vec();
        let stored = ObserverCheckpoint::from_bytes(bytes);
        let fresh = reference();
        let mut session = Session::resume(&record, session.record().identity(), case.body(), &fresh, &store)
            .expect("resumed");
        let mut observer = Observer::resume(&lens, seed(), &stored).expect("resumed");
        let mut rest = observed.clone();
        while session.record().accepted().time() < case.end() {
            (session, _) = step(session);
            rest.extend(read(&mut observer, &session, &lens));
        }
        assert_eq!(rest, whole.observations, "resumed after {pending} steps");
    }
}

#[test]
fn an_observer_checkpoint_stays_valid_across_a_rewind() {
    let case = water_case();
    let lens = lab(&case);
    let core = reference();
    let whole = watched(&case, &core, Some((&lens, seed())));
    let mut store = Store::default();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let mut observed = read(&mut observer, &session, &lens);
    for _ in 0..6 {
        (session, _) = step(session);
        observed.extend(read(&mut observer, &session, &lens));
    }
    let saved = session.checkpoint(&mut store).expect("saved");
    let kept = observer.checkpoint();
    let mut parent = session;
    while parent.record().accepted().time() < at(150) {
        (parent, _) = step(parent);
        read(&mut observer, &parent, &lens);
    }
    let mut fork = parent
        .rewind(&saved, BranchId::new(Digest::of_canonical(b"a fork")))
        .expect("a fork");
    let mut forked = Observer::resume(&lens, seed(), &kept).expect("resumed");
    while fork.record().accepted().time() < case.end() {
        (fork, _) = step(fork);
        observed.extend(read(&mut forked, &fork, &lens));
    }
    assert_eq!(observed, whole.observations);
}

#[test]
fn a_resumed_observer_refuses_a_record_it_does_not_stand_beside() {
    let case = water_case();
    let lens = lab(&case);
    let core = reference();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    let saved = observer.checkpoint();
    (session, _) = step(session);
    (session, _) = step(session);
    let mut resumed = Observer::resume(&lens, seed(), &saved).expect("the bytes are good");
    let view = session.observation(&lens).expect("a view");
    assert!(matches!(
        resumed.advance(&view),
        Err(ObservationRefusal::Gap { .. })
    ));
}

#[test]
fn a_checkpoint_before_the_first_read_is_a_new_observer() {
    let case = water_case();
    let lens = lab(&case);
    let core = reference();
    let saved = Observer::new(&lens, seed()).checkpoint();
    let mut resumed = Observer::resume(&lens, seed(), &saved).expect("resumed");
    let session = case.admit(&core).expect("admitted");
    let (session, _) = step(session);
    let mut new = Observer::new(&lens, seed());
    assert_eq!(
        read(&mut resumed, &session, &lens),
        read(&mut new, &session, &lens)
    );
}

// Check 5.

#[test]
fn measurement_error_leaves_physiology_draws_unchanged() {
    let case = ChanceCase::ectopy().expect("a body").until(at(120));
    let core = reference();
    let [a, b] = case
        .rows()
        .map(|row| ChannelSpec::new("a synthetic method", kind(case.body(), row)));
    let sd = || measure::<Water>(SYNTHETIC_SD);
    let one = lens(case.body(), vec![a.erring(sd())], SECOND);
    let [a, _] = case
        .rows()
        .map(|row| ChannelSpec::new("a synthetic method", kind(case.body(), row)));
    let two = lens(case.body(), vec![b.erring(sd()), a.erring(sd())], SECOND);
    let detached = watched(&case, &core, None);
    let first = watched(&case, &core, Some((&one, seed())));
    let second = watched(&case, &core, Some((&two, seed())));
    let reseeded = watched(&case, &core, Some((&one, ObservationSeed::new([9, 9, 9, 9]))));
    assert!(!detached.fired.is_empty());
    same_run(&detached, &first);
    same_run(&detached, &second);
    same_run(&detached, &reseeded);
    assert_ne!(first.observations, reseeded.observations);
    let shared: Vec<ObservedValue> = (second.observations.iter())
        .filter(|observation| observation.channel() == 1)
        .map(|observation| observation.value())
        .collect();
    let alone: Vec<ObservedValue> = first
        .observations
        .iter()
        .map(|observation| observation.value())
        .collect();
    assert_eq!(alone, shared, "adding a channel moves no other channel's error");
}

#[test]
fn measurement_error_is_a_function_of_seed_channel_and_instant_with_mean_zero_and_its_declared_spread() {
    const COUNT: u64 = 200_000;
    let case = water_case();
    let [a, b] = bare(&case);
    let sd = measure::<Water>(SYNTHETIC_SD);
    let lens = lens(
        case.body(),
        vec![a.erring(measure::<Water>(SYNTHETIC_SD)), b.erring(sd)],
        SECOND,
    );
    let [first, second] = lens.channels() else {
        panic!("two channels");
    };
    let error = |channel: &Channel, index| channel.error(seed(), SimTime::from_ns(index * SECOND));
    assert_eq!(error(first, 3), error(first, 3));
    assert_ne!(error(first, 3), error(first, 4));
    assert_ne!(error(first, 3), error(second, 3));
    assert_ne!(
        first.error(ObservationSeed::new([1, 2, 3, 4]), at(3)),
        error(first, 3)
    );
    let (mut sum, mut square) = (0.0, 0.0);
    for index in 0..COUNT {
        let draw = f64::from_bits(error(first, index).bits()[0]) / SYNTHETIC_SD;
        assert!(draw.abs() < 6.0);
        sum += draw;
        square += draw * draw;
    }
    let count = COUNT as f64;
    assert!((sum / count).abs() < 4.0 / count.sqrt(), "mean {}", sum / count);
    assert!((square / count - 1.0).abs() < 0.02, "variance {}", square / count);
}

// One value at an instant, one rounding, a window, a delay and typed absence.

#[test]
fn a_channel_reads_what_a_firing_left_at_its_instant() {
    let case = ChanceCase::ectopy().expect("a body").until(at(120));
    let core = reference();
    let fired = watched(&case, &core, None).fired;
    let firing = fired.first().expect("a firing");
    let row = case.memory("onset-hazard").expect("the integral");
    let lens = lens(
        case.body(),
        vec![ChannelSpec::new("a synthetic method", kind(case.body(), row))],
        firing.at().nanoseconds(),
    );
    let mut session = case.admit(&core).expect("admitted");
    while session.record().accepted().time() < firing.at() {
        (session, _) = step(session);
    }
    let accepted = session.record().accepted();
    assert_eq!(accepted.time(), firing.at());
    assert_eq!(accepted.value(row), Compensated::ZERO);
    let left = accepted
        .last_segment()
        .sample(row, firing.at())
        .expect("the segment's end");
    assert!(
        firing.threshold().at_most(left),
        "the left limit reached the draw"
    );
    let view = session.observation(&lens).expect("a view");
    assert_eq!(view.samples(), vec![(firing.at(), vec![Compensated::ZERO])]);
}

/// The mean's reference comes straight from the view, so a rounding anywhere past the view moves a bit of it.
#[test]
fn a_window_keeps_every_low_part() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let window = 4;
    let lens = lens(case.body(), vec![a.averaged(count(window))], SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let (mut read_from_view, mut windows) = (Vec::new(), Vec::new());
    loop {
        let view = session.observation(&lens).expect("a view");
        read_from_view.extend(view.samples().into_iter().map(|(at, values)| (at, values[0])));
        windows.extend(observer.advance(&view).expect("the next revision"));
        if session.record().accepted().time() >= case.end() {
            break;
        }
        (session, _) = step(session);
    }
    assert!(
        (read_from_view.iter()).any(|(_, value)| value.bits()[1] != 0),
        "no low part to lose"
    );
    assert_eq!(windows[0].value(), ObservedValue::NotCollected);
    for observation in &windows[1..] {
        let ObservedValue::Present(mean) = observation.value() else {
            panic!("{observation:?}");
        };
        let end = (read_from_view.iter())
            .position(|(at, _)| *at == observation.collected())
            .expect("one instant");
        let sum = (read_from_view[end + 1 - window as usize..=end].iter())
            .fold(Compensated::ZERO, |sum, (_, value)| sum.plus(*value));
        assert_eq!(
            mean.bits(),
            Compensated::between(Compensated::ZERO, sum, 1, window).bits()
        );
    }
}

#[test]
fn a_window_that_did_not_fill_reports_not_collected_never_a_partial_mean() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.averaged(count(4))], SECOND);
    let observations = watched(&case, &core, Some((&lens, seed()))).observations;
    assert_eq!(observations[0].collected(), at(0));
    assert_eq!(observations[0].value(), ObservedValue::NotCollected);
    assert_eq!(observations[1].collected(), at(4));
    assert!(matches!(observations[1].value(), ObservedValue::Present(_)));
}

#[test]
fn a_delayed_reading_is_pending_until_the_step_that_reaches_its_exact_instant() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let delay = 11 * SECOND + 1;
    let lens = lens(case.body(), vec![a.delayed(delay)], 2 * SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let mut steps = vec![read(&mut observer, &session, &lens)];
    let mut heads = vec![session.record().accepted().time()];
    while session.record().accepted().time() < at(40) {
        (session, _) = step(session);
        heads.push(session.record().accepted().time());
        steps.push(read(&mut observer, &session, &lens));
    }
    let instant = at(2);
    let shown: Vec<(usize, Observation)> = (0..steps.len())
        .flat_map(|index| steps[index].iter().map(move |observation| (index, *observation)))
        .filter(|(_, observation)| observation.collected() == instant)
        .collect();
    let [(held, pending), (released, result)] = shown[..] else {
        panic!("{shown:?}");
    };
    assert_eq!(pending.value(), ObservedValue::Pending);
    assert_eq!(pending.available(), result.available());
    assert_eq!(result.available(), SimTime::from_ns(2 * SECOND + delay));
    assert!(matches!(result.value(), ObservedValue::Present(_)));
    assert!(heads[held] < result.available(), "never early");
    assert!(heads[released - 1] < result.available() && result.available() <= heads[released]);
}

#[test]
fn a_reading_that_results_past_the_clock_is_an_invalid_signal() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.delayed(u64::MAX)], SECOND);
    let observations = watched(&case, &core, Some((&lens, seed()))).observations;
    assert_eq!(observations[0].value(), ObservedValue::Pending);
    assert_eq!(observations[0].available(), SimTime::from_ns(u64::MAX));
    assert_eq!(observations[1].collected(), at(1));
    assert_eq!(observations[1].available(), at(1));
    assert_eq!(observations[1].value(), ObservedValue::InvalidSignal);
}

#[test]
fn a_window_closes_on_its_own_multiples_whatever_its_length() {
    let case = water_case();
    let [a, b] = bare(&case);
    let lens = lens(
        case.body(),
        vec![a.averaged(count(4)), b.averaged(count(u64::MAX))],
        3,
    );
    let [short, long] = lens.channels() else {
        panic!("two channels");
    };
    let closes = |channel: &Channel, ns| channel.closes(SimTime::from_ns(ns));
    assert!(closes(short, 12) && closes(short, 24));
    assert!(!closes(short, 9) && !closes(short, 15));
    assert!(closes(long, 0));
    assert!(!closes(long, 3) && !closes(long, u64::MAX));
}

#[test]
fn a_reading_due_at_a_head_is_released_at_that_head() {
    let case = water_case();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.delayed(6 * SECOND)], 2 * SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    (session, _) = step(session);
    assert_eq!(session.record().accepted().time(), at(8));
    let released: Vec<SimTime> = (read(&mut observer, &session, &lens).iter())
        .filter(|observation| matches!(observation.value(), ObservedValue::Present(_)))
        .map(Observation::available)
        .collect();
    assert_eq!(released, vec![at(6), at(8)]);
}

fn huge() -> TwoReservoirs {
    let start = [measure(8e307), measure(8e307)];
    TwoReservoirs::delivering(start, Vec::new())
        .expect("a body")
        .until(at(40))
}

/// A sum that overflows part way through a window crosses a checkpoint, and a non-finite pair has no bytes.
#[test]
fn a_sum_past_the_finite_range_is_an_invalid_signal_and_survives_a_checkpoint() {
    let case = huge();
    let core = reference();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.averaged(count(20))], SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    let mut observed = read(&mut observer, &session, &lens);
    for _ in 0..3 {
        (session, _) = step(session);
        observed.extend(read(&mut observer, &session, &lens));
        observer = Observer::resume(&lens, seed(), &observer.checkpoint()).expect("resumed");
    }
    assert_eq!(observed.last().map(Observation::collected), Some(at(20)));
    assert_eq!(
        observed.last().map(Observation::value),
        Some(ObservedValue::InvalidSignal)
    );
}

#[test]
fn an_error_past_the_finite_range_is_an_invalid_signal() {
    let case = huge();
    let core = reference();
    let [a, _] = bare(&case);
    let sd = measure::<Water>(1.7e308);
    let lens = lens(case.body(), vec![a.erring(sd)], SECOND);
    let values: Vec<ObservedValue> = (watched(&case, &core, Some((&lens, seed()))).observations.iter())
        .map(|observation| observation.value())
        .collect();
    assert!(values.contains(&ObservedValue::InvalidSignal));
    assert!(values.iter().all(|value| match value {
        ObservedValue::Present(value) => AdmissibleSet::Finite.holds(*value),
        other => *other == ObservedValue::InvalidSignal,
    }));
}

// Refusals.

#[test]
fn a_lens_digest_covers_its_body_its_grid_and_each_channel() {
    let (case, other) = (water_case(), TwoReservoirs::closed().expect("a body"));
    let row = |case: &TwoReservoirs| kind(case.body(), case.rows()[0]);
    let channel = |case: &TwoReservoirs| ChannelSpec::new("a synthetic method", row(case));
    let digest = |channel, period_ns| lens(case.body(), vec![channel], period_ns).digest();
    let base = digest(channel(&case), SECOND);
    assert_eq!(base, digest(channel(&case), SECOND));
    assert_ne!(base, digest(channel(&case), 2 * SECOND));
    assert_ne!(base, digest(channel(&case).averaged(count(2)), SECOND));
    assert_ne!(base, digest(channel(&case).delayed(1), SECOND));
    assert_ne!(base, digest(channel(&case).erring(measure::<Water>(1.0)), SECOND));
    assert_ne!(
        base,
        digest(ChannelSpec::new("another method", row(&case)), SECOND)
    );
    assert_ne!(base, lens(other.body(), vec![channel(&other)], SECOND).digest());
}

#[test]
fn a_lens_refuses_a_row_the_body_does_not_hold() {
    let case = water_case();
    let row = RowKind::Memory(LawName {
        owner: "CV",
        local: "no such slot",
    });
    let refused = case
        .body()
        .lens(vec![ChannelSpec::new("a synthetic method", row)], count(SECOND));
    assert_eq!(refused.err(), Some(AdmissionError::UnknownChannel { row }));
}

#[test]
fn a_lens_refuses_an_error_in_another_dimension() {
    let case = water_case();
    let [a, _] = bare(&case);
    let row = kind(case.body(), case.rows()[0]);
    let refused = case
        .body()
        .lens(vec![a.erring(measure::<Pressure>(1.0))], count(SECOND));
    assert_eq!(refused.err(), Some(AdmissionError::ChannelDimension { row }));
}

#[test]
fn a_lens_refuses_a_negative_error() {
    let case = water_case();
    let [a, b] = bare(&case);
    let row = kind(case.body(), case.rows()[0]);
    let refused = case
        .body()
        .lens(vec![a.erring(measure::<Water>(-1.0))], count(SECOND));
    assert_eq!(refused.err(), Some(AdmissionError::NegativeChannelError { row }));
    assert!(
        case.body()
            .lens(vec![b.erring(measure::<Water>(0.0))], count(SECOND))
            .is_ok()
    );
}

#[test]
fn a_lens_refuses_two_channels_that_read_the_same_way() {
    let case = water_case();
    let [a, _] = bare(&case);
    let [same, _] = bare(&case);
    let row = kind(case.body(), case.rows()[0]);
    let refused = case.body().lens(vec![a, same], count(SECOND));
    assert_eq!(refused.err(), Some(AdmissionError::DuplicateChannel { row }));
    let [a, _] = bare(&case);
    let [windowed, _] = bare(&case);
    let legal = vec![a, windowed.averaged(count(2))];
    assert!(case.body().lens(legal, count(SECOND)).is_ok());
}

/// Two devices on one row are two methods, each with its own errors.
#[test]
fn a_method_names_a_channel_and_keeps_its_draws_apart() {
    let case = water_case();
    let row = kind(case.body(), case.rows()[0]);
    let device = |method| ChannelSpec::new(method, row).erring(measure::<Water>(SYNTHETIC_SD));
    let lens = lens(
        case.body(),
        vec![device("left device"), device("right device")],
        SECOND,
    );
    let [left, right] = lens.channels() else {
        panic!("two channels");
    };
    assert_eq!((left.method(), right.method()), ("left device", "right device"));
    assert_eq!(left.row(), case.rows()[0]);
    assert_ne!(left.error(seed(), at(3)), right.error(seed(), at(3)));
}

#[test]
fn a_lens_for_another_body_is_refused() {
    let (case, other) = (water_case(), TwoReservoirs::closed().expect("a body"));
    let core = reference();
    let [a, _] = bare(&other);
    let foreign = lens(other.body(), vec![a], SECOND);
    let session = case.admit(&core).expect("admitted");
    assert_eq!(
        session.observation(&foreign).err(),
        Some(ObservationRefusal::OtherLens)
    );
}

/// Refuses the first attempt from `from` on, then faults on its retry.
struct StopsFrom {
    inner: ReferenceIntegrator,
    from: SimTime,
}
impl CoupledSolver for StopsFrom {
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
        if ticket.attempt() == 0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        Err(ticket.mismatch().into())
    }
}

#[test]
fn a_view_reads_the_last_accepted_revision_during_a_retry_and_after_a_fault() {
    let case = water_case();
    let core = StopsFrom {
        inner: reference(),
        from: at(8),
    };
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a], SECOND);
    let (session, _) = step(case.admit(&core).expect("admitted"));
    let head = session.record().seal().digest();
    let samples = session.observation(&lens).expect("a view").samples();
    assert_eq!(samples.len(), 8);
    let Step::Refused(retrying) = session.step() else {
        panic!("the first attempt is refused");
    };
    let view = retrying.observation(&lens).expect("a view");
    assert_eq!((view.seal(), view.samples()), (head, samples.clone()));
    let Step::Faulted(stopped) = retrying.retry() else {
        panic!("the retry faults");
    };
    let view = stopped.observation(&lens).expect("a view");
    assert_eq!((view.seal(), view.samples()), (head, samples));
}

#[test]
fn a_view_through_another_lens_is_refused() {
    let case = water_case();
    let core = reference();
    let [a, b] = bare(&case);
    let (own, other) = (
        lens(case.body(), vec![a], SECOND),
        lens(case.body(), vec![b], SECOND),
    );
    let session = case.admit(&core).expect("admitted");
    let view = session.observation(&other).expect("a view");
    let mut observer = Observer::new(&own, seed());
    assert_eq!(observer.advance(&view).err(), Some(ObservationRefusal::OtherLens));
}

#[test]
fn resume_refuses_another_lens_and_another_seed() {
    let case = water_case();
    let [a, b] = bare(&case);
    let (own, other) = (
        lens(case.body(), vec![a], SECOND),
        lens(case.body(), vec![b], SECOND),
    );
    let saved = Observer::new(&own, seed()).checkpoint();
    let refused = |result: Result<Observer<'_>, ObservationRefusal>| result.err();
    assert_eq!(
        refused(Observer::resume(&other, seed(), &saved)),
        Some(ObservationRefusal::OtherLens)
    );
    let reseeded = ObservationSeed::new([9, 9, 9, 9]);
    assert_eq!(
        refused(Observer::resume(&own, reseeded, &saved)),
        Some(ObservationRefusal::OtherLens)
    );
}

#[test]
fn any_change_to_checkpoint_bytes_is_refused_and_never_panics() {
    let case = water_case();
    let lens = lab(&case);
    let core = reference();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(&lens, seed());
    read(&mut observer, &session, &lens);
    for _ in 0..3 {
        (session, _) = step(session);
        read(&mut observer, &session, &lens);
    }
    let bytes = observer.checkpoint().as_bytes().to_vec();
    assert!(bytes.len() > 100, "held readings and windows are in the bytes");
    for at in 0..bytes.len() {
        let mut flipped = bytes.clone();
        flipped[at] ^= 1;
        let resumed = Observer::resume(&lens, seed(), &ObserverCheckpoint::from_bytes(flipped));
        assert_eq!(resumed.err(), Some(ObservationRefusal::Malformed), "byte {at}");
        let cut = ObserverCheckpoint::from_bytes(bytes[..at].to_vec());
        assert_eq!(
            Observer::resume(&lens, seed(), &cut).err(),
            Some(ObservationRefusal::Malformed)
        );
    }
}

/// The bytes after the raw digest: the binding, the last seal's flag and seal, then each channel's window and
/// queue. A digest has no key, so a file can recompute it.
fn resealed(saved: &ObserverCheckpoint, edit: impl FnOnce(&mut Vec<u8>)) -> ObserverCheckpoint {
    let mut bytes = saved.as_bytes().to_vec();
    let mut body = bytes.split_off(RAW_DIGEST_END);
    edit(&mut body);
    bytes[RAW_DIGEST_END - 32..].copy_from_slice(Digest::of_canonical(&body).as_bytes());
    bytes.extend_from_slice(&body);
    ObserverCheckpoint::from_bytes(bytes)
}

fn word_at(body: &mut [u8], at: usize, word: u64) {
    body[at..at + 8].copy_from_slice(&word.to_be_bytes());
}

const RAW_DIGEST_END: usize = 40;
const LAST_FLAG: usize = 32;
const WINDOW: usize = 72;
const SUM: usize = WINDOW + 8;
const SEEN: usize = SUM + 16;
const COUNT: usize = SEEN + 8;
const QUEUE: usize = COUNT + 8;
const HELD_BYTES: usize = 24;

fn saved_after_reads(case: &TwoReservoirs, lens: &ObservationLens) -> ObserverCheckpoint {
    let core = reference();
    let mut session = case.admit(&core).expect("admitted");
    let mut observer = Observer::new(lens, seed());
    read(&mut observer, &session, lens);
    for _ in 0..2 {
        (session, _) = step(session);
        read(&mut observer, &session, lens);
    }
    observer.checkpoint()
}

#[test]
fn a_resealed_window_is_held_to_what_an_observer_can_leave() {
    let case = water_case();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.averaged(count(7))], SECOND);
    let saved = saved_after_reads(&case, &lens);
    let resumed =
        |edit: &dyn Fn(&mut Vec<u8>)| Observer::resume(&lens, seed(), &resealed(&saved, edit)).err();
    let malformed = Some(ObservationRefusal::Malformed);
    assert_eq!(resumed(&|_| ()), None, "the same bytes, resealed, are good");
    assert_eq!(
        resumed(&|body| word_at(body, SEEN, 6)),
        None,
        "one short of closing"
    );
    assert_eq!(resumed(&|body| word_at(body, SEEN, 7)), malformed);
    assert_eq!(
        resumed(&|body| word_at(body, SEEN, u64::MAX)),
        malformed,
        "it would overflow"
    );
    let infinite = f64::INFINITY.to_bits();
    assert_eq!(resumed(&|body| word_at(body, SUM, infinite)), malformed);
    assert_eq!(resumed(&|body| word_at(body, WINDOW, 2)), malformed);
    assert_eq!(resumed(&|body| word_at(body, LAST_FLAG, 2)), malformed);
    assert_eq!(resumed(&|body| body.push(0)), malformed);
}

#[test]
fn a_resealed_queue_is_held_to_what_an_observer_can_leave() {
    let case = water_case();
    let [a, _] = bare(&case);
    let lens = lens(case.body(), vec![a.delayed(30 * SECOND)], 3 * SECOND);
    let saved = saved_after_reads(&case, &lens);
    let resumed =
        |edit: &dyn Fn(&mut Vec<u8>)| Observer::resume(&lens, seed(), &resealed(&saved, edit)).err();
    let malformed = Some(ObservationRefusal::Malformed);
    let queue = |body: &[u8]| {
        let bytes = body[COUNT..QUEUE].first_chunk().expect("eight bytes");
        u64::from_be_bytes(*bytes)
    };
    resealed(&saved, |body| assert!(queue(body) > 1, "two held readings"));
    let second = QUEUE + HELD_BYTES;
    assert_eq!(resumed(&|_| ()), None, "the same bytes, resealed, are good");
    assert_eq!(
        resumed(&|body| word_at(body, second, 0)),
        malformed,
        "out of order"
    );
    assert_eq!(
        resumed(&|body| word_at(body, second, u64::MAX)),
        malformed,
        "past the clock"
    );
    let infinite = f64::INFINITY.to_bits();
    assert_eq!(resumed(&|body| word_at(body, QUEUE + 8, infinite)), malformed);
    assert_eq!(
        resumed(&|body| word_at(body, COUNT, u64::MAX)),
        malformed,
        "more than the bytes hold"
    );
}
