//! The numerical seam: contract-B's `CoupledSolver`, revised to bind a ticket and return typed
//! failure. Every method is required; there are no default
//! bodies. Both arena cores implement these traits directly, importing only this contract.
use super::*;

/// An immutable implementor holding algorithm configuration, never accepted continuation. Rust
/// cannot forbid interior mutability here; a core that keeps a counter or cache outside its
/// `MethodHistory` fails the arena's resume gate, which decodes into a freshly built core. It
/// carries no `Send + Sync` bound: nothing here threads, and the resume gate's must-fail core
/// needs a `Cell`, because no thread-safe cell can be named in the workspace.
pub trait CoupledSolver {
    /// The declared resolution this core runs under; part of the run identity.
    fn resolution(&self) -> DeclaredResolution;
    /// The span this core commits to for its next step, in ns, from the accepted view alone, so its step
    /// cannot depend on pace or on a later input.
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64;
    /// Solves exactly the ticketed interval or returns a refusal or fault. A proposal is owned and
    /// verified against the record's own problem, so nothing the core built it from is trusted.
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure>;
}

/// Contract-A's explicit bench: only for bodies without residual rows. No production core is
/// wrapped behind it and no DAE is relabelled explicit.
pub struct ExplicitBench<'p> {
    _problem: &'p CoupledProblem<'p>,
}
pub trait Rhs {
    fn rates(&self, point: &StagePoint<'_, '_>) -> Result<StageEvaluation, TrialFailure>;
}
impl Rhs for ExplicitBench<'_> {
    fn rates(&self, _point: &StagePoint<'_, '_>) -> Result<StageEvaluation, TrialFailure> {
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
        quadrature: &mut Quadrature,
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
        _quadrature: &mut Quadrature,
    ) -> Result<(), TrialFailure> {
        todo!()
    }
}
