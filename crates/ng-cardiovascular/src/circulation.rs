use crate::shared::drive;
use ng_contract::catalog::{Reads, TubuloglomerularFeedback};
use ng_contract::*;

pub(crate) fn plain_edges<'b>(
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
        let effect = k.at_junction_if_supplied(&alpha);
        let tuned = k.scale(base, gain);
        let toned = k.scale(tuned, effect);
        let with_tone = k.add(base, toned);
        // f^-4 where the edge's feedback share is one, because feedback acts on diameter (research 02).
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
