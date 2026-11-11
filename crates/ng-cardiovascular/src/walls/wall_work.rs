use super::shed;
use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    power: &AlgebraicSlot<'b, Cardiovascular, Rate<Energy>>,
    soft: &Parameter<'b, Cardiovascular, Rate<Energy>>,
    wave: RateClass,
) {
    let paid = ports.wall_work(cv);
    let gaining = cv.kernel(|mut k| {
        let net = k.algebraic(power);
        let heat = shed(&mut k, net, soft);
        let gain = k.subtract(heat, net);
        k.finish(gain, cv.every_mode(), wave)
    });
    cv.transfer(Transfer::new(&paid, gaining));
}
