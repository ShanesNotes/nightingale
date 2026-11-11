//! Binding of a transmitter or hormone at the sites that hold it, and the effect that binding has. A receptor is a site
//! parameter, its total, so binding is a two-ended transfer of the ligand and its moiety closes by pairing. The schema's
//! `Bound` pools say where each ligand binds, so beta-1, alpha-1, M2 and V2 are one law read at different sites.
//!
//! # Type rules
//!
//! A binding is no reaction written from raw terms: a transfer takes its pair from a scope, so a law cannot forge one.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn forged<'b>(rate: PureKernel<'b, Receptor, Rate<Norepinephrine>>) -> Transfer<'b, Receptor, Norepinephrine> {
//!     Transfer { serves: todo!(), rate }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn paired<'b>(
//!     pair: &Pair<'b, Receptor, Norepinephrine>,
//!     rate: PureKernel<'b, Receptor, Rate<Norepinephrine>>,
//! ) -> Transfer<'b, Receptor, Norepinephrine> {
//!     Transfer::new(pair, rate)
//! }
//! ```

use ng_contract::*;

mod binding_acetylcholine;
mod binding_norepinephrine;
mod binding_vasopressin;
mod effect_acetylcholine;
mod effect_norepinephrine;
mod effect_vasopressin;
mod shared;

use shared::sites;

const BINDING_NS: u64 = 1_000_000_000;

pub fn declare<'b>(scope: ReceptorScope<'b>) -> Result<Fragment<'b, Receptor>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, BINDING_NS)?;
    let (route, total) = sites::<Junction, Norepinephrine>(&mut laws, &ports, "rc.ne-sites");
    binding_norepinephrine::declare(&mut laws, &route, &total, class);
    effect_norepinephrine::declare(&mut laws, &ports, &route, &total, class);
    let (route, total) = sites::<Junction, Acetylcholine>(&mut laws, &ports, "rc.ach-sites");
    binding_acetylcholine::declare(&mut laws, &route, &total, class);
    effect_acetylcholine::declare(&mut laws, &ports, &route, &total, class);
    let (route, total) = sites::<Blood, Vasopressin>(&mut laws, &ports, "rc.avp-sites");
    binding_vasopressin::declare(&mut laws, &route, &total, class);
    effect_vasopressin::declare(&mut laws, &ports, &route, &total, class);
    Ok(laws.finish())
}
