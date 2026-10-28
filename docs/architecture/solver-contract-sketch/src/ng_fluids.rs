//! Proposed ng-fluids: per-region Starling exchange, lymph, osmotic distribution and vessel wall.
use crate::ng_contract::*;
/// Dependent and nondependent interstitia are distinct; no global edema or duplicate I/O book.
/// Reads capillary and venous pressure (line 80) and permeability (line 106, held).
pub fn declare<'b>(_scope: FluidsScope<'b>) -> Fragment<'b, Fluids> {
    todo!()
}
