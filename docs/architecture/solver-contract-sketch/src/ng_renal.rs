//! Proposed ng-renal. Returns a value; no record, solver, dt, or persistent organ instance.
use crate::ng_contract::*;

/// The renal parameter slots. Values arrive later from the bound packet, by key.
pub struct RenalParameters<'b> {
    pub filtration_coefficient: Parameter<'b, Renal, Conductance<Water>>,
    pub feedback_gain: Parameter<'b, Renal, Dimensionless>,
}
impl<'b> RenalParameters<'b> {
    pub fn declare(_laws: &mut FragmentBuilder<'b, Renal>) -> Self {
        todo!()
    }
}
/// Declares renal bed mechanics, filtration and reclamation, urine and TGF (emits line 97,
/// reads held line 102).
/// Material stays in body pools; private state is only nonmaterial controller memory.
pub fn declare<'b>(_scope: RenalScope<'b>) -> Fragment<'b, Renal> {
    todo!()
}
/// Builds the filtration kernel inside the kernel scope the caller opened: a closed graph over
/// glomerular, tubular and oncotic pressure and Kf, not a live callback.
pub fn filtration_law<'b, 'k>(
    _graph: Kernel<'b, 'k, Renal>,
    _inputs: FiltrationInputs<'b>,
    _parameters: &RenalParameters<'b>,
) -> Finished<'b, 'k, Renal, Rate<Water>> {
    todo!()
}
