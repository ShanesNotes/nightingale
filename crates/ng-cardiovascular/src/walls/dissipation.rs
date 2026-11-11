use super::shed;
use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    store: &Pair<'b, Cardiovascular, Energy>,
    power: &AlgebraicSlot<'b, Cardiovascular, Rate<Energy>>,
    soft: &Parameter<'b, Cardiovascular, Rate<Energy>>,
    wave: RateClass,
) {
    let leaving = cv.kernel(|mut k| {
        let net = k.algebraic(power);
        let heat = shed(&mut k, net, soft);
        k.finish(heat, cv.every_mode(), wave)
    });
    cv.transfer(Transfer::new(store, leaving));
}
