//! Metabolism's gas laws: oxygen's wall diffusion across each bed's capillary wall and cell membrane, a rate on the
//! difference of the partial pressures either side. The use in cells stays a held stand-in, so the root declares it.
//!
//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_metabolism::diffusion::Side;
//! ```
//! ```
//! use ng_metabolism::{OxygenRoutes, wall_diffusion};
//! ```

mod diffusion;

pub use diffusion::{OxygenRoutes, wall_diffusion};
