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
//!     Transfer { rate }
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

const BINDING_NS: u64 = 1_000_000_000;

fn ligand<'b, C: SiteSet, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    keys: [&'static str; 4],
    class: RateClass,
) {
    let [total_key, on_key, off_key, efficacy_key] = keys;
    let route = ports.binding::<C, Q>(laws);
    let total = laws.site_parameter::<C, Q>(ParameterKey::new(total_key), Admissible::positive());
    let on = laws.site_parameter::<C, Affinity<Q>>(ParameterKey::new(on_key), Admissible::positive());
    let off =
        laws.site_parameter::<C, Rate<Dimensionless>>(ParameterKey::new(off_key), Admissible::positive());
    let efficacy =
        laws.site_parameter::<C, Dimensionless>(ParameterKey::new(efficacy_key), Admissible::positive());

    let rate = laws.site_kernel(|mut k| {
        let (free, bound) = (
            k.at_site(&route.from_amount::<Q>()),
            k.at_site(&route.to_amount::<Q>()),
        );
        let sites = k.site_parameter(&total);
        let empty = k.subtract(sites, bound);
        let meeting = k.multiply(free, empty);
        let speed = k.site_parameter(&on);
        let product = k.multiply(speed, meeting);
        let binding: Expr<'_, '_, _, Rate<Q>> = k.as_dimension(product);
        let (zero, leaving) = (k.zero(), k.site_parameter(&off));
        let unbinding = k.relax(bound, zero, leaving);
        let net = k.subtract(binding, unbinding);
        k.finish(net, laws.every_mode(), class)
    });
    laws.transfer_at(&route, rate);

    let output = ports.effect::<C, Q>(laws);
    // E = eps theta / (1 + (eps - 1) theta) with theta = B / R (research 02), written in amounts, so it holds no literal.
    let effect = laws.site_kernel(|mut k| {
        let bound = k.at_site(&route.to_amount::<Q>());
        let sites = k.site_parameter(&total);
        let empty = k.subtract(sites, bound);
        let eps = k.site_parameter(&efficacy);
        let weighted = k.scale(bound, eps);
        let whole = k.add(empty, weighted);
        let ratio = k.divide(weighted, whole);
        let value = k.as_dimension(ratio);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&output, effect);
}

pub fn declare<'b>(scope: ReceptorScope<'b>) -> Result<Fragment<'b, Receptor>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, BINDING_NS)?;
    ligand::<Junction, Norepinephrine>(
        &mut laws,
        &ports,
        ["rc.ne-sites", "rc.ne-on", "rc.ne-off", "rc.ne-efficacy"],
        class,
    );
    ligand::<Junction, Acetylcholine>(
        &mut laws,
        &ports,
        ["rc.ach-sites", "rc.ach-on", "rc.ach-off", "rc.ach-efficacy"],
        class,
    );
    ligand::<Blood, Vasopressin>(
        &mut laws,
        &ports,
        ["rc.avp-sites", "rc.avp-on", "rc.avp-off", "rc.avp-efficacy"],
        class,
    );
    Ok(laws.finish())
}
