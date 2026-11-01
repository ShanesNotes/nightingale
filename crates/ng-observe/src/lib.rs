//! Observers: windows, delays, measurement error and observations through a lens, one revision at a time.
//!
//! # Type rules
//!
//! Seed domains never mix: an observer cannot draw measurement error from the run seed.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! use ng_observe::Observer;
//! fn noisy<'l>(lens: &'l ObservationLens, run: Seed) -> Observer<'l> { Observer::new(lens, run) }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_observe::Observer;
//! fn noisy<'l>(lens: &'l ObservationLens, seed: ObservationSeed) -> Observer<'l> { Observer::new(lens, seed) }
//! ```
//! An observation comes only from an observer.
//! ```compile_fail,E0451
//! use ng_contract::SimTime;
//! use ng_observe::*;
//! fn forged(at: SimTime, value: ObservedValue) -> Observation {
//!     Observation { channel: 0, collected: at, available: at, value }
//! }
//! ```
//! ```
//! use ng_contract::SimTime;
//! use ng_observe::*;
//! fn read(seen: &Observation) -> (usize, SimTime, SimTime, ObservedValue) {
//!     (seen.channel(), seen.collected(), seen.available(), seen.value())
//! }
//! ```
//! A typed absence is never a zero: a host must say what it does with one.
//! ```compile_fail,E0308
//! use ng_contract::Compensated;
//! use ng_observe::ObservedValue;
//! fn number(value: ObservedValue) -> Compensated { value }
//! ```
//! ```
//! use ng_contract::Compensated;
//! use ng_observe::ObservedValue;
//! fn number(value: ObservedValue) -> Option<Compensated> {
//!     match value {
//!         ObservedValue::Present(value) => Some(value),
//!         ObservedValue::NotCollected | ObservedValue::Pending | ObservedValue::InvalidSignal => None,
//!     }
//! }
//! ```

mod checkpoint;
mod observer;

pub use checkpoint::ObserverCheckpoint;
pub use observer::{Observation, ObservedValue, Observer, chained};
