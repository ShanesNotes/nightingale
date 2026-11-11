use ng_contract::*;

mod activation;
mod phase;
mod rate;

pub(crate) struct Heart<'b> {
    pub(crate) activation: StateSlot<'b, Cardiovascular, Dimensionless>,
    pub(crate) peak: Parameter<'b, Cardiovascular, Dimensionless>,
    pub(crate) rise: Parameter<'b, Cardiovascular, Rate<Dimensionless>>,
    pub(crate) systole: Mode<'b, Cardiovascular>,
    pub(crate) diastole: Mode<'b, Cardiovascular>,
}

/// A phase runs from zero to the end of the cycle at the pacemaker's rate. Systole lasts to a set phase, and the heart's
/// activation rises through it and falls after it, first order. A wrap resets the phase and starts the next systole.
/// No operator is periodic: the beat is two guards and a reset, so its rate can move beat to beat. Model default.
pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    wave: RateClass,
) -> Result<Heart<'b>, AdmissionError> {
    let [systole, diastole] = cv.modes("rhythm", ["systole", "diastole"]);
    let activation = cv.state::<Dimensionless>("activation", Admissible::nonnegative());
    let rate = cv.algebraic::<Rate<Dimensionless>>("rate", Admissible::positive());
    let peak = cv.parameter::<Dimensionless>(ParameterKey::new("cv.activation-peak"), Admissible::positive());
    let rise =
        cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.rise-speed"), Admissible::positive());
    rate::declare(cv, ports, &rate, wave);
    phase::declare(cv, &rate, &systole, &diastole, wave)?;
    activation::declare(cv, &activation, &peak, &rise, &systole, &diastole, wave);

    Ok(Heart {
        activation,
        peak,
        rise,
        systole,
        diastole,
    })
}
