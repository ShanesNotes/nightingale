use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{Running, Session, Step, StepReceipt};

const ONE_MILLISECOND: u64 = 1_000_000;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn every(ns: u64) -> NonZeroU64 {
    NonZeroU64::new(ns).expect("not zero")
}

fn scales(absolute: f64) -> ErrorScales {
    ErrorScales::uniform(measure(absolute), measure(0.0)).expect("a positive scale")
}

const MUTED: f64 = 1e12;
const REFERENCE_STEP: u64 = 8 * SECOND;
const OFF_STEP: u64 = 7 * SECOND;

/// The last scale is the event time's, in seconds.
fn budget(families: [f64; 4], share: f64) -> SplitBudget {
    let [state, exchange, observable, event_time] = families;
    let (state, exchange, observable) = (scales(state), scales(exchange), scales(observable));
    SplitBudget::new(state, exchange, observable, measure(event_time), measure(share))
        .expect("a share inside (0, 1)")
}

fn gauge(body: &CompiledBody, row: RowId) -> ObservationLens {
    let kind = (body.rows().iter())
        .find(|candidate| candidate.id() == row)
        .expect("a row")
        .kind();
    body.lens(vec![ChannelSpec::new("a synthetic gauge", kind)], every(SECOND))
        .expect("a lens")
}

fn stepped() -> TwoReservoirs {
    TwoReservoirs::stepped_inflow().expect("a synthetic case")
}

fn off_step_grid() -> TraceGrid {
    TraceGrid::new(SimTime::from_ns(0), every(OFF_STEP), 1_000).expect("a grid")
}

fn reference() -> ReferenceIntegrator {
    ReferenceIntegrator::new(REFERENCE_STEP)
}

fn trace(
    case: &dyn ReplayCase,
    core: &dyn CoupledSolver,
    lens: &ObservationLens,
    grid: TraceGrid,
) -> SplitTrace {
    draw_split_trace(case, core, lens, grid).expect("a trace")
}

fn qualified(
    split: &dyn CoupledSolver,
    families: [f64; 4],
    share: f64,
) -> Result<SplitQualification, SplitFailure> {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    split_gate(
        &case,
        split,
        &reference(),
        &lens,
        off_step_grid(),
        budget(families, share),
    )
}

fn refused_with(failure: SplitFailure) -> SplitAssessment {
    match failure {
        SplitFailure::Rejected(SplitRejected::OverBudget(assessment))
        | SplitFailure::Rejected(SplitRejected::ReferenceTooCoarse(assessment)) => assessment,
        other => panic!("not a measured refusal: {other:?}"),
    }
}

fn accepted(mut attempt: Step<'_>) -> (Session<'_, Running>, StepReceipt) {
    loop {
        match attempt {
            Step::Accepted(session, receipt) => return (session, receipt),
            Step::Refused(session) => attempt = session.retry(),
            Step::Faulted(session) => panic!("faulted: {:?}", session.fault()),
        }
    }
}

fn read_alone<F: MetricFamily>(split: &dyn CoupledSolver, families: [f64; 4]) -> (Compensated, Compensated) {
    let assessment = refused_with(qualified(split, families, 0.99).expect_err("over a unit budget"));
    (assessment.maximum::<F>(), assessment.reference_error())
}

fn about_thrice((split, own): (Compensated, Compensated)) -> bool {
    own.times(&measure(2.9)).at_most(split) && split.at_most(own.times(&measure(3.1)))
}

/// At first order a core at twice the reference's interval errs four times as much as the reference's half, and
/// the reference twice as much, so the core is three times as far from the half as the reference is. Each family
/// shows that ratio only when it reads the instants it claims.
#[test]
fn a_split_within_its_budget_is_qualified_against_the_reference_integrator() {
    let split = ReferenceIntegrator::new(2 * REFERENCE_STEP);
    let qualification = qualified(&split, [10.0, 10.0, 10.0, 1.0], 0.25).expect("within the budget");
    assert_eq!(qualification.plan(), split.resolution().digest());
    let case = stepped();
    assert_eq!(qualification.model(), case.body().model_hash());
    let assessment = qualification.assessment();
    assert!(
        !assessment.reference_error().at_most(Compensated::ZERO),
        "the reference's own error is measured"
    );
    let grid_families = [
        assessment.maximum::<StateFamily>(),
        assessment.maximum::<ExchangeFamily>(),
        assessment.maximum::<ObservableFamily>(),
    ];
    for family in grid_families {
        assert!(
            !family.at_most(Compensated::ZERO),
            "every grid family differs: {assessment:?}"
        );
    }
    assert_eq!(
        assessment.maximum::<EventTimeFamily>(),
        Compensated::ZERO,
        "the case has no hazard"
    );

    assert!(about_thrice(read_alone::<StateFamily>(
        &split,
        [1.0, MUTED, MUTED, MUTED]
    )));
    assert!(about_thrice(read_alone::<ExchangeFamily>(
        &split,
        [MUTED, 1.0, MUTED, MUTED]
    )));
    assert!(about_thrice(read_alone::<ObservableFamily>(
        &split,
        [MUTED, MUTED, 1.0, MUTED]
    )));
}

#[test]
fn a_split_over_its_budget_is_refused() {
    let coarse = ReferenceIntegrator::new(32 * REFERENCE_STEP);
    let failure = qualified(&coarse, [10.0, 10.0, 10.0, 1.0], 0.25).expect_err("over the budget");
    assert!(
        matches!(failure, SplitFailure::Rejected(SplitRejected::OverBudget(_))),
        "{failure:?}"
    );
}

#[test]
fn a_reference_coarser_than_its_share_is_refused() {
    let split = ReferenceIntegrator::new(2 * REFERENCE_STEP);
    let failure = qualified(&split, [10.0, 10.0, 10.0, 1.0], 0.05).expect_err("the reference is too coarse");
    assert!(
        matches!(
            failure,
            SplitFailure::Rejected(SplitRejected::ReferenceTooCoarse(_))
        ),
        "{failure:?}"
    );
}

#[test]
fn a_core_is_never_measured_against_itself() {
    let half = ReferenceIntegrator::new(REFERENCE_STEP / 2);
    let failure = qualified(&half, [10.0, 10.0, 10.0, 1.0], 0.25).expect_err("its own run");
    assert!(
        matches!(failure, SplitFailure::Rejected(SplitRejected::SameRun)),
        "{failure:?}"
    );
}

#[test]
fn a_reference_error_measured_on_another_reference_is_refused() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let easy = trace(
        &case,
        &ReferenceIntegrator::new(4 * SECOND),
        &lens,
        off_step_grid(),
    );
    let finer = trace(
        &case,
        &ReferenceIntegrator::new(2 * SECOND),
        &lens,
        off_step_grid(),
    );
    let error = ReferenceError::self_convergence(&easy, &finer).expect("comparable");
    let expected = trace(&case, &reference(), &lens, off_step_grid());
    let split = trace(
        &case,
        &ReferenceIntegrator::new(2 * REFERENCE_STEP),
        &lens,
        off_step_grid(),
    );
    let refused = SplitMeasurement::new(expected, split, error, budget([10.0; 4], 0.25));
    assert!(matches!(refused, Err(SplitRejected::OtherReference)));
}

/// Read the other way round, the pair would measure the coarser run's error as the finer run's distance from it,
/// which is about half that error.
#[test]
fn a_reference_whose_steps_are_not_half_the_coarser_runs_is_refused() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let coarse = trace(&case, &reference(), &lens, off_step_grid());
    let half = trace(
        &case,
        &ReferenceIntegrator::new(REFERENCE_STEP / 2),
        &lens,
        off_step_grid(),
    );
    let nearly = trace(
        &case,
        &ReferenceIntegrator::new(REFERENCE_STEP / 2 + 1),
        &lens,
        off_step_grid(),
    );
    ReferenceError::self_convergence(&coarse, &half).expect("a refined reference");
    assert!(matches!(
        ReferenceError::self_convergence(&half, &coarse),
        Err(SplitRejected::NotRefined)
    ));
    assert!(matches!(
        ReferenceError::self_convergence(&coarse, &nearly),
        Err(SplitRejected::NotRefined)
    ));
}

#[test]
fn traces_of_another_grid_schedule_or_seed_are_not_compared() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let expected = trace(&case, &reference(), &lens, off_step_grid());
    let shorter = TraceGrid::new(SimTime::from_ns(0), every(OFF_STEP), 999).expect("a grid");
    let other_grid = trace(
        &case,
        &ReferenceIntegrator::new(2 * REFERENCE_STEP),
        &lens,
        shorter,
    );
    let refused = ReferenceError::self_convergence(&expected, &other_grid);
    assert!(matches!(refused, Err(SplitRejected::OtherGrid)));

    let later = Delivery {
        id: b"stepped inflow",
        from: SimTime::from_ns(2_100 * SECOND),
        to: SimTime::from_ns(6_000 * SECOND),
        amount: measure(20_000.0),
    };
    let start = [measure(500_500.0), measure(499_500.0)];
    let moved = TwoReservoirs::delivering(start, vec![later]).expect("a synthetic case");
    let other_schedule = trace(
        &moved,
        &ReferenceIntegrator::new(2 * REFERENCE_STEP),
        &lens,
        off_step_grid(),
    );
    let refused = ReferenceError::self_convergence(&expected, &other_schedule);
    assert!(
        matches!(refused, Err(SplitRejected::OtherSchedule)),
        "{:?}",
        refused.err()
    );

    let chance = ChanceCase::ectopy()
        .expect("a synthetic case")
        .until(SimTime::from_ns(60 * SECOND));
    let chance_lens = gauge(chance.body(), chance.rows()[0]);
    let short = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 61).expect("a grid");
    let first = trace(
        &chance,
        &ReferenceIntegrator::new(SECOND / 10),
        &chance_lens,
        short,
    );
    let reseeded = ChanceCase::ectopy()
        .expect("a synthetic case")
        .seeded(Seed::new([9, 9, 9, 9]));
    let second = trace(
        &reseeded,
        &ReferenceIntegrator::new(SECOND / 20),
        &chance_lens,
        short,
    );
    let refused = ReferenceError::self_convergence(&first, &second);
    assert!(matches!(refused, Err(SplitRejected::OtherCourse)));
}

/// Each core locates a hazard's crossing for itself, so a grid that ends between the two runs' times of one firing
/// holds it in one trace only.
#[test]
fn the_event_family_compares_each_firing_and_refuses_a_set_that_differs() {
    let case = ChanceCase::ectopy()
        .expect("a synthetic case")
        .until(SimTime::from_ns(60 * SECOND));
    let lens = gauge(case.body(), case.rows()[0]);
    let grid = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 61).expect("a grid");
    let reference = ReferenceIntegrator::new(SECOND / 10);
    let split = ReferenceIntegrator::new(SECOND / 5);
    let qualification = split_gate(
        &case,
        &split,
        &reference,
        &lens,
        grid,
        budget([1e3, 1e3, 1e3, 1.0], 0.5),
    )
    .expect("within the budget");
    let event_time = qualification.assessment().maximum::<EventTimeFamily>();
    assert!(
        !event_time.at_most(Compensated::ZERO),
        "a firing moved between the two runs"
    );

    let firings = |core: &ReferenceIntegrator| -> Vec<SimTime> {
        let store = MemoryStore::default();
        let mut session = case.admit(core, &store).expect("admitted");
        let mut times = Vec::new();
        while session.record().accepted().time() < grid.end() {
            let (next, receipt) = accepted(session.step());
            times.extend(receipt.fired().iter().map(|firing| firing.at()));
            session = next;
        }
        times
    };
    let (expected, measured) = (firings(&reference), firings(&split));
    let (first, second) = (expected.iter().zip(&measured))
        .map(|(a, b)| if a < b { (*a, *b) } else { (*b, *a) })
        .find(|(a, b)| a < b)
        .expect("a firing the two runs place apart");
    let between =
        TraceGrid::new(SimTime::from_ns(first.nanoseconds() - SECOND), every(SECOND), 2).expect("a grid");
    assert!(first <= between.end() && between.end() < second);
    let expected = trace(&case, &reference, &lens, between);
    let measured = trace(&case, &split, &lens, between);
    assert!(matches!(
        ReferenceError::self_convergence(&expected, &measured),
        Err(SplitRejected::OtherEvents)
    ));
}

fn drawn_from<'m>(
    mut session: Session<'m, Running>,
    draw: &mut TraceDraw,
    until: SimTime,
) -> Session<'m, Running> {
    while session.record().accepted().time() < until {
        let (next, receipt) = accepted(session.step());
        next.record()
            .split_trace(draw, Some(&receipt))
            .expect("the next head");
        session = next;
    }
    session
}

#[test]
fn a_draw_refuses_a_head_of_another_course_of_the_same_run() {
    let start = || [measure(500_500.0), measure(499_500.0)];
    let intake = |amount| Delivery {
        id: b"a synthetic intake",
        from: SimTime::from_ns(8 * SECOND),
        to: SimTime::from_ns(16 * SECOND),
        amount: measure(amount),
    };
    let one = TwoReservoirs::delivering(start(), vec![intake(100.0)]).expect("a synthetic case");
    let other = TwoReservoirs::delivering(start(), vec![intake(200.0)]).expect("a synthetic case");
    let lens = gauge(one.body(), one.rows()[0]);
    let (core, store) = (reference(), MemoryStore::default());
    let session = one.admit(&core, &store).expect("admitted");
    let mut draw = TraceDraw::new(one.body(), &lens, off_step_grid()).expect("a draw");
    session
        .record()
        .split_trace(&mut draw, None)
        .expect("opens at admission");
    let session = drawn_from(session, &mut draw, SimTime::from_ns(16 * SECOND));
    let mut parted = other.admit(&core, &store).expect("admitted");
    while parted.record().accepted().time() < SimTime::from_ns(16 * SECOND) {
        parted = accepted(parted.step()).0;
    }
    assert_eq!(
        parted.record().identity().digest(),
        session.record().identity().digest(),
        "one run"
    );
    assert_ne!(parted.record().seal(), session.record().seal(), "two courses");
    let (parted, receipt) = accepted(parted.step());
    let started = parted
        .record()
        .accepted()
        .last_segment()
        .interval()
        .map(|step| step.start());
    assert_eq!(
        started,
        Some(session.record().accepted().time()),
        "its step starts where the draw stopped"
    );
    assert_eq!(
        parted.record().split_trace(&mut draw, Some(&receipt)),
        Err(SplitRejected::Gap)
    );
}

#[test]
fn a_draw_reads_each_accepted_head_once_and_in_order() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let store = MemoryStore::default();
    let core = reference();
    let opened = case.admit(&core, &store).expect("admitted");
    let mut draw = TraceDraw::new(case.body(), &lens, off_step_grid()).expect("a draw");
    let (first, receipt) = accepted(opened.step());
    let refused = first.record().split_trace(&mut draw, Some(&receipt));
    assert_eq!(refused, Err(SplitRejected::Gap), "a step before the draw opened");
    assert_eq!(
        first.record().split_trace(&mut draw, None),
        Err(SplitRejected::NotAdmission)
    );

    let session = case.admit(&core, &store).expect("admitted");
    session
        .record()
        .split_trace(&mut draw, None)
        .expect("opens at admission");
    assert_eq!(
        session.record().split_trace(&mut draw, None),
        Err(SplitRejected::Gap),
        "opened twice"
    );
    let (first, one) = accepted(session.step());
    let (second, two) = accepted(first.step());
    assert_eq!(
        second.record().split_trace(&mut draw, Some(&one)),
        Err(SplitRejected::OtherHead)
    );
    assert_eq!(
        second.record().split_trace(&mut draw, Some(&two)),
        Err(SplitRejected::Gap),
        "skipped a head"
    );

    let other = ReferenceIntegrator::new(2 * REFERENCE_STEP);
    let foreign = case.admit(&other, &store).expect("admitted");
    let (foreign, three) = accepted(foreign.step());
    assert_eq!(
        foreign.record().split_trace(&mut draw, Some(&three)),
        Err(SplitRejected::Gap)
    );

    let (session, one) = accepted(case.admit(&core, &store).expect("admitted").step());
    session
        .record()
        .split_trace(&mut draw, Some(&one))
        .expect("the next head");
    let short = SimTime::from_ns(off_step_grid().end().nanoseconds() - SECOND);
    let session = drawn_from(session, &mut draw, short);
    assert!(session.record().accepted().time() < off_step_grid().end());
    assert!(matches!(draw.finish(), Err(SplitRejected::Unfinished)));
}

#[test]
fn a_draw_needs_every_family_on_a_grid_of_two_instants_or_more() {
    let case = stepped();
    let other = ChanceCase::ectopy().expect("a synthetic case");
    let foreign = gauge(other.body(), other.rows()[0]);
    assert!(matches!(
        TraceDraw::new(case.body(), &foreign, off_step_grid()),
        Err(SplitRejected::OtherBody)
    ));
    let blind = case.body().lens(Vec::new(), every(SECOND)).expect("a lens");
    assert!(matches!(
        TraceDraw::new(case.body(), &blind, off_step_grid()),
        Err(SplitRejected::MissingFamily)
    ));
    let one = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 1);
    assert!(matches!(one, Err(AdmissionError::TraceGridOutOfRange)));
    let past = TraceGrid::new(SimTime::from_ns(u64::MAX - SECOND), every(SECOND), 3);
    assert!(matches!(past, Err(AdmissionError::TraceGridOutOfRange)));
    let whole = SplitBudget::new(scales(1.0), scales(1.0), scales(1.0), measure(1.0), measure(1.0));
    assert!(matches!(whole, Err(AdmissionError::ReferenceShare)));
}

#[test]
fn inputs_of_another_payload_under_one_id_are_not_compared() {
    let start = || [measure(500_500.0), measure(499_500.0)];
    let intake = |amount| Delivery {
        id: b"a synthetic intake",
        from: SimTime::from_ns(80 * SECOND),
        to: SimTime::from_ns(160 * SECOND),
        amount: measure(amount),
    };
    let one = TwoReservoirs::delivering(start(), vec![intake(100.0)]).expect("a synthetic case");
    let other = TwoReservoirs::delivering(start(), vec![intake(200.0)]).expect("a synthetic case");
    let lens = gauge(one.body(), one.rows()[0]);
    let expected = trace(&one, &reference(), &lens, off_step_grid());
    let measured = trace(
        &other,
        &ReferenceIntegrator::new(2 * REFERENCE_STEP),
        &lens,
        off_step_grid(),
    );
    let refused = ReferenceError::self_convergence(&expected, &measured);
    assert!(
        matches!(refused, Err(SplitRejected::OtherSchedule)),
        "{:?}",
        refused.err()
    );
}

#[test]
fn a_draw_opens_only_at_admission() {
    let case = ChanceCase::ectopy().expect("a synthetic case");
    let lens = gauge(case.body(), case.rows()[0]);
    let (core, store) = (ReferenceIntegrator::new(SECOND / 10), MemoryStore::default());
    let mut session = case.admit(&core, &store).expect("admitted");
    let fired = loop {
        let (next, receipt) = accepted(session.step());
        session = next;
        if let Some(firing) = receipt.fired().first() {
            break firing.at();
        }
    };
    assert_eq!(
        session.record().accepted().time(),
        fired,
        "a firing ends its step"
    );
    let grid = TraceGrid::new(fired, every(SECOND), 2).expect("a grid");
    let mut draw = TraceDraw::new(case.body(), &lens, grid).expect("a draw");
    assert_eq!(
        session.record().split_trace(&mut draw, None),
        Err(SplitRejected::NotAdmission)
    );
}

/// A forcing's knot period caps every step of this case, so a 100 s reference and its half take one course.
#[test]
fn a_reference_its_refinement_cannot_move_is_refused() {
    let case = ChanceCase::ectopy().expect("a synthetic case");
    let lens = gauge(case.body(), case.rows()[0]);
    let grid = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 61).expect("a grid");
    let loose = || ErrorScales::uniform(measure(1e6), measure(0.0)).expect("a positive scale");
    let reference = ReferenceIntegrator::new(100 * SECOND).with_scales(loose());
    let split = ReferenceIntegrator::new(400 * SECOND).with_scales(loose());
    let outcome = split_gate(&case, &split, &reference, &lens, grid, budget([1e-6; 4], 0.5));
    assert!(
        matches!(outcome, Err(SplitFailure::Rejected(SplitRejected::NotRefined))),
        "{outcome:?}"
    );
}

fn first_firing(case: &ChanceCase, core: &ReferenceIntegrator) -> SimTime {
    let store = MemoryStore::default();
    let mut session = case.admit(core, &store).expect("admitted");
    loop {
        let (next, receipt) = accepted(session.step());
        session = next;
        if let Some(firing) = receipt.fired().first() {
            return firing.at();
        }
    }
}

/// A firing at the grid's first instant belongs to the step that ends there, so it is in no trace on that grid.
#[test]
fn a_firing_at_the_first_instant_is_in_no_trace() {
    let case = ChanceCase::ectopy().expect("a synthetic case");
    let lens = gauge(case.body(), case.rows()[0]);
    let (reference, refined) = (
        ReferenceIntegrator::new(SECOND / 10),
        ReferenceIntegrator::new(SECOND / 20),
    );
    let (one, other) = (first_firing(&case, &reference), first_firing(&case, &refined));
    assert_ne!(one, other, "two cores place the firing apart");
    let later = if one < other { other } else { one };
    let grid = TraceGrid::new(later, every(ONE_MILLISECOND), 2).expect("a grid");
    let expected = trace(&case, &reference, &lens, grid);
    let measured = trace(&case, &refined, &lens, grid);
    ReferenceError::self_convergence(&expected, &measured).expect("neither trace holds the firing");
}

/// A genuine admission of the case's doubled conductance, which no session of the case admits.
fn admitted_elsewhere(case: &TwoReservoirs, core: &ReferenceIntegrator) -> InitialRevision {
    let origin = SimTime::from_ns(0);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let other = case.other_values().expect("bound");
    let system = (case.body())
        .initialization(other, port, InitializationRequest::at(origin))
        .expect("a system");
    let start = MethodHistory::new(core.resolution().policy(), origin, Vec::new());
    let solved = system.workspace();
    system.admit(solved, start).expect("admitted")
}

#[test]
fn a_draw_opens_only_on_an_admission_of_its_identitys_packet() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let (core, store) = (reference(), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let identity = session.record().identity().clone();
    let mut draw = TraceDraw::new(case.body(), &lens, off_step_grid()).expect("a draw");
    let elsewhere = AcceptedHead::admitted(admitted_elsewhere(&case, &core), RecordHash::Blake2b);
    assert_eq!(
        draw.open(&identity, &elsewhere),
        Err(SplitRejected::NotAdmission),
        "another packet"
    );
    let parts = admitted_elsewhere(&case, &core).into_parts();
    let packet = parts.parameters().packet_hash();
    let restored = AcceptedHead::restored(parts, RecordHash::Blake2b);
    let own = RunIdentity::new(
        case.body().model_hash(),
        packet,
        Seed::new([1, 2, 3, 4]),
        core.resolution(),
    );
    assert_eq!(
        draw.open(&own, &restored),
        Err(SplitRejected::NotAdmission),
        "parts from elsewhere"
    );
    session
        .record()
        .split_trace(&mut draw, None)
        .expect("its own admission opens");
}

/// One step of a course driven by hand, settled through the head as a record settles it, retrying each refusal.
fn settle(
    case: &TwoReservoirs,
    core: &ReferenceIntegrator,
    head: &AcceptedHead,
    identity: &RunIdentity,
) -> NextHead {
    let base = Digest::of_canonical(b"a synthetic base");
    let mut refusal: Option<Refused> = None;
    loop {
        let attempt = match &refusal {
            None => Attempt::First {
                span_ns: core.next_step(head.view()),
            },
            Some(refused) => Attempt::After(refused),
        };
        let solve = |problem: &CoupledProblem<'_>, ticket: &StepTicket| core.solve(problem, ticket);
        match head.attempt(case.body(), identity, base, attempt, solve, |_| Ok(())) {
            Ok((next, ())) => return next,
            Err(TrialFailure::Refused(refused)) => refusal = Some(refused),
            Err(TrialFailure::EngineFault(fault)) => panic!("faulted: {fault:?}"),
        }
    }
}

fn run_of(case: &TwoReservoirs, core: &ReferenceIntegrator, words: [u64; 4]) -> RunIdentity {
    let packet = admitted_elsewhere(case, core).packet_hash();
    RunIdentity::new(
        case.body().model_hash(),
        packet,
        Seed::new(words),
        core.resolution(),
    )
}

/// A head moves only by installing a step the contract settled, so a course driven by hand draws only steps
/// that start at the head read last, under the run the draw opened.
#[test]
fn a_draw_refuses_a_step_of_another_head_or_run() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let core = reference();
    let (identity, reseeded) = (
        run_of(&case, &core, [1, 2, 3, 4]),
        run_of(&case, &core, [5, 6, 7, 8]),
    );
    let head = || AcceptedHead::admitted(admitted_elsewhere(&case, &core), RecordHash::Blake2b);
    let mut draw = TraceDraw::new(case.body(), &lens, off_step_grid()).expect("a draw");
    let mut opened = head();
    draw.open(&identity, &opened).expect("an admission head opens");

    let mut other = head();
    let extents = other.install(settle(&case, &core, &other, &reseeded));
    assert_eq!(
        draw.read(&other, &extents),
        Err(SplitRejected::Gap),
        "another seed"
    );
    let mut ahead = head();
    let _first = ahead.install(settle(&case, &core, &ahead, &identity));
    let extents = opened.install(settle(&case, &core, &ahead, &identity));
    assert_eq!(
        draw.read(&opened, &extents),
        Err(SplitRejected::Gap),
        "a step settled on another head"
    );

    let mut course = head();
    while course.parts().time < off_step_grid().end() {
        let extents = course.install(settle(&case, &core, &course, &identity));
        draw.read(&course, &extents).expect("the next head");
    }
    draw.finish().expect("a trace from a course driven by hand");
}

#[test]
fn a_head_settles_steps_only_for_its_own_body_and_packet() {
    let case = stepped();
    let (core, store) = (reference(), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let head = AcceptedHead::admitted(admitted_elsewhere(&case, &core), RecordHash::Blake2b);
    let other = TwoReservoirs::closed().expect("a synthetic case");
    let own = run_of(&case, &core, [1, 2, 3, 4]);
    let base = Digest::of_canonical(b"a synthetic base");
    let solve = |problem: &CoupledProblem<'_>, ticket: &StepTicket| core.solve(problem, ticket);
    let attempt = Attempt::First {
        span_ns: REFERENCE_STEP,
    };
    for (body, identity) in [(other.body(), &own), (case.body(), session.record().identity())] {
        let refused = head.attempt(body, identity, base, attempt, solve, |_| Ok(()));
        assert!(
            matches!(&refused, Err(TrialFailure::EngineFault(fault)) if fault.kind() == EngineFaultKind::BindingMismatch),
            "{:?}",
            refused.err()
        );
    }
}

/// Parts from bytes carry waiting inputs no admission took, and an input that arrives between a step's attempt
/// and its install is one that step could not apply.
#[test]
fn a_draw_refuses_a_head_whose_inputs_no_admission_took() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let core = reference();
    let identity = run_of(&case, &core, [1, 2, 3, 4]);
    let mut draw = TraceDraw::new(case.body(), &lens, off_step_grid()).expect("a draw");
    let opened = AcceptedHead::admitted(admitted_elsewhere(&case, &core), RecordHash::Blake2b);
    draw.open(&identity, &opened).expect("an admission head opens");

    let parts = admitted_elsewhere(&case, &core).into_parts();
    let mut restored = AcceptedHead::restored(parts, RecordHash::Blake2b);
    assert_eq!(restored.image_digest(), opened.image_digest(), "the same image");
    let extents = restored.install(settle(&case, &core, &restored, &identity));
    assert_eq!(
        draw.read(&restored, &extents),
        Err(SplitRejected::Gap),
        "restored parts"
    );

    let mut late = AcceptedHead::admitted(admitted_elsewhere(&case, &core), RecordHash::Blake2b);
    let next = settle(&case, &core, &late, &identity);
    let event = case.events().expect("an event").remove(0);
    late.admit_input(case.body(), event)
        .expect("admitted at the head");
    let extents = late.install(next);
    assert_eq!(
        draw.read(&late, &extents),
        Err(SplitRejected::Gap),
        "a late input"
    );
}

#[test]
fn a_draw_refuses_a_grid_that_starts_before_admission() {
    let body = arena_body().expect("compiled");
    let (lens, store) = (arena_lens(&body).expect("a lens"), MemoryStore::default());
    let core = ReferenceIntegrator::new(SECOND / 1_000);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let packet = arena_packet(&body).expect("a packet");
    let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment(), &store).expect("admitted");
    let grid = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 2).expect("a grid");
    let mut draw = TraceDraw::new(&body, &lens, grid).expect("a draw");
    assert_eq!(
        admitted.session.record().split_trace(&mut draw, None),
        Err(SplitRejected::PastStart)
    );
}

#[test]
fn a_draw_is_done_once_it_reads_the_head_at_its_last_instant() {
    let case = stepped();
    let lens = gauge(case.body(), case.rows()[0]);
    let (core, store) = (reference(), MemoryStore::default());
    let grid = TraceGrid::new(SimTime::from_ns(0), every(REFERENCE_STEP), 2).expect("a grid");
    let mut draw = TraceDraw::new(case.body(), &lens, grid).expect("a draw");
    let session = case.admit(&core, &store).expect("admitted");
    session
        .record()
        .split_trace(&mut draw, None)
        .expect("opens at admission");
    assert!(!draw.done(), "the last instant is not drawn yet");
    let session = drawn_from(session, &mut draw, grid.end());
    assert_eq!(session.record().accepted().time(), grid.end());
    assert!(draw.done());
    draw.finish().expect("a trace");
}
