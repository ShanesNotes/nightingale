//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_metabolism::diffusion::Side;
//! ```
//! ```
//! use ng_metabolism::{GasRoutes, carbon_dioxide_diffusion, oxygen_diffusion};
//! ```

mod diffusion;

pub use diffusion::{GasRoutes, PlasmaPressure, carbon_dioxide_diffusion, oxygen_diffusion};
