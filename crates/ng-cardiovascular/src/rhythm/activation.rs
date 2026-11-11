use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    activation: &StateSlot<'b, Cardiovascular, Dimensionless>,
    peak: &Parameter<'b, Cardiovascular, Dimensionless>,
    rise: &Parameter<'b, Cardiovascular, Rate<Dimensionless>>,
    systole: &Mode<'b, Cardiovascular>,
    diastole: &Mode<'b, Cardiovascular>,
    wave: RateClass,
) {
    let fall =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.fall-speed"), Admissible::positive());

    let rising = cv.kernel(|mut k| {
        let (target, current, speed) = (k.parameter(peak), k.state(activation), k.parameter(rise));
        let rate = k.relax(target, current, speed);
        k.finish(rate, systole.only(), wave)
    });
    cv.memory_rate(activation, rising);
    let falling = cv.kernel(|mut k| {
        let (target, current, speed) = (k.zero(), k.state(activation), k.parameter(&fall));
        let rate = k.relax(target, current, speed);
        k.finish(rate, diastole.only(), wave)
    });
    cv.memory_rate(activation, falling);
}
