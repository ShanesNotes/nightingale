//! Proposed ng-receptor: shared ligand binding and transduction declarations.
use crate::ng_contract::*;
/// Beta-1 (sinoatrial node, myocardium), alpha-1 (arterioles, veins, renal arterioles), M2
/// (sinoatrial node) and V2 (collecting duct) as balanced binding reactions. A reflex transmitter
/// and a future drug compete at the same receptor. Outputs are occupancy and transduction that
/// target organ laws consume, never a vital setter.
pub fn declare<'b>(_scope: ReceptorScope<'b>) -> Fragment<'b, Receptor> {
    todo!()
}
