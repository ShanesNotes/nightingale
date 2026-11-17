//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_metabolism::diffusion::Side;
//! ```
//! ```
//! use ng_metabolism::{CellOxygen, GasRoutes, bed_heat, burn, carbon_dioxide_diffusion, oxygen_diffusion};
//! ```

mod burn;
mod diffusion;

pub use burn::{bed_heat, burn};
pub use diffusion::{CellOxygen, GasRoutes, PlasmaPressure, carbon_dioxide_diffusion, oxygen_diffusion};
