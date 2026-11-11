use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
    wave: RateClass,
) {
    let fallen = ports.gravity_work(cv);
    let falling = cv.edge_kernel(|mut k| {
        let per_length = k.parameter(head);
        let fall = k.fall();
        let product = k.multiply(per_length, fall);
        let head: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let flow = k.edge_flow();
        let moved = k.volume_flow(flow);
        let product = k.multiply(head, moved);
        let power = k.as_dimension(product);
        k.finish(power, cv.every_mode(), wave)
    });
    cv.transfer_along(&fallen, falling);
}
