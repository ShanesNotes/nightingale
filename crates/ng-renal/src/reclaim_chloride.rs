use crate::shared::Saturable;
use ng_contract::*;

pub(crate) fn declare<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    sodium: &EdgeRoute<'b, Renal, Sodium, ReclaimEdge>,
    potassium: &EdgeRoute<'b, Renal, Potassium, ReclaimEdge>,
    transport: &Saturable<'b>,
    class: RateClass,
) {
    let chloride = ports.reclaim::<Chloride>(rn);
    // Chloride follows the cations, one for one, so reclaiming moves no charge. Model default.
    let back = rn.edge_kernel(|mut k| {
        let na = transport.crossing(&mut k, sodium, sodium);
        let beside = transport.crossing(&mut k, sodium, potassium);
        let (na, beside): (Expr<'_, '_, _, Rate<Chloride>>, Expr<'_, '_, _, Rate<Chloride>>) =
            (k.as_dimension(na), k.as_dimension(beside));
        let rate = k.add(na, beside);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&chloride, back);
}
