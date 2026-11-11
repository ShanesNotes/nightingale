use ng_contract::catalog::{BaroreceptorAfferents, Defines};
use ng_contract::*;

/// What a sensor feels is the pressure of the vessel it sits on, less the column of blood up to its own height.
/// Model default.
pub(crate) fn afferents<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
    wave: RateClass,
) -> Result<(), AdmissionError> {
    let reference =
        cv.parameter::<Pressure>(ParameterKey::new("cv.stretch-reference"), Admissible::positive());
    let afferents: BaroreceptorAfferents<Defines<'_, Cardiovascular>> = cv.emit();
    for (output, sensor) in [
        (&afferents.carotid_sinus_stretch, Landmark::CarotidSinus),
        (&afferents.aortic_arch_stretch, Landmark::AorticArch),
    ] {
        let stretch = cv.kernel(|mut k| {
            let inside = k.at_landmark(sensor, pressure);
            let per_length = k.parameter(head);
            let rise = k.rise(sensor);
            let product = k.multiply(per_length, rise);
            let column: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
            let felt = k.subtract(inside, column);
            let scale = k.parameter(&reference);
            let ratio = k.divide(felt, scale);
            let stretch = k.as_dimension(ratio);
            k.finish(stretch, cv.every_mode(), wave)
        });
        cv.define(output, stretch);
    }
    Ok(())
}
