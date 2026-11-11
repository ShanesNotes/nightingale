use crate::shared::release;
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    sympathetic: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    reflex: RateClass,
) {
    release::<Norepinephrine>(ns, ports, sympathetic, "ns.release-ne", reflex);
}
