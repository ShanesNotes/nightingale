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
