//! The heart and vessels: one pressure law at every blood site, one flow law on every plain edge, one inertial law on
//! every valve, the rhythm, and the heart's work into the elastic store.
//!
//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_cardiovascular::walls::elastance;
//! ```
//! ```
//! use ng_cardiovascular::declare;
//! ```

use ng_contract::catalog::{
    BaroreceptorAfferents, CapillaryAndVenousPressure, Defines, Reads, TubuloglomerularFeedback,
};
use ng_contract::*;

mod rhythm;
mod walls;

const WAVE_NS: u64 = 10_000_000;

/// Emits baroreceptor afferents (line 77) and capillary and venous pressure (line 80). Reads tubuloglomerular feedback
/// (line 97) and the effect of each transmitter at its junction. Each law is declared once, and the map says where.
pub fn declare<'b>(scope: CardiovascularScope<'b>) -> Result<Fragment<'b, Cardiovascular>, AdmissionError> {
    let (mut cv, ports) = scope.parts();
    let wave = RateClass::new(RateBand::Wave, WAVE_NS)?;
    let heart = rhythm::declare(&mut cv, &ports, wave)?;
    let (pressure_row, pressure) = ports.blood_pressure(&mut cv);
    let walls = walls::Walls::declare(&mut cv, &ports, &heart);
    let head = cv.parameter::<Quotient<Pressure, Length>>(
        ParameterKey::new("cv.head-per-length"),
        Admissible::positive(),
    );

    let value = cv.site_kernel::<Blood, _, _>(|mut k| {
        let stressed = walls.stressed(&mut k);
        let stiffness = walls.elastance(&mut k);
        let product = k.multiply(stiffness, stressed);
        let elastic = k.as_dimension(product);
        let outside = k.at_site(&walls.outside);
        let total = k.add(elastic, outside);
        k.finish(total, cv.every_mode(), wave)
    });
    cv.define_at(&pressure_row, value);

    let circulation = ports.circulation(&mut cv);
    plain_edges(&mut cv, &ports, &circulation, &pressure, &head, wave)?;
    valves(&mut cv, &ports, &pressure, &head, wave)?;
    afferents(&mut cv, &pressure, &head, wave)?;
    capillary_fields(&mut cv, &pressure, wave);
    walls.energy(&mut cv, &ports, &heart, &head, wave);
    Ok(cv.finish())
}

fn drive<'b, 'k, E>(
    k: &mut Kernel<'b, 'k, Cardiovascular, E>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
) -> Expr<'b, 'k, Cardiovascular, Pressure>
where
    E: EdgeKind<From = Blood, To = Blood>,
{
    let (up, down) = (k.upstream(pressure), k.downstream(pressure));
    let difference = k.subtract(up, down);
    let per_length = k.parameter(head);
    let fall = k.fall();
    let product = k.multiply(per_length, fall);
    let gravity = k.as_dimension(product);
    k.add(difference, gravity)
}

fn plain_edges<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    route: &EdgeRoute<'b, Cardiovascular, Water>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
    wave: RateClass,
) -> Result<(), AdmissionError> {
    let resistance =
        cv.edge_parameter::<_, Resistance>(ParameterKey::new("cv.resistance"), Admissible::positive());
    let tone =
        cv.edge_parameter::<_, Dimensionless>(ParameterKey::new("cv.tone-gain"), Admissible::nonnegative());
    let share = cv.edge_parameter::<_, Dimensionless>(
        ParameterKey::new("cv.feedback-share"),
        Admissible::unit_interval(),
    );
    let alpha = ports.effect::<Norepinephrine>();
    let feedback: TubuloglomerularFeedback<Reads<'_, Cardiovascular>> = cv.receive();
    let flow = cv.edge_kernel(|mut k| {
        let head_across = drive(&mut k, pressure, head);
        let base = k.edge_parameter(&resistance);
        let gain = k.edge_parameter(&tone);
        let effect = k.at_junction(&alpha);
        let tuned = k.scale(base, gain);
        let toned = k.scale(tuned, effect);
        let with_tone = k.add(base, toned);
        // f^-4 on the one edge whose share is one, because feedback acts on diameter (research 02).
        let f = k.read(&feedback.afferent_tone);
        let square = k.scale(f, f);
        let fourth = k.scale(square, square);
        let narrowed = k.divide(with_tone, fourth);
        let narrowed: Expr<'_, '_, _, Resistance> = k.as_dimension(narrowed);
        let extra = k.subtract(narrowed, with_tone);
        let weight = k.edge_parameter(&share);
        let shared = k.scale(extra, weight);
        let total = k.add(with_tone, shared);
        let quotient = k.divide(head_across, total);
        let flow = k.as_dimension(quotient);
        k.finish(flow, cv.every_mode(), wave)
    });
    cv.transfer_along(route, flow);
    Ok(())
}

/// Open: `L dq/dt = dp - B q |q|` (research 02). Shut: `q` holds at zero. Shutting resets the flow, so no column ever
/// carries blood through a shut valve.
fn valves<'b>(
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
    // Strictly past zero: a shut valve at no head and no flow stays shut, or a still body would fire at every step.
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

/// What a sensor feels is the pressure of the vessel it sits on, less the column of blood up to its own height.
/// Model default.
fn afferents<'b>(
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

fn capillary_fields<'b>(
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
