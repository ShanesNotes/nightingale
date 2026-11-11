use crate::shared::Saturable;
use ng_contract::*;

pub(crate) fn declare<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    sodium: &EdgeRoute<'b, Renal, Sodium, ReclaimEdge>,
    transport: &Saturable<'b>,
    class: RateClass,
) {
    let back = rn.edge_kernel(|mut k| {
        let rate = transport.crossing(&mut k, sodium, sodium);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(sodium, back);
}
