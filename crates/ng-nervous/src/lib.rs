//! The minimal baroreflex: stretch through a filter, a set point that follows it slowly, traffic out by a logistic, and
//! transmitter released and taken back at every junction that holds it. No law writes a heart rate or a
//! resistance.

use ng_contract::*;

mod afferent_filter;
mod baroreflex_set_point;
mod release_acetylcholine;
mod release_norepinephrine;
mod reuptake_acetylcholine;
mod reuptake_norepinephrine;
mod shared;
mod sympathetic_traffic;
mod vagal_traffic;

const REFLEX_NS: u64 = 1_000_000_000;
const SET_POINT_NS: u64 = 3_600_000_000_000;

pub fn declare<'b>(scope: NervousScope<'b>) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    let (mut ns, ports) = scope.parts();
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    let slow = RateClass::new(RateBand::Hours, SET_POINT_NS)?;
    let filtered = ns.state::<Dimensionless>("afferent filter", Admissible::finite());
    let set_point = ns.set_point::<Dimensionless>("baroreflex set point", Admissible::finite());
    afferent_filter::declare(&mut ns, &filtered, reflex);
    baroreflex_set_point::declare(&mut ns, &filtered, &set_point, slow);
    let sympathetic = sympathetic_traffic::declare(&mut ns, &filtered, &set_point, reflex);
    let vagal = vagal_traffic::declare(&mut ns, &filtered, &set_point, reflex);
    release_norepinephrine::declare(&mut ns, &ports, &sympathetic, reflex);
    reuptake_norepinephrine::declare(&mut ns, &ports, reflex);
    release_acetylcholine::declare(&mut ns, &ports, &vagal, reflex);
    reuptake_acetylcholine::declare(&mut ns, &ports, reflex);
    Ok(ns.finish())
}
