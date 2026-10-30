use super::{fresh_history, resolution};
use ng_contract::*;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// Evaluates at the workspace's point with the tangent at the rates there: an explicit stage.
pub fn explicit_stage(
    problem: &CoupledProblem<'_>,
    workspace: &mut SolveWorkspace,
    time: StageTime<'_>,
) -> Result<StageEvaluation, TrialFailure> {
    workspace.derivative_mut().fill(0.0);
    let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
        .map(|residual| -residual)
        .collect();
    workspace.derivative_mut().copy_from_slice(&rates);
    problem.evaluate(&workspace.point(time))
}

/// Forward Euler's one stage, with the workspace still at the start and its tangent at the rates there.
pub fn euler_step(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
) -> Result<(SolveWorkspace, StageEvaluation), TrialFailure> {
    let mut workspace = problem.workspace();
    let stage = explicit_stage(problem, &mut workspace, ticket.at(0)?)?;
    Ok((workspace, stage))
}

pub fn advance(workspace: &mut SolveWorkspace, ticket: &StepTicket) {
    let seconds = ticket.length_ns() as f64 / NANOSECONDS_PER_SECOND;
    let tangent = workspace.derivative_mut().to_vec();
    for (unknown, slope) in workspace.unknowns_mut().iter_mut().zip(tangent) {
        *unknown += seconds * slope;
    }
}

/// An honest explicit method that follows its declared scheme, so `validate` passes it and every ledger
/// closes even while the difference between the reservoirs grows.
pub struct ForwardEuler {
    pub interval_ns: u64,
}
impl CoupledSolver for ForwardEuler {
    fn resolution(&self) -> DeclaredResolution {
        resolution(StageScheme::forward_euler(), b"forward euler", 12)
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// An honest Heun core: the start and the Euler-predicted end, a half each. The legal twin of every
/// must-fail core that declares this scheme.
pub struct Heun {
    pub interval_ns: u64,
}
impl CoupledSolver for Heun {
    fn resolution(&self) -> DeclaredResolution {
        resolution(StageScheme::heun(), b"heun", 12)
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (first, second, workspace) = heun_stages(problem, ticket)?;
        let half = Ratio::new(1, 2).expect("a fraction");
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&first, half)?;
        quadrature.add(&second, half)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Both Heun stages, with the workspace at the second.
pub fn heun_stages(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
) -> Result<(StageEvaluation, StageEvaluation, SolveWorkspace), TrialFailure> {
    let (mut workspace, first) = euler_step(problem, ticket)?;
    advance(&mut workspace, ticket);
    let second = explicit_stage(problem, &mut workspace, ticket.at(ticket.length_ns())?)?;
    Ok((first, second, workspace))
}
