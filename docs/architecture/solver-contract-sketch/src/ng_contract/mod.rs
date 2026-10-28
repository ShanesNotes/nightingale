//! Proposed ng-contract: organ laws as data, one compiled body, one numerical seam.
//!
//! Organ crates return [`Fragment`] values built from capabilities their scope grants. [`assemble`]
//! checks every fragment into one immutable [`CompiledBody`]. A solver reaches that body only
//! through [`CoupledProblem`], so no organ code runs inside a solve, and every value a solver core
//! produces or consumes has a public producer in this module. Declarations carry capabilities,
//! never storage indices or a record reference.
//!
//! Submodules group one body of knowledge each; everything except the generated coupling
//! [`catalog`] is re-exported here.

use core::marker::PhantomData;

mod admission;
mod anatomy;
mod body;
mod boundary;
pub mod catalog;
mod conservation;
mod failure;
mod identity;
mod law;
mod observation;
mod problem;
mod provenance;
mod quantity;
mod scope;
mod seam;
mod split;
mod system;

pub use admission::*;
pub use anatomy::*;
pub use body::*;
pub use boundary::*;
pub use conservation::*;
pub use failure::*;
pub use identity::*;
pub use law::*;
pub use observation::*;
pub use problem::*;
pub use provenance::*;
pub use quantity::*;
pub use scope::*;
pub use seam::*;
pub use split::*;
pub use system::*;

/// A model-local lifetime is invariant, so declarations from two assemblies cannot mix.
pub struct Brand<'b>(PhantomData<fn(&'b ()) -> &'b ()>);

/// Only the record's registry owns pools. This type is intentionally private: no organ can name it.
struct PoolRef {
    _slot: u32,
}
