use ng_contract::catalog::{BloodViscosity, Defines};
use ng_contract::*;

/// Research 02's row, Pries's in vivo form at one fixed diameter (Secomb and Pries 2013 Eqs 5 to 7), so `m` (Eq 6),
/// `C` (Eq 5) and the wall factor `W = (D / (D - 1.1))^2` are numbers:
/// `mu = mu_plasma W (1 + (m - 1) W ((1 - Hct)^C - 1) / ((1 - H_ref)^C - 1))`, with `Hct = c_Hb / c_rbc`. The quotient
/// is taken over the denominator's square, a positive divisor whatever the exponent's sign.
pub(crate) fn viscosity<'b>(
    hc: &mut FragmentBuilder<'b, Hematology>,
    central_veins: &Pair<'b, Hematology, Hemoglobin>,
    class: RateClass,
) {
    let key = ParameterKey::new;
    let plasma = hc.parameter::<Viscosity>(key("hc.plasma-viscosity"), Admissible::positive());
    let red_cell =
        hc.parameter::<Concentration<Hemoglobin>>(key("hc.red-cell-hemoglobin"), Admissible::positive());
    let reference =
        hc.parameter::<Dimensionless>(key("hc.reference-hematocrit"), Admissible::unit_interval());
    let at_reference =
        hc.parameter::<Dimensionless>(key("hc.relative-viscosity-at-reference"), Admissible::positive());
    let exponent = hc.parameter::<Dimensionless>(key("hc.viscosity-exponent"), Admissible::finite());
    let wall = hc.parameter::<Dimensionless>(key("hc.wall-factor"), Admissible::positive());
    let unit = hc.parameter::<Dimensionless>(key("hc.unit"), Admissible::positive());
    let (hemoglobin, water) = (
        central_veins.amount::<Hemoglobin>(),
        central_veins.amount::<Water>(),
    );
    let out: BloodViscosity<Defines<'b, Hematology>> = hc.emit();
    let law = hc.kernel(|mut k| {
        let (amount, water) = (k.read(&hemoglobin), k.read(&water));
        let volume = k.volume(water);
        let quotient = k.divide(amount, volume);
        let concentration: Expr<'_, '_, _, Concentration<Hemoglobin>> = k.as_dimension(quotient);
        let per_cell = k.parameter(&red_cell);
        let plasma_part = k.subtract(per_cell, concentration);
        let share = k.divide(plasma_part, per_cell);
        let plasma_share: Expr<'_, '_, _, Dimensionless> = k.as_dimension(share);
        let (c, one) = (k.parameter(&exponent), k.parameter(&unit));
        let log = k.ln(plasma_share);
        let power = k.scale(c, log);
        let raised = k.exp(power);
        let at = k.subtract(raised, one);
        let reference = k.parameter(&reference);
        let reference_share = k.subtract(one, reference);
        let log = k.ln(reference_share);
        let power = k.scale(c, log);
        let raised = k.exp(power);
        let whole = k.subtract(raised, one);
        let (product, square) = (k.scale(at, whole), k.scale(whole, whole));
        let quotient = k.divide(product, square);
        let fraction: Expr<'_, '_, _, Dimensionless> = k.as_dimension(quotient);
        let (m, w) = (k.parameter(&at_reference), k.parameter(&wall));
        let above = k.subtract(m, one);
        let term = k.scale(above, fraction);
        let term = k.scale(term, w);
        let inner = k.add(one, term);
        let relative = k.scale(inner, w);
        let base = k.parameter(&plasma);
        let value = k.scale(base, relative);
        k.finish(value, hc.every_mode(), class)
    });
    hc.define(&out.viscosity, law);
}
