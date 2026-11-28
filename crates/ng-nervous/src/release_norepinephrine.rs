use crate::shared::{release, release_in_shares};
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    sympathetic: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    reflex: RateClass,
) {
    release::<Norepinephrine>(ns, ports, sympathetic, "ns.release-ne", reflex);
}

pub(crate) fn declare_with_the_constrictor<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    (sympathetic, constrictor): (
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
    ),
    reflex: RateClass,
) {
    let traffics = [
        (sympathetic, "ns.sympathetic-share"),
        (constrictor, "ns.constrictor-share"),
    ];
    release_in_shares::<Norepinephrine>(ns, ports, traffics, "ns.release-ne", reflex);
}
