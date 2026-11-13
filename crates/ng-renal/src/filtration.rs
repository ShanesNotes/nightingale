use ng_contract::*;

pub(crate) fn declare<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    lumen: &ReadPort<'b, Renal, PerSite<Urinary, Pressure>>,
    class: RateClass,
) {
    let blood = ports.blood_pressure();

    let filtration = ports.filtration(rn);
    filtration.keeps_back::<Albumin>(rn);
    filtration.keeps_back::<Vasopressin>(rn);
    filtration.keeps_back::<Oxygen>(rn);
    filtration.keeps_back::<CarbonDioxide>(rn);
    filtration.keeps_back::<Glucose>(rn);
    filtration.keeps_back::<Triglyceride>(rn);
    filtration.keeps_back::<FattyAcids>(rn);
    filtration.keeps_back::<Glycerol>(rn);
    filtration.keeps_back::<AminoAcids>(rn);
    filtration.keeps_back::<Lactate>(rn);
    filtration.keeps_back::<Ketones>(rn);
    filtration.keeps_back::<Urea>(rn);
    filtration.keeps_back::<Ammonium>(rn);
    filtration.keeps_back::<Creatinine>(rn);
    filtration.keeps_back::<FixedAcid>(rn);
    filtration.keeps_back::<DietBase>(rn);
    filtration.keeps_back::<Calcium>(rn);
    filtration.keeps_back::<Magnesium>(rn);
    filtration.keeps_back::<Phosphate>(rn);
    filtration.keeps_back::<Hemoglobin>(rn);
    filtration.keeps_back::<Iron>(rn);
    filtration.keeps_back::<Insulin>(rn);
    filtration.keeps_back::<Glucagon>(rn);
    filtration.keeps_back::<Renin>(rn);
    filtration.keeps_back::<AngiotensinII>(rn);
    filtration.keeps_back::<Aldosterone>(rn);
    filtration.keeps_back::<NatriureticPeptide>(rn);
    filtration.keeps_back::<ParathyroidHormone>(rn);
    filtration.keeps_back::<Cortisol>(rn);
    let coefficient = rn.edge_parameter::<FilterEdge, Conductance<Water>>(
        ParameterKey::new("rn.filtration"),
        Admissible::positive(),
    );
    let slope = rn.parameter::<Quotient<Pressure, Concentration<Albumin>>>(
        ParameterKey::new("rn.oncotic-slope"),
        Admissible::positive(),
    );
    let gfr = rn.edge_kernel(|mut k| {
        let (inside, outside) = (k.upstream(&blood), k.downstream(lumen));
        let head = k.subtract(inside, outside);
        let (albumin, water) = (
            k.at_edge(&filtration.from_amount()),
            k.at_edge(&filtration.from_amount()),
        );
        let volume = k.volume(water);
        let ratio = k.divide(albumin, volume);
        let concentration = k.as_dimension(ratio);
        let per = k.parameter(&slope);
        let product = k.multiply(per, concentration);
        let pull: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let net = k.subtract(head, pull);
        let conductance = k.edge_parameter(&coefficient);
        let flow = k.conductance_flow(conductance, net);
        k.finish(flow, rn.every_mode(), class)
    });
    rn.transfer_along(&filtration, gfr);
}
