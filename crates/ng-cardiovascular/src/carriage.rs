use ng_contract::catalog::PerfusionTemperatures;
use ng_contract::*;

type BloodHeat = Quotient<Quotient<Rate<Energy>, Temperature>, Rate<Volume>>;

/// Heat blood carries from the core store into the store a bed sits in, at that bed, `c_bl F (T_core - T_tissue)` with
/// `F` the bed inflow as a volume flow (research 12, the row 'Core to skin', its blood term). The inflow is signed, so
/// blood that runs back carries heat back (record 0011, consequence 6).
pub(crate) fn carriage<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    carriage: BloodCarriage<'b>,
    wave: RateClass,
) {
    let temperatures: PerfusionTemperatures<'b> = cv.receive();
    let blood = cv.parameter::<BloodHeat>(ParameterKey::new("cv.blood-heat"), Admissible::positive());
    let rate = cv.site_kernel(|mut k| {
        let water = k.bed_inflow();
        let flow = k.volume_flow(water);
        let (core, tissue) = (temperatures.core(&mut k), temperatures.tissue(&mut k));
        let across = k.subtract(core, tissue);
        let blood = k.parameter(&blood);
        let product = k.multiply(blood, flow);
        let per_kelvin = k.as_dimension(product);
        let product = k.multiply(per_kelvin, across);
        let value = k.as_dimension(product);
        k.finish(value, cv.every_mode(), wave)
    });
    carriage.post(cv, rate);
}
