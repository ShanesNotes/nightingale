//! The blood's cells: viscosity from the hematocrit, which the vessels read on coupling arrow 114. Hemoglobin's
//! production and destruction stay held stand-ins, so the root hands this crate the pool production reaches.
//!
//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_hematology::viscosity::viscosity;
//! ```
//! ```
//! use ng_hematology::declare;
//! ```

use ng_contract::*;

mod viscosity;

const VISCOSITY_NS: u64 = 1_000_000_000;

/// Defines viscosity (line 114) from the hematocrit of the pool hemoglobin production reaches, the central veins.
pub fn declare<'b>(
    scope: HematologyScope<'b>,
    production: &BoundaryRoute<'b, Hemoglobin, Hematology>,
) -> Result<Fragment<'b, Hematology>, AdmissionError> {
    let (mut hc, _ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, VISCOSITY_NS)?;
    viscosity::viscosity(&mut hc, &production.route, class);
    Ok(hc.finish())
}
