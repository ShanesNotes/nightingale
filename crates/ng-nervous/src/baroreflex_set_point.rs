use ng_contract::*;

pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    filtered: &StateSlot<'b, Nervous, Dimensionless>,
    set_point: &StateSlot<'b, Nervous, Reference<Dimensionless>>,
    slow: RateClass,
) {
    let drift =
        ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.set-point-speed"), Admissible::positive());
    let following = ns.kernel(|mut k| {
        let (measured, reference) = (k.state(filtered), k.state(set_point));
        let off = k.deviation(measured, reference);
        let speed = k.parameter(&drift);
        let product = k.multiply(off, speed);
        let rate = k.as_dimension(product);
        k.finish(rate, ns.every_mode(), slow)
    });
    ns.memory_rate(set_point, following);
}
