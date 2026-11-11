use ng_contract::catalog::{Defines, TubuloglomerularFeedback};
use ng_contract::*;

/// `f = f0 + S (1 / (1 + exp((x - x0) / m)) - h)` on the sodium the macula densa sees: research 02's form with
/// `f0` at 1 and `h` at 0.5. Line 97 carries it to the afferent edge, and no law here says which edge.
pub(crate) fn feedback<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let tone: TubuloglomerularFeedback<Defines<'_, Renal>> = rn.emit();
    let (sodium, water) = (ports.lumen::<Sodium>(), ports.lumen::<Water>());
    let rest = rn.parameter::<Dimensionless>(ParameterKey::new("rn.feedback-rest"), Admissible::positive());
    let reach = rn.parameter::<Dimensionless>(ParameterKey::new("rn.feedback-gain"), Admissible::positive());
    let middle = rn.parameter::<Dimensionless>(
        ParameterKey::new("rn.feedback-middle"),
        Admissible::unit_interval(),
    );
    let set = rn
        .parameter::<Concentration<Sodium>>(ParameterKey::new("rn.feedback-sodium"), Admissible::positive());
    let width =
        rn.parameter::<Concentration<Sodium>>(ParameterKey::new("rn.feedback-width"), Admissible::positive());
    let value = rn.kernel(|mut k| {
        let amount = k.at_landmark(Landmark::MaculaDensa, &sodium);
        let solvent = k.at_landmark(Landmark::MaculaDensa, &water);
        let volume = k.volume(solvent);
        let ratio = k.divide(amount, volume);
        let seen: Expr<'_, '_, _, Concentration<Sodium>> = k.as_dimension(ratio);
        let center = k.parameter(&set);
        let above = k.subtract(center, seen);
        let spread = k.parameter(&width);
        let ratio = k.divide(above, spread);
        let drive = k.as_dimension(ratio);
        let opened = k.logistic(drive);
        let half = k.parameter(&middle);
        let shift = k.subtract(opened, half);
        let gain = k.parameter(&reach);
        let moved = k.scale(gain, shift);
        let base = k.parameter(&rest);
        let tone = k.add(base, moved);
        k.finish(tone, rn.every_mode(), class)
    });
    rn.define(&tone.afferent_tone, value);
    Ok(())
}
