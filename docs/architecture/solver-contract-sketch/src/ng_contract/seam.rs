//! The numerical seam: contract-B's `CoupledSolver` (and `Initializer`, in admission), revised to
//! bind a ticket and return typed failure. Every method is required; there are no default
//! bodies. Both arena cores implement these traits directly, importing only this contract.
use super::*;

/// An immutable implementor holding algorithm configuration, never accepted continuation. Rust
/// cannot forbid interior mutability here; a core that keeps a counter or cache outside its
/// `SolverContinuation` fails the arena's resume gate, which decodes into a freshly built core.
pub trait CoupledSolver: Send + Sync {
    /// The declared resolution this core runs under; part of the run identity.
    fn resolution(&self) -> DeclaredResolution;
    /// Chooses an intrinsic interval from accepted continuation alone and asks
    /// `problem.ticket(start, intrinsic_end)`; the contract clips it at the next breakpoint. A core
    /// never sees the timeline, so its step cannot depend on pace or on a later input.
    fn next_step(&self, problem: &CoupledProblem<'_>, start: AcceptedStateView<'_>) -> Result<StepTicket, EngineFault>;
    /// Solves exactly the ticketed interval or returns a refusal or fault.
    fn solve<'p>(
        &self,
        problem: &'p CoupledProblem<'_>,
        start: AcceptedStateView<'_>,
        ticket: &StepTicket,
    ) -> Result<StepProposal<'p>, TrialFailure>;
}

/// Contract-A's explicit bench: only for bodies without residual rows. No production core is
/// wrapped behind it and no DAE is relabelled explicit.
pub struct ExplicitBench<'p> {
    _problem: &'p CoupledProblem<'p>,
}
pub trait Rhs {
    fn rates(&self, point: &StagePoint<'_>) -> Result<StageEvaluation, TrialFailure>;
}
impl Rhs for ExplicitBench<'_> {
    fn rates(&self, _point: &StagePoint<'_>) -> Result<StageEvaluation, TrialFailure> {
        todo!()
    }
}
pub trait Integrator {
    /// Advances the workspace over the ticket and books its stages into the quadrature.
    fn substep(
        &self,
        rhs: &dyn Rhs,
        workspace: &mut SolveWorkspace,
        ticket: &StepTicket,
        quadrature: &mut Quadrature<'_>,
    ) -> Result<(), TrialFailure>;
}
/// Two stages, weights one half each.
pub struct Heun;
impl Integrator for Heun {
    fn substep(
        &self,
        _rhs: &dyn Rhs,
        _workspace: &mut SolveWorkspace,
        _ticket: &StepTicket,
        _quadrature: &mut Quadrature<'_>,
    ) -> Result<(), TrialFailure> {
        todo!()
    }
}
