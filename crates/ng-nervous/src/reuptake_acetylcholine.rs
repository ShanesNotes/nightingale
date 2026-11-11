use crate::shared::reuptake;
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    reflex: RateClass,
) {
    reuptake::<Acetylcholine>(ns, ports, "ns.reuptake-ach", reflex);
}
