use crate::shared::reuptake;
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    reflex: RateClass,
) {
    reuptake::<Norepinephrine>(ns, ports, "ns.reuptake-ne", reflex);
}
