//! Blood chemistry's speciation: each bed's plasma oxygen partial pressure and saturation, and its bicarbonate and
//! acidity, from the plasma's oxygen, carbon dioxide, hemoglobin, strong ions and water. Blood chemistry's ports read
//! no amount yet, so the forms are written over any owner, and the body's root hands this crate the builder of the
//! system that reads those amounts.
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
//! use ng_blood_chemistry::{Speciated, speciate};
//! ```

mod acidity;
mod oxygen;

pub use acidity::StrongIons;
use ng_contract::*;

type Port<'b, O, U> = ReadPort<'b, O, PerSite<CapillaryBed, U>>;
type Bed<'b, 'k, O> = Kernel<'b, 'k, O, CapillaryBed>;
type Value<'b, 'k, O, U> = Expr<'b, 'k, O, U>;
type Field<'b, O, U> = Output<'b, O, PerSite<CapillaryBed, U>>;

const SPECIATION_NS: u64 = 1_000_000_000;

pub struct Plasma<'b, O> {
    pub water: Port<'b, O, Water>,
    pub oxygen: Port<'b, O, Oxygen>,
    pub carbon_dioxide: Port<'b, O, CarbonDioxide>,
    pub hemoglobin: Port<'b, O, Hemoglobin>,
    pub ions: StrongIons<'b, O>,
}

/// What speciation states at each bed's plasma: four fields, and the carbon dioxide partial pressure they give.
pub struct Speciated<'b, O> {
    pub oxygen_partial_pressure: Port<'b, O, Pressure>,
    pub saturation: Port<'b, O, Dimensionless>,
    pub bicarbonate: Port<'b, O, Concentration<CarbonDioxide>>,
    pub acidity: Port<'b, O, Dimensionless>,
    henry: acidity::Henry<'b, O>,
}

impl<'b, O: Owner> Speciated<'b, O> {
    /// `P = HCO3 / (alpha 10^(pH - pK))`, explicit from the two acid fields, so it is no unknown of its own.
    pub fn carbon_dioxide_partial_pressure<'k>(&self, k: &mut Bed<'b, 'k, O>) -> Value<'b, 'k, O, Pressure> {
        let (bicarbonate, acidity) = (k.at_site(&self.bicarbonate), k.at_site(&self.acidity));
        let dissolved = self.henry.dissolved(k, bicarbonate, acidity);
        self.henry.pressure(k, dissolved)
    }
}

/// States each bed's plasma speciation as four fields that read each other: the oxygen partial pressure and
/// saturation, whose half-saturation pressure reads the plasma's acidity and carbon dioxide, and the bicarbonate and
/// acidity, whose carbamino term reads the saturation.
pub fn speciate<'b, O: Owner>(
    laws: &mut FragmentBuilder<'b, O>,
    plasma: &Plasma<'b, O>,
) -> Result<Speciated<'b, O>, AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, SPECIATION_NS)?;
    let (pressure_out, oxygen_partial_pressure) =
        laws.site_field::<CapillaryBed, Pressure>("oxygen partial pressure", Admissible::positive());
    let (saturation_out, saturation) =
        laws.site_field::<CapillaryBed, Dimensionless>("oxygen saturation", Admissible::unit_interval());
    let (bicarbonate_out, bicarbonate) =
        laws.site_field::<CapillaryBed, Concentration<CarbonDioxide>>("bicarbonate", Admissible::positive());
    let (acidity_out, acidity) =
        laws.site_field::<CapillaryBed, Dimensionless>("acidity", Admissible::finite());
    let speciated = Speciated {
        oxygen_partial_pressure,
        saturation,
        bicarbonate,
        acidity,
        henry: acidity::Henry::declare(laws),
    };
    oxygen::define(laws, plasma, &speciated, (&pressure_out, &saturation_out), class);
    acidity::define(laws, plasma, &speciated, (&bicarbonate_out, &acidity_out), class);
    Ok(speciated)
}
