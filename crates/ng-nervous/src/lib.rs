//! The minimal baroreflex: stretch through a filter, a set point that follows it slowly, traffic out by a logistic, and
//! transmitter released and taken back at every junction that holds it. No law writes a heart rate, a resistance or the
//! pump's force. In the whole body the card also owns the breath, whose drive is released at the respiratory pump's
//! junction and whose rate the chemoreflex's drive scale sets, and the thermostat, whose sudomotor and constrictor
//! commands are released at the skin junction and whose shiver command becomes arrow 87's shiver drive.

use ng_contract::*;

mod afferent_filter;
mod baroreflex_set_point;
mod breath;
mod chemoreflex;
mod release_acetylcholine;
mod release_norepinephrine;
mod reuptake_acetylcholine;
mod reuptake_norepinephrine;
mod shared;
mod sympathetic_traffic;
mod thermostat;
mod vagal_traffic;

const REFLEX_NS: u64 = 1_000_000_000;
const SET_POINT_NS: u64 = 3_600_000_000_000;

/// The slice's card: the baroreflex alone.
pub fn declare<'b>(scope: NervousScope<'b>) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    declare_with(scope, false)
}

/// As `declare`, and the card owns the breath. Only the whole body's root calls it, so the slice keeps its model.
pub fn declare_in_the_whole_body<'b>(
    scope: NervousScope<'b>,
) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    declare_with(scope, true)
}

fn declare_with<'b>(
    scope: NervousScope<'b>,
    whole_body: bool,
) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    let (mut ns, ports) = scope.parts();
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    let slow = RateClass::new(RateBand::Hours, SET_POINT_NS)?;
    let filtered = ns.state::<Dimensionless>("afferent filter", Admissible::finite());
    let set_point = ns.set_point::<Dimensionless>("baroreflex set point", Admissible::finite());
    afferent_filter::declare(&mut ns, &filtered, reflex);
    baroreflex_set_point::declare(&mut ns, &filtered, &set_point, slow);
    let sympathetic = sympathetic_traffic::declare(&mut ns, &filtered, &set_point, reflex);
    let vagal = vagal_traffic::declare(&mut ns, &filtered, &set_point, reflex);
    if whole_body {
        let commands = thermostat::declare(&mut ns, reflex, slow);
        let traffics = (&sympathetic, &commands.constrictor);
        release_norepinephrine::declare_with_the_constrictor(&mut ns, &ports, traffics, reflex);
        reuptake_norepinephrine::declare(&mut ns, &ports, reflex);
        let scale = chemoreflex::declare(&mut ns, reflex, slow);
        breathing_with(&mut ns, &ports, (&vagal, &scale), Some(&commands.sudomotor))?;
    } else {
        release_norepinephrine::declare(&mut ns, &ports, &sympathetic, reflex);
        reuptake_norepinephrine::declare(&mut ns, &ports, reflex);
        release_acetylcholine::declare(&mut ns, &ports, &vagal, reflex);
        reuptake_acetylcholine::declare(&mut ns, &ports, reflex);
    }
    Ok(ns.finish())
}

/// The breath at the drive scale's rate, and acetylcholine released at every junction that holds it by the vagal
/// traffic and the pump drive in that junction's shares, and taken back. A fixture with no baroreflex states its own
/// vagal traffic, and one with no sensors its own drive scale.
pub fn breathing<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    (vagal, scale): (
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
    ),
) -> Result<(), AdmissionError> {
    breathing_with(ns, ports, (vagal, scale), None)
}

/// As `breathing`, and in the whole body acetylcholine's release also answers the thermostat's sudomotor command in
/// each junction's share.
fn breathing_with<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    (vagal, scale): (
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
    ),
    sudomotor: Option<&AlgebraicSlot<'b, Nervous, Dimensionless>>,
) -> Result<(), AdmissionError> {
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    let drive = breath::breathe(ns, scale)?;
    release_acetylcholine::declare_with_the_pump(ns, ports, (vagal, &drive), sudomotor, reflex);
    reuptake_acetylcholine::declare(ns, ports, reflex);
    Ok(())
}

/// Arrow 87's shiver drive from a shiver command, for a fixture that states its own command.
pub fn shiver_drive<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    command: &AlgebraicSlot<'b, Nervous, Dimensionless>,
) -> Result<(), AdmissionError> {
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    thermostat::shiver_drive(ns, command, reflex);
    Ok(())
}

/// The chemoreflex alone, for a fixture that stages what blood chemistry states at the two chemoreceptors: its drive
/// scale, which `breathing` takes.
pub fn chemoreflex<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
) -> Result<AlgebraicSlot<'b, Nervous, Dimensionless>, AdmissionError> {
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    let slow = RateClass::new(RateBand::Hours, SET_POINT_NS)?;
    Ok(chemoreflex::declare(ns, reflex, slow))
}
