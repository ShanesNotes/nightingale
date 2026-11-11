use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    rate: &AlgebraicSlot<'b, Cardiovascular, Rate<Dimensionless>>,
    wave: RateClass,
) {
    let intrinsic =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.intrinsic-rate"), Admissible::positive());
    let beta_gain =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.beta-gain"), Admissible::nonnegative());
    let vagal_gain =
        cv.parameter::<Dimensionless>(ParameterKey::new("cv.vagal-gain"), Admissible::unit_interval());
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
        let row = k.algebraic(rate);
        let residual = k.subtract(row, value);
        k.finish(residual, cv.every_mode(), wave)
    });
    cv.residual(rate, residual);
}
