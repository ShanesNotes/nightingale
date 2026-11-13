use ng_contract::catalog::{CapillaryAndVenousPressure, Reads};
use ng_contract::*;

/// `pi = slope c`, with `c` albumin over the volume of the space's own water. Model default: linear.
fn oncotic<'b, 'k>(
    k: &mut Kernel<'b, 'k, Fluids, CapillaryBed>,
    albumin: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Albumin>>,
    water: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Water>>,
    slope: &Parameter<'b, Fluids, Quotient<Pressure, Concentration<Albumin>>>,
) -> Expr<'b, 'k, Fluids, Pressure> {
    let (amount, solvent) = (k.at_site(albumin), k.at_site(water));
    let volume = k.volume(solvent);
    let ratio = k.divide(amount, volume);
    let concentration = k.as_dimension(ratio);
    let per = k.parameter(slope);
    let product = k.multiply(per, concentration);
    k.as_dimension(product)
}

pub(crate) fn declare<'b>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    route: &SiteRoute<'b, Fluids, CapillaryBed, Water>,
    interstitial: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
    class: RateClass,
) {
    let pressures: CapillaryAndVenousPressure<Reads<'_, Fluids>> = fl.receive();
    let filtration = fl.site_parameter::<CapillaryBed, Conductance<Water>>(
        ParameterKey::new("fl.filtration"),
        Admissible::positive(),
    );
    let slope = fl.parameter(ParameterKey::new("fl.oncotic-slope"), Admissible::positive());
    let reflected = fl.reflect::<Albumin>(route, ParameterKey::new("fl.albumin-reflection"));
    route.keeps_back::<Vasopressin>(fl);
    route.keeps_back::<Oxygen>(fl);
    route.keeps_back::<CarbonDioxide>(fl);
    route.keeps_back::<Glucose>(fl);
    route.keeps_back::<Triglyceride>(fl);
    route.keeps_back::<FattyAcids>(fl);
    route.keeps_back::<Glycerol>(fl);
    route.keeps_back::<AminoAcids>(fl);
    route.keeps_back::<Lactate>(fl);
    route.keeps_back::<Ketones>(fl);
    route.keeps_back::<Urea>(fl);
    route.keeps_back::<Ammonium>(fl);
    route.keeps_back::<Creatinine>(fl);
    route.keeps_back::<FixedAcid>(fl);
    route.keeps_back::<DietBase>(fl);
    route.keeps_back::<Calcium>(fl);
    route.keeps_back::<Magnesium>(fl);
    route.keeps_back::<Phosphate>(fl);
    route.keeps_back::<Hemoglobin>(fl);
    route.keeps_back::<Iron>(fl);
    route.keeps_back::<Insulin>(fl);
    route.keeps_back::<Glucagon>(fl);
    route.keeps_back::<Renin>(fl);
    route.keeps_back::<AngiotensinII>(fl);
    route.keeps_back::<Aldosterone>(fl);
    route.keeps_back::<NatriureticPeptide>(fl);
    route.keeps_back::<ParathyroidHormone>(fl);
    route.keeps_back::<Cortisol>(fl);
    let exchange = fl.site_kernel(|mut k| {
        let inside = k.at_site(&pressures.capillary_pressure);
        let outside = k.at_site(interstitial);
        let head = k.subtract(inside, outside);
        let held = oncotic(&mut k, &route.from_amount(), &route.from_amount(), &slope);
        let drawn = oncotic(&mut k, &route.to_amount(), &route.to_amount(), &slope);
        let pull = k.subtract(held, drawn);
        let sigma = k.site_parameter(&reflected);
        let pulled = k.scale(pull, sigma);
        let net = k.subtract(head, pulled);
        let conductance = k.site_parameter(&filtration);
        let flow = k.conductance_flow(conductance, net);
        k.finish(flow, fl.every_mode(), class)
    });
    fl.transfer_at(route, exchange);
}
