use ng_contract::*;

pub(crate) fn declare<'b>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    ports: &FluidsPorts<'b>,
    interstitial: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
    class: RateClass,
) {
    // Lymph runs when tissue pressure stands past a threshold, softly. Model default.
    let lymph = ports.lymph(fl);
    lymph.keeps_back::<Vasopressin>(fl);
    lymph.keeps_back::<Oxygen>(fl);
    lymph.keeps_back::<CarbonDioxide>(fl);
    lymph.keeps_back::<Glucose>(fl);
    lymph.keeps_back::<Triglyceride>(fl);
    lymph.keeps_back::<FattyAcids>(fl);
    lymph.keeps_back::<Glycerol>(fl);
    lymph.keeps_back::<AminoAcids>(fl);
    lymph.keeps_back::<Lactate>(fl);
    lymph.keeps_back::<Ketones>(fl);
    lymph.keeps_back::<Urea>(fl);
    lymph.keeps_back::<Ammonium>(fl);
    lymph.keeps_back::<Creatinine>(fl);
    lymph.keeps_back::<FixedAcid>(fl);
    lymph.keeps_back::<DietBase>(fl);
    lymph.keeps_back::<Calcium>(fl);
    lymph.keeps_back::<Magnesium>(fl);
    lymph.keeps_back::<Phosphate>(fl);
    lymph.keeps_back::<Hemoglobin>(fl);
    lymph.keeps_back::<Iron>(fl);
    lymph.keeps_back::<Insulin>(fl);
    lymph.keeps_back::<Glucagon>(fl);
    lymph.keeps_back::<Renin>(fl);
    lymph.keeps_back::<AngiotensinII>(fl);
    lymph.keeps_back::<Aldosterone>(fl);
    lymph.keeps_back::<NatriureticPeptide>(fl);
    lymph.keeps_back::<ParathyroidHormone>(fl);
    lymph.keeps_back::<Cortisol>(fl);
    let conductance = fl.edge_parameter::<LymphEdge, Conductance<Water>>(
        ParameterKey::new("fl.lymph-conductance"),
        Admissible::positive(),
    );
    let threshold = fl
        .edge_parameter::<LymphEdge, Pressure>(ParameterKey::new("fl.lymph-threshold"), Admissible::finite());
    let width =
        fl.edge_parameter::<LymphEdge, Pressure>(ParameterKey::new("fl.lymph-width"), Admissible::positive());
    let drain = fl.edge_kernel(|mut k| {
        let inside = k.upstream(interstitial);
        let start = k.edge_parameter(&threshold);
        let above = k.subtract(inside, start);
        let (zero, soft) = (k.zero(), k.edge_parameter(&width));
        let head = k.smooth_max(above, zero, soft);
        let open = k.edge_parameter(&conductance);
        let flow = k.conductance_flow(open, head);
        k.finish(flow, fl.every_mode(), class)
    });
    fl.transfer_along(&lymph, drain);
}
