//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_respiratory::airway::Gas;
//! ```
//! ```
//! use ng_respiratory::{Plasma, Ventilation, exchange};
//! ```
//!
//! The membrane reads the plasma of the pulmonary bed alone: a law over every bed cannot read the lung's gases.
//! ```compile_fail,E0308
//! use ng_contract::catalog::{LungGases, Reads};
//! use ng_contract::*;
//! fn read<'b, 'k>(
//!     k: &mut Kernel<'b, 'k, Respiratory, CapillaryBed>,
//!     gases: &LungGases<Reads<'b, Respiratory>>,
//! ) -> Expr<'b, 'k, Respiratory, Pressure> {
//!     k.at_site(&gases.oxygen_partial_pressure)
//! }
//! ```
//! ```
//! use ng_contract::catalog::{LungGases, Reads};
//! use ng_contract::*;
//! fn read<'b, 'k>(
//!     k: &mut Kernel<'b, 'k, Respiratory, Beds<Lung>>,
//!     gases: &LungGases<Reads<'b, Respiratory>>,
//! ) -> Expr<'b, 'k, Respiratory, Pressure> {
//!     k.at_site(&gases.oxygen_partial_pressure)
//! }
//! ```

mod airway;
mod membrane;

pub use airway::Ventilation;
pub use membrane::{Plasma, exchange};
