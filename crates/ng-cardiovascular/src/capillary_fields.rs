use ng_contract::catalog::{CapillaryAndVenousPressure, Defines};
use ng_contract::*;

pub(crate) fn capillary_fields<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    wave: RateClass,
) {
    let fields: CapillaryAndVenousPressure<Defines<'_, Cardiovascular>> = cv.emit();
    let capillary = cv.site_kernel(|mut k| {
        let value = k.in_blood(pressure);
        k.finish(value, cv.every_mode(), wave)
    });
    cv.define_at(&fields.capillary_pressure, capillary);
    let venous = cv.site_kernel(|mut k| {
        let value = k.drained(pressure);
        k.finish(value, cv.every_mode(), wave)
    });
    cv.define_at(&fields.venous_pressure, venous);
}
