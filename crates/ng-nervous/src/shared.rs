use ng_contract::*;

/// Research 02's release `q_i f`, linear in traffic, with `q_i = q A_vesicle`, so a store can run down and never below
/// zero. Model adaptation.
pub(crate) fn release<'b, Q: Substance>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    traffic: &AlgebraicSlot<'b, Nervous, Dimensionless>,
    release_key: &'static str,
    class: RateClass,
) {
    let release = ports.release::<Q>(ns);
    let released = ns.site_parameter::<Junction, Rate<Dimensionless>>(
        ParameterKey::new(release_key),
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
}

/// Release at every junction that holds `Q` by two traffics in that junction's shares, `q_i (s_v f_v + s_m a)`: the
/// vagal traffic `f_v` and the pump drive `a`, and in the whole body the sudomotor command `u` in its own share,
/// `+ s_u u`. Model adaptation of research 02's `q_i f`.
pub(crate) fn release_by_shares<'b, Q: Substance>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    (traffic, drive): (
        &AlgebraicSlot<'b, Nervous, Dimensionless>,
        &StateSlot<'b, Nervous, Dimensionless>,
    ),
    sudomotor: Option<&AlgebraicSlot<'b, Nervous, Dimensionless>>,
    release_key: &'static str,
    class: RateClass,
) {
    let release = ports.release::<Q>(ns);
    let key = ParameterKey::new;
    let released =
        ns.site_parameter::<Junction, Rate<Dimensionless>>(key(release_key), Admissible::positive());
    let vagal =
        ns.site_parameter::<Junction, Dimensionless>(key("ns.vagal-share"), Admissible::unit_interval());
    let motor =
        ns.site_parameter::<Junction, Dimensionless>(key("ns.motor-share"), Admissible::unit_interval());
    let sudomotor = sudomotor.map(|command| {
        let share = ns.site_parameter::<Junction, Dimensionless>(
            key("ns.sudomotor-share"),
            Admissible::unit_interval(),
        );
        (command, share)
    });
    let out = ns.site_kernel(|mut k| {
        let store = k.at_site(&release.from_amount::<Q>());
        let (zero, speed) = (k.zero(), k.site_parameter(&released));
        let emptying = k.relax(store, zero, speed);
        let (firing, pumping) = (k.algebraic(traffic), k.state(drive));
        let (vagal, motor) = (k.site_parameter(&vagal), k.site_parameter(&motor));
        let firing = k.scale(firing, vagal);
        let pumping = k.scale(pumping, motor);
        let both = k.add(firing, pumping);
        let all = match &sudomotor {
            Some((command, share)) => {
                let (command, share) = (k.algebraic(command), k.site_parameter(share));
                let sweating = k.scale(command, share);
                k.add(both, sweating)
            }
            None => both,
        };
        let rate = k.scale(emptying, all);
        k.finish(rate, ns.every_mode(), class)
    });
    ns.transfer_at(&release, out);
}

/// Release at every junction that holds `Q` by two traffics, each in that junction's share, `q_i (s_1 f_1 + s_2 f_2)`:
/// in the whole body norepinephrine's sympathetic traffic and the thermostat's constrictor command. Model adaptation
/// of research 02's `q_i f`.
pub(crate) fn release_in_shares<'b, Q: Substance>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    traffics: [(&AlgebraicSlot<'b, Nervous, Dimensionless>, &'static str); 2],
    release_key: &'static str,
    class: RateClass,
) {
    let release = ports.release::<Q>(ns);
    let key = ParameterKey::new;
    let released =
        ns.site_parameter::<Junction, Rate<Dimensionless>>(key(release_key), Admissible::positive());
    let [first, second] = traffics.map(|(traffic, share)| {
        let share = ns.site_parameter::<Junction, Dimensionless>(key(share), Admissible::unit_interval());
        (traffic, share)
    });
    let out = ns.site_kernel(|mut k| {
        let store = k.at_site(&release.from_amount::<Q>());
        let (zero, speed) = (k.zero(), k.site_parameter(&released));
        let emptying = k.relax(store, zero, speed);
        let (one, two) = (k.algebraic(first.0), k.algebraic(second.0));
        let (one_share, two_share) = (k.site_parameter(&first.1), k.site_parameter(&second.1));
        let one = k.scale(one, one_share);
        let two = k.scale(two, two_share);
        let both = k.add(one, two);
        let rate = k.scale(emptying, both);
        k.finish(rate, ns.every_mode(), class)
    });
    ns.transfer_at(&release, out);
}

/// Reuptake is first order.
pub(crate) fn reuptake<'b, Q: Substance>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    ports: &NervousPorts<'b>,
    reuptake_key: &'static str,
    class: RateClass,
) {
    let reuptake = ports.reuptake::<Q>(ns);
    let taken = ns.site_parameter::<Junction, Rate<Dimensionless>>(
        ParameterKey::new(reuptake_key),
        Admissible::positive(),
    );
    let back = ns.site_kernel(|mut k| {
        let free = k.at_site(&reuptake.from_amount::<Q>());
        let (zero, speed) = (k.zero(), k.site_parameter(&taken));
        let rate = k.relax(free, zero, speed);
        k.finish(rate, ns.every_mode(), class)
    });
    ns.transfer_at(&reuptake, back);
}

/// A logistic of how far the filter stands from the set point, shifted so the body can rest low on the curve and
/// leave traffic room to rise. Model default.
pub(crate) fn traffic<'b>(
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
