use super::fresh_history;
use ng_contract::*;

/// Who refuses the forced attempt.
#[derive(Clone, Copy)]
pub enum Refuser {
    /// The core books its stages unsolved, so the record refuses, as with a real unconverged proposal.
    Record,
    /// The core refuses it itself.
    Core,
}

/// Passes everything through except the first attempt at one instant. It keeps no state:
/// `ticket.attempt()` says when to stop.
pub struct ForcedRefusal<'c> {
    pub inner: &'c dyn CoupledSolver,
    pub at: SimTime,
    pub by: Refuser,
}
impl CoupledSolver for ForcedRefusal<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() != self.at || ticket.attempt() > 0 {
            return self.inner.solve(problem, ticket);
        }
        if matches!(self.by, Refuser::Core) {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        let workspace = problem.workspace();
        let length = ticket.length_ns();
        let half = length / 2;
        let mut quadrature = problem.quadrature(ticket);
        let unsolved = |offset| problem.evaluate(&workspace.point(ticket.at(offset)?));
        quadrature.add(&unsolved(length)?, Ratio::ZERO)?;
        quadrature.add(&unsolved(half)?, ticket.fraction(half))?;
        quadrature.add(&unsolved(length)?, ticket.fraction(length - half))?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}
