mod cores;

use core::cell::RefCell;
use cores::forced_refusal::{ForcedRefusal, Refuser};
use cores::must_fail::{
    EarlierAttempt, ForeignBody, ForeignHistory, ForeignProposal, ForeignRefusal, ForeignWorkspace,
    KeptHistory, MisplacedStage, NeverConverges, OneSample, OtherValues, PoisonedPoint,
    ProposesThroughAnotherBody, ReadsItsTicket, ShortWorkspace, Spans, StaleIncrement, StaleProposal,
    SwappedTicket, TrialOutside, UnevenWeights, UnfitWeights, UnreachedStage, WrongWeight,
};
use cores::stages::{ForwardEuler, Heun, KeepsItsSpan};
use ng_contract::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3600 * SECOND;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn scales(absolute: f64, relative: f64) -> ErrorScales {
    ErrorScales::uniform(measure(absolute), measure(relative)).expect("positive")
}

fn stepped() -> TwoReservoirs {
    TwoReservoirs::stepped_inflow().expect("a body")
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    ReferenceIntegrator::new(interval_ns)
}

fn euler(interval_ns: u64) -> ForwardEuler {
    ForwardEuler { interval_ns }
}

fn to<'m>(session: Session<'m, Running>, end: SimTime) -> Session<'m, Running> {
    match run_until(session, end) {
        Ok(session) => session,
        Err(stopped) => panic!("{:?}", stopped.fault()),
    }
}

fn accepted(step: Step<'_>) -> (Session<'_, Running>, StepReceipt) {
    match step {
        Step::Accepted(session, receipt) => (session, receipt),
        Step::Refused(session) => panic!("refused: {:?}", session.refused()),
        Step::Faulted(session) => panic!("faulted: {:?}", session.fault()),
    }
}

fn refused(step: Step<'_>) -> Session<'_, Retrying> {
    match step {
        Step::Refused(session) => session,
        Step::Accepted(..) => panic!("accepted"),
        Step::Faulted(session) => panic!("faulted: {:?}", session.fault()),
    }
}

fn fault_of(step: Step<'_>) -> EngineFault {
    match step {
        Step::Faulted(stopped) => *stopped.fault(),
        Step::Accepted(..) => panic!("accepted"),
        Step::Refused(session) => panic!("refused: {:?}", session.refused()),
    }
}

fn exhaust(mut attempt: Step<'_>) -> Session<'_, Faulted> {
    loop {
        attempt = match attempt {
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => return stopped,
            Step::Accepted(..) => panic!("accepted"),
        };
    }
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn gate(case: TwoReservoirs) -> AccuracyPolicy {
    let cases = vec![AccuracyCase::TwoReservoir(case)];
    AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy")
}

fn ledger_policy(basis: &ConservedBasis, drift: f64) -> CertificatePolicy {
    ledger_policy_with(basis, 1e-24, 1e-9, drift)
}

fn ledger_policy_with(basis: &ConservedBasis, absolute: f64, relative: f64, drift: f64) -> CertificatePolicy {
    let quantities = (basis.ids())
        .map(|quantity| {
            QuantityPolicy::new::<Water>(quantity, measure(absolute), measure(relative), measure(drift))
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("a policy of water");
    let windows = Sliding24Hours::covering(48 * HOUR).expect("a coverage");
    CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows).expect("a policy")
}

fn total_received(record: &PatientRecord<'_>, column: ColumnId) -> Compensated {
    let view = record.conservation();
    let segment = view.segments().next_back().expect("an accepted step");
    let (forward, reverse) = segment.exchange(column);
    let piece = segment.interval();
    forward.on(piece).expect("its own segment")[1].minus(reverse.on(piece).expect("its own segment")[1])
}

#[test]
fn the_stepped_inflow_matches_its_closed_form_at_first_order_over_three_refinements() {
    let error = |interval| accuracy(&reference(interval), &gate(stepped())).expect("within the policy");
    let (coarse, medium, fine) = (error(16 * SECOND), error(8 * SECOND), error(4 * SECOND));
    assert_eq!(coarse.policy(), gate(stepped()).digest());
    assert!(fine.worst()[0].at_most(medium.worst()[0]) && medium.worst()[0].at_most(coarse.worst()[0]));
    observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1)).expect("first order");
    let reversed = observed_order([&fine, &medium, &coarse], measure(1.9), measure(2.1));
    assert!(matches!(reversed, Err(AccuracyFailure::Order { .. })));
}

#[test]
fn forward_euler_past_rc_closes_every_ledger_and_fails_the_accuracy_gate() {
    let case = stepped();
    let core = euler(case.relaxation_ns() * 102 / 100);
    let mut session = case.admit(&core).expect("admitted");
    while session.record().accepted().time() < SimTime::from_ns(48 * HOUR) {
        (session, _) = accepted(session.step());
    }
    let view = session.record().conservation();
    let unbounded = ledger_policy(view.basis(), 1e12);
    certify(&view, &unbounded).expect("the ledger closes though the difference grows");

    let failure = accuracy(&core, &gate(stepped()));
    let Err(AccuracyFailure::Exceeded { at, .. }) = failure else {
        panic!("not rejected on accuracy: {failure:?}");
    };
    assert_eq!(
        at,
        SimTime::from_ns(100 * SECOND),
        "the first grid instant inside the first step"
    );
    let (first, _) = accepted(case.admit(&core).expect("admitted").step());
    for row in case.rows() {
        assert!(!first.record().accepted().value(row).at_most(Compensated::ZERO));
    }
}

#[test]
fn a_forced_refusal_changes_nothing_and_the_retry_applies_the_delivery_once() {
    for by in [Refuser::Record, Refuser::Core] {
        let case = stepped();
        let (instant, delivery) = case.delivery().expect("a delivery");
        let inner = reference(8 * SECOND);
        let core = ForcedRefusal {
            inner: &inner,
            at: instant,
            by,
        };
        let session = to(case.admit(&core).expect("admitted"), instant);
        let before = session.record().digests();
        let seal = session.record().seal();
        let session = refused(session.step());
        let wanted = match by {
            Refuser::Record => matches!(session.refused().reason(), NumericalRefusal::StageResidual { .. }),
            Refuser::Core => session.refused().reason() == NumericalRefusal::IterationLimit,
        };
        assert!(wanted, "{:?}", session.refused());
        assert_eq!(session.record().digests(), before);
        assert_eq!(session.record().seal(), seal);
        let (session, receipt) = accepted(session.retry());
        assert_eq!(receipt.applied(), [delivery]);
        let (session, next) = accepted(session.step());
        assert_eq!(next.applied(), []);
        let session = to(session, SimTime::from_ns(7_000 * SECOND));
        let received = total_received(session.record(), case.exterior().expect("an open body"));
        assert!((hi(received) - 20_000.0).abs() < 1e-7, "{received:?}");
    }
}

#[test]
fn a_core_that_books_one_sample_while_its_endpoint_came_from_other_stages_faults() {
    let case = stepped();
    let core = OneSample {
        interval_ns: 8 * SECOND,
    };
    let session = case.admit(&core).expect("admitted");
    let before = session.record().digests();
    let Step::Faulted(stopped) = session.step() else {
        panic!("not faulted");
    };
    assert_eq!(stopped.fault().kind(), EngineFaultKind::MethodMismatch);
    assert_eq!(stopped.record().digests(), before);
}

#[test]
fn an_honest_heun_core_passes_what_its_broken_twins_fail() {
    let case = stepped();
    let core = Heun {
        interval_ns: 8 * SECOND,
    };
    let session = to(
        case.admit(&core).expect("admitted"),
        SimTime::from_ns(80 * SECOND),
    );
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(80 * SECOND));
}

#[test]
fn the_certificate_reads_compensated_amounts_where_a_plain_ledger_loses_the_exchange() {
    const RATE: f64 = 1e-18;
    const STEP_SECONDS: u64 = 60;
    const STEPS: u64 = 48 * 3600 / STEP_SECONDS;
    let delivered = RATE * (STEPS * STEP_SECONDS) as f64;
    let delivery = Delivery {
        id: b"tiny",
        from: SimTime::from_ns(0),
        to: SimTime::from_ns(48 * HOUR),
        amount: measure(delivered),
    };
    let case = TwoReservoirs::delivering([measure(1.0), measure(1.0)], vec![delivery]).expect("a body");
    let core = reference(STEP_SECONDS * SECOND);
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    let certificate = certify(&view, &ledger_policy(view.basis(), 1.0)).expect("it closes");
    assert!(certificate.pieces() > 0);

    let (mut inventory, mut exchange) = (2.0_f64, 0.0_f64);
    for _ in 0..STEPS {
        inventory += RATE * STEP_SECONDS as f64;
        exchange += RATE * STEP_SECONDS as f64;
    }
    let plain_closure = ((inventory - 2.0) - exchange).abs();
    let allowed = 1e-24 + 1e-9 * exchange;
    assert!(plain_closure > allowed, "a plain f64 ledger fails this same body");

    let scaled = ledger_policy_with(view.basis(), 1e-40, 1e-9, 1.0);
    certify(&view, &scaled).expect("the residual is small against this window's own exchange");
    let unscaled = ledger_policy_with(view.basis(), 1e-40, 0.0, 1.0);
    let failure = certify(&view, &unscaled).expect_err("the same residual is above the absolute floor alone");
    assert_eq!(failure.check(), CertificateCheck::Closure);
}

#[test]
fn a_misplaced_stage_and_a_wrong_weight_fault_with_method_mismatch() {
    let case = stepped();
    let core = MisplacedStage {
        interval_ns: 8 * SECOND,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch);
    let core = WrongWeight {
        interval_ns: 8 * SECOND,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch);
}

#[test]
fn a_stage_evaluated_on_other_values_faults_with_method_mismatch() {
    let case = stepped();
    let values = case.other_values().expect("bound");
    let inner = reference(8 * SECOND);
    let identity = RunIdentity::new(
        case.body().model_hash(),
        values.packet_hash(),
        Seed::new([0, 0, 0, 0]),
        inner.resolution(),
    );
    let core = OtherValues {
        body: case.body(),
        values: &values,
        identity: &identity,
        inner,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch);
}

#[test]
fn a_history_kept_across_a_breakpoint_faults_with_method_mismatch() {
    let case = stepped();
    let (start, _) = case.delivery().expect("a delivery");
    let end = SimTime::from_ns(6_000 * SECOND);
    for (keeps_after, breakpoint) in [(SimTime::from_ns(0), start), (start, end)] {
        let core = KeptHistory {
            inner: euler(8 * SECOND),
            keeps_after,
        };
        let session = to(case.admit(&core).expect("admitted"), breakpoint);
        let fault = fault_of(session.step());
        assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch);
        assert_eq!(
            fault.at(),
            breakpoint,
            "a delivery's start and its end each restart a core"
        );
    }
}

#[test]
fn a_continuation_of_another_core_faults_with_binding_mismatch() {
    let case = stepped();
    let core = ForeignHistory {
        inner: euler(8 * SECOND),
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_stale_proposal_is_a_binding_mismatch_not_a_refusal() {
    let case = stepped();
    let core = StaleProposal {
        inner: euler(8 * SECOND),
        kept: RefCell::new(None),
    };
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn an_increment_finished_for_an_earlier_ticket_is_a_binding_mismatch() {
    let case = stepped();
    let core = StaleIncrement {
        inner: euler(SECOND),
        kept: RefCell::new(None),
    };
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn an_evaluation_from_an_earlier_attempt_is_a_binding_mismatch() {
    let case = stepped();
    let core = EarlierAttempt {
        inner: euler(8 * SECOND),
        kept: RefCell::new(None),
    };
    let session = refused(case.admit(&core).expect("admitted").step());
    assert_eq!(fault_of(session.retry()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_kept_ticket_swapped_for_the_issued_one_is_a_binding_mismatch() {
    let case = stepped();
    let core = SwappedTicket {
        inner: euler(8 * SECOND),
        kept: RefCell::new(None),
    };
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_refusal_of_another_ticket_is_a_binding_mismatch_so_no_core_resets_its_budget() {
    let case = stepped();
    let core = ForeignRefusal {
        inner: euler(8 * SECOND),
        kept: RefCell::new(None),
    };
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_refusal_of_a_foreign_base_cannot_be_retried() {
    let case = stepped();
    let core = euler(8 * SECOND);
    let foreign = NeverConverges {
        interval_ns: 8 * SECOND,
        budget: 12,
    };
    let session = refused(case.admit(&foreign).expect("admitted").step());
    let (moved, _) = accepted(case.admit(&core).expect("admitted").step());
    let fault = moved.record().attempt(&core, Attempt::After(session.refused()));
    let Err(TrialFailure::EngineFault(fault)) = fault.map(|_| ()) else {
        panic!("not a fault");
    };
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn weights_that_do_not_sum_to_one_are_a_binding_mismatch_at_finish() {
    let case = stepped();
    let core = UnevenWeights {
        interval_ns: 8 * SECOND,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn weights_whose_sum_does_not_fit_in_lowest_terms_are_a_binding_mismatch() {
    let case = stepped();
    let core = UnfitWeights {
        interval_ns: 8 * SECOND,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn another_bodys_stage_workspace_or_proposal_is_a_binding_mismatch() {
    let (case, other) = (stepped(), TwoReservoirs::closed().expect("a body"));
    let inner = euler(8 * SECOND);
    let host = other.admit(&inner).expect("admitted");
    let values = other.other_values().expect("bound");
    let (body, identity) = (other.body(), host.record().identity());
    let accepted = host.record().accepted();
    let core = ForeignBody {
        inner: euler(8 * SECOND),
        body,
        values: &values,
        identity,
        accepted,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
    {
        // One row where the record's body has two, so a read past its end would panic, not refuse.
        let lone = filter_only_body();
        let host = admit_fixture(&lone, filter_only_packet(), no_events(), &inner).expect("admitted");
        let values = lone.bind(filter_only_packet()).expect("bound");
        let core = ForeignWorkspace {
            inner: euler(8 * SECOND),
            body: &lone,
            values: &values,
            identity: host.record().identity(),
            accepted: host.record().accepted(),
        };
        let fault = fault_of(case.admit(&core).expect("admitted").step());
        assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
    }
    let core = ForeignProposal {
        inner: euler(8 * SECOND),
        body,
        values: &values,
        identity,
        accepted,
    };
    let fault = fault_of(case.admit(&core).expect("admitted").step());
    assert_eq!(fault.kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_zero_span_and_a_span_past_the_end_of_time_are_binding_mismatches() {
    let case = stepped();
    let zero = Spans {
        inner: euler(8 * SECOND),
        then_ns: 0,
    };
    let (session, _) = accepted(case.admit(&zero).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
    let past = Spans {
        inner: euler(8 * SECOND),
        then_ns: u64::MAX,
    };
    let (session, _) = accepted(case.admit(&past).expect("admitted").step());
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_ticket_refuses_an_offset_past_its_end_and_a_zero_shift() {
    let case = stepped();
    let core = ReadsItsTicket {
        inner: euler(8 * SECOND),
        reads: RefCell::new(Vec::new()),
    };
    accepted(case.admit(&core).expect("admitted").step());
    let reads = core.reads.borrow();
    let mismatch = Some(EngineFaultKind::BindingMismatch);
    assert_eq!(*reads, [(None, mismatch, mismatch)]);
}

#[test]
fn an_estimate_above_one_is_refused_and_the_retry_is_accepted() {
    let case = stepped();
    let core = reference(8 * SECOND).with_scales(scales(0.01, 0.0));
    let session = refused(case.admit(&core).expect("admitted").step());
    assert!(matches!(
        session.refused().reason(),
        NumericalRefusal::Accuracy { .. }
    ));
    assert_eq!(session.refused().retry_ns(), 4 * SECOND);
    let (session, _) = accepted(session.retry());
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(4 * SECOND));
}

#[test]
fn an_estimate_above_one_at_the_floor_faults_and_never_publishes() {
    let case = stepped();
    let core = reference(8 * SECOND)
        .with_scales(scales(1e-3, 0.0))
        .with_floor(4 * SECOND);
    let session = case.admit(&core).expect("admitted");
    let before = session.record().digests();
    let stopped = exhaust(session.step());
    let fault = stopped.fault();
    assert_eq!(stopped.refusals().len(), 1);
    assert_eq!(fault.kind(), EngineFaultKind::NoAdmissibleStep);
    assert!(matches!(
        fault.last_refusal(),
        Some(NumericalRefusal::Accuracy { .. })
    ));
    assert_eq!(fault.at(), SimTime::from_ns(0));
    assert_eq!(fault.attempted_ns(), 4 * SECOND);
    assert_eq!(stopped.record().digests(), before);
}

#[test]
fn an_exhausted_retry_budget_faults_with_no_admissible_step() {
    let case = stepped();
    let core = NeverConverges {
        interval_ns: 8 * SECOND,
        budget: 3,
    };
    let stopped = exhaust(case.admit(&core).expect("admitted").step());
    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    assert_eq!(
        stopped.fault().last_refusal(),
        Some(NumericalRefusal::IterationLimit)
    );
    assert_eq!(stopped.refusals().len(), 3);
}

#[test]
fn a_fault_keeps_the_refusals_before_it() {
    let case = stepped();
    let core = NeverConverges {
        interval_ns: 8 * SECOND,
        budget: 2,
    };
    let stopped = exhaust(case.admit(&core).expect("admitted").step());
    assert_eq!(stopped.refusals().len(), 2);
    assert_eq!(stopped.refusals()[0].attempted_ns(), 8 * SECOND);
    assert_eq!(stopped.refusals()[1].attempted_ns(), 4 * SECOND);
    assert_eq!(stopped.refusals()[1].at(), SimTime::from_ns(0));
}

#[test]
fn a_stage_that_does_not_reach_the_value_its_tangent_says_is_refused() {
    let case = stepped();
    let core = UnreachedStage {
        inner: reference(8 * SECOND),
    };
    let session = refused(case.admit(&core).expect("admitted").step());
    assert!(matches!(
        session.refused().reason(),
        NumericalRefusal::StageResidual { .. }
    ));
}

#[test]
fn a_negative_amount_is_refused_on_the_row_it_left() {
    // A step of 20 RC multiplies the difference by -39: from 1,000 to -39,000, then to 1,521,000, which
    // is more than the sum of 1,000,000, so `b` would go negative on the second step.
    let case =
        TwoReservoirs::delivering([measure(500_500.0), measure(499_500.0)], Vec::new()).expect("a body");
    let core = euler(20 * case.relaxation_ns());
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    let session = refused(session.step());
    let NumericalRefusal::TrialDomain { source } = session.refused().reason() else {
        panic!("not a trial domain");
    };
    let sources = case.body().refusal_sources();
    assert_eq!(
        sources[source.index()],
        RefusalSource::Row { row: case.rows()[1] }
    );
}

#[test]
fn a_non_finite_coordinate_is_refused_on_its_row() {
    let case = stepped();
    let core = PoisonedPoint {
        inner: euler(8 * SECOND),
    };
    let session = refused(case.admit(&core).expect("admitted").step());
    let NumericalRefusal::TrialDomain { source } = session.refused().reason() else {
        panic!("not a trial domain");
    };
    let sources = case.body().refusal_sources();
    assert_eq!(
        sources[source.index()],
        RefusalSource::Row { row: case.rows()[1] }
    );
}

#[derive(Clone, Copy, PartialEq)]
enum EdgeLaw {
    Divides,
    TakesTheLog,
    TakesTheLogNearZero,
    Exponentiates,
}

/// `q = g h f(a, b)`, where `f` reads the downstream or the upstream amount through one operator.
/// A start inside the law's domain, so admission passes it.
fn edge_law_body(law: EdgeLaw) -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let start = match law {
        EdgeLaw::TakesTheLogNearZero => [1.0, 1e-320],
        EdgeLaw::Divides | EdgeLaw::TakesTheLog | EdgeLaw::Exponentiates => [1.0, 1.0],
    };
    edge_law_body_from(law, start)
}

fn refused_amounts(law: EdgeLaw) -> [f64; 2] {
    match law {
        EdgeLaw::Divides | EdgeLaw::TakesTheLog => [1.0, 0.0],
        EdgeLaw::TakesTheLogNearZero => [1.0, 1e-320],
        EdgeLaw::Exponentiates => [1e6, 1.0],
    }
}

fn edge_law_body_from(law: EdgeLaw, start: [f64; 2]) -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema.partial_volume(measure(1.0)).expect("a coefficient");
    let (conductance, head, reach) = (
        ParameterKey::new("cv.g"),
        ParameterKey::new("cv.head"),
        ParameterKey::new("cv.reach"),
    );
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let g = laws.edge_parameter::<Conductance<Water>>(conductance, Admissible::nonnegative());
        let h = laws.edge_parameter::<Pressure>(head, Admissible::finite());
        let c = laws.edge_parameter::<Volume>(reach, Admissible::positive());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let reach = k.edge_parameter(&c);
            let factor = match law {
                EdgeLaw::Divides => {
                    let ratio = k.divide(upstream, downstream);
                    k.as_dimension::<_, Dimensionless>(ratio)
                }
                EdgeLaw::TakesTheLog | EdgeLaw::TakesTheLogNearZero => {
                    let ratio = k.divide(downstream, reach);
                    let ratio = k.as_dimension::<_, Dimensionless>(ratio);
                    k.ln(ratio)
                }
                EdgeLaw::Exponentiates => {
                    let ratio = k.divide(upstream, reach);
                    let ratio = k.as_dimension::<_, Dimensionless>(ratio);
                    k.exp(ratio)
                }
            };
            let head = k.edge_parameter(&h);
            let head = k.scale(head, factor);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let default = Binding::Network(Network::Vascular);
    let lumen = |site| RowKind::Amount {
        site,
        space: Space::Lumen,
    };
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            conductance,
            default,
            measure::<Conductance<Water>>(1.0),
            "synthetic",
        )
        .expect("once");
    packet
        .synthetic(head, default, measure::<Pressure>(1.0), "synthetic")
        .expect("once");
    if law != EdgeLaw::Divides {
        packet
            .synthetic(reach, default, measure::<Volume>(1.0), "synthetic")
            .expect("once");
    }
    packet
        .start(lumen(a.id()), measure::<Water>(start[0]), "synthetic")
        .expect("once");
    packet
        .start(lumen(b.id()), measure::<Water>(start[1]), "synthetic")
        .expect("once");
    (body, packet)
}

fn refused_source_at_the_floor(law: EdgeLaw) -> RefusalSource {
    let (body, packet) = edge_law_body(law);
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let inner = reference(4 * MILLISECOND);
    let core = TrialOutside {
        inner: &inner,
        amounts: refused_amounts(law),
    };
    let session = admit_fixture(&body, packet, boundary, &core).expect("admitted");
    let stopped = exhaust(session.step());
    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    assert_eq!(
        stopped.refusals().len(),
        2,
        "4 ms and 2 ms refuse; the refusal at 1 ms is the floor fault"
    );
    let Some(NumericalRefusal::TrialDomain { source }) = stopped.fault().last_refusal() else {
        panic!("not a trial domain");
    };
    body.refusal_sources()[source.index()]
}

#[test]
fn a_trial_outside_a_laws_domain_refuses_until_a_fault_names_its_operator() {
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    assert_eq!(
        refused_source_at_the_floor(EdgeLaw::Divides),
        RefusalSource::Divisor { law }
    );
    assert_eq!(
        refused_source_at_the_floor(EdgeLaw::TakesTheLog),
        RefusalSource::LogarithmArgument { law }
    );
    assert_eq!(
        refused_source_at_the_floor(EdgeLaw::Exponentiates),
        RefusalSource::NonFinite { law }
    );
}

#[test]
fn a_finite_value_with_a_non_finite_slope_is_refused_where_it_linearizes() {
    // `ln` of a subnormal is finite and its slope is not, so `evaluate` passes and `linearize` refuses.
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    assert_eq!(
        refused_source_at_the_floor(EdgeLaw::TakesTheLogNearZero),
        RefusalSource::NonFinite { law }
    );
}

#[test]
fn a_non_finite_value_is_refused_at_its_law_not_at_the_row_it_reaches() {
    // Forward Euler never linearizes, so only the operator's own check can name the law.
    let (body, packet) = edge_law_body(EdgeLaw::Exponentiates);
    let inner = euler(4 * MILLISECOND);
    let core = TrialOutside {
        inner: &inner,
        amounts: refused_amounts(EdgeLaw::Exponentiates),
    };
    let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
    let session = refused(session.step());
    let NumericalRefusal::TrialDomain { source } = session.refused().reason() else {
        panic!("not a trial domain");
    };
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    assert_eq!(
        body.refusal_sources()[source.index()],
        RefusalSource::NonFinite { law }
    );
}

#[test]
fn the_refusal_sources_list_each_rows_set_and_each_laws_operators() {
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    let (body, _) = edge_law_body(EdgeLaw::Divides);
    let rows: Vec<RefusalSource> = (body.rows().iter())
        .map(|row| RefusalSource::Row { row: row.id() })
        .collect();
    let wanted = [
        rows[0],
        rows[1],
        RefusalSource::Divisor { law },
        RefusalSource::NonFinite { law },
    ];
    assert_eq!(body.refusal_sources(), wanted);
    let (body, _) = edge_law_body(EdgeLaw::TakesTheLog);
    assert!(
        body.refusal_sources()
            .contains(&RefusalSource::LogarithmArgument { law })
    );
}

#[test]
fn a_published_state_is_evaluated_once_so_a_law_undefined_there_is_refused() {
    // The stage at the start is fine; one second of a forward Euler flow of one empties `a` exactly, so
    // only the published state divides by zero, and the retry's half step stops short of it.
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema.partial_volume(measure(1.0)).expect("a coefficient");
    let (conductance, head) = (ParameterKey::new("cv.g"), ParameterKey::new("cv.head"));
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let g = laws.edge_parameter::<Conductance<Water>>(conductance, Admissible::nonnegative());
        let h = laws.edge_parameter::<Pressure>(head, Admissible::finite());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let ratio = k.divide(downstream, upstream);
            let factor = k.as_dimension::<_, Dimensionless>(ratio);
            let head = k.edge_parameter(&h);
            let head = k.scale(head, factor);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let default = Binding::Network(Network::Vascular);
    let lumen = |site| RowKind::Amount {
        site,
        space: Space::Lumen,
    };
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            conductance,
            default,
            measure::<Conductance<Water>>(1.0),
            "synthetic",
        )
        .expect("once");
    packet
        .synthetic(head, default, measure::<Pressure>(1.0), "synthetic")
        .expect("once");
    packet
        .start(lumen(a.id()), measure::<Water>(1.0), "synthetic")
        .expect("once");
    packet
        .start(lumen(b.id()), measure::<Water>(1.0), "synthetic")
        .expect("once");
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let core = euler(SECOND);
    let session = admit_fixture(&body, packet, boundary, &core).expect("admitted");
    let session = refused(session.step());
    let NumericalRefusal::TrialDomain { source } = session.refused().reason() else {
        panic!("not a trial domain");
    };
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    assert_eq!(
        body.refusal_sources()[source.index()],
        RefusalSource::Divisor { law }
    );
    let (session, _) = accepted(session.retry());
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(SECOND / 2));
}

#[test]
fn a_body_with_a_mode_is_refused_by_name_at_binding() {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let _modes = laws.modes("valve", ["open", "shut"]);
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let refused = body.bind(ParameterPacket::<SyntheticFixture>::new()).err();
    let variable = LawName {
        owner: "CV",
        local: "valve",
    };
    assert_eq!(refused, Some(AdmissionError::ModesNotYetRun { variable }));
}

fn reservoir_packet(body: &CompiledBody, e: f64, g: f64) -> ParameterPacket<SyntheticFixture> {
    let default = Binding::Network(Network::Vascular);
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            ParameterKey::new("cv.e"),
            default,
            measure::<Elastance>(e),
            "synthetic",
        )
        .expect("once");
    packet
        .synthetic(
            ParameterKey::new("cv.g"),
            default,
            measure::<Conductance<Water>>(g),
            "synthetic",
        )
        .expect("once");
    for row in body.rows() {
        packet
            .start(row.kind(), measure::<Water>(5.0), "synthetic")
            .expect("once");
    }
    packet
}

#[test]
fn a_packet_binds_its_values_and_its_hash_ignores_declaration_order() {
    let case = stepped();
    let body = case.body();
    let bound = body.bind(reservoir_packet(body, 1.0, 0.5)).expect("bound");
    let default = Binding::Network(Network::Vascular);
    let mut reversed = ParameterPacket::new();
    let rows: Vec<RowKind> = body.rows().iter().map(Row::kind).rev().collect();
    for row in rows {
        reversed
            .start(row, measure::<Water>(5.0), "synthetic")
            .expect("once");
    }
    let g = measure::<Conductance<Water>>(0.5);
    reversed
        .synthetic(ParameterKey::new("cv.g"), default, g, "synthetic")
        .expect("once");
    reversed
        .synthetic(
            ParameterKey::new("cv.e"),
            default,
            measure::<Elastance>(1.0),
            "synthetic",
        )
        .expect("once");
    assert_eq!(
        body.bind(reversed).expect("bound").packet_hash(),
        bound.packet_hash()
    );
    let other = body.bind(reservoir_packet(body, 1.0, 0.25)).expect("bound");
    assert_ne!(other.packet_hash(), bound.packet_hash());
}

#[test]
fn a_site_binding_wins_over_the_network_default() {
    let case = stepped();
    let body = case.body();
    let site = (body.parameters().iter())
        .find(|slot| slot.key() == ParameterKey::new("cv.e"))
        .map(|slot| slot.binding())
        .expect("a slot");
    let mut packet = reservoir_packet(body, 1.0, 0.5);
    packet
        .synthetic(
            ParameterKey::new("cv.e"),
            site,
            measure::<Elastance>(2.0),
            "synthetic",
        )
        .expect("once");
    let overridden = body.bind(packet).expect("bound").packet_hash();
    let plain = body
        .bind(reservoir_packet(body, 1.0, 0.5))
        .expect("bound")
        .packet_hash();
    assert_ne!(overridden, plain);
}

#[test]
fn each_bad_packet_is_refused_by_name() {
    let case = stepped();
    let body = case.body();
    let (e, g) = (ParameterKey::new("cv.e"), ParameterKey::new("cv.g"));
    let default = Binding::Network(Network::Vascular);
    let refusal = |packet: ParameterPacket<SyntheticFixture>| body.bind(packet).err().expect("refused");

    let mut unbound = ParameterPacket::new();
    unbound
        .synthetic(e, default, measure::<Elastance>(1.0), "synthetic")
        .expect("once");
    assert!(matches!(refusal(unbound), AdmissionError::UnboundParameter { key, .. } if key == g));

    let mut unknown = reservoir_packet(body, 1.0, 0.5);
    let stray = ParameterKey::new("cv.stray");
    unknown
        .synthetic(stray, default, measure::<Elastance>(1.0), "synthetic")
        .expect("once");
    assert_eq!(
        refusal(unknown),
        AdmissionError::UnknownParameter {
            key: stray,
            binding: default
        }
    );

    let mut twice = reservoir_packet(body, 1.0, 0.5);
    assert_eq!(
        twice.synthetic(e, default, measure::<Elastance>(2.0), "synthetic"),
        Err(AdmissionError::DuplicateParameter {
            key: e,
            binding: default
        })
    );

    let outside = reservoir_packet(body, -1.0, 0.5);
    assert!(matches!(refusal(outside), AdmissionError::ParameterOutsideSet { key, .. } if key == e));

    let mut wrong = ParameterPacket::new();
    wrong
        .synthetic(e, default, measure::<Pressure>(1.0), "synthetic")
        .expect("once");
    wrong
        .synthetic(g, default, measure::<Conductance<Water>>(1.0), "synthetic")
        .expect("once");
    assert!(matches!(refusal(wrong), AdmissionError::ParameterDimension { key, .. } if key == e));

    let rows: Vec<RowKind> = body.rows().iter().map(Row::kind).collect();
    let mut missing = ParameterPacket::new();
    missing
        .synthetic(e, default, measure::<Elastance>(1.0), "synthetic")
        .expect("once");
    missing
        .synthetic(g, default, measure::<Conductance<Water>>(1.0), "synthetic")
        .expect("once");
    assert_eq!(refusal(missing), AdmissionError::MissingStart { row: rows[0] });

    let mut no_such_row = reservoir_packet(body, 1.0, 0.5);
    let memory = RowKind::Memory(LawName {
        owner: "NS",
        local: "nothing",
    });
    no_such_row
        .start(memory, measure::<Dimensionless>(0.0), "synthetic")
        .expect("once");
    assert_eq!(refusal(no_such_row), AdmissionError::UnknownStart { row: memory });

    let mut again = reservoir_packet(body, 1.0, 0.5);
    assert_eq!(
        again.start(rows[0], measure::<Water>(1.0), "synthetic"),
        Err(AdmissionError::DuplicateStart { row: rows[0] })
    );

    let mut pressure = ParameterPacket::new();
    pressure
        .synthetic(e, default, measure::<Elastance>(1.0), "synthetic")
        .expect("once");
    pressure
        .synthetic(g, default, measure::<Conductance<Water>>(1.0), "synthetic")
        .expect("once");
    pressure
        .start(rows[0], measure::<Pressure>(1.0), "synthetic")
        .expect("once");
    pressure
        .start(rows[1], measure::<Water>(1.0), "synthetic")
        .expect("once");
    assert_eq!(refusal(pressure), AdmissionError::StartDimension { row: rows[0] });

    let mut negative = ParameterPacket::new();
    negative
        .synthetic(e, default, measure::<Elastance>(1.0), "synthetic")
        .expect("once");
    negative
        .synthetic(g, default, measure::<Conductance<Water>>(1.0), "synthetic")
        .expect("once");
    negative
        .start(rows[0], measure::<Water>(-1.0), "synthetic")
        .expect("once");
    negative
        .start(rows[1], measure::<Water>(1.0), "synthetic")
        .expect("once");
    assert_eq!(
        refusal(negative),
        AdmissionError::StartOutsideSet { row: rows[0] }
    );
}

fn water_event(name: &[u8], boundary: &'static str, from: u64, to: u64, amount: f64) -> BoundaryEvent {
    let id = InputId::new(Digest::of_canonical(name));
    let delivery = WaterDelivery {
        boundary,
        amount: measure(amount),
        ends: SimTime::from_ns(to),
    };
    BoundaryEvent::new(id, SimTime::from_ns(from), BoundaryInput::Water(delivery))
}

#[test]
fn each_bad_timeline_is_refused_by_name() {
    let id = InputId::new(Digest::of_canonical(b"x"));
    let refusal = |events| BoundaryPort::new(events).err();
    assert_eq!(
        refusal(vec![water_event(b"x", "exterior", 5, 5, 1.0)]),
        Some(AdmissionError::EmptyDelivery { id })
    );
    assert_eq!(
        refusal(vec![water_event(b"x", "exterior", 5, 4, 1.0)]),
        Some(AdmissionError::EmptyDelivery { id })
    );
    assert_eq!(
        refusal(vec![water_event(b"x", "exterior", 5, 6, -1.0)]),
        Some(AdmissionError::NegativeDelivery { id })
    );
    assert_eq!(
        refusal(vec![
            water_event(b"x", "exterior", 5, 6, 1.0),
            water_event(b"x", "exterior", 5, 6, 2.0),
        ]),
        Some(AdmissionError::InputConflict { id })
    );

    let case = stepped();
    let body = case.body();
    let bound = || body.bind(reservoir_packet(body, 1.0, 0.5)).expect("bound");
    let at = |ns| InitializationRequest::at(SimTime::from_ns(ns));
    let unknown = BoundaryPort::new(vec![water_event(b"x", "outflow", 5, 6, 1.0)]).expect("parsed");
    assert_eq!(
        body.initialization(bound(), unknown, at(0)).err(),
        Some(AdmissionError::UnknownBoundary { name: "outflow" })
    );
    let early = BoundaryPort::new(vec![water_event(b"x", "exterior", 5, 6, 1.0)]).expect("parsed");
    assert_eq!(
        body.initialization(bound(), early, at(6)).err(),
        Some(AdmissionError::EventBeforeHead { id })
    );
}

#[test]
fn one_id_with_one_payload_collapses_and_applies_once() {
    let open = TwoReservoirs::delivering([measure(10.0), measure(10.0)], Vec::new()).expect("a body");
    let body = open.body();
    let port = BoundaryPort::new(vec![
        water_event(b"x", "exterior", SECOND, 2 * SECOND, 3.0),
        water_event(b"x", "exterior", SECOND, 2 * SECOND, 3.0),
    ])
    .expect("collapsed");
    let core = reference(SECOND);
    let packet = reservoir_packet(body, 1.0, 0.001);
    let session = admit_fixture(body, packet, port, &core).expect("admitted");
    let session = to(session, SimTime::from_ns(3 * SECOND));
    let received = total_received(session.record(), open.exterior().expect("an open body"));
    assert!((hi(received) - 3.0).abs() < 1e-12, "{received:?}");
}

#[test]
fn admission_refuses_a_workspace_that_moved_a_start_or_came_from_another_body() {
    let (case, other) = (stepped(), TwoReservoirs::closed().expect("a body"));
    let body = case.body();
    let at = SimTime::from_ns(0);
    let history = || MethodHistory::new(Digest::of_canonical(b"a schema"), at, Vec::new());
    let system = || {
        let parameters = body.bind(reservoir_packet(body, 1.0, 0.5)).expect("bound");
        let request = InitializationRequest::at(at);
        let port = BoundaryPort::new(Vec::new()).expect("no event");
        body.initialization(parameters, port, request).expect("a system")
    };
    let moved = {
        let system = system();
        let mut solved = system.workspace();
        solved.unknowns_mut()[1] += 1.0;
        system.admit(solved, history()).err()
    };
    let row = body.rows()[1].id();
    let row = AdmissionRow::Body(row);
    assert_eq!(moved, Some(InitializationFailure::Inconsistent { row }));

    let parameters = other
        .body()
        .bind(reservoir_packet(other.body(), 1.0, 0.5))
        .expect("bound");
    let request = InitializationRequest::at(at);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let foreign = other
        .body()
        .initialization(parameters, port, request)
        .expect("a system");
    let foreign = system().admit(foreign.workspace(), history()).err();
    assert_eq!(
        foreign,
        Some(InitializationFailure::Admission(AdmissionError::ForeignWorkspace))
    );

    let parameters = other
        .body()
        .bind(reservoir_packet(other.body(), 1.0, 0.5))
        .expect("bound");
    let request = InitializationRequest::at(at);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    assert_eq!(
        body.initialization(parameters, port, request).err(),
        Some(AdmissionError::ForeignParameters)
    );
}

#[test]
fn a_session_refuses_an_identity_of_another_body_origin_or_solver_and_a_zero_floor() {
    let (case, other) = (stepped(), TwoReservoirs::closed().expect("a body"));
    let body = case.body();
    let core = reference(8 * SECOND);
    let revision = |target: &TwoReservoirs| {
        let body = target.body();
        let parameters = body.bind(reservoir_packet(body, 1.0, 0.5)).expect("bound");
        let at = SimTime::from_ns(0);
        let request = InitializationRequest::at(at);
        let port = BoundaryPort::new(Vec::new()).expect("no event");
        let system = body.initialization(parameters, port, request).expect("a system");
        let solved = system.workspace();
        let history = MethodHistory::new(core.resolution().policy(), at, Vec::new());
        system.admit(solved, history).expect("admitted")
    };
    let seed = Seed::new([1, 2, 3, 4]);
    let admit =
        |origin: InitialRevision, identity: RunIdentity| Session::admit(body, &core, origin, identity).err();

    let origin = revision(&case);
    let packet = origin.packet_hash();
    let wrong_model = RunIdentity::new(other.body().model_hash(), packet, seed, core.resolution());
    assert_eq!(admit(origin, wrong_model), Some(AdmissionError::IdentityMismatch));

    let origin = revision(&other);
    let packet = origin.packet_hash();
    let right_identity = RunIdentity::new(body.model_hash(), packet, seed, core.resolution());
    assert_eq!(
        admit(origin, right_identity),
        Some(AdmissionError::IdentityMismatch)
    );

    let foreign_packet = {
        let parameters = body.bind(reservoir_packet(body, 1.0, 0.25)).expect("bound");
        let origin = SimTime::from_ns(0);
        let request = InitializationRequest::at(origin);
        let port = BoundaryPort::new(Vec::new()).expect("no event");
        let system = body.initialization(parameters, port, request).expect("a system");
        let solved = system.workspace();
        let history = MethodHistory::new(core.resolution().policy(), origin, Vec::new());
        system.admit(solved, history).expect("admitted").packet_hash()
    };
    let origin = revision(&case);
    let own_packet = origin.packet_hash();
    assert_ne!(foreign_packet, own_packet);
    let wrong_packet = RunIdentity::new(body.model_hash(), foreign_packet, seed, core.resolution());
    assert_eq!(
        admit(origin, wrong_packet),
        Some(AdmissionError::IdentityMismatch)
    );

    let origin = revision(&case);
    let packet = origin.packet_hash();
    let foreign_core = reference(4 * SECOND);
    let wrong_core = RunIdentity::new(body.model_hash(), packet, seed, foreign_core.resolution());
    assert_eq!(admit(origin, wrong_core), Some(AdmissionError::IdentityMismatch));

    let bottomless = reference(8 * SECOND).with_floor(0);
    let origin = revision(&case);
    let packet = origin.packet_hash();
    let identity = RunIdentity::new(body.model_hash(), packet, seed, bottomless.resolution());
    let refused = Session::admit(body, &bottomless, origin, identity).err();
    assert_eq!(refused, Some(AdmissionError::ZeroMinimumInterval));
}

#[test]
fn two_settings_of_one_core_never_share_an_identity() {
    let digest = |interval| reference(interval).resolution().digest();
    assert_ne!(digest(2 * SECOND), digest(8 * SECOND));
    let tight = reference(8 * SECOND)
        .with_scales(scales(1e-3, 0.0))
        .resolution()
        .digest();
    assert_ne!(tight, digest(8 * SECOND));
    let floored = reference(8 * SECOND).with_floor(SECOND).resolution().digest();
    assert_ne!(floored, digest(8 * SECOND));
    let identity = |seed: Seed| {
        let case = stepped();
        let packet = case
            .body()
            .bind(reservoir_packet(case.body(), 1.0, 0.5))
            .expect("bound");
        let core = reference(8 * SECOND);
        RunIdentity::new(
            case.body().model_hash(),
            packet.packet_hash(),
            seed,
            core.resolution(),
        )
        .digest()
    };
    assert_ne!(
        identity(Seed::new([1, 2, 3, 4])),
        identity(Seed::new([1, 2, 3, 5]))
    );
}

#[test]
fn a_step_is_the_cores_own_interval_cut_short_only_at_the_next_breakpoint() {
    // 2,000 s is no multiple of 7 s, so the step that would cross the delivery's start stops at it.
    let case = stepped();
    let (instant, _) = case.delivery().expect("a delivery");
    let core = reference(7 * SECOND);
    let session = to(case.admit(&core).expect("admitted"), instant);
    assert_eq!(session.record().accepted().time(), instant);
    let last = session
        .record()
        .accepted()
        .last_segment()
        .interval()
        .expect("a step");
    assert_eq!(last.length_ns(), 2_000 * SECOND % (7 * SECOND));
    let (session, _) = accepted(session.step());
    let next = session
        .record()
        .accepted()
        .last_segment()
        .interval()
        .expect("a step");
    assert_eq!(next.length_ns(), 7 * SECOND, "the core's own interval again");
    let end = SimTime::from_ns(6_000 * SECOND);
    let session = to(session, end);
    assert_eq!(
        session.record().accepted().time(),
        end,
        "and at the delivery's end"
    );
}

#[test]
fn the_ending_interval_sees_the_left_value_and_the_next_the_new_one() {
    let breakpoint = 7 * SECOND + 1;
    let end = breakpoint + 3 * SECOND;
    let first = 7.000000001;
    let second = 30.0;
    let last = 1e-9;
    let delivery = |id, from, to, amount: f64| Delivery {
        id,
        from: SimTime::from_ns(from),
        to: SimTime::from_ns(to),
        amount: measure(amount),
    };
    let case = TwoReservoirs::delivering(
        [measure(100.0), measure(100.0)],
        vec![
            delivery(b"slow", 0, breakpoint, first),
            delivery(b"fast", breakpoint, end, second),
            delivery(b"one nanosecond", end, end + 1, last),
        ],
    )
    .expect("a body");
    let inner = reference(2 * SECOND);
    let core = ForcedRefusal {
        inner: &inner,
        at: SimTime::from_ns(6 * SECOND),
        by: Refuser::Core,
    };
    let inflow = case.exterior().expect("an open body");
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(breakpoint));
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(breakpoint));
    let received = hi(total_received(session.record(), inflow));
    assert!((received - first).abs() < 1e-12, "{received}");
    let session = to(session, SimTime::from_ns(end));
    let received = hi(total_received(session.record(), inflow));
    assert!((received - (first + second)).abs() < 1e-12, "{received}");
    let (session, receipt) = accepted(session.step());
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(end + 1));
    assert_eq!(receipt.applied().len(), 1);
    let received = hi(total_received(session.record(), inflow));
    assert!((received - (first + second + last)).abs() < 1e-12, "{received}");
}

fn no_events() -> BoundaryPort {
    BoundaryPort::new(Vec::new()).expect("no event")
}

fn filter_only_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let filter = laws.state::<Dimensionless>("filter", Admissible::finite());
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            k.finish(zero, laws.every_mode(), reflex)
        });
        laws.memory_rate(&filter, rate);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn filter_only_packet() -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    let row = RowKind::Memory(LawName {
        owner: "NS",
        local: "filter",
    });
    packet
        .start(row, measure::<Dimensionless>(1.0), "synthetic")
        .expect("once");
    packet
}

/// A filter `m' = s (t - m)` and a reading closed algebraically at `x = g m`.
fn filter_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let filter = laws.state::<Dimensionless>("filter", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
        let target = laws.parameter::<Dimensionless>(ParameterKey::new("ns.target"), Admissible::finite());
        let gain = laws.parameter::<Dimensionless>(ParameterKey::new("ns.gain"), Admissible::finite());
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let rate = laws.kernel(|mut k| {
            let goal = k.parameter(&target);
            let now = k.state(&filter);
            let speed = k.parameter(&speed);
            let rate = k.relax(goal, now, speed);
            k.finish(rate, laws.every_mode(), reflex)
        });
        laws.memory_rate(&filter, rate);
        let closure = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let g = k.parameter(&gain);
            let now = k.state(&filter);
            let scaled = k.scale(g, now);
            let residual = k.subtract(x, scaled);
            k.finish(residual, laws.every_mode(), reflex)
        });
        laws.residual(&reading, closure);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn filter_packet() -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    let bind = Binding::Body;
    let speed = measure::<Rate<Dimensionless>>(0.01);
    packet
        .synthetic(ParameterKey::new("ns.speed"), bind, speed, "synthetic")
        .expect("once");
    let target = measure::<Dimensionless>(2.0);
    packet
        .synthetic(ParameterKey::new("ns.target"), bind, target, "synthetic")
        .expect("once");
    let gain = measure::<Dimensionless>(3.0);
    packet
        .synthetic(ParameterKey::new("ns.gain"), bind, gain, "synthetic")
        .expect("once");
    let name = |local| LawName { owner: "NS", local };
    packet
        .start(
            RowKind::Memory(name("filter")),
            measure::<Dimensionless>(1.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Algebraic(name("reading").into()),
            measure::<Dimensionless>(3.0),
            "synthetic",
        )
        .expect("once");
    packet
}

#[test]
fn memory_and_algebraic_rows_run_against_their_closed_form() {
    const END: u64 = 100 * SECOND;
    let body = filter_body();
    let error = |interval| {
        let core = reference(interval);
        let port = BoundaryPort::new(Vec::new()).expect("no event");
        let session = admit_fixture(&body, filter_packet(), port, &core).expect("admitted");
        let session = to(session, SimTime::from_ns(END));
        let accepted = session.record().accepted();
        let (filter, reading) = (body.rows()[0].id(), body.rows()[1].id());
        let exact = 2.0 - exp(-0.01 * 100.0);
        let values = [hi(accepted.value(filter)), hi(accepted.value(reading))];
        assert!(
            (values[1] - 3.0 * values[0]).abs() < 1e-9,
            "the closure holds at the head"
        );
        (values[0] - exact).abs()
    };
    // Backward Euler's global error here is `h / 4 * s^2 * T * exp(-s T)` for doubled steps of `h`.
    let (coarse, fine) = (error(2 * SECOND), error(SECOND));
    let predicted = 0.25 * 1e-4 * 100.0 * exp(-1.0);
    assert!(fine < 1.2 * predicted, "{fine} against {predicted}");
    assert!(coarse / fine > 1.8 && coarse / fine < 2.2, "{coarse} over {fine}");
}

#[test]
fn an_unsolved_closure_at_the_published_state_is_refused_on_its_row() {
    // The stage solves the closure at the start, and the end keeps that reading while the filter moved
    // by 0.01 * 1 * 1 s, so the published residual is 0.03, above a scale of 1e-3.
    let body = filter_body();
    let core = ForwardEulerWithScale {
        inner: euler(SECOND),
        scales: scales(1e-3, 0.0),
    };
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let session = admit_fixture(&body, filter_packet(), port, &core).expect("admitted");
    let session = refused(session.step());
    let reading = body.rows()[1].id();
    assert_eq!(
        session.refused().reason(),
        NumericalRefusal::StageResidual { row: reading }
    );
}

struct ForwardEulerWithScale {
    inner: ForwardEuler,
    scales: ErrorScales,
}
impl CoupledSolver for ForwardEulerWithScale {
    fn resolution(&self) -> DeclaredResolution {
        DeclaredResolution::new(
            Digest::of_canonical(b"forward euler, tight"),
            MILLISECOND,
            12,
            ArithmeticProfile::contract_v1(),
            StageScheme::forward_euler(),
            self.scales,
        )
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn an_algebraic_start_that_does_not_solve_is_refused_with_its_row() {
    let body = filter_body();
    let mut packet = ParameterPacket::new();
    let bind = Binding::Body;
    let speed = measure::<Rate<Dimensionless>>(0.01);
    packet
        .synthetic(ParameterKey::new("ns.speed"), bind, speed, "synthetic")
        .expect("once");
    let target = measure::<Dimensionless>(2.0);
    packet
        .synthetic(ParameterKey::new("ns.target"), bind, target, "synthetic")
        .expect("once");
    let gain = measure::<Dimensionless>(3.0);
    packet
        .synthetic(ParameterKey::new("ns.gain"), bind, gain, "synthetic")
        .expect("once");
    let name = |local| LawName { owner: "NS", local };
    packet
        .start(
            RowKind::Memory(name("filter")),
            measure::<Dimensionless>(1.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Algebraic(name("reading").into()),
            measure::<Dimensionless>(4.0),
            "synthetic",
        )
        .expect("once");
    let core = reference(SECOND);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let refused = admit_fixture(&body, packet, port, &core).err();
    let row = AdmissionRow::Body(body.rows()[1].id());
    assert_eq!(refused, Some(InitializationFailure::Inconsistent { row }));
}

#[test]
fn a_rate_law_undefined_at_the_start_is_refused_by_its_name_at_admission() {
    let law = LawName {
        owner: "CV",
        local: "circulation",
    };
    for edge in [EdgeLaw::Divides, EdgeLaw::TakesTheLog, EdgeLaw::Exponentiates] {
        let (body, packet) = edge_law_body_from(edge, refused_amounts(edge));
        let core = reference(SECOND);
        let refused = admit_fixture(&body, packet, no_events(), &core).err();
        assert_eq!(refused, Some(InitializationFailure::Undefined { law }));
    }
}

#[test]
fn a_law_undefined_at_the_start_is_refused_by_its_name_at_admission() {
    // `x = u / m` has no value at `m = 0`, and `m`'s own row, the first algebraic row, is solved.
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let level = laws.algebraic::<Pressure>("level", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let low = laws.parameter::<Pressure>(ParameterKey::new("ns.low"), Admissible::finite());
        let unit = laws.parameter::<Pressure>(ParameterKey::new("ns.unit"), Admissible::positive());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let held = laws.kernel(|mut k| {
            let m = k.algebraic(&level);
            let target = k.parameter(&low);
            let residual = k.subtract(m, target);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&level, held);
        let closure = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let u = k.parameter(&unit);
            let m = k.algebraic(&level);
            let ratio = k.divide(u, m);
            let ratio = k.as_dimension::<_, Dimensionless>(ratio);
            let residual = k.subtract(x, ratio);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&reading, closure);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let name = |local| LawName { owner: "NS", local };
    let mut packet = ParameterPacket::new();
    for (key, value) in [("ns.low", 0.0), ("ns.unit", 1.0)] {
        packet
            .synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Pressure>(value),
                "synthetic",
            )
            .expect("once");
    }
    packet
        .start(
            RowKind::Algebraic(name("level").into()),
            measure::<Pressure>(0.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Algebraic(name("reading").into()),
            measure::<Dimensionless>(1.0),
            "synthetic",
        )
        .expect("once");
    let core = reference(SECOND);
    let refused = admit_fixture(&body, packet, no_events(), &core).err();
    let law = name("reading");
    assert_eq!(refused, Some(InitializationFailure::Undefined { law }));
}

#[test]
fn a_solved_algebraic_value_outside_its_set_is_refused_with_its_row() {
    // `x = g m` with `g = -3` and `m = 1` solves at `x = -3`, which a positive slot cannot hold.
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let filter = laws.state::<Dimensionless>("filter", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::positive());
        let gain = laws.parameter::<Dimensionless>(ParameterKey::new("ns.gain"), Admissible::finite());
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            k.finish(zero, laws.every_mode(), reflex)
        });
        laws.memory_rate(&filter, rate);
        let closure = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let g = k.parameter(&gain);
            let now = k.state(&filter);
            let scaled = k.scale(g, now);
            let residual = k.subtract(x, scaled);
            k.finish(residual, laws.every_mode(), reflex)
        });
        laws.residual(&reading, closure);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let name = |local| LawName { owner: "NS", local };
    let mut packet = ParameterPacket::new();
    let gain = measure::<Dimensionless>(-3.0);
    packet
        .synthetic(ParameterKey::new("ns.gain"), Binding::Body, gain, "synthetic")
        .expect("once");
    let (filter, reading) = (
        RowKind::Memory(name("filter")),
        RowKind::Algebraic(name("reading").into()),
    );
    packet
        .start(filter, measure::<Dimensionless>(1.0), "synthetic")
        .expect("once");
    packet
        .start(reading, measure::<Dimensionless>(1.0), "synthetic")
        .expect("once");
    let at = SimTime::from_ns(0);
    let parameters = body.bind(packet).expect("the guess is in its set");
    let request = InitializationRequest::at(at);
    let system = body
        .initialization(parameters, no_events(), request)
        .expect("a system");
    let mut solved = system.workspace();
    solved.unknowns_mut()[1] = -3.0;
    let history = MethodHistory::new(Digest::of_canonical(b"a schema"), at, Vec::new());
    let refused = system.admit(solved, history).err();
    let row = AdmissionRow::Body(body.rows()[1].id());
    assert_eq!(refused, Some(InitializationFailure::Inconsistent { row }));
}

#[test]
fn a_validity_excursion_is_published_and_named_never_refused() {
    let flow =
        |lower, upper| ValidityRange::new(measure(lower), measure(upper), Basis::Fixture("synthetic range"));
    let excursions = |range| {
        let case = TwoReservoirs::trusted_within(range).expect("a body");
        let core = reference(8 * SECOND);
        let (_, receipt) = accepted(case.admit(&core).expect("admitted").step());
        receipt.excursions().to_vec()
    };
    // `q` starts at k (a - b) = 1 and decays slowly, so these ranges sit either side of it.
    let above = excursions(flow(-1.0, 0.5).expect("a range"));
    assert_eq!(above.len(), 1);
    assert_eq!(above[0].side(), ExcursionSide::Above);
    assert_eq!(
        above[0].law(),
        LawName {
            owner: "CV",
            local: "circulation"
        }
    );
    assert_eq!(above[0].at(), SimTime::from_ns(8 * SECOND));
    assert!(matches!(above[0].context(), Context::Edge { .. }));
    assert!(hi(above[0].value()) > 0.9 && hi(above[0].value()) < 1.0);
    let below = excursions(flow(2.0, 3.0).expect("a range"));
    assert_eq!(below[0].side(), ExcursionSide::Below);
    assert_eq!(excursions(flow(0.0, 2.0).expect("a range")), []);
}

fn delivery(id: &'static [u8], from_s: u64, to_s: u64, rate: f64) -> Delivery {
    Delivery {
        id,
        from: SimTime::from_ns(from_s * SECOND),
        to: SimTime::from_ns(to_s * SECOND),
        amount: measure(rate * (to_s - from_s) as f64),
    }
}

#[test]
fn closure_holds_over_every_window_and_drift_tells_a_closed_body_from_an_open_one() {
    let closed = TwoReservoirs::closed().expect("a body");
    let core = reference(60 * SECOND);
    let session = to(
        closed.admit(&core).expect("admitted"),
        SimTime::from_ns(48 * HOUR),
    );
    let view = session.record().conservation();
    let policy = ledger_policy(view.basis(), 1e-6);
    let certificate = certify(&view, &policy).expect("a closed body holds still");
    assert_eq!(
        certificate.pieces(),
        1440,
        "one piece per window start, none sampled"
    );
    assert_eq!(certificate.seal(), view.seal());
    assert_eq!(certificate.basis(), view.basis().digest());
    assert_eq!(certificate.policy(), policy.digest());
    let result = certificate.results()[0];
    assert_eq!(
        result.quantity(),
        view.basis().ids().next().expect("one combination")
    );
    assert!(
        result
            .worst_closure()
            .at_most(Compensated::of(&measure::<Water>(1e-24)))
    );
    assert_eq!(result.closure_window().length_ns(), 24 * HOUR);
    assert!(
        result
            .worst_drift()
            .at_most(Compensated::of(&measure::<Water>(1e-6)))
    );

    let open = stepped();
    let session = to(open.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    certify(&view, &ledger_policy(view.basis(), 1e12)).expect("an open body's ledger still closes");
    let failure = certify(&view, &ledger_policy(view.basis(), 1.0)).expect_err("it does not hold still");
    assert_eq!(failure.check(), CertificateCheck::Drift);
    assert!(failure.window().is_some() && failure.quantity().is_some());
    assert!(hi(failure.residual()) > 1.0);
}

#[test]
fn the_worst_window_can_start_inside_a_segment() {
    // The change over a day peaks with the window ending where the high rate stops: 96,400 + 20,000 s,
    // so the window starts at 30,000 s, between the 70 s grid's 29,960 and 30,030.
    let case = TwoReservoirs::delivering(
        [measure(500_500.0), measure(499_500.0)],
        vec![
            delivery(b"baseline", 0, 96_400, 1.0),
            delivery(b"high", 96_400, 116_400, 5.0),
        ],
    )
    .expect("a body");
    let core = reference(70 * SECOND);
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    let certificate = certify(&view, &ledger_policy(view.basis(), 1e12)).expect("it closes");
    let result = certificate.results()[0];
    assert_eq!(result.drift_window().start(), SimTime::from_ns(30_000 * SECOND));
    assert!(
        (hi(result.worst_drift()) - 166_400.0).abs() < 1e-3,
        "{:?}",
        result.worst_drift()
    );
}

#[test]
fn a_ledger_that_does_not_close_fails_closure_before_drift() {
    // The bulk fills the low limb of `a`, about 16 on a high limb of 1e18, and a 6e-17 extent is then
    // below that limb's rounding, so the inventory stands still while the exchange grows. Only an extent
    // this far below an amount can leave a compensated ledger open.
    let bulk = delivery(b"bulk", 0, 3_600, 1.0);
    let tiny = Delivery {
        id: b"below the pair's precision",
        from: SimTime::from_ns(3_600 * SECOND),
        to: SimTime::from_ns(48 * HOUR),
        amount: measure(1e-18 * 169_200.0),
    };
    let case = TwoReservoirs::delivering([measure(1e18), measure(1e18)], vec![bulk, tiny]).expect("a body");
    let core = reference(60 * SECOND);
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    let failure = certify(&view, &ledger_policy(view.basis(), 0.0)).expect_err("it does not close");
    assert_eq!(failure.check(), CertificateCheck::Closure);
    assert!(failure.window().is_some());
}

#[test]
fn coverage_under_48_hours_and_a_policy_of_another_basis_fail() {
    let core = reference(60 * SECOND);
    let case = stepped();
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(30 * HOUR));
    let view = session.record().conservation();
    let failure = certify(&view, &ledger_policy(view.basis(), 1e12)).expect_err("too short");
    assert_eq!(failure.check(), CertificateCheck::Coverage);

    let session = case.admit(&core).expect("admitted");
    let view = session.record().conservation();
    let failure = certify(&view, &ledger_policy(view.basis(), 1e12)).expect_err("no step yet");
    assert_eq!(failure.check(), CertificateCheck::Coverage);

    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let sites = [
        map.site::<VesselSegment>("a", origin()),
        map.site::<VesselSegment>("b", origin()),
        map.site::<VesselSegment>("c", origin()),
    ];
    for site in &sites {
        map.contain(root.id(), site.id());
    }
    map.connect(Network::Vascular, sites[0].id(), sites[1].id());
    map.connect(Network::Vascular, sites[1].id(), sites[2].id());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let three = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    let foreign = ledger_policy(three.conserved_basis(), 1e12);
    let failure = certify(&view, &foreign).expect_err("another basis");
    assert_eq!(failure.check(), CertificateCheck::Coverage);
}

#[test]
fn a_certificate_policy_is_checked_when_it_is_built() {
    let case = stepped();
    let basis = case.body().conserved_basis();
    let quantity = basis.ids().next().expect("one combination");
    let wrong = QuantityPolicy::new::<Pressure>(quantity, measure(1.0), measure(0.0), measure(1.0)).err();
    assert_eq!(wrong, Some(AdmissionError::QuantityDimension { quantity }));
    let windows = Sliding24Hours::covering(48 * HOUR).expect("a coverage");
    let short = CertificatePolicy::new(basis, Vec::new(), ExpectedDrift::Zero, windows).err();
    assert_eq!(short, Some(AdmissionError::PolicyCoverage));
    assert_eq!(
        Sliding24Hours::covering(24 * HOUR).err(),
        Some(AdmissionError::ShortCoverage)
    );
}

#[test]
fn an_accuracy_policy_needs_a_case_and_a_grid() {
    let no_case = AccuracyPolicy::new(Vec::new(), measure(5.0), measure(0.0), SECOND).err();
    assert_eq!(no_case, Some(AdmissionError::EmptyAccuracyPolicy));
    let cases = vec![AccuracyCase::TwoReservoir(stepped())];
    let no_grid = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 0).err();
    assert_eq!(no_grid, Some(AdmissionError::EmptyAccuracyPolicy));
    let limit = |absolute| {
        let cases = vec![AccuracyCase::TwoReservoir(stepped())];
        AccuracyPolicy::new(cases, measure(absolute), measure(0.0), SECOND).expect("a policy")
    };
    assert_eq!(limit(5.0).digest(), limit(5.0).digest());
    assert_ne!(limit(5.0).digest(), limit(6.0).digest());
}

#[test]
fn a_core_that_cannot_be_admitted_has_not_failed_on_accuracy() {
    let core = reference(8 * SECOND).with_floor(0);
    let failure = accuracy(&core, &gate(stepped()));
    assert!(
        matches!(failure, Err(AccuracyFailure::Admission(_))),
        "{failure:?}"
    );
}

#[test]
fn a_core_that_cannot_finish_a_case_has_not_failed_on_accuracy() {
    let core = NeverConverges {
        interval_ns: 8 * SECOND,
        budget: 2,
    };
    let failure = accuracy(&core, &gate(stepped()));
    assert!(matches!(failure, Err(AccuracyFailure::Faulted(_))), "{failure:?}");
}

#[test]
fn a_flow_that_runs_backward_keeps_its_gross_reverse_apart() {
    // `b` starts above `a`, so the circulation's rate is negative throughout.
    let case = TwoReservoirs::delivering([measure(400.0), measure(600.0)], Vec::new()).expect("a body");
    let core = reference(SECOND);
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    let circulation = (case.body().instances().iter())
        .find_map(|instance| match (instance.context(), instance.target()) {
            (Context::Edge { .. }, Target::Column(column)) => Some(column),
            _ => None,
        })
        .expect("the body circulates");
    let view = session.record().conservation();
    let segment = view.segments().next_back().expect("an accepted step");
    let (forward, reverse) = segment.exchange(circulation);
    let piece = segment.interval();
    assert_eq!(forward.on(piece).expect("its own segment")[1], Compensated::ZERO);
    let moved = hi(reverse.on(piece).expect("its own segment")[1]);
    assert!(moved > 0.19 && moved < 0.2, "{moved}");
    let [a, b] = case.rows().map(|row| hi(session.record().accepted().value(row)));
    assert!((a - (400.0 + moved)).abs() < 1e-12 && (b - (600.0 - moved)).abs() < 1e-12);
    let quantity = view.basis().ids().next().expect("one combination");
    assert_eq!(
        segment.inflow(quantity).on(piece).expect("its own segment")[1],
        Compensated::ZERO,
        "no combination sees it"
    );
    assert_eq!(
        segment.outflow(quantity).on(piece).expect("its own segment")[1],
        Compensated::ZERO
    );
}

#[test]
fn a_boundary_out_of_the_body_counts_as_outflow_and_the_ledger_closes() {
    let drained = 30.0;
    let case = TwoReservoirs::draining(
        [measure(100.0), measure(100.0)],
        vec![delivery(b"drain", 0, 100, drained / 100.0)],
    )
    .expect("a body");
    let core = reference(8 * SECOND);
    let session = to(
        case.admit(&core).expect("admitted"),
        SimTime::from_ns(120 * SECOND),
    );
    let view = session.record().conservation();
    let quantity = view.basis().ids().next().expect("one combination");
    let segment = view.segments().next_back().expect("an accepted step");
    let piece = segment.interval();
    assert!((hi(segment.outflow(quantity).on(piece).expect("its own segment")[1]) - drained).abs() < 1e-9);
    assert_eq!(
        segment.inflow(quantity).on(piece).expect("its own segment")[1],
        Compensated::ZERO
    );
    assert!(
        (hi(segment.inventory(quantity).on(piece).expect("its own segment")[1]) - (200.0 - drained)).abs()
            < 1e-9
    );
}

/// Two reservoirs with no flow between them and a boundary law that runs backward at a fixed rate.
fn backward_boundary_body(direction: BoundaryDirection) -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema
        .water_boundary("exterior", direction, a.id(), Space::Lumen)
        .expect("one boundary");
    let leak = ParameterKey::new("boundary.leak");
    let body = assemble(schema, |mut body| {
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryWater { route: pair, .. } = ports.water("exterior")?;
        let rate = laws.parameter::<Rate<Water>>(leak, Admissible::positive());
        let backward = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            let rate = k.parameter(&rate);
            let backward = k.subtract(zero, rate);
            k.finish(backward, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&pair, backward));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(leak, Binding::Body, measure::<Rate<Water>>(0.5), "synthetic")
        .expect("once");
    for row in body.rows() {
        packet
            .start(row.kind(), measure::<Water>(100.0), "synthetic")
            .expect("once");
    }
    (body, packet)
}

#[test]
fn a_boundary_flow_that_runs_backward_counts_on_the_side_it_moves() {
    // Against an inward boundary a reverse flow leaves the body; against an outward one it enters.
    for (direction, gained, lost) in [
        (BoundaryDirection::IntoBody, 0.0, 5.0),
        (BoundaryDirection::OutOfBody, 5.0, 0.0),
    ] {
        let (body, packet) = backward_boundary_body(direction);
        let core = reference(SECOND);
        let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
        let session = to(session, SimTime::from_ns(10 * SECOND));
        let view = session.record().conservation();
        let quantity = view.basis().ids().next().expect("one combination");
        let segment = view.segments().next_back().expect("an accepted step");
        let piece = segment.interval();
        assert_eq!(
            hi(segment.inflow(quantity).on(piece).expect("its own segment")[1]),
            gained
        );
        assert_eq!(
            hi(segment.outflow(quantity).on(piece).expect("its own segment")[1]),
            lost
        );
        assert_eq!(
            hi(segment.inventory(quantity).on(piece).expect("its own segment")[1]),
            200.0 + gained - lost
        );
    }
}

#[test]
fn a_segment_samples_only_its_own_interval() {
    let case = stepped();
    let core = reference(8 * SECOND);
    let session = case.admit(&core).expect("admitted");
    let row = case.rows()[0];
    let opening = session.record().accepted().last_segment();
    assert_eq!(opening.interval(), None);
    assert_eq!(opening.sample(row, SimTime::from_ns(0)), None);
    let (session, _) = accepted(session.step());
    let segment = session.record().accepted().last_segment();
    let start = Compensated::of(&measure::<Water>(500_500.0));
    assert_eq!(segment.sample(row, SimTime::from_ns(0)), Some(start));
    let end = session.record().accepted().value(row);
    assert_eq!(segment.sample(row, SimTime::from_ns(8 * SECOND)), Some(end));
    let halfway = segment.sample(row, SimTime::from_ns(4 * SECOND)).expect("inside");
    assert!(!start.at_most(halfway) && !halfway.at_most(end));
    assert_eq!(segment.sample(row, SimTime::from_ns(8 * SECOND + 1)), None);
    let view = session.record().conservation();
    let quantity = view.basis().ids().next().expect("one combination");
    let curve = view
        .segments()
        .next_back()
        .expect("an accepted step")
        .inventory(quantity);
    let piece = |from, to| {
        Interval::new(SimTime::from_ns(from * SECOND), SimTime::from_ns(to * SECOND)).expect("an interval")
    };
    assert!(curve.on(piece(0, 8)).is_some());
    assert_eq!(curve.on(piece(8, 9)), None);
    assert_eq!(curve.on(piece(1, 9)), None);
}

#[test]
fn a_relative_accuracy_limit_reads_the_exact_value() {
    let case = |relative| {
        let cases = vec![AccuracyCase::TwoReservoir(stepped())];
        AccuracyPolicy::new(cases, measure(1e-300), measure(relative), 100 * SECOND).expect("a policy")
    };
    let core = reference(8 * SECOND);
    accuracy(&core, &case(1e-2)).expect("within one part in a hundred of each exact value");
    let failure = accuracy(&core, &case(1e-9));
    assert!(
        matches!(failure, Err(AccuracyFailure::Exceeded { .. })),
        "{failure:?}"
    );
}

#[test]
fn a_relative_limit_holds_the_exchange_to_its_own_size() {
    // Amounts near 5e5 carry at most 2 in error, four parts in a million. The first step's error of about
    // 0.03 sits on an exchange near 8, about 0.4%, so only reading the exchange can reject this.
    let cases = vec![AccuracyCase::TwoReservoir(stepped())];
    let policy = AccuracyPolicy::new(cases, measure(1e-300), measure(1e-4), 100 * SECOND).expect("a policy");
    let failure = accuracy(&reference(8 * SECOND), &policy);
    let Err(AccuracyFailure::Exceeded { at, .. }) = failure else {
        panic!("not rejected on the exchange: {failure:?}");
    };
    assert_eq!(at, SimTime::from_ns(8 * SECOND));
}

#[test]
fn the_same_inputs_give_the_same_digests_and_seal_twice() {
    let run = || {
        let case = stepped();
        let core = reference(64 * SECOND);
        let session = to(case.admit(&core).expect("admitted"), case.end());
        (session.record().digests(), session.record().seal())
    };
    assert_eq!(run(), run());
}

/// Amounts so large that a chord read at an integer-ns fraction overflows, so a compared value is a NaN.
fn overflowing() -> (TwoReservoirs, ReferenceIntegrator) {
    let delivery = Delivery {
        id: b"large",
        from: SimTime::from_ns(0),
        to: SimTime::from_ns(48 * HOUR),
        amount: measure(1e300),
    };
    let case = TwoReservoirs::delivering([measure(1e290), measure(1e290)], vec![delivery]).expect("a body");
    (case, reference(5_000 * SECOND).with_scales(scales(1e295, 0.1)))
}

#[test]
fn a_certificate_does_not_pass_on_a_value_that_overflowed() {
    let (case, core) = overflowing();
    let session = to(case.admit(&core).expect("admitted"), SimTime::from_ns(48 * HOUR));
    let view = session.record().conservation();
    let failure = certify(&view, &ledger_policy_with(view.basis(), 1e280, 0.0, 0.0)).expect_err("a NaN");
    assert_eq!(failure.check(), CertificateCheck::Closure);
}

#[test]
fn the_accuracy_gate_does_not_pass_on_a_value_that_overflowed() {
    let (case, core) = overflowing();
    let cases = vec![AccuracyCase::TwoReservoir(case)];
    let policy = AccuracyPolicy::new(cases, measure(1e299), measure(0.0), 100 * SECOND).expect("a policy");
    let failure = accuracy(&core, &policy);
    assert!(
        matches!(failure, Err(AccuracyFailure::Exceeded { .. })),
        "{failure:?}"
    );
}

/// One step of `m' = r` and `x x = m` from `(1, 1)`, under scales that leave no room to stop early.
fn newton_reaches_the_root(rate: f64, interval_ns: u64) {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let memory = laws.state::<Dimensionless>("memory", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let rate =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.rate"), Admissible::positive());
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000)?;
        let rise = laws.kernel(|mut k| {
            let value = k.parameter(&rate);
            k.finish(value, laws.every_mode(), reflex)
        });
        laws.memory_rate(&memory, rise);
        let root = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let again = k.algebraic(&reading);
            let square = k.scale(x, again);
            let m = k.state(&memory);
            let residual = k.subtract(square, m);
            k.finish(residual, laws.every_mode(), reflex)
        });
        laws.residual(&reading, root);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let name = |local| LawName { owner: "NS", local };
    let mut packet = ParameterPacket::new();
    let value = measure::<Rate<Dimensionless>>(rate);
    packet
        .synthetic(ParameterKey::new("ns.rate"), Binding::Body, value, "synthetic")
        .expect("once");
    for start in [
        RowKind::Memory(name("memory")),
        RowKind::Algebraic(name("reading").into()),
    ] {
        packet
            .start(start, measure::<Dimensionless>(1.0), "synthetic")
            .expect("once");
    }
    let core = reference(interval_ns)
        .with_floor(1)
        .with_scales(scales(1e-14, 0.0));
    let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
    let (session, _) = accepted(session.step());
    let root = (1.0 + rate * interval_ns as f64 / 1e9).sqrt();
    let solved = hi(session.record().accepted().value(body.rows()[1].id()));
    assert!((solved - root).abs() < 1e-12, "{solved} against {root}");
}

#[test]
fn newton_solves_an_algebraic_row_to_rounding_on_a_short_step() {
    // Scaling every residual by the span stops the 1 ns stage with the root's residual near 9.6e-13, above
    // these scales, though the next update solves it.
    newton_reaches_the_root(0.001, MILLISECOND);
    newton_reaches_the_root(2.8e6, 1);
}

#[test]
fn newton_reads_the_stage_its_last_update_reaches() {
    // From `x = 1` each of these roots takes all eight updates. Some square back to their target exactly and
    // stop on a zero residual, so only the run of them shows the last update is read.
    for rate in 15..36 {
        newton_reaches_the_root(f64::from(rate), SECOND);
    }
}

#[test]
fn the_closed_form_follows_a_boundary_out_of_the_body() {
    let start = [measure(500_500.0), measure(499_500.0)];
    let case = TwoReservoirs::draining(start, vec![delivery(b"drain", 0, 100, 1.0)]).expect("a body");
    let case = case.until(SimTime::from_ns(1_000 * SECOND));
    accuracy(&reference(16 * SECOND), &gate(case)).expect("within the policy");
}

#[test]
fn the_closed_form_counts_a_repeated_delivery_once() {
    let start = [measure(500_500.0), measure(499_500.0)];
    let once = || delivery(b"once", 0, 100, 1.0);
    let case = TwoReservoirs::delivering(start, vec![once(), once()]).expect("a body");
    let case = case.until(SimTime::from_ns(1_000 * SECOND));
    accuracy(&reference(16 * SECOND), &gate(case)).expect("within the policy");
}

#[test]
fn a_policy_names_the_whole_case_and_not_only_its_body() {
    let digest = |case| gate(case).digest();
    let start = || [measure(500_500.0), measure(499_500.0)];
    let quiet = || TwoReservoirs::delivering(start(), Vec::new()).expect("a body");
    assert_eq!(digest(quiet()), digest(quiet()));
    let moved = TwoReservoirs::delivering([measure(500_000.0), measure(500_000.0)], Vec::new());
    assert_ne!(digest(quiet()), digest(moved.expect("a body")));
    let fed = TwoReservoirs::delivering(start(), vec![delivery(b"fed", 0, 100, 1.0)]);
    assert_ne!(digest(quiet()), digest(fed.expect("a body")));
    let same_delivery = Delivery {
        id: b"stepped inflow",
        from: SimTime::from_ns(2_000 * SECOND),
        to: SimTime::from_ns(6_000 * SECOND),
        amount: measure(20_000.0),
    };
    let longer = TwoReservoirs::delivering(start(), vec![same_delivery]).expect("a body");
    assert_ne!(digest(stepped()), digest(longer), "only the case's end differs");
}

#[test]
fn a_ladder_of_two_policies_states_no_order() {
    let error = |policy: &AccuracyPolicy, interval| {
        accuracy(&reference(interval), policy).expect("within the policy")
    };
    let short = TwoReservoirs::closed().expect("a body");
    let short = short.until(SimTime::from_ns(1_000 * SECOND));
    let cases = vec![
        AccuracyCase::TwoReservoir(stepped()),
        AccuracyCase::TwoReservoir(short),
    ];
    let wider = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy");
    let (coarse, medium, fine) = (
        error(&gate(stepped()), 16 * SECOND),
        error(&wider, 8 * SECOND),
        error(&gate(stepped()), 4 * SECOND),
    );
    let failure = observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1));
    assert!(matches!(
        failure,
        Err(AccuracyFailure::MixedEvidence { refinement: 0 })
    ));
}

#[test]
fn an_overflowed_order_limit_fails_instead_of_passing() {
    let error = |interval| accuracy(&reference(interval), &gate(stepped())).expect("within the policy");
    let (coarse, medium, fine) = (error(16 * SECOND), error(8 * SECOND), error(4 * SECOND));
    let failure = observed_order([&coarse, &medium, &fine], measure(1.7e308), measure(1.79e308));
    assert!(matches!(
        failure,
        Err(AccuracyFailure::Order {
            case: 0,
            refinement: 0
        })
    ));
}

#[test]
fn exact_answers_show_no_order() {
    let error = |interval| {
        let start = [measure::<Water>(5e5), measure::<Water>(5e5)];
        let still = TwoReservoirs::delivering(start, Vec::new()).expect("a body");
        let still = still.until(SimTime::from_ns(1_000 * SECOND));
        accuracy(&reference(interval), &gate(still)).expect("within the policy")
    };
    let (coarse, medium, fine) = (error(16 * SECOND), error(8 * SECOND), error(4 * SECOND));
    assert_eq!(hi(coarse.worst()[0]), 0.0);
    let failure = observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1));
    assert!(matches!(
        failure,
        Err(AccuracyFailure::Order {
            case: 0,
            refinement: 0
        })
    ));
}

#[test]
fn an_amount_past_the_split_limit_still_balances_in_the_ledger() {
    let start = [measure::<Water>(1e301), measure::<Water>(1e301)];
    let case = TwoReservoirs::delivering(start, Vec::new()).expect("a body");
    let core = reference(SECOND);
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    let view = session.record().conservation();
    let quantity = view.basis().ids().next().expect("one combination");
    let segment = view.segments().next().expect("one step");
    let inventory = segment.inventory(quantity).on(segment.interval());
    let total = Compensated::of(&measure::<Water>(2e301));
    assert_eq!(inventory, Some(vec![total, total]));
}

#[test]
fn a_ledger_past_the_finite_range_is_refused_before_it_publishes() {
    let start = [measure::<Water>(1.5e308), measure::<Water>(1.5e308)];
    let case = TwoReservoirs::delivering(start, Vec::new()).expect("a body");
    let core = reference(SECOND);
    let stopped = exhaust(case.admit(&core).expect("admitted").step());
    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    assert_eq!(
        stopped.fault().last_refusal(),
        Some(NumericalRefusal::LedgerOverflow)
    );
    assert_eq!(stopped.record().revision(), 0);
}

#[test]
fn a_core_reads_back_the_continuation_it_wrote() {
    let case = TwoReservoirs::closed().expect("a body");
    let core = KeepsItsSpan { first_ns: SECOND };
    let mut session = case.admit(&core).expect("admitted");
    for span in [SECOND, 2 * SECOND, 4 * SECOND] {
        session = accepted(session.step()).0;
        let segment = session.record().accepted().last_segment();
        let interval = segment.interval().expect("a step");
        assert_eq!(interval.length_ns(), span);
    }
}

#[test]
fn the_certificate_walk_reads_each_combination_and_each_segments_basis() {
    let case = TwoReservoirs::closed().expect("a body");
    let core = reference(SECOND);
    let (session, _) = accepted(case.admit(&core).expect("admitted").step());
    let view = session.record().conservation();
    let ids: Vec<QuantityId> = view.basis().ids().collect();
    let named: Vec<QuantityId> = view.basis().combinations().iter().map(Combination::id).collect();
    assert_eq!(named, ids);
    let segment = view.segments().next().expect("one step");
    let basis = segment.basis();
    assert_eq!(basis.family(), BasisFamily::Bernstein);
    let curve = segment
        .inventory(ids[0])
        .on(segment.interval())
        .expect("its own segment");
    assert_eq!(curve.len(), usize::from(basis.degree()) + 1);
}

fn two_slot_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        for name in ["first", "second"] {
            let slot = laws.algebraic::<Dimensionless>(name, Admissible::finite());
            let closure = laws.kernel(|mut k| {
                let value = k.algebraic(&slot);
                k.finish(value, laws.every_mode(), wave)
            });
            laws.residual(&slot, closure);
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn two_slot_packet() -> ParameterPacket<SyntheticFixture> {
    let name = |local| LawName { owner: "NS", local };
    let mut packet = ParameterPacket::new();
    for slot in ["first", "second"] {
        packet
            .start(
                RowKind::Algebraic(name(slot).into()),
                measure::<Dimensionless>(0.0),
                "synthetic",
            )
            .expect("once");
    }
    packet
}

#[test]
fn a_proposal_short_of_algebraic_values_is_a_binding_mismatch() {
    let foreign_case = TwoReservoirs::closed().expect("a body");
    let inner = euler(SECOND);
    let host = foreign_case.admit(&inner).expect("admitted");
    let values = foreign_case.other_values().expect("bound");
    let core = ProposesThroughAnotherBody {
        inner: euler(SECOND),
        foreign: foreign_case.body(),
        values: &values,
        identity: host.record().identity(),
        accepted: host.record().accepted(),
        stash: None,
    };
    let body = filter_body();
    let session = admit_fixture(&body, filter_packet(), no_events(), &core).expect("admitted");
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_core_cannot_stash_numbers_in_the_accepted_state() {
    let foreign = two_slot_body();
    let inner = euler(SECOND);
    let host = admit_fixture(&foreign, two_slot_packet(), no_events(), &inner).expect("admitted");
    let values = foreign.bind(two_slot_packet()).expect("bound");
    let core = ProposesThroughAnotherBody {
        inner: euler(SECOND),
        foreign: &foreign,
        values: &values,
        identity: host.record().identity(),
        accepted: host.record().accepted(),
        stash: Some([1234.5, 6789.25]),
    };
    let case = TwoReservoirs::closed().expect("a body");
    let session = case.admit(&core).expect("admitted");
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

#[test]
fn a_workspace_of_another_length_is_a_binding_mismatch() {
    let case = TwoReservoirs::closed().expect("a body");
    let inner = euler(SECOND);
    let host = case.admit(&inner).expect("admitted");
    let values = case.other_values().expect("bound");
    let lone = filter_only_body();
    let small = admit_fixture(&lone, filter_only_packet(), no_events(), &inner).expect("admitted");
    let core = ShortWorkspace {
        inner: euler(SECOND),
        body: case.body(),
        values: &values,
        identity: host.record().identity(),
        accepted: small.record().accepted(),
    };
    let session = case.admit(&core).expect("admitted");
    assert_eq!(fault_of(session.step()).kind(), EngineFaultKind::BindingMismatch);
}

fn singular_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let memory = laws.state::<Dimensionless>("memory", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let rate =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.rate"), Admissible::positive());
        let held = laws.parameter::<Dimensionless>(ParameterKey::new("ns.held"), Admissible::finite());
        let reflex = RateClass::new(RateBand::Reflex, SECOND)?;
        let rise = laws.kernel(|mut k| {
            let value = k.parameter(&rate);
            k.finish(value, laws.every_mode(), reflex)
        });
        laws.memory_rate(&memory, rise);
        let closure = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let again = k.algebraic(&reading);
            let nothing = k.subtract(x, again);
            let m = k.state(&memory);
            let c = k.parameter(&held);
            let off = k.subtract(m, c);
            let residual = k.add(nothing, off);
            k.finish(residual, laws.every_mode(), reflex)
        });
        laws.residual(&reading, closure);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

#[test]
fn a_singular_row_is_named_by_the_fault_that_ends_the_run() {
    // `(x - x) + (m - c)` reads `x`, so it matches structurally, and it has no slope in `x`, so the
    // stage's Jacobian has no pivot on that row.
    let body = singular_body();
    let name = |local| LawName { owner: "NS", local };
    let mut packet = ParameterPacket::new();
    let bind = Binding::Body;
    let rate = measure::<Rate<Dimensionless>>(0.5);
    packet
        .synthetic(ParameterKey::new("ns.rate"), bind, rate, "synthetic")
        .expect("once");
    let held = measure::<Dimensionless>(1.0);
    packet
        .synthetic(ParameterKey::new("ns.held"), bind, held, "synthetic")
        .expect("once");
    for (row, start) in [
        (RowKind::Memory(name("memory")), 1.0),
        (RowKind::Algebraic(name("reading").into()), 0.0),
    ] {
        packet
            .start(row, measure::<Dimensionless>(start), "synthetic")
            .expect("once");
    }
    let core = reference(SECOND);
    let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
    let stopped = exhaust(session.step());
    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    let row = body.rows()[1].id();
    assert_eq!(
        stopped.fault().last_refusal(),
        Some(NumericalRefusal::StageResidual { row })
    );
}

fn deaf_boundary_body() -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema
        .water_boundary("exterior", BoundaryDirection::IntoBody, a.id(), Space::Lumen)
        .expect("one boundary");
    let leak = ParameterKey::new("boundary.leak");
    let body = assemble(schema, |mut body| {
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryWater { route: pair, .. } = ports.water("exterior")?;
        let rate = laws.parameter::<Rate<Water>>(leak, Admissible::nonnegative());
        let fixed = laws.kernel(|mut k| {
            let rate = k.parameter(&rate);
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&pair, fixed));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(leak, Binding::Body, measure::<Rate<Water>>(0.0), "synthetic")
        .expect("once");
    for row in body.rows() {
        packet
            .start(row.kind(), measure::<Water>(100.0), "synthetic")
            .expect("once");
    }
    (body, packet)
}

#[test]
fn a_delivery_no_law_reads_is_refused_at_admission() {
    let (body, packet) = deaf_boundary_body();
    let delivery = WaterDelivery {
        boundary: "exterior",
        amount: measure(30.0),
        ends: SimTime::from_ns(10 * SECOND),
    };
    let id = InputId::new(Digest::of_canonical(b"meal"));
    let event = BoundaryEvent::new(id, SimTime::from_ns(0), BoundaryInput::Water(delivery));
    let port = BoundaryPort::new(vec![event]).expect("parsed");
    let core = reference(SECOND);
    let refused = admit_fixture(&body, packet, port, &core).err();
    assert_eq!(
        refused,
        Some(InitializationFailure::Admission(AdmissionError::DriveNotRead {
            name: "exterior"
        }))
    );
}

#[test]
fn a_late_delivery_no_law_reads_is_refused_at_enqueue() {
    let (body, packet) = deaf_boundary_body();
    let core = reference(SECOND);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let mut session = admit_fixture(&body, packet, port, &core).expect("admitted");
    let delivery = WaterDelivery {
        boundary: "exterior",
        amount: measure(30.0),
        ends: SimTime::from_ns(10 * SECOND),
    };
    let id = InputId::new(Digest::of_canonical(b"meal"));
    let event = BoundaryEvent::new(id, SimTime::from_ns(SECOND), BoundaryInput::Water(delivery));
    let input = AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a command"), event);
    assert_eq!(
        session.enqueue(input).err(),
        Some(AdmissionError::DriveNotRead { name: "exterior" })
    );
}

#[test]
fn a_delivery_whose_rate_leaves_the_finite_range_is_refused_when_it_is_stated() {
    let id = InputId::new(Digest::of_canonical(b"overflow"));
    let delivery = WaterDelivery {
        boundary: "exterior",
        amount: measure(1e300),
        ends: SimTime::from_ns(1),
    };
    let event = BoundaryEvent::new(id, SimTime::from_ns(0), BoundaryInput::Water(delivery));
    assert_eq!(
        BoundaryPort::new(vec![event]).err(),
        Some(AdmissionError::UnboundedDelivery { id })
    );
}

#[test]
fn newton_applies_a_correction_too_small_to_read_as_a_change() {
    // A rate of 1e6 per second makes a change of one ulp in the filter a defect far above these scales, so
    // a stage that stopped before its last correction fails its own equation.
    let body = filter_body();
    let mut packet = ParameterPacket::new();
    let bind = Binding::Body;
    let speed = measure::<Rate<Dimensionless>>(1e6);
    packet
        .synthetic(ParameterKey::new("ns.speed"), bind, speed, "synthetic")
        .expect("once");
    let target = measure::<Dimensionless>(1.000_000_000_000_001);
    packet
        .synthetic(ParameterKey::new("ns.target"), bind, target, "synthetic")
        .expect("once");
    let gain = measure::<Dimensionless>(1.0);
    packet
        .synthetic(ParameterKey::new("ns.gain"), bind, gain, "synthetic")
        .expect("once");
    let name = |local| LawName { owner: "NS", local };
    for start in [
        RowKind::Memory(name("filter")),
        RowKind::Algebraic(name("reading").into()),
    ] {
        packet
            .start(start, measure::<Dimensionless>(1.0), "synthetic")
            .expect("once");
    }
    let core = reference(MILLISECOND).with_scales(scales(1e-14, 0.0));
    let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
    accepted(session.step());
}

#[test]
fn a_stage_already_solved_needs_no_factorization() {
    // `x x = 0` from `x = 0` is solved, and its one Jacobian entry is zero.
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let root = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let again = k.algebraic(&reading);
            let square = k.scale(x, again);
            k.finish(square, laws.every_mode(), wave)
        });
        laws.residual(&reading, root);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    let row = RowKind::Algebraic(
        LawName {
            owner: "NS",
            local: "reading",
        }
        .into(),
    );
    packet
        .start(row, measure::<Dimensionless>(0.0), "synthetic")
        .expect("once");
    let core = reference(SECOND);
    let session = admit_fixture(&body, packet, no_events(), &core).expect("admitted");
    accepted(session.step());
}

#[test]
fn a_case_that_never_steps_is_refused_as_no_case() {
    let cases = vec![AccuracyCase::TwoReservoir(stepped().until(SimTime::from_ns(0)))];
    let empty = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), SECOND).err();
    assert_eq!(empty, Some(AdmissionError::EmptyAccuracyPolicy));
}
