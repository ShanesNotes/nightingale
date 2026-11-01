use ng_contract::*;

pub(crate) struct Heart<'b> {
    pub(crate) activation: StateSlot<'b, Cardiovascular, Dimensionless>,
    pub(crate) peak: Parameter<'b, Cardiovascular, Dimensionless>,
    pub(crate) rise: Parameter<'b, Cardiovascular, Rate<Dimensionless>>,
    pub(crate) systole: ModeSetSource<'b>,
    pub(crate) diastole: ModeSetSource<'b>,
}
pub(crate) type ModeSetSource<'b> = Mode<'b, Cardiovascular>;

/// A phase runs from zero to the end of the cycle at the pacemaker's rate. Systole lasts to a set phase, and the heart's
/// activation rises through it and falls after it, first order. A wrap resets the phase and starts the next systole.
/// No operator is periodic: the beat is two guards and a reset, so its rate can move beat to beat. Model default.
pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    wave: RateClass,
) -> Result<Heart<'b>, AdmissionError> {
    let [systole, diastole] = cv.modes("rhythm", ["systole", "diastole"]);
    let phase = cv.state::<Dimensionless>("phase", Admissible::finite());
    let activation = cv.state::<Dimensionless>("activation", Admissible::nonnegative());
    let rate = cv.algebraic::<Rate<Dimensionless>>("rate", Admissible::positive());
    let intrinsic =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.intrinsic-rate"), Admissible::positive());
    let beta_gain =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.beta-gain"), Admissible::nonnegative());
    let vagal_gain =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.vagal-gain"), Admissible::unit_interval());
    let systole_end =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.systole-end"), Admissible::positive());
    let cycle_end = cv.parameter::<Dimensionless>(ParameterKey::new("cv.cycle-end"), Admissible::positive());
    let peak = cv.parameter::<Dimensionless>(ParameterKey::new("cv.activation-peak"), Admissible::positive());
    let rise =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.rise-speed"), Admissible::positive());
    let fall =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.fall-speed"), Admissible::positive());
    let beta = ports.effect::<Norepinephrine>();
    let vagal = ports.effect::<Acetylcholine>();

    // rate = R0 m n (research 02), with m = 1 + S_b E_b1 and n = 1 - S_m E_m2 written with no literal.
    let residual = cv.kernel(|mut k| {
        let base = k.parameter(&intrinsic);
        let gain = k.parameter(&beta_gain);
        let effect = k.at_landmark(Landmark::Pacemaker, &beta);
        let tuned = k.scale(base, gain);
        let sped = k.scale(tuned, effect);
        let raised = k.add(base, sped);
        let gain = k.parameter(&vagal_gain);
        let effect = k.at_landmark(Landmark::Pacemaker, &vagal);
        let tuned = k.scale(raised, gain);
        let slowed = k.scale(tuned, effect);
        let value = k.subtract(raised, slowed);
        let row = k.algebraic(&rate);
        let residual = k.subtract(row, value);
        k.finish(residual, cv.every_mode(), wave)
    });
    cv.residual(&rate, residual);

    let running = cv.kernel(|mut k| {
        let value = k.algebraic(&rate);
        k.finish(value, cv.every_mode(), wave)
    });
    cv.memory_rate(&phase, running);

    let rising = cv.kernel(|mut k| {
        let (target, current, speed) = (k.parameter(&peak), k.state(&activation), k.parameter(&rise));
        let rate = k.relax(target, current, speed);
        k.finish(rate, systole.only(), wave)
    });
    cv.memory_rate(&activation, rising);
    let falling = cv.kernel(|mut k| {
        let (target, current, speed) = (k.zero(), k.state(&activation), k.parameter(&fall));
        let rate = k.relax(target, current, speed);
        k.finish(rate, diastole.only(), wave)
    });
    cv.memory_rate(&activation, falling);

    let past_systole = cv.kernel(|mut k| {
        let (now, end) = (k.state(&phase), k.parameter(&systole_end));
        let gap = k.subtract(now, end);
        k.finish(gap, systole.only(), wave)
    });
    cv.event(Guard::new(past_systole, Crossing::Rising, systole.to(&diastole)?));
    let past_cycle = cv.kernel(|mut k| {
        let (now, end) = (k.state(&phase), k.parameter(&cycle_end));
        let gap = k.subtract(now, end);
        k.finish(gap, diastole.only(), wave)
    });
    cv.event(Guard::new(past_cycle, Crossing::Rising, diastole.to(&systole)?).resetting(&phase));

    Ok(Heart {
        activation,
        peak,
        rise,
        systole,
        diastole,
    })
}
