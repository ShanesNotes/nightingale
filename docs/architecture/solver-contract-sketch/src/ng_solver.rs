//! Proposed ng-solver: two arena implementations, one eventual winner. Neither imports an organ.
//! Each implements every seam method itself; there are no default trait bodies to inherit.
use crate::ng_contract::*;
/// Solver-A's Newton core, replacing hand-coded model arrays with compiled evaluation.
pub struct CompiledNewton {
    _policy: Digest,
}
impl CoupledSolver for CompiledNewton {
    fn resolution(&self) -> DeclaredResolution {
        todo!()
    }
    fn next_step(&self, _problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        todo!()
    }
    fn solve(&self, _problem: &CoupledProblem<'_>, _ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        todo!()
    }
}
impl Initializer for CompiledNewton {
    fn initialize(&self, _system: InitializationSystem<'_>) -> Result<InitialRevision, InitializationFailure> {
        todo!()
    }
}
/// Solver-B's beat core; partitions and exchange columns come from compiled blocks.
/// No beat reuse at full resolution, no stored Slice object, no Organ::evolve callback.
pub struct BeatStepper {
    _policy: Digest,
    _qualification: SplitQualification,
}
impl CoupledSolver for BeatStepper {
    fn resolution(&self) -> DeclaredResolution {
        todo!()
    }
    fn next_step(&self, _problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        todo!()
    }
    fn solve(&self, _problem: &CoupledProblem<'_>, _ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        todo!()
    }
}
impl Initializer for BeatStepper {
    fn initialize(&self, _system: InitializationSystem<'_>) -> Result<InitialRevision, InitializationFailure> {
        todo!()
    }
}
