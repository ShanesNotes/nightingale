use core::cell::Cell;
use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::Step;
use ng_solver::Core;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const INTERVAL_NS: u64 = 500 * MILLISECOND;
const NEWTON_LIMIT: u32 = 64;
const NEWTON_STOP: f64 = 8.0 * f64::EPSILON;
const NANOSECONDS_PER_SECOND: f64 = 1e9;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn reference() -> ReferenceIntegrator {
    ReferenceIntegrator::new(INTERVAL_NS)
}

fn resolution(tag: &[u8]) -> DeclaredResolution {
    let scales = ErrorScales::uniform(measure(1.0), measure(1e-5)).expect("positive");
    DeclaredResolution::new(
        Digest::of_canonical(tag),
        NonZeroU64::new(1_000_000).expect("a floor"),
        12,
        ArithmeticProfile::contract_v1(),
        StageScheme::backward_euler_doubled(),
        scales,
    )
}

fn amount_slots(problem: &CoupledProblem<'_>) -> Vec<usize> {
    let rows = problem.body().rows().iter().enumerate();
    rows.filter(|(_, row)| matches!(row.kind(), RowKind::Amount { .. }))
        .map(|(slot, _)| slot)
        .collect()
}

struct RefusesNearEmpty {
    inner: ReferenceIntegrator,
}
impl CoupledSolver for RefusesNearEmpty {
    fn resolution(&self) -> DeclaredResolution {
        resolution(b"refuses near empty")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let held: Vec<Compensated> = (amount_slots(problem).into_iter())
            .filter_map(|slot| problem.row(slot))
            .map(|row| problem.accepted().value(row))
            .collect();
        let tenfold = |amount: &Compensated| amount.times(&measure(10.0));
        let near_empty = held
            .iter()
            .any(|low| held.iter().any(|high| !high.at_most(tenfold(low))));
        if near_empty {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

struct LeakGuard {
    inner: ReferenceIntegrator,
}
impl CoupledSolver for LeakGuard {
    fn resolution(&self) -> DeclaredResolution {
        resolution(b"leak guard")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let workspace = problem.workspace();
        let head = problem.evaluate(&workspace.point(ticket.at(0)?))?;
        let total_rate: f64 = amount_slots(problem)
            .into_iter()
            .map(|slot| -head.residuals()[slot])
            .sum();
        if total_rate < 0.0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

#[derive(Clone, Copy)]
enum Stop {
    Rounding,
    RatioToFirst,
}

struct Newton {
    stop: Stop,
    remembers_refusal: bool,
    damping: Cell<f64>,
}
impl Newton {
    fn relative_stop() -> Self {
        Self {
            stop: Stop::RatioToFirst,
            remembers_refusal: false,
            damping: Cell::new(1.0),
        }
    }
    fn remembering_refusals() -> Self {
        Self {
            stop: Stop::Rounding,
            remembers_refusal: true,
            damping: Cell::new(1.0),
        }
    }
    fn refused<T>(&self, outcome: Result<T, TrialFailure>) -> Result<T, TrialFailure> {
        if outcome.is_err() && self.remembers_refusal {
            self.damping.set(0.5);
        }
        outcome
    }
    fn stage(
        &self,
        problem: &CoupledProblem<'_>,
        workspace: &mut SolveWorkspace,
        ticket: &StepTicket,
        offset_ns: u64,
        span_ns: u64,
    ) -> Result<StageEvaluation, TrialFailure> {
        let time = ticket.at(offset_ns)?;
        let shift = ticket.shift(span_ns)?;
        let per_second = NANOSECONDS_PER_SECOND / span_ns as f64;
        let start = workspace.unknowns_mut().to_vec();
        let mut first = None;
        let mut settled = false;
        for _ in 0..NEWTON_LIMIT {
            let unknowns = workspace.unknowns_mut().to_vec();
            for ((tangent, now), then) in workspace.derivative_mut().iter_mut().zip(&unknowns).zip(&start) {
                *tangent = (now - then) * per_second;
            }
            let point = workspace.point(time);
            let stage = self.refused(problem.evaluate(&point))?;
            let size =
                (stage.residuals().iter()).fold(
                    0.0,
                    |most, value| {
                        if value.abs() > most { value.abs() } else { most }
                    },
                );
            let first = *first.get_or_insert(size);
            let reduction = size / first;
            settled |= matches!(self.stop, Stop::RatioToFirst) && reduction <= 1e-3;
            if settled {
                return Ok(stage);
            }
            let jacobian = self.refused(problem.linearize(&point, shift))?;
            let factors = (problem.body().factor(&jacobian))
                .map_err(|_| ticket.refuse(NumericalRefusal::IterationLimit))?;
            let mut update = stage.residuals().to_vec();
            factors.solve(&mut update);
            let damping = self.damping.get();
            let moved = (update.iter().zip(&unknowns).enumerate())
                .map(|(slot, (change, unknown))| {
                    damping * change.abs() / (problem.row_size(slot) + unknown.abs())
                })
                .fold(0.0, |most, share| if share > most { share } else { most });
            settled = matches!(self.stop, Stop::Rounding) && moved <= NEWTON_STOP;
            for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
                *unknown -= damping * change;
            }
        }
        Err(ticket.refuse(NumericalRefusal::IterationLimit))
    }
}
impl CoupledSolver for Newton {
    fn resolution(&self) -> DeclaredResolution {
        match self.stop {
            Stop::Rounding => resolution(b"keeps its damping"),
            Stop::RatioToFirst => resolution(b"relative stop"),
        }
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        INTERVAL_NS
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns();
        let half = length / 2;
        let mut quadrature = problem.quadrature(ticket);
        let whole = self.stage(problem, &mut problem.workspace(), ticket, length, length)?;
        quadrature.add(&whole, Ratio::ZERO)?;
        let mut workspace = problem.workspace();
        let first = self.stage(problem, &mut workspace, ticket, half, half)?;
        quadrature.add(&first, ticket.fraction(half))?;
        let second = self.stage(problem, &mut workspace, ticket, length, length - half)?;
        quadrature.add(&second, ticket.fraction(length - half))?;
        let schema = problem.accepted().method_history().schema();
        let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

struct RefusesBeforeEvaluating {
    inner: ReferenceIntegrator,
}
impl CoupledSolver for RefusesBeforeEvaluating {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() == SimTime::from_ns(0) && ticket.attempt() == 0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

fn manufactured() -> Vec<AccuracyCase> {
    AccuracyCase::manufactured().expect("every case builds")
}

fn alone(case: AccuracyCase) -> AccuracyPolicy {
    AccuracyPolicy::new(vec![case], measure(5.0), measure(0.0), 100 * SECOND).expect("a policy")
}

fn verdict(core: &dyn CoupledSolver, case: AccuracyCase) -> &'static str {
    match accuracy(core, &alone(case)) {
        Ok(_) => "pass",
        Err(AccuracyFailure::Exceeded { .. }) => "exceeded",
        Err(AccuracyFailure::Faulted(fault)) if fault.kind() == EngineFaultKind::NoAdmissibleStep => {
            "no step"
        }
        Err(AccuracyFailure::Dependent { .. }) => "dependent",
        Err(AccuracyFailure::NothingRefused { .. }) => "nothing refused",
        Err(other) => panic!("{other:?}"),
    }
}

#[test]
fn the_reference_passes_every_manufactured_case_at_first_order() {
    let policy = AccuracyPolicy::manufactured().expect("the model default");
    let cases = manufactured();
    assert_eq!(cases.len(), 6);
    let combined =
        accuracy(&ReferenceIntegrator::new(500 * MILLISECOND), &policy).expect("within the policy");
    assert_eq!(combined.worst().len(), 6);
    let expected_tolerance = Compensated::of(&Oscillation::self_sustained().expect("a body").tolerance());
    assert_eq!(combined.amplitude_tolerance(), Some(expected_tolerance));

    let reservoirs = AccuracyPolicy::new(
        cases
            .into_iter()
            .filter(|case| !matches!(case, AccuracyCase::SelfSustainedOscillation))
            .collect(),
        measure(5.0),
        measure(0.0),
        100 * SECOND,
    )
    .expect("a policy");
    let reservoir_error =
        |interval| accuracy(&ReferenceIntegrator::new(interval), &reservoirs).expect("within the policy");
    let (coarse, medium, fine) = (
        reservoir_error(16 * SECOND),
        reservoir_error(8 * SECOND),
        reservoir_error(4 * SECOND),
    );
    observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1)).expect("first order");

    let oscillation = alone(AccuracyCase::SelfSustainedOscillation);
    let osc_error =
        |interval| accuracy(&ReferenceIntegrator::new(interval), &oscillation).expect("within the policy");
    let (coarse, medium, fine) = (
        osc_error(500 * MILLISECOND),
        osc_error(250 * MILLISECOND),
        osc_error(125 * MILLISECOND),
    );
    observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1)).expect("first order");
}

#[test]
fn each_case_rejects_the_core_built_to_fail_it_and_the_stepped_inflow_passes_them_all() {
    let cores: [(&str, &dyn CoupledSolver, [&str; 6]); 7] = [
        ("reference", &reference(), ["pass"; 6]),
        (
            "refuses near empty",
            &RefusesNearEmpty { inner: reference() },
            ["pass", "no step", "pass", "pass", "pass", "pass"],
        ),
        (
            "leak guard",
            &LeakGuard { inner: reference() },
            ["pass", "no step", "pass", "no step", "pass", "pass"],
        ),
        (
            "relative stop",
            &Newton::relative_stop(),
            ["pass", "pass", "no step", "pass", "pass", "pass"],
        ),
        (
            "keeps its damping",
            &Newton::remembering_refusals(),
            ["pass", "pass", "pass", "pass", "dependent", "pass"],
        ),
        (
            "reference past damping bound",
            &ReferenceIntegrator::new(6 * SECOND),
            ["pass", "pass", "pass", "pass", "pass", "exceeded"],
        ),
        (
            "halved step past damping bound",
            &Core::new(6 * SECOND),
            ["pass", "pass", "pass", "pass", "pass", "exceeded"],
        ),
    ];
    for (name, core, expected) in cores {
        let seen: Vec<&str> = manufactured()
            .into_iter()
            .map(|case| verdict(core, case))
            .collect();
        assert_eq!(seen, expected, "{name}");
    }
}

fn high(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

#[test]
fn an_honest_core_past_four_growth_over_omega_squared_closes_every_ledger_and_fails_only_the_oscillation_case()
 {
    let case = Oscillation::self_sustained().expect("a body");
    let cap = 6 * SECOND;
    let cores: [&dyn CoupledSolver; 2] = [&Core::new(cap), &ReferenceIntegrator::new(cap)];
    let others: Vec<AccuracyCase> = manufactured()
        .into_iter()
        .filter(|case| !matches!(case, AccuracyCase::SelfSustainedOscillation))
        .collect();
    let without_oscillation =
        AccuracyPolicy::new(others, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy");
    let full = AccuracyPolicy::manufactured().expect("the model default");

    for core in cores {
        let store = MemoryStore::default();
        let mut session = case.admit(core, &store).expect("admitted");
        let mut refusals = 0_u64;
        while session.record().accepted().time() < case.end() {
            let mut step = session.step();
            loop {
                match step {
                    Step::Accepted(next, _) => {
                        session = next;
                        break;
                    }
                    Step::Refused(retry) => {
                        refusals += 1;
                        step = retry.retry();
                    }
                    Step::Faulted(stopped) => panic!("unexpected fault: {:?}", stopped.fault()),
                }
            }
        }
        assert_eq!(refusals, 0, "every step passes verify without refusal");

        let view = session.conservation();
        let basis = view.basis();
        let quantities = basis
            .ids()
            .map(|quantity| {
                QuantityPolicy::new(quantity, measure::<Water>(1e-24), measure(1e-9), measure(1e-9))
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("water ledger policy");
        let window = NonZeroU64::new(case.end().nanoseconds()).expect("a positive end");
        let windows = SlidingWindows::new(window, case.end().nanoseconds()).expect("one full window");
        let certificate_policy =
            CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows).expect("a policy");
        let certificate = certify(&view, &certificate_policy).expect("every ledger closes");
        assert_eq!(certificate.results().len(), 1);
        assert_eq!(certificate.results()[0].worst_closure(), Compensated::ZERO);

        accuracy(core, &without_oscillation).expect("every other manufactured case passes");
        let failure = accuracy(core, &full);
        assert!(
            matches!(failure, Err(AccuracyFailure::Exceeded { case: 5, .. })),
            "refused by the oscillation case alone: {failure:?}"
        );

        let [x_row, y_row] = case.rows();
        let accepted = session.record().accepted();
        let (x, y) = (high(accepted.value(x_row)), high(accepted.value(y_row)));
        let radius = (x * x + y * y).sqrt();
        let exact = high(Compensated::of(&case.exact_amplitude(accepted.time())));
        assert!(exact > 0.999, "exact amplitude reaches 1: {exact}");
        assert!(radius < 1e-3, "oscillation decayed under damping: {radius}");
    }
}

#[test]
fn oscillation_tolerance_and_reference_step_bound_the_damping() {
    let case = Oscillation::self_sustained().expect("a body");
    assert_eq!(high(Compensated::of(&case.period())), 60.0);
    assert_eq!(high(Compensated::of(&case.growth())), 0.01);
    assert_eq!(high(Compensated::of(&case.amplitude())), 1.0);
    assert_eq!(
        high(Compensated::of(&case.exact_amplitude(SimTime::from_ns(0)))),
        0.5
    );
    assert_eq!(case.reference_step_ns(), 500 * MILLISECOND);
    assert!(case.damping_step_ns() > 3_600 * MILLISECOND && case.damping_step_ns() < 3_700 * MILLISECOND);
    assert!(6 * SECOND > case.damping_step_ns());

    let policy = alone(AccuracyCase::SelfSustainedOscillation);
    let tolerance = Compensated::of(&case.tolerance());
    for passing in [
        &ReferenceIntegrator::new(case.reference_step_ns()) as &dyn CoupledSolver,
        &Core::new(case.reference_step_ns()),
    ] {
        let evidence = accuracy(passing, &policy).expect("passes at the stated reference step");
        assert!(
            evidence.worst()[0].at_most(tolerance),
            "worst amplitude error {:?}",
            evidence.worst()[0]
        );
    }
    let damped = accuracy(&Core::new(SECOND), &policy);
    assert!(
        matches!(damped, Err(AccuracyFailure::Exceeded { case: 0, .. })),
        "1 s damps the steady amplitude by 0.148, past 0.1: {damped:?}"
    );
}

struct OvershootsEnd {
    inner: ReferenceIntegrator,
    switch_at: SimTime,
    tail_ns: u64,
}
impl CoupledSolver for OvershootsEnd {
    fn resolution(&self) -> DeclaredResolution {
        resolution(b"overshoots end")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        if accepted.time() >= self.switch_at {
            self.tail_ns
        } else {
            self.inner.next_step(accepted)
        }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn oscillation_scoring_clips_at_case_end() {
    let core = OvershootsEnd {
        inner: ReferenceIntegrator::new(500 * MILLISECOND),
        switch_at: SimTime::from_ns(1_199 * SECOND + 500 * MILLISECOND),
        tail_ns: 6 * SECOND,
    };
    accuracy(&core, &alone(AccuracyCase::SelfSustainedOscillation))
        .expect("amplitude at 1200 s is within tolerance even when the final step ends past it");
}

#[test]
fn a_core_that_remembers_a_refusal_still_meets_the_closed_form() {
    let core = Newton::remembering_refusals();
    let closed = TwoReservoirs::closed()
        .expect("a body")
        .until(SimTime::from_ns(2_000 * SECOND));
    accuracy(&core, &alone(AccuracyCase::TwoReservoir(Box::new(closed)))).expect("within the policy");
    let failure = accuracy(&core, &alone(AccuracyCase::EvaluationIndependence));
    assert!(
        matches!(failure, Err(AccuracyFailure::Dependent { case: 0 })),
        "{failure:?}"
    );
}

#[test]
fn a_domain_refusal_at_the_floor_still_counts_as_refused() {
    let floor = NonZeroU64::new(INTERVAL_NS).expect("a floor");
    let core = reference().with_floor(floor);
    assert_eq!(verdict(&core, AccuracyCase::EvaluationIndependence), "pass");
}

#[test]
fn an_independence_case_whose_core_evaluated_nothing_tested_nothing() {
    let core = RefusesBeforeEvaluating { inner: reference() };
    let closed = TwoReservoirs::closed()
        .expect("a body")
        .until(SimTime::from_ns(2_000 * SECOND));
    accuracy(&core, &alone(AccuracyCase::TwoReservoir(Box::new(closed)))).expect("within the policy");
    assert_eq!(
        verdict(&core, AccuracyCase::EvaluationIndependence),
        "nothing refused"
    );
}

#[test]
fn each_case_has_its_own_identity_in_the_policy() {
    let digests: Vec<Digest> = (manufactured().into_iter())
        .map(|case| alone(case).digest())
        .collect();
    for (index, digest) in digests.iter().enumerate() {
        assert!(!digests[..index].contains(digest), "case {index}");
    }
    let closed = || {
        TwoReservoirs::closed()
            .expect("a body")
            .until(SimTime::from_ns(2_000 * SECOND))
    };
    let plain = alone(AccuracyCase::TwoReservoir(Box::new(closed()))).digest();
    assert_ne!(
        plain,
        alone(AccuracyCase::EvaluationIndependence).digest(),
        "the identity it checks"
    );
}
