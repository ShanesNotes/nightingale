use crate::shared::drive;
use ng_contract::*;

/// Open: `L dq/dt = dp - B q |q|` (research 02). Shut: `q` holds at zero. Shutting resets the flow, so no column ever
/// carries blood through a shut valve.
pub(crate) fn valves<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
    wave: RateClass,
) -> Result<(), AdmissionError> {
    let route = ports.valves(cv);
    let flow = route.flow_memory();
    let [open, shut] = cv.modes_along(&route, "valve", ["open", "shut"]);
    let inertance =
        cv.edge_parameter::<ValveEdge, Inertance>(ParameterKey::new("cv.inertance"), Admissible::positive());
    let orifice =
        cv.edge_parameter::<ValveEdge, Orifice>(ParameterKey::new("cv.orifice"), Admissible::positive());

    let accelerating = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
        let head_across = drive(&mut k, pressure, head);
        let q = k.state(&flow);
        let zero = k.zero::<Rate<Water>>();
        let reverse = k.subtract(zero, q);
        let size = k.smooth_max(q, reverse, zero);
        let squared = k.multiply(q, size);
        let loss = k.edge_parameter(&orifice);
        let product = k.multiply(loss, squared);
        let loss: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let net = k.subtract(head_across, loss);
        let mass = k.edge_parameter(&inertance);
        let quotient = k.divide(net, mass);
        let rate = k.as_dimension(quotient);
        k.finish(rate, open.only(), wave)
    });
    cv.memory_rate(&flow, accelerating);
    let held = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
        let zero = k.zero::<Rate<Rate<Water>>>();
        k.finish(zero, shut.only(), wave)
    });
    cv.memory_rate(&flow, held);

    // Research 02: open while the head or the flow is positive, so a shut needs both at or below zero. A flow that
    // reaches zero against a forward head does not shut, or the valve would flutter through every quiet filling.
    let reading = cv.edge_parameter::<ValveEdge, Resistance>(
        ParameterKey::new("cv.valve-reading"),
        Admissible::positive(),
    );
    let falling = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
        let head_across = drive(&mut k, pressure, head);
        let q = k.state(&flow);
        let per = k.edge_parameter(&reading);
        let product = k.multiply(per, q);
        let flowing: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let zero = k.zero();
        let open_while = k.smooth_max(head_across, flowing, zero);
        k.finish(open_while, open.only(), wave)
    });
    cv.event(Guard::new(falling, Crossing::Falling, open.to(&shut)?).resetting(&flow));
    // A positive opening head: a shut valve at no head and no flow stays shut, or a still body would fire at
    // every step.
    let opening = cv
        .edge_parameter::<ValveEdge, Pressure>(ParameterKey::new("cv.opening-head"), Admissible::positive());
    let rising = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
        let head_across = drive(&mut k, pressure, head);
        let least = k.edge_parameter(&opening);
        let past = k.subtract(head_across, least);
        k.finish(past, shut.only(), wave)
    });
    cv.event(Guard::new(rising, Crossing::Rising, shut.to(&open)?));
    Ok(())
}
