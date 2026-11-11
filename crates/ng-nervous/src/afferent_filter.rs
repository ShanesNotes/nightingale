use ng_contract::catalog::{BaroreceptorAfferents, Reads};
use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    filtered: &StateSlot<'b, Nervous, Dimensionless>,
    reflex: RateClass,
) {
    let afferents: BaroreceptorAfferents<Reads<'_, Nervous>> = ns.receive();
    let pace =
        ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.filter-speed"), Admissible::positive());
    let carotid =
        ns.parameter::<Dimensionless>(ParameterKey::new("ns.carotid-share"), Admissible::unit_interval());
    let arch = ns.parameter::<Dimensionless>(ParameterKey::new("ns.arch-share"), Admissible::unit_interval());

    let relax = ns.kernel(|mut k| {
        let (first, second) = (
            k.read(&afferents.carotid_sinus_stretch),
            k.read(&afferents.aortic_arch_stretch),
        );
        let (one, two) = (k.parameter(&carotid), k.parameter(&arch));
        let (first, second) = (k.scale(first, one), k.scale(second, two));
        let target = k.add(first, second);
        let (current, speed) = (k.state(filtered), k.parameter(&pace));
        let rate = k.relax(target, current, speed);
        k.finish(rate, ns.every_mode(), reflex)
    });
    ns.memory_rate(filtered, relax);
}
