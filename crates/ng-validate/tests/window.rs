use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const NANOSECONDS_PER_SECOND: f64 = 1e9;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn number(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn loose() -> ErrorScales {
    ErrorScales::uniform(measure(1e9), measure(1.0)).expect("positive")
}

fn fixed(interval_ns: u64, floor_ns: u64) -> ReferenceIntegrator {
    let floor = NonZeroU64::new(floor_ns).expect("a floor");
    ReferenceIntegrator::new(interval_ns)
        .with_scales(loose())
        .with_floor(floor)
}

/// Doubled backward Euler that stops each stage after its first Newton update, and declares scales loose enough
/// for the record to accept what that leaves: §7's core that accepts after its sweep limit with its change still
/// above tolerance. On a linear law one update is exact, so every manufactured case passes it.
struct OneUpdate {
    interval_ns: u64,
}
impl OneUpdate {
    fn stage(
        problem: &CoupledProblem<'_>,
        workspace: &mut SolveWorkspace,
        ticket: &StepTicket,
        offset_ns: u64,
        span_ns: u64,
    ) -> Result<StageEvaluation, TrialFailure> {
        let time = ticket.at(offset_ns)?;
        let per_second = NANOSECONDS_PER_SECOND / span_ns as f64;
        let start = workspace.unknowns_mut().to_vec();
        workspace.derivative_mut().fill(0.0);
        let point = workspace.point(time);
        let stage = problem.evaluate(&point)?;
        let jacobian = problem.linearize(&point, ticket.shift(span_ns)?)?;
        let factors = (problem.body().factor(&jacobian))
            .map_err(|_| ticket.refuse(NumericalRefusal::IterationLimit))?;
        let mut update = stage.residuals().to_vec();
        factors.solve(&mut update);
        for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
            *unknown -= change;
        }
        let unknowns = workspace.unknowns_mut().to_vec();
        for ((tangent, now), then) in workspace.derivative_mut().iter_mut().zip(&unknowns).zip(&start) {
            *tangent = (now - then) * per_second;
        }
        problem.evaluate(&workspace.point(time))
    }
}
impl CoupledSolver for OneUpdate {
    fn resolution(&self) -> DeclaredResolution {
        let mut policy = b"one update".to_vec();
        policy.extend_from_slice(&self.interval_ns.to_be_bytes());
        DeclaredResolution::new(
            Digest::of_canonical(&policy),
            NonZeroU64::new(100_000).expect("a floor"),
            12,
            ArithmeticProfile::contract_v1(),
            StageScheme::backward_euler_doubled(),
            loose(),
        )
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns();
        let half = length / 2;
        let mut quadrature = problem.quadrature(ticket);
        let whole = Self::stage(problem, &mut problem.workspace(), ticket, length, length)?;
        quadrature.add(&whole, Ratio::ZERO)?;
        let mut workspace = problem.workspace();
        let first = Self::stage(problem, &mut workspace, ticket, half, half)?;
        quadrature.add(&first, ticket.fraction(half))?;
        let second = Self::stage(problem, &mut workspace, ticket, length, length - half)?;
        quadrature.add(&second, ticket.fraction(length - half))?;
        let schema = problem.accepted().method_history().schema();
        let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// One window serves every core here: the reference's three runs of it cost more than everything else.
#[test]
fn the_first_window_holds_each_core_to_the_references_own_distances() {
    let window = SliceWindow::first().expect("the reference runs the window at each refinement");
    let cases = vec![AccuracyCase::SliceWindow(Box::new(window))];
    let policy = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy");

    let arena_scales = ErrorScales::uniform(measure(1.0), measure(1e-3)).expect("positive");
    let adapting = ReferenceIntegrator::new(5 * MILLISECOND)
        .with_scales(arena_scales)
        .with_floor(NonZeroU64::new(100_000).expect("a floor"));
    for honest in [fixed(5 * MILLISECOND, 100_000), adapting] {
        let evidence = accuracy(&honest, &policy).expect("within every distance");
        let share = number(evidence.worst()[0]);
        assert!(share > 0.0 && share < 1.0, "{share}");
    }

    let to_a_millisecond = accuracy(&fixed(5 * MILLISECOND, MILLISECOND), &policy);
    assert!(
        matches!(to_a_millisecond, Err(AccuracyFailure::Outside { case: 0, .. })),
        "{to_a_millisecond:?}"
    );

    let manufactured = AccuracyPolicy::manufactured().expect("the model default");
    let sixteen = OneUpdate {
        interval_ns: 16 * SECOND,
    };
    accuracy(&sixteen, &manufactured).expect("one update solves a linear law");
    let one_update = OneUpdate {
        interval_ns: 10 * MILLISECOND,
    };
    let failure = accuracy(&one_update, &policy);
    assert!(
        matches!(failure, Err(AccuracyFailure::Outside { case: 0, .. })),
        "{failure:?}"
    );

    let to_its_step = accuracy(&fixed(10 * MILLISECOND, 10 * MILLISECOND), &policy);
    assert!(
        matches!(to_its_step, Err(AccuracyFailure::ModesApart { case: 0, .. })),
        "{to_its_step:?}"
    );
}
