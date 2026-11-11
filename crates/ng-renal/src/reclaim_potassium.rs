use crate::shared::Saturable;
use ng_contract::*;

pub(crate) fn declare<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    sodium: &EdgeRoute<'b, Renal, Sodium, ReclaimEdge>,
    potassium: &EdgeRoute<'b, Renal, Potassium, ReclaimEdge>,
    transport: &Saturable<'b>,
    class: RateClass,
) {
    let back = rn.edge_kernel(|mut k| {
        let rate = transport.crossing(&mut k, sodium, potassium);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(potassium, back);
}
