use crate::shared::drive;
use ng_contract::catalog::{BloodViscosity, OxygenDebtPerBed, Reads, TubuloglomerularFeedback};
use ng_contract::*;

pub(crate) fn plain_edges<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    route: &EdgeRoute<'b, Cardiovascular, Water>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
    wave: RateClass,
    whole_body: bool,
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
    let viscosity = whole_body.then(|| {
        let line: BloodViscosity<Reads<'b, Cardiovascular>> = cv.receive();
        let reciprocal = cv.parameter::<Quotient<Dimensionless, Viscosity>>(
            ParameterKey::new("cv.reference-viscosity-reciprocal"),
            Admissible::positive(),
        );
        (line.viscosity, reciprocal)
    });
    let debt = whole_body.then(|| {
        let line: OxygenDebtPerBed<Reads<'b, Cardiovascular>> = cv.receive();
        let share =
            cv.end_parameter::<Dimensionless>(ParameterKey::new("cv.debt-share"), Admissible::unit_interval());
        let half = cv.end_parameter::<Oxygen>(ParameterKey::new("cv.debt-half"), Admissible::positive());
        (line.oxygen_debt_per_bed, share, half)
    });
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
        let total = match &viscosity {
            Some((mu, reciprocal)) => {
                let (mu, reciprocal) = (k.read(mu), k.parameter(reciprocal));
                let ratio = k.multiply(reciprocal, mu);
                let ratio: Expr<'_, '_, _, Dimensionless> = k.as_dimension(ratio);
                k.scale(total, ratio)
            }
            None => total,
        };
        let total = match &debt {
            Some((owed, share, half)) => {
                let owed = k.downstream_if_in(owed);
                let (share, half) = (k.downstream_parameter(share), k.downstream_parameter(half));
                let kept = resistance_share_at_debt(&mut k, owed, share, half);
                k.scale(total, kept)
            }
            None => total,
        };
        let quotient = k.divide(head_across, total);
        let flow = k.as_dimension(quotient);
        k.finish(flow, cv.every_mode(), wave)
    });
    cv.transfer_along(route, flow);
    Ok(())
}

/// A bed's resistance over its resistance with no oxygen debt, `1 - S d / (d + K)`, written as `(d + K - S d) / (d + K)` so
/// that it is one to the bit at no debt. `S` is the greatest share of its resistance a bed can lose and `K` the debt at
/// which it has lost half of that. A debt at `-K` or under leaves no positive divisor, and the step is refused.
pub fn resistance_share_at_debt<'b, 'k, O, At>(
    k: &mut Kernel<'b, 'k, O, At>,
    debt: Expr<'b, 'k, O, Oxygen>,
    share: Expr<'b, 'k, O, Dimensionless>,
    half: Expr<'b, 'k, O, Oxygen>,
) -> Expr<'b, 'k, O, Dimensionless> {
    let whole = k.add(debt, half);
    let lost = k.scale(debt, share);
    let kept = k.subtract(whole, lost);
    let share = k.divide(kept, whole);
    k.as_dimension(share)
}
