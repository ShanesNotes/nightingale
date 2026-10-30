//! A core sees the compiled body only through `CoupledProblem`, so it never sees the timeline, a pending
//! input or a stream position.
use crate::{CoupledProblem, DeclaredResolution, EngineFault, StepProposal, StepTicket, TrialFailure};

/// An implementor holds algorithm configuration, never accepted continuation.
pub trait CoupledSolver {
    fn resolution(&self) -> DeclaredResolution;
    /// Chooses an intrinsic span from accepted continuation alone and asks `problem.ticket(span)`; the
    /// contract clips it at the next breakpoint, which no core can see.
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault>;
    /// Solves exactly the ticketed interval. A proposal is owned, and it is verified against the record's
    /// own problem, so nothing the core built it from is trusted.
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure>;
}
