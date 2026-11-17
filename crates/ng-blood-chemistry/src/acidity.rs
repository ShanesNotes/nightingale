use super::{Bed, Field, Plasma, Port, Stated, Value};
use ng_contract::*;

type Charges = Concentration<Charge>;
type PerAcid<Q> = Quotient<Charges, Concentration<Q>>;

/// The strong and fixed ions whose charge, cations less anions, makes a plasma's strong ion difference.
pub(crate) struct StrongIons<'b> {
    pub sodium: Port<'b, Sodium>,
    pub potassium: Port<'b, Potassium>,
    pub magnesium: Port<'b, Magnesium>,
    pub calcium: Port<'b, Calcium>,
    pub chloride: Port<'b, Chloride>,
    pub lactate: Port<'b, Lactate>,
    pub ketones: Port<'b, Ketones>,
    pub diet_base: Port<'b, DietBase>,
    pub fixed_acid: Port<'b, FixedAcid>,
}

/// Dissolved carbon dioxide from bicarbonate and acidity, `HCO3 / 10^(pH - pK)`, and its partial pressure by Henry's
/// law. The host's logarithm is banned, so `ln 10` is a stated value.
pub(crate) struct Henry<'b> {
    solubility: Parameter<'b, BloodChemistry, Quotient<Concentration<CarbonDioxide>, Pressure>>,
    pk: Parameter<'b, BloodChemistry, Dimensionless>,
    ln_ten: Parameter<'b, BloodChemistry, Dimensionless>,
}

impl<'b> Henry<'b> {
    pub(crate) fn declare(laws: &mut FragmentBuilder<'b, BloodChemistry>) -> Self {
        let key = ParameterKey::new;
        Self {
            solubility: laws.parameter(key("bc.carbon-dioxide-solubility"), Admissible::positive()),
            pk: laws.parameter(key("bc.pk"), Admissible::finite()),
            ln_ten: laws.parameter(key("bc.ln-ten"), Admissible::positive()),
        }
    }

    pub(crate) fn dissolved<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        bicarbonate: Value<'b, 'k, Concentration<CarbonDioxide>>,
        acidity: Value<'b, 'k, Dimensionless>,
    ) -> Value<'b, 'k, Concentration<CarbonDioxide>> {
        let (pk, ln_ten) = (k.parameter(&self.pk), k.parameter(&self.ln_ten));
        let above = k.subtract(acidity, pk);
        let power = k.scale(above, ln_ten);
        let ratio = k.exp(power);
        let quotient = k.divide(bicarbonate, ratio);
        k.as_dimension(quotient)
    }

    pub(crate) fn pressure<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        dissolved: Value<'b, 'k, Concentration<CarbonDioxide>>,
    ) -> Value<'b, 'k, Pressure> {
        let solubility = k.parameter(&self.solubility);
        let quotient = k.divide(dissolved, solubility);
        k.as_dimension(quotient)
    }
}

/// A weak acid's charge after speciation, `c (a pH - b)`.
struct Weak<'b, Q> {
    slope: Parameter<'b, BloodChemistry, PerAcid<Q>>,
    offset: Parameter<'b, BloodChemistry, PerAcid<Q>>,
}

impl<'b, Q: Substance + Plain> Weak<'b, Q> {
    fn declare(laws: &mut FragmentBuilder<'b, BloodChemistry>, slope: &'static str, offset: &'static str) -> Self {
        Self {
            slope: laws.parameter(ParameterKey::new(slope), Admissible::finite()),
            offset: laws.parameter(ParameterKey::new(offset), Admissible::finite()),
        }
    }

    fn charge<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        acidity: Value<'b, 'k, Dimensionless>,
        held: Value<'b, 'k, Concentration<Q>>,
    ) -> Value<'b, 'k, Charges> {
        let (slope, offset) = (k.parameter(&self.slope), k.parameter(&self.offset));
        let sloped = k.scale(slope, acidity);
        let per = k.subtract(sloped, offset);
        let product = k.multiply(per, held);
        k.as_dimension(product)
    }
}

/// The plasma's charge balance and its carbon dioxide. Bicarbonate is the strong ion difference less the weak acids'
/// charge, `HCO3 = SID - A(pH) - c_Hb (a_Hb pH - b_Hb)`, where the strong ion difference is the strong and fixed ions'
/// charge plus a stated offset and `A` reads stated albumin and phosphate concentrations. Carbamino carbon dioxide is
/// `4 c_Hb D / (D + K)`, with `D` the dissolved carbon dioxide and the half-saturation `K` rising with saturation, so
/// deoxygenated blood holds more (research 11's carbamino row, with its oxygen dependence a line in saturation).
struct Acid<'b> {
    offset: Parameter<'b, BloodChemistry, Charges>,
    albumin: (Parameter<'b, BloodChemistry, Concentration<Albumin>>, Weak<'b, Albumin>),
    phosphate: (Parameter<'b, BloodChemistry, Concentration<Phosphate>>, Weak<'b, Phosphate>),
    hemoglobin: Weak<'b, Hemoglobin>,
    per_charge: Parameter<'b, BloodChemistry, Quotient<Concentration<CarbonDioxide>, Charges>>,
    sites: Parameter<'b, BloodChemistry, Quotient<Concentration<CarbonDioxide>, Concentration<Hemoglobin>>>,
    half: Parameter<'b, BloodChemistry, Concentration<CarbonDioxide>>,
    rise: Parameter<'b, BloodChemistry, Concentration<CarbonDioxide>>,
}

impl<'b> Acid<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, BloodChemistry>) -> Self {
        let key = ParameterKey::new;
        Self {
            offset: laws.parameter(key("bc.strong-ion-offset"), Admissible::finite()),
            albumin: (
                laws.parameter(key("bc.plasma-albumin"), Admissible::nonnegative()),
                Weak::declare(laws, "bc.albumin-slope", "bc.albumin-offset"),
            ),
            phosphate: (
                laws.parameter(key("bc.plasma-phosphate"), Admissible::nonnegative()),
                Weak::declare(laws, "bc.phosphate-slope", "bc.phosphate-offset"),
            ),
            hemoglobin: Weak::declare(laws, "bc.hemoglobin-slope", "bc.hemoglobin-offset"),
            per_charge: laws.parameter(key("bc.bicarbonate-per-charge"), Admissible::positive()),
            sites: laws.parameter(key("bc.carbamino-sites"), Admissible::positive()),
            half: laws.parameter(key("bc.carbamino-half"), Admissible::positive()),
            rise: laws.parameter(key("bc.carbamino-half-rise"), Admissible::nonnegative()),
        }
    }

    fn carbamino<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        dissolved: Value<'b, 'k, Concentration<CarbonDioxide>>,
        saturation: Value<'b, 'k, Dimensionless>,
        carrier: Value<'b, 'k, Concentration<Hemoglobin>>,
    ) -> Value<'b, 'k, Concentration<CarbonDioxide>> {
        let (half, rise) = (k.parameter(&self.half), k.parameter(&self.rise));
        let risen = k.scale(rise, saturation);
        let half = k.add(half, risen);
        let reach = k.add(dissolved, half);
        let quotient = k.divide(dissolved, reach);
        let share: Value<'b, 'k, Dimensionless> = k.as_dimension(quotient);
        let sites = k.parameter(&self.sites);
        let product = k.multiply(sites, carrier);
        let capacity: Value<'b, 'k, Concentration<CarbonDioxide>> = k.as_dimension(product);
        k.scale(capacity, share)
    }
}

fn concentration<'b, 'k, Q: Substance + Plain>(
    k: &mut Bed<'b, 'k>,
    port: &Port<'b, Q>,
    volume: Value<'b, 'k, Volume>,
) -> Value<'b, 'k, Concentration<Q>> {
    let amount = k.at_site(port);
    let quotient = k.divide(amount, volume);
    k.as_dimension(quotient)
}

fn charge<'b, 'k, Q: Substance + SameDimension<Charge>>(
    k: &mut Bed<'b, 'k>,
    port: &Port<'b, Q>,
) -> Value<'b, 'k, Charge> {
    let amount = k.at_site(port);
    k.as_dimension(amount)
}

impl<'b> StrongIons<'b> {
    fn difference<'k>(
        &self,
        k: &mut Bed<'b, 'k>,
        volume: Value<'b, 'k, Volume>,
    ) -> Value<'b, 'k, Charges> {
        let (sodium, potassium) = (charge(k, &self.sodium), charge(k, &self.potassium));
        let (magnesium, calcium) = (charge(k, &self.magnesium), charge(k, &self.calcium));
        let mut net = k.add(sodium, potassium);
        for divalent in [magnesium, magnesium, calcium, calcium] {
            net = k.add(net, divalent);
        }
        let anions = [
            charge(k, &self.chloride),
            charge(k, &self.lactate),
            charge(k, &self.ketones),
            charge(k, &self.diet_base),
            charge(k, &self.fixed_acid),
        ];
        for anion in anions {
            net = k.subtract(net, anion);
        }
        let quotient = k.divide(net, volume);
        k.as_dimension(quotient)
    }
}

/// Each bed's plasma bicarbonate and acidity as two fields that read each other. Acidity keeps its difference inside
/// the logarithm, `pH = pK + log10(HCO3 / (C_T - HCO3 - carbamino))`, so no row subtracts two near-equal numbers but
/// there, and a bicarbonate at or above what the total leaves has no acidity.
pub(crate) fn define<'b>(
    laws: &mut FragmentBuilder<'b, BloodChemistry>,
    plasma: &Plasma<'b>,
    speciated: &Stated<'b>,
    (bicarbonate_out, acidity_out): (
        &Field<'b, Concentration<CarbonDioxide>>,
        &Field<'b, Dimensionless>,
    ),
    class: RateClass,
) {
    let acid = Acid::declare(laws);
    let law = laws.site_kernel(|mut k| {
        let water = k.at_site(&plasma.water);
        let volume = k.volume(water);
        let acidity = k.at_site(&speciated.acidity);
        let strong = plasma.ions.difference(&mut k, volume);
        let offset = k.parameter(&acid.offset);
        let strong = k.add(strong, offset);
        let (albumin, weak) = &acid.albumin;
        let held = k.parameter(albumin);
        let albumin = weak.charge(&mut k, acidity, held);
        let (phosphate, weak) = &acid.phosphate;
        let held = k.parameter(phosphate);
        let phosphate = weak.charge(&mut k, acidity, held);
        let carrier = concentration(&mut k, &plasma.hemoglobin, volume);
        let hemoglobin = acid.hemoglobin.charge(&mut k, acidity, carrier);
        let weak = k.add(albumin, phosphate);
        let weak = k.add(weak, hemoglobin);
        let balance = k.subtract(strong, weak);
        let per_charge = k.parameter(&acid.per_charge);
        let product = k.multiply(per_charge, balance);
        let value: Value<'b, '_, Concentration<CarbonDioxide>> = k.as_dimension(product);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(bicarbonate_out, law);
    let henry = &speciated.henry;
    let law = laws.site_kernel(|mut k| {
        let water = k.at_site(&plasma.water);
        let volume = k.volume(water);
        let (bicarbonate, acidity) = (k.at_site(&speciated.bicarbonate), k.at_site(&speciated.acidity));
        let saturation = k.at_site(&speciated.saturation);
        let total = concentration(&mut k, &plasma.carbon_dioxide, volume);
        let dissolved = henry.dissolved(&mut k, bicarbonate, acidity);
        let carrier = concentration(&mut k, &plasma.hemoglobin, volume);
        let carbamino = acid.carbamino(&mut k, dissolved, saturation, carrier);
        let held = k.add(bicarbonate, carbamino);
        let left = k.subtract(total, held);
        let quotient = k.divide(bicarbonate, left);
        let ratio: Value<'b, '_, Dimensionless> = k.as_dimension(quotient);
        let log = k.ln(ratio);
        let ln_ten = k.parameter(&henry.ln_ten);
        let quotient = k.divide(log, ln_ten);
        let decades: Value<'b, '_, Dimensionless> = k.as_dimension(quotient);
        let pk = k.parameter(&henry.pk);
        let value = k.add(pk, decades);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(acidity_out, law);
}
