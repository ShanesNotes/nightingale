use core::cell::Cell;
use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const INTERVAL_NS: u64 = 16 * SECOND;
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

/// Refuses any head where one amount holds under a tenth of another: a reference range kept in a core.
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

/// Takes a falling total for a leak and refuses the step.
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
        // At a zero tangent each amount's residual is minus its rate.
        let falling: f64 = amount_slots(problem)
            .into_iter()
            .map(|slot| head.residuals()[slot])
            .sum();
        if falling > 0.0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

#[derive(Clone, Copy)]
enum Stop {
    /// Settles when an update moves each unknown by no more than rounding, as the reference does.
    Rounding,
    /// Settles when the residual falls a thousandfold from the stage's first, which is 0 / 0 at rest.
    Relative,
}

/// Doubled backward Euler with its own Newton. With the reference's stop it remembers a refused evaluation.
struct Newton {
    stop: Stop,
    /// One until an evaluation is refused, and a half after.
    damping: Cell<f64>,
}
impl Newton {
    fn new(stop: Stop) -> Self {
        Self {
            stop,
            damping: Cell::new(1.0),
        }
    }
    fn refused<T>(&self, outcome: Result<T, TrialFailure>) -> Result<T, TrialFailure> {
        if outcome.is_err() && matches!(self.stop, Stop::Rounding) {
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
            settled |= matches!(self.stop, Stop::Relative) && size / first <= 1e-3;
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
            Stop::Relative => resolution(b"relative stop"),
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

/// Refuses its first attempt at admission without evaluating anything.
struct RefusesUnseen {
    inner: ReferenceIntegrator,
}
impl CoupledSolver for RefusesUnseen {
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

/// Which failure a case gave, by name, so one table holds the whole matrix.
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
    assert_eq!(manufactured().len(), 5);
    let error = |interval| accuracy(&ReferenceIntegrator::new(interval), &policy).expect("within the policy");
    let (coarse, medium, fine) = (error(16 * SECOND), error(8 * SECOND), error(4 * SECOND));
    assert_eq!(coarse.worst().len(), 5);
    observed_order([&coarse, &medium, &fine], measure(1.9), measure(2.1)).expect("first order");
}

#[test]
fn each_case_rejects_the_core_built_to_fail_it_and_the_stepped_inflow_passes_them_all() {
    let cores: [(&str, &dyn CoupledSolver, [&str; 5]); 5] = [
        ("reference", &reference(), ["pass"; 5]),
        (
            "refuses near empty",
            &RefusesNearEmpty { inner: reference() },
            ["pass", "no step", "pass", "pass", "pass"],
        ),
        (
            "leak guard",
            &LeakGuard { inner: reference() },
            ["pass", "no step", "pass", "no step", "pass"],
        ),
        (
            "relative stop",
            &Newton::new(Stop::Relative),
            ["pass", "pass", "no step", "pass", "pass"],
        ),
        (
            "keeps its damping",
            &Newton::new(Stop::Rounding),
            ["pass", "pass", "pass", "pass", "dependent"],
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

#[test]
fn a_core_that_remembers_a_refusal_still_meets_the_closed_form() {
    let core = Newton::new(Stop::Rounding);
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
fn an_independence_case_whose_core_evaluated_nothing_tested_nothing() {
    let core = RefusesUnseen { inner: reference() };
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
