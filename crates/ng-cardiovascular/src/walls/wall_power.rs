use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    store: &Pair<'b, Cardiovascular, Energy>,
    power: &AlgebraicSlot<'b, Cardiovascular, Rate<Energy>>,
    elastic: &AlgebraicSlot<'b, Cardiovascular, Energy>,
    wave: RateClass,
) {
    let closing = cv.kernel(|mut k| {
        let held = k.read(&store.amount::<Energy>());
        let sum = k.algebraic(elastic);
        let gap = k.subtract(held, sum);
        k.finish(gap, cv.every_mode(), wave)
    });
    cv.closes(power, closing);
}
