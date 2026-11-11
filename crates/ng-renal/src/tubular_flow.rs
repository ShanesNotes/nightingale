use ng_contract::*;

pub(crate) fn declare<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    lumen: &ReadPort<'b, Renal, PerSite<Urinary, Pressure>>,
    class: RateClass,
) {
    // Flow down the tubule follows the head between lumens. Model default.
    let tubular = ports.tubular(rn);
    let drag = rn.edge_parameter::<TubuleEdge, Resistance>(
        ParameterKey::new("rn.tubule-resistance"),
        Admissible::positive(),
    );
    let down = rn.edge_kernel(|mut k| {
        let (above, below) = (k.upstream(lumen), k.downstream(lumen));
        let head = k.subtract(above, below);
        let resistance = k.edge_parameter(&drag);
        let quotient = k.divide(head, resistance);
        let flow = k.as_dimension(quotient);
        k.finish(flow, rn.every_mode(), class)
    });
    rn.transfer_along(&tubular, down);
}
