use crate::shared::traffic;
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    filtered: &StateSlot<'b, Nervous, Dimensionless>,
    set_point: &StateSlot<'b, Nervous, Reference<Dimensionless>>,
    reflex: RateClass,
) -> AlgebraicSlot<'b, Nervous, Dimensionless> {
    traffic(
        ns,
        filtered,
        set_point,
        (
            "sympathetic traffic",
            "ns.sympathetic-width",
            "ns.sympathetic-rest",
            true,
        ),
        reflex,
    )
}
