use crate::run::accept;
use crate::{MemoryStore, ReferenceIntegrator, ReplayCase};
use ng_contract::{
    CoupledSolver, EngineFault, InitializationFailure, ObservationLens, ReferenceError, SplitBudget,
    SplitMeasurement, SplitQualification, SplitRejected, SplitTrace, TraceDraw, TraceGrid,
};

#[derive(Debug)]
pub enum SplitFailure {
    Admission(InitializationFailure),
    Faulted(EngineFault),
    Rejected(SplitRejected),
}
impl From<SplitRejected> for SplitFailure {
    fn from(rejected: SplitRejected) -> Self {
        Self::Rejected(rejected)
    }
}

/// The split gate (contract §7). The reference's own error comes from the gate's run of it at half its interval,
/// and the gate returns the qualification and nothing else, so no run it measured is ever scored.
pub fn split_gate(
    case: &dyn ReplayCase,
    split: &dyn CoupledSolver,
    reference: &ReferenceIntegrator,
    lens: &ObservationLens,
    grid: TraceGrid,
    budget: SplitBudget,
) -> Result<SplitQualification, SplitFailure> {
    let expected = draw_split_trace(case, reference, lens, grid)?;
    let refined = draw_split_trace(case, &reference.refined(), lens, grid)?;
    let error = ReferenceError::self_convergence(&expected, &refined)?;
    let measured = draw_split_trace(case, split, lens, grid)?;
    Ok(SplitMeasurement::new(expected, measured, error, budget)?.qualify()?)
}

pub fn draw_split_trace(
    case: &dyn ReplayCase,
    core: &dyn CoupledSolver,
    lens: &ObservationLens,
    grid: TraceGrid,
) -> Result<SplitTrace, SplitFailure> {
    let store = MemoryStore::default();
    let mut session = case.admit(core, &store).map_err(SplitFailure::Admission)?;
    let mut draw = TraceDraw::new(case.body(), lens, grid)?;
    session.record().split_trace(&mut draw, None)?;
    while !draw.done() {
        let (next, receipt) = accept(session).map_err(|stopped| SplitFailure::Faulted(*stopped.fault()))?;
        next.record().split_trace(&mut draw, Some(&receipt))?;
        session = next;
    }
    Ok(draw.finish()?)
}
