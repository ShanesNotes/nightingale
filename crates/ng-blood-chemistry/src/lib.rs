//! Blood chemistry's speciation: each bed's plasma oxygen partial pressure and saturation from the plasma's oxygen,
//! hemoglobin and water. Blood chemistry's ports read no amount yet, so the forms are written over any owner, and the
//! body's root hands this crate the builder of the system that reads those amounts.
//!
//! # Type rules
//!
//! A neighbouring system reads what speciation states, never its forms.
//! ```compile_fail,E0603
//! use ng_blood_chemistry::oxygen::Curve;
//! ```
//! ```
//! use ng_blood_chemistry::oxygenate;
//! ```

mod oxygen;

pub use oxygen::{Oxygenated, Plasma, oxygenate};
