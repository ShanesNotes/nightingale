use crate::shared::{release, release_by_shares};
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    vagal: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    reflex: RateClass,
) {
    release::<Acetylcholine>(ns, ports, vagal, "ns.release-ach", reflex);
}

pub(crate) fn declare_with_the_pump<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    traffics: (&AlgebraicSlot<'b, Nervous, Dimensionless>, &StateSlot<'b, Nervous, Dimensionless>),
    reflex: RateClass,
) {
    release_by_shares::<Acetylcholine>(ns, ports, traffics, "ns.release-ach", reflex);
}
