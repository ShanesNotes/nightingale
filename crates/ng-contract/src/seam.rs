use crate::{AcceptedStateView, CoupledProblem, DeclaredResolution, StepProposal, StepTicket, TrialFailure};

/// An implementor holds algorithm configuration, never accepted continuation.
pub trait CoupledSolver {
    fn resolution(&self) -> DeclaredResolution;
    /// The span this core commits to for its next step, in ns, from the accepted view alone. It takes
    /// no problem and returns no ticket, so a core has nothing to ask before it commits.
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64;
    /// Solves exactly the ticketed interval. A proposal is owned, and it is verified against the record's
    /// own problem, so nothing the core built it from is trusted.
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure>;
}
