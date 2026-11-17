use super::{Bed, Field, Plasma, Stated, Value};
use ng_contract::*;

/// The dissociation curve: `S = logistic(n ln(P / P50))`, with `n = a - b exp(-P / g)` and
/// `P50 = P50s (1 + c1 d + c2 d^2 + c3 d^3) (1 + q1 u + q2 u^2)`, `d` the plasma's acidity less its standard and `u` its
/// carbon dioxide partial pressure less its standard, over a scale. Temperature's factor is one, because its arrow is
/// held.
pub(crate) struct Curve<'b> {
    solubility: Parameter<'b, BloodChemistry, Quotient<Concentration<Oxygen>, Pressure>>,
    heme: Parameter<'b, BloodChemistry, Quotient<Concentration<Oxygen>, Concentration<Hemoglobin>>>,
    half: Parameter<'b, BloodChemistry, Pressure>,
    ceiling: Parameter<'b, BloodChemistry, Dimensionless>,
    fall: Parameter<'b, BloodChemistry, Dimensionless>,
    reach: Parameter<'b, BloodChemistry, Pressure>,
    standard_acidity: Parameter<'b, BloodChemistry, Dimensionless>,
    by_acidity: [Parameter<'b, BloodChemistry, Dimensionless>; 3],
    carbon_dioxide: [Parameter<'b, BloodChemistry, Pressure>; 2],
    by_carbon_dioxide: [Parameter<'b, BloodChemistry, Dimensionless>; 2],
}

impl<'b> Curve<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, BloodChemistry>) -> Self {
        let key = ParameterKey::new;
        Self {
            solubility: laws.parameter(key("bc.oxygen-solubility"), Admissible::positive()),
            heme: laws.parameter(key("bc.heme-oxygen"), Admissible::positive()),
            half: laws.parameter(key("bc.p50"), Admissible::positive()),
            ceiling: laws.parameter(key("bc.hill-ceiling"), Admissible::positive()),
            fall: laws.parameter(key("bc.hill-fall"), Admissible::finite()),
            reach: laws.parameter(key("bc.hill-reach"), Admissible::positive()),
            standard_acidity: laws.parameter(key("bc.standard-acidity"), Admissible::finite()),
            by_acidity: [
                laws.parameter(key("bc.p50-acidity-1"), Admissible::finite()),
                laws.parameter(key("bc.p50-acidity-2"), Admissible::finite()),
                laws.parameter(key("bc.p50-acidity-3"), Admissible::finite()),
            ],
            carbon_dioxide: [
                laws.parameter(key("bc.standard-carbon-dioxide"), Admissible::finite()),
                laws.parameter(key("bc.carbon-dioxide-scale"), Admissible::positive()),
            ],
            by_carbon_dioxide: [
                laws.parameter(key("bc.p50-carbon-dioxide-1"), Admissible::finite()),
                laws.parameter(key("bc.p50-carbon-dioxide-2"), Admissible::finite()),
            ],
        }
    }

    fn exponent<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        pressure: Value<'b, 'k, Pressure>,
    ) -> Value<'b, 'k, Dimensionless> {
        let reach = k.parameter(&self.reach);
        let quotient = k.divide(pressure, reach);
        let reached: Value<'b, 'k, Dimensionless> = k.as_dimension(quotient);
        let zero = k.zero();
        let negative = k.subtract(zero, reached);
        let falling = k.exp(negative);
        let (ceiling, fall) = (k.parameter(&self.ceiling), k.parameter(&self.fall));
        let less = k.scale(fall, falling);
        k.subtract(ceiling, less)
    }

    fn half<'k>(&self, k: &mut Bed<'b, 'k>, speciated: &Stated<'b>) -> Value<'b, 'k, Pressure> {
        let (acidity, standard) = (k.at_site(&speciated.acidity), k.parameter(&self.standard_acidity));
        let d = k.subtract(acidity, standard);
        let [c1, c2, c3] = self.by_acidity.each_ref().map(|value| k.parameter(value));
        let square = k.scale(d, d);
        let cube = k.scale(square, d);
        let (first, second, third) = (k.scale(c1, d), k.scale(c2, square), k.scale(c3, cube));
        let shift = k.add(first, second);
        let shift = k.add(shift, third);
        let standard = k.parameter(&self.half);
        let shifted = k.scale(standard, shift);
        let by_acidity = k.add(standard, shifted);
        let read = k.at_site(&speciated.carbon_dioxide_partial_pressure);
        let [standard, scale] = self.carbon_dioxide.each_ref().map(|value| k.parameter(value));
        let apart = k.subtract(read, standard);
        let quotient = k.divide(apart, scale);
        let u: Value<'b, 'k, Dimensionless> = k.as_dimension(quotient);
        let [q1, q2] = self.by_carbon_dioxide.each_ref().map(|value| k.parameter(value));
        let square = k.scale(u, u);
        let (first, second) = (k.scale(q1, u), k.scale(q2, square));
        let shift = k.add(first, second);
        let shifted = k.scale(by_acidity, shift);
        k.add(by_acidity, shifted)
    }
}

/// Each bed's plasma oxygen partial pressure and saturation. The partial pressure is the curve's inverse at the bound
/// share of capacity, `P = P50 exp(logit(S_c) / n(P))` with `S_c = (C - alpha P) / (4 c_Hb)`, so its row subtracts no
/// two near-equal numbers but inside a logarithm, and saturation is the curve at that pressure.
pub(crate) fn define<'b>(
    laws: &mut FragmentBuilder<'b, BloodChemistry>,
    plasma: &Plasma<'b>,
    speciated: &Stated<'b>,
    (pressure_out, saturation_out): (&Field<'b, Pressure>, &Field<'b, Dimensionless>),
    class: RateClass,
) {
    let curve = Curve::declare(laws);
    let pressure = &speciated.oxygen_partial_pressure;
    let law = laws.site_kernel(|mut k| {
        let (oxygen, hemoglobin, water) = (
            k.at_site(&plasma.oxygen),
            k.at_site(&plasma.hemoglobin),
            k.at_site(&plasma.water),
        );
        let volume = k.volume(water);
        let quotient = k.divide(oxygen, volume);
        let content: Value<'b, '_, Concentration<Oxygen>> = k.as_dimension(quotient);
        let quotient = k.divide(hemoglobin, volume);
        let carrier: Value<'b, '_, Concentration<Hemoglobin>> = k.as_dimension(quotient);
        let heme = k.parameter(&curve.heme);
        let product = k.multiply(heme, carrier);
        let capacity: Value<'b, '_, Concentration<Oxygen>> = k.as_dimension(product);
        let now = k.at_site(pressure);
        let solubility = k.parameter(&curve.solubility);
        let product = k.multiply(solubility, now);
        let dissolved: Value<'b, '_, Concentration<Oxygen>> = k.as_dimension(product);
        let bound = k.subtract(content, dissolved);
        let empty = k.subtract(capacity, bound);
        let quotient = k.divide(bound, empty);
        let odds: Value<'b, '_, Dimensionless> = k.as_dimension(quotient);
        let logit = k.ln(odds);
        let exponent = curve.exponent(&mut k, now);
        let quotient = k.divide(logit, exponent);
        let power: Value<'b, '_, Dimensionless> = k.as_dimension(quotient);
        let rise = k.exp(power);
        let half = curve.half(&mut k, speciated);
        let value = k.scale(half, rise);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(pressure_out, law);
    let law = laws.site_kernel(|mut k| {
        let now = k.at_site(pressure);
        let half = curve.half(&mut k, speciated);
        let quotient = k.divide(now, half);
        let ratio: Value<'b, '_, Dimensionless> = k.as_dimension(quotient);
        let log = k.ln(ratio);
        let exponent = curve.exponent(&mut k, now);
        let argument = k.scale(log, exponent);
        let value = k.logistic(argument);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(saturation_out, law);
}
