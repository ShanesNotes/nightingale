//! The minimal baroreflex: stretch through a filter, a set point that follows it slowly, traffic out by a logistic, and
//! transmitter released and taken back at every junction that holds it. No law writes a rate or a resistance.

use ng_contract::catalog::{BaroreceptorAfferents, Reads};
use ng_contract::*;

const REFLEX_NS: u64 = 1_000_000_000;
const SET_POINT_NS: u64 = 3_600_000_000_000;

/// Research 02's release `q_i f`, linear in traffic, with `q_i = q A_vesicle`, so a store can run down and never below
/// zero. Model adaptation. Reuptake is first order.
fn transmitter<'b, Q: Substance>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    traffic: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    keys: [&'static str; 2],
    class: RateClass,
) {
    let [release_key, reuptake_key] = keys;
    let release = ports.release::<Q>(ns);
    let reuptake = ports.reuptake::<Q>(ns);
    let released = ns.site_parameter::<Junction, Rate<Dimensionless>>(
        ParameterKey::new(release_key),
        Admissible::positive(),
    );
    let taken = ns.site_parameter::<Junction, Rate<Dimensionless>>(
        ParameterKey::new(reuptake_key),
        Admissible::positive(),
    );
    let out = ns.site_kernel(|mut k| {
        let store = k.at_site(&release.from_amount::<Q>());
        let (zero, speed) = (k.zero(), k.site_parameter(&released));
        let emptying = k.relax(store, zero, speed);
        let firing = k.algebraic(traffic);
        let rate = k.scale(emptying, firing);
        k.finish(rate, ns.every_mode(), class)
    });
    ns.transfer_at(&release, out);
    let back = ns.site_kernel(|mut k| {
        let free = k.at_site(&reuptake.from_amount::<Q>());
        let (zero, speed) = (k.zero(), k.site_parameter(&taken));
        let rate = k.relax(free, zero, speed);
        k.finish(rate, ns.every_mode(), class)
    });
    ns.transfer_at(&reuptake, back);
}

/// A logistic of how far the filter stands from the set point, shifted so the body rests low on the curve and traffic
/// has room to rise. Model default.
fn traffic<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    filtered: &StateSlot<'b, Nervous, Dimensionless>,
    set_point: &StateSlot<'b, Nervous, Reference<Dimensionless>>,
    (name, width_key, rest_key, falls): (&'static str, &'static str, &'static str, bool),
    class: RateClass,
) -> AlgebraicSlot<'b, Nervous, Dimensionless> {
    let row = ns.algebraic::<Dimensionless>(name, Admissible::unit_interval());
    let width =
        ns.parameter::<Deviation<Dimensionless>>(ParameterKey::new(width_key), Admissible::positive());
    let rest = ns.parameter::<Dimensionless>(ParameterKey::new(rest_key), Admissible::finite());
    let residual = ns.kernel(|mut k| {
        let (measured, reference) = (k.state(filtered), k.state(set_point));
        let off = k.deviation(measured, reference);
        let scale = k.parameter(&width);
        let ratio = k.divide(off, scale);
        let rising = k.as_dimension(ratio);
        let zero = k.zero();
        let drive = if falls { k.subtract(zero, rising) } else { rising };
        let resting = k.parameter(&rest);
        let shifted = k.add(drive, resting);
        let value = k.logistic(shifted);
        let held = k.algebraic(&row);
        let residual = k.subtract(held, value);
        k.finish(residual, ns.every_mode(), class)
    });
    ns.residual(&row, residual);
    row
}

pub fn declare<'b>(scope: NervousScope<'b>) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    let (mut ns, ports) = scope.parts();
    let reflex = RateClass::new(RateBand::Reflex, REFLEX_NS)?;
    let slow = RateClass::new(RateBand::Hours, SET_POINT_NS)?;
    let afferents: BaroreceptorAfferents<Reads<'_, Nervous>> = ns.receive();
    let filtered = ns.state::<Dimensionless>("afferent filter", Admissible::finite());
    let set_point = ns.set_point::<Dimensionless>("baroreflex set point", Admissible::finite());
    let pace =
        ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.filter-speed"), Admissible::positive());
    let carotid =
        ns.parameter::<Dimensionless>(ParameterKey::new("ns.carotid-share"), Admissible::unit_interval());
    let arch = ns.parameter::<Dimensionless>(ParameterKey::new("ns.arch-share"), Admissible::unit_interval());
    let drift =
        ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.set-point-speed"), Admissible::positive());

    let relax = ns.kernel(|mut k| {
        let (first, second) = (
            k.read(&afferents.carotid_sinus_stretch),
            k.read(&afferents.aortic_arch_stretch),
        );
        let (one, two) = (k.parameter(&carotid), k.parameter(&arch));
        let (first, second) = (k.scale(first, one), k.scale(second, two));
        let target = k.add(first, second);
        let (current, speed) = (k.state(&filtered), k.parameter(&pace));
        let rate = k.relax(target, current, speed);
        k.finish(rate, ns.every_mode(), reflex)
    });
    ns.memory_rate(&filtered, relax);

    let following = ns.kernel(|mut k| {
        let (measured, reference) = (k.state(&filtered), k.state(&set_point));
        let off = k.deviation(measured, reference);
        let speed = k.parameter(&drift);
        let product = k.multiply(off, speed);
        let rate = k.as_dimension(product);
        k.finish(rate, ns.every_mode(), slow)
    });
    ns.memory_rate(&set_point, following);

    let sympathetic = traffic(
        &mut ns,
        &filtered,
        &set_point,
        (
            "sympathetic traffic",
            "ns.sympathetic-width",
            "ns.sympathetic-rest",
            true,
        ),
        reflex,
    );
    let vagal = traffic(
        &mut ns,
        &filtered,
        &set_point,
        ("vagal traffic", "ns.vagal-width", "ns.vagal-rest", false),
        reflex,
    );
    transmitter::<Norepinephrine>(
        &mut ns,
        &ports,
        &sympathetic,
        ["ns.release-ne", "ns.reuptake-ne"],
        reflex,
    );
    transmitter::<Acetylcholine>(
        &mut ns,
        &ports,
        &vagal,
        ["ns.release-ach", "ns.reuptake-ach"],
        reflex,
    );
    Ok(ns.finish())
}
