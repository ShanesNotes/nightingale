use crate::shared::release;
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    vagal: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    reflex: RateClass,
) {
    release::<Acetylcholine>(ns, ports, vagal, "ns.release-ach", reflex);
}
