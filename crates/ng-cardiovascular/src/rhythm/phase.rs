use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    rate: &AlgebraicSlot<'b, Cardiovascular, Rate<Dimensionless>>,
    systole: &Mode<'b, Cardiovascular>,
    diastole: &Mode<'b, Cardiovascular>,
    wave: RateClass,
) -> Result<(), AdmissionError> {
    let phase = cv.state::<Dimensionless>("phase", Admissible::finite());
    let systole_end =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.systole-end"), Admissible::positive());
    let cycle_end = cv.parameter::<Dimensionless>(ParameterKey::new("cv.cycle-end"), Admissible::positive());

    let running = cv.kernel(|mut k| {
        let value = k.algebraic(rate);
        k.finish(value, cv.every_mode(), wave)
    });
    cv.memory_rate(&phase, running);

    let past_systole = cv.kernel(|mut k| {
        let (now, end) = (k.state(&phase), k.parameter(&systole_end));
        let gap = k.subtract(now, end);
        k.finish(gap, systole.only(), wave)
    });
    cv.event(Guard::new(past_systole, Crossing::Rising, systole.to(diastole)?));
    let past_cycle = cv.kernel(|mut k| {
        let (now, end) = (k.state(&phase), k.parameter(&cycle_end));
        let gap = k.subtract(now, end);
        k.finish(gap, diastole.only(), wave)
    });
    cv.event(Guard::new(past_cycle, Crossing::Rising, diastole.to(systole)?).resetting(&phase));
    Ok(())
}
