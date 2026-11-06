use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{Running, Session, Step, StepReceipt};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const UNTIL: u64 = 6_993 * SECOND;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn every(ns: u64) -> NonZeroU64 {
    NonZeroU64::new(ns).expect("not zero")
}

fn scales(absolute: f64) -> ErrorScales {
    ErrorScales::uniform(measure(absolute), measure(0.0)).expect("a positive scale")
}

/// A scale so wide that a family it holds adds nothing, so one family's error can be read alone.
const ALONE: f64 = 1e12;

fn budget(families: [f64; 4], share: f64) -> SplitBudget {
    let [state, exchange, observable, event_time] = families.map(scales);
    SplitBudget::new(state, exchange, observable, event_time, measure(share)).expect("a share inside (0, 1)")
}

fn lens(body: &CompiledBody, row: RowId) -> ObservationLens {
    let kind = (body.rows().iter()).find(|candidate| candidate.id() == row).expect("a row").kind();
    body.lens(vec![ChannelSpec::new("a synthetic gauge", kind)], every(SECOND))
        .expect("a lens")
}

fn stepped() -> TwoReservoirs {
    TwoReservoirs::stepped_inflow().expect("a synthetic case")
}

/// Seven seconds against steps of 4, 8 and 16 s, so most instants fall inside a step and read its chord.
fn grid() -> TraceGrid {
    TraceGrid::new(SimTime::from_ns(0), every(7 * SECOND), 1_000).expect("a grid")
}

fn reference() -> ReferenceIntegrator {
    ReferenceIntegrator::new(8 * SECOND)
}

fn trace(case: &dyn ReplayCase, core: &dyn CoupledSolver, lens: &ObservationLens, grid: TraceGrid) -> SplitTrace {
    draw_split_trace(case, core, lens, grid).expect("a trace")
}

fn qualified(split: &dyn CoupledSolver, families: [f64; 4], share: f64) -> Result<SplitQualification, SplitFailure> {
    let case = stepped();
    let lens = lens(case.body(), case.rows()[0]);
    split_gate(&case, split, &reference(), &lens, grid(), budget(families, share))
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

/// The split is a coupled core at twice the reference's interval, so its error is about twice the reference's own:
/// a ratio the families show only when each reads the instants it claims.
#[test]
fn a_split_within_its_budget_is_qualified_against_the_reference_integrator() {
    let split = ReferenceIntegrator::new(16 * SECOND);
    let qualification = qualified(&split, [10.0, 10.0, 10.0, 1.0], 0.25).expect("within the budget");
    assert_eq!(qualification.plan(), split.resolution().digest());
    let case = stepped();
    assert_eq!(qualification.model(), case.body().model_hash());
    let assessment = qualification.assessment();
    assert!(!assessment.reference_error().at_most(Compensated::ZERO), "the reference's own error is measured");
    let [state, exchange, observable, event_time] = assessment.maxima();
    for family in [state, exchange, observable] {
        assert!(!family.at_most(Compensated::ZERO), "every grid family differs: {assessment:?}");
    }
    assert_eq!(event_time, Compensated::ZERO, "the case has no hazard");

    for alone in 0..3 {
        let mut families = [ALONE; 4];
        families[alone] = 1.0;
        let failure = qualified(&split, families, 0.99).expect_err("over a unit budget");
        let assessment = refused_with(failure);
        let (split, own) = (assessment.maxima()[alone], assessment.reference_error());
        let within = own.times(&measure(1.9)).at_most(split) && split.at_most(own.times(&measure(2.1)));
        assert!(within, "family {alone}: split {split:?} against the reference's own {own:?}");
    }
}

#[test]
fn a_split_over_its_budget_is_refused() {
    let coarse = ReferenceIntegrator::new(256 * SECOND);
    let failure = qualified(&coarse, [10.0, 10.0, 10.0, 1.0], 0.25).expect_err("over the budget");
    assert!(matches!(failure, SplitFailure::Rejected(SplitRejected::OverBudget(_))), "{failure:?}");
}

#[test]
fn a_reference_coarser_than_its_share_is_refused() {
    let split = ReferenceIntegrator::new(16 * SECOND);
    let failure = qualified(&split, [10.0, 10.0, 10.0, 1.0], 0.05).expect_err("the reference is too coarse");
    assert!(
        matches!(failure, SplitFailure::Rejected(SplitRejected::ReferenceTooCoarse(_))),
        "{failure:?}"
    );
}

#[test]
fn a_core_is_never_measured_against_itself() {
    let failure = qualified(&reference(), [10.0, 10.0, 10.0, 1.0], 0.25).expect_err("its own run");
    assert!(matches!(failure, SplitFailure::Rejected(SplitRejected::SameRun)), "{failure:?}");
}

#[test]
fn a_reference_error_measured_on_another_reference_is_refused() {
    let case = stepped();
    let lens = lens(case.body(), case.rows()[0]);
    let easy = trace(&case, &ReferenceIntegrator::new(4 * SECOND), &lens, grid());
    let finer = trace(&case, &ReferenceIntegrator::new(2 * SECOND), &lens, grid());
    let error = ReferenceError::self_convergence(&easy, &finer).expect("comparable");
    let expected = trace(&case, &reference(), &lens, grid());
    let split = trace(&case, &ReferenceIntegrator::new(16 * SECOND), &lens, grid());
    let refused = SplitMeasurement::new(expected, split, error, budget([10.0; 4], 0.25));
    assert!(matches!(refused, Err(SplitRejected::OtherReference)));
}

#[test]
fn traces_of_another_grid_schedule_or_seed_are_not_compared() {
    let case = stepped();
    let lens = lens(case.body(), case.rows()[0]);
    let expected = trace(&case, &reference(), &lens, grid());
    let shorter = TraceGrid::new(SimTime::from_ns(0), every(7 * SECOND), 999).expect("a grid");
    let other_grid = trace(&case, &ReferenceIntegrator::new(16 * SECOND), &lens, shorter);
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
    let other_schedule = trace(&moved, &ReferenceIntegrator::new(16 * SECOND), &lens, grid());
    let refused = ReferenceError::self_convergence(&expected, &other_schedule);
    assert!(matches!(refused, Err(SplitRejected::OtherSchedule)), "{:?}", refused.err());

    let chance = ChanceCase::ectopy().expect("a synthetic case").until(SimTime::from_ns(60 * SECOND));
    let chance_lens = lens(chance.body(), chance.rows()[0]);
    let short = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 61).expect("a grid");
    let first = trace(&chance, &ReferenceIntegrator::new(SECOND / 10), &chance_lens, short);
    let reseeded = ChanceCase::ectopy().expect("a synthetic case").seeded(Seed::new([9, 9, 9, 9]));
    let second = trace(&reseeded, &ReferenceIntegrator::new(SECOND / 20), &chance_lens, short);
    let refused = ReferenceError::self_convergence(&first, &second);
    assert!(matches!(refused, Err(SplitRejected::OtherCourse)));
}

/// Hazards fire at instants each core locates for itself, so the event family compares one firing's times, and a
/// grid that ends between them sees one run's firing and not the other's.
#[test]
fn the_event_family_compares_each_firing_and_refuses_a_set_that_differs() {
    let case = ChanceCase::ectopy().expect("a synthetic case").until(SimTime::from_ns(60 * SECOND));
    let lens = lens(case.body(), case.rows()[0]);
    let grid = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 61).expect("a grid");
    let reference = ReferenceIntegrator::new(SECOND / 10);
    let split = ReferenceIntegrator::new(SECOND / 5);
    let qualification = split_gate(&case, &split, &reference, &lens, grid, budget([1e3, 1e3, 1e3, 1.0], 0.5))
        .expect("within the budget");
    let event_time = qualification.assessment().maxima()[3];
    assert!(!event_time.at_most(Compensated::ZERO), "a firing moved between the two runs");

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
    let between = TraceGrid::new(SimTime::from_ns(first.nanoseconds() - SECOND), every(SECOND), 2).expect("a grid");
    assert!(first <= between.end() && between.end() < second);
    let expected = trace(&case, &reference, &lens, between);
    let measured = trace(&case, &split, &lens, between);
    assert!(matches!(ReferenceError::self_convergence(&expected, &measured), Err(SplitRejected::OtherEvents)));
}

#[test]
fn a_draw_reads_each_accepted_head_once_and_in_order() {
    let case = stepped();
    let lens = lens(case.body(), case.rows()[0]);
    let store = MemoryStore::default();
    let core = reference();
    let opened = case.admit(&core, &store).expect("admitted");
    let mut draw = TraceDraw::new(case.body(), &lens, grid()).expect("a draw");
    let (first, receipt) = accepted(opened.step());
    let refused = first.record().split_trace(&mut draw, Some(&receipt));
    assert_eq!(refused, Err(SplitRejected::Gap), "a step before the draw opened");
    assert_eq!(first.record().split_trace(&mut draw, None), Err(SplitRejected::PastStart));

    let session = case.admit(&core, &store).expect("admitted");
    session.record().split_trace(&mut draw, None).expect("opens at admission");
    assert_eq!(session.record().split_trace(&mut draw, None), Err(SplitRejected::Gap), "opened twice");
    let (first, one) = accepted(session.step());
    let (second, two) = accepted(first.step());
    assert_eq!(second.record().split_trace(&mut draw, Some(&one)), Err(SplitRejected::OtherHead));
    assert_eq!(second.record().split_trace(&mut draw, Some(&two)), Err(SplitRejected::Gap), "skipped a head");

    let other = ReferenceIntegrator::new(16 * SECOND);
    let foreign = case.admit(&other, &store).expect("admitted");
    let (foreign, three) = accepted(foreign.step());
    assert_eq!(foreign.record().split_trace(&mut draw, Some(&three)), Err(SplitRejected::OtherRun));

    let (session, one) = accepted(case.admit(&core, &store).expect("admitted").step());
    session.record().split_trace(&mut draw, Some(&one)).expect("the next head");
    let mut session = session;
    while session.record().accepted().time() < SimTime::from_ns(UNTIL - SECOND) {
        let (next, receipt) = accepted(session.step());
        next.record().split_trace(&mut draw, Some(&receipt)).expect("the next head");
        session = next;
    }
    assert!(matches!(draw.finish(), Err(SplitRejected::Unfinished)));
}

#[test]
fn a_draw_needs_every_family_on_a_grid_of_two_instants_or_more() {
    let case = stepped();
    let other = ChanceCase::ectopy().expect("a synthetic case");
    let foreign = lens(other.body(), other.rows()[0]);
    assert!(matches!(TraceDraw::new(case.body(), &foreign, grid()), Err(SplitRejected::OtherBody)));
    let blind = case.body().lens(Vec::new(), every(SECOND)).expect("a lens");
    assert!(matches!(TraceDraw::new(case.body(), &blind, grid()), Err(SplitRejected::MissingFamily)));
    let one = TraceGrid::new(SimTime::from_ns(0), every(SECOND), 1);
    assert!(matches!(one, Err(AdmissionError::EmptyTraceGrid)));
    let past = TraceGrid::new(SimTime::from_ns(u64::MAX - SECOND), every(SECOND), 3);
    assert!(matches!(past, Err(AdmissionError::EmptyTraceGrid)));
    let whole = SplitBudget::new(scales(1.0), scales(1.0), scales(1.0), scales(1.0), measure(1.0));
    assert!(matches!(whole, Err(AdmissionError::ReferenceShare)));
}
