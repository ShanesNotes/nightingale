//! Blood chemistry's speciation: at every speciated site, the blood's oxygen partial pressure and saturation, its
//! bicarbonate and acidity, and its carbon dioxide partial pressure, from the blood's oxygen, carbon dioxide,
//! hemoglobin, strong ions and water, read through blood chemistry's own port.
//!
//! # Type rules
//!
//! A neighbouring system reads what speciation states, never its forms.
//! ```compile_fail,E0603
//! use ng_blood_chemistry::oxygen::Curve;
//! ```
//! ```compile_fail,E0603
//! use ng_blood_chemistry::acidity::Henry;
//! ```
//! ```
//! use ng_blood_chemistry::speciate;
//! ```

mod acidity;
mod oxygen;

use acidity::StrongIons;
use ng_contract::*;

type Port<'b, U> = ReadPort<'b, BloodChemistry, PerSite<Speciated, U>>;
type Bed<'b, 'k> = Kernel<'b, 'k, BloodChemistry, Speciated>;
type Value<'b, 'k, U> = Expr<'b, 'k, BloodChemistry, U>;
type Field<'b, U> = Output<'b, BloodChemistry, PerSite<Speciated, U>>;

const SPECIATION_NS: u64 = 1_000_000_000;

struct Plasma<'b> {
    water: Port<'b, Water>,
    oxygen: Port<'b, Oxygen>,
    carbon_dioxide: Port<'b, CarbonDioxide>,
    hemoglobin: Port<'b, Hemoglobin>,
    ions: StrongIons<'b>,
}

/// Blood chemistry's own reads of what it states, and the Henry's law terms its carbon dioxide partial pressure uses.
struct Stated<'b> {
    oxygen_partial_pressure: Port<'b, Pressure>,
    saturation: Port<'b, Dimensionless>,
    bicarbonate: Port<'b, Concentration<CarbonDioxide>>,
    acidity: Port<'b, Dimensionless>,
    carbon_dioxide_partial_pressure: Port<'b, Pressure>,
    henry: acidity::Henry<'b>,
}

/// States speciation at every speciated site: four rows that read each other, the oxygen partial pressure and
/// saturation, whose half-saturation pressure reads the blood's acidity and carbon dioxide, and the bicarbonate and
/// acidity, whose carbamino term reads the saturation; and the carbon dioxide partial pressure,
/// `P = HCO3 / (alpha 10^(pH - pK))`, explicit from the two acid rows, so it is no unknown of its own.
pub fn speciate<'b>(
    laws: &mut FragmentBuilder<'b, BloodChemistry>,
    ports: &BloodChemistryPorts<'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, SPECIATION_NS)?;
    let plasma = Plasma {
        water: ports.blood(),
        oxygen: ports.blood(),
        carbon_dioxide: ports.blood(),
        hemoglobin: ports.blood(),
        ions: StrongIons {
            sodium: ports.blood(),
            potassium: ports.blood(),
            magnesium: ports.blood(),
            calcium: ports.blood(),
            chloride: ports.blood(),
            lactate: ports.blood(),
            ketones: ports.blood(),
            diet_base: ports.blood(),
            fixed_acid: ports.blood(),
        },
    };
    let outputs = ports.speciation(laws);
    let speciated = Stated {
        oxygen_partial_pressure: outputs.oxygen_partial_pressure.1,
        saturation: outputs.oxygen_saturation.1,
        bicarbonate: outputs.bicarbonate.1,
        acidity: outputs.acidity.1,
        carbon_dioxide_partial_pressure: outputs.carbon_dioxide_partial_pressure.1,
        henry: acidity::Henry::declare(laws),
    };
    let henry = &speciated.henry;
    laws.express_at(&outputs.carbon_dioxide_partial_pressure.0, |mut k| {
        let (bicarbonate, acidity) = (k.at_site(&speciated.bicarbonate), k.at_site(&speciated.acidity));
        let dissolved = henry.dissolved(&mut k, bicarbonate, acidity);
        let value = henry.pressure(&mut k, dissolved);
        k.finish_term(value)
    });
    let pressure = (&outputs.oxygen_partial_pressure.0, &outputs.oxygen_saturation.0);
    oxygen::define(laws, &plasma, &speciated, pressure, class);
    let acid = (&outputs.bicarbonate.0, &outputs.acidity.0);
    acidity::define(laws, &plasma, &speciated, acid, class);
    Ok(())
}
