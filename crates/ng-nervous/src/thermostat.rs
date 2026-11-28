use ng_contract::catalog::{CoreTemperature, Defines, Reads, ShiverDrive, SkinThermoreceptor};
use ng_contract::*;

/// The thermostat's commands that release a transmitter at the skin junction.
pub(crate) struct Commands<'b> {
    pub(crate) sudomotor: AlgebraicSlot<'b, Nervous, Dimensionless>,
    pub(crate) constrictor: AlgebraicSlot<'b, Nervous, Dimensionless>,
}

struct Sensed<'b> {
    core: CoreTemperature<Reads<'b, Nervous>>,
    skin: SkinThermoreceptor<'b>,
    set_point: StateSlot<'b, Nervous, Reference<Temperature>>,
    reference: Parameter<'b, Nervous, Reference<Temperature>>,
    width: Parameter<'b, Nervous, Dimensionless>,
}

#[derive(Clone, Copy)]
enum Side {
    Warm,
    Cold,
}

/// A command's row and the keys of its core width `D_c`, skin width `D_s` and threshold `theta`.
#[derive(Clone, Copy)]
struct Command {
    row: &'static str,
    core_width: &'static str,
    skin_width: &'static str,
    threshold: &'static str,
}

const SUDOMOTOR: Command = Command {
    row: "sudomotor command",
    core_width: "ns.sudomotor-core-width",
    skin_width: "ns.sudomotor-skin-width",
    threshold: "ns.sudomotor-threshold",
};
const CONSTRICTOR: Command = Command {
    row: "constrictor command",
    core_width: "ns.constrictor-core-width",
    skin_width: "ns.constrictor-skin-width",
    threshold: "ns.constrictor-threshold",
};
const SHIVER: Command = Command {
    row: "shiver command",
    core_width: "ns.shiver-core-width",
    skin_width: "ns.shiver-skin-width",
    threshold: "ns.shiver-threshold",
};

/// The core's set point `T_set`, memory that moves toward its target, `dT_set/dt = k_set deviation(T_target, T_set)`,
/// with `T_target = M` until the clock and sleep move it, and three commands, each a weighted sum of the core's and the
/// skin thermoreceptor's deviations against a threshold of its own: warm `smooth_max(e_c / D_c + e_s / D_s - theta, 0,
/// w)` and cold `smooth_max(theta - e_c / D_c - e_s / D_s, 0, w)`, `e_c` the core against the set point and `e_s` the
/// skin against the fixed skin reference (research 12, the rows 'The thermostat' and 'The set point'). The shiver
/// command becomes arrow 87's shiver drive (the row 'Shivering').
pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    reflex: RateClass,
    slow: RateClass,
) -> Commands<'b> {
    let key = ParameterKey::new;
    let sensed = Sensed {
        core: ns.receive(),
        skin: ns.receive(),
        set_point: ns.set_point::<Temperature>("thermostat set point", Admissible::finite()),
        reference: ns.parameter(key("ns.skin-reference"), Admissible::finite()),
        width: ns.parameter(key("ns.command-width"), Admissible::positive()),
    };
    let target = ns.parameter::<Temperature>(key("ns.thermostat-target"), Admissible::finite());
    let speed = ns.parameter::<Rate<Dimensionless>>(key("ns.thermostat-speed"), Admissible::positive());
    let following = ns.kernel(|mut k| {
        let (target, current) = (k.parameter(&target), k.state(&sensed.set_point));
        let off = k.deviation(target, current);
        let speed = k.parameter(&speed);
        let product = k.multiply(off, speed);
        let rate = k.as_dimension(product);
        k.finish(rate, ns.every_mode(), slow)
    });
    ns.memory_rate(&sensed.set_point, following);
    let sudomotor = command(ns, &sensed, (Side::Warm, SUDOMOTOR), reflex);
    let constrictor = command(ns, &sensed, (Side::Cold, CONSTRICTOR), reflex);
    let shiver = command(ns, &sensed, (Side::Cold, SHIVER), reflex);
    shiver_drive(ns, &shiver, reflex);
    Commands {
        sudomotor,
        constrictor,
    }
}

fn command<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    sensed: &Sensed<'b>,
    (side, keys): (Side, Command),
    class: RateClass,
) -> AlgebraicSlot<'b, Nervous, Dimensionless> {
    let key = ParameterKey::new;
    let row = ns.algebraic::<Dimensionless>(keys.row, Admissible::nonnegative());
    let core_width = ns.parameter::<Deviation<Temperature>>(key(keys.core_width), Admissible::positive());
    let skin_width = ns.parameter::<Deviation<Temperature>>(key(keys.skin_width), Admissible::positive());
    let threshold = ns.parameter::<Dimensionless>(key(keys.threshold), Admissible::finite());
    let residual = ns.kernel(|mut k| {
        let (core, set_point) = (k.read(&sensed.core.core_temperature), k.state(&sensed.set_point));
        let core_off = k.deviation(core, set_point);
        let (skin, reference) = (sensed.skin.temperature(&mut k), k.parameter(&sensed.reference));
        let skin_off = k.deviation(skin, reference);
        let (core_width, skin_width) = (k.parameter(&core_width), k.parameter(&skin_width));
        let ratio = k.divide(core_off, core_width);
        let from_core: Expr<'_, '_, Nervous, Dimensionless> = k.as_dimension(ratio);
        let ratio = k.divide(skin_off, skin_width);
        let from_skin = k.as_dimension(ratio);
        let load = k.add(from_core, from_skin);
        let threshold = k.parameter(&threshold);
        let past = match side {
            Side::Warm => k.subtract(load, threshold),
            Side::Cold => k.subtract(threshold, load),
        };
        let (zero, width) = (k.zero(), k.parameter(&sensed.width));
        let value = k.smooth_max(past, zero, width);
        let held = k.algebraic(&row);
        let residual = k.subtract(held, value);
        k.finish(residual, ns.every_mode(), class)
    });
    ns.residual(&row, residual);
    row
}

/// Arrow 87's shiver drive from the shiver command `u`, `(smooth_min(u, 1, w) - s_0) / (1 - s_0)` with
/// `s_0 = smooth_min(0, 1, w)`: zero at a zero command, rising with it and under one for every command. Model default:
/// research 12's `smooth_min(u, 1, w)` alone dips below zero for a small command, and the offset alone passes one at
/// `u = 2`.
pub(crate) fn shiver_drive<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    command: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    class: RateClass,
) {
    let key = ParameterKey::new;
    let line: ShiverDrive<Defines<'b, Nervous>> = ns.emit();
    let cap = ns.parameter::<Dimensionless>(key("ns.shiver-cap"), Admissible::positive());
    let width = ns.parameter::<Dimensionless>(key("ns.shiver-cap-width"), Admissible::positive());
    let drive = ns.kernel(|mut k| {
        let (command, cap, width) = (k.algebraic(command), k.parameter(&cap), k.parameter(&width));
        let capped = k.smooth_min(command, cap, width);
        let zero = k.zero();
        let at_zero = k.smooth_min(zero, cap, width);
        let raised = k.subtract(capped, at_zero);
        let span = k.subtract(cap, at_zero);
        let share = k.divide(raised, span);
        let value = k.as_dimension(share);
        k.finish(value, ns.every_mode(), class)
    });
    ns.define(&line.shiver_drive, drive);
}
