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

const BINDING_NS: u64 = 1_000_000_000;

/// What an empty site meets, read from its binding route.
type Meets<'b, C, Q, M> =
    for<'k> fn(&mut Kernel<'b, 'k, Receptor, C>, &SiteRoute<'b, Receptor, C, Q>) -> Expr<'b, 'k, Receptor, M>;

/// A cleft has no volume of its own, so what a site meets there is the amount. Model default.
fn in_cleft<'b, 'k, Q: Substance>(
    k: &mut Kernel<'b, 'k, Receptor, Junction>,
    route: &SiteRoute<'b, Receptor, Junction, Q>,
) -> Expr<'b, 'k, Receptor, Q> {
    k.at_site(&route.from_amount())
}

/// Mass action reads a concentration (research 02), so blood that pools in a bed moves no receptor. A ligand dissolves in
/// plasma water, the volume every concentration in the slice is read on.
fn in_plasma<'b, 'k, Q: Substance>(
    k: &mut Kernel<'b, 'k, Receptor, Blood>,
    route: &SiteRoute<'b, Receptor, Blood, Q>,
) -> Expr<'b, 'k, Receptor, Concentration<Q>> {
    let held = k.at_site(&route.from_amount());
    let water = k.at_site(&route.from_amount());
    let volume = k.volume(water);
    let ratio = k.divide(held, volume);
    k.as_dimension(ratio)
}

fn ligand<'b, C: SiteSet, Q: Substance, M: Dimension>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    keys: [&'static str; 4],
    class: RateClass,
    meets: Meets<'b, C, Q, M>,
) where
    Product<Affinity<M>, Product<M, Q>>: SameDimension<Rate<Q>>,
{
    let [total_key, on_key, off_key, efficacy_key] = keys;
    let route = ports.binding::<C, Q>(laws);
    let total = laws.site_parameter::<C, Q>(ParameterKey::new(total_key), Admissible::positive());
    let on = laws.site_parameter::<C, Affinity<M>>(ParameterKey::new(on_key), Admissible::positive());
    let off =
        laws.site_parameter::<C, Rate<Dimensionless>>(ParameterKey::new(off_key), Admissible::positive());
    let efficacy =
        laws.site_parameter::<C, Dimensionless>(ParameterKey::new(efficacy_key), Admissible::positive());

    let rate = laws.site_kernel(|mut k| {
        let free = meets(&mut k, &route);
        let bound = k.at_site(&route.to_amount::<Q>());
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
    ligand::<Junction, Norepinephrine, Norepinephrine>(
        &mut laws,
        &ports,
        ["rc.ne-sites", "rc.ne-on", "rc.ne-off", "rc.ne-efficacy"],
        class,
        in_cleft,
    );
    ligand::<Junction, Acetylcholine, Acetylcholine>(
        &mut laws,
        &ports,
        ["rc.ach-sites", "rc.ach-on", "rc.ach-off", "rc.ach-efficacy"],
        class,
        in_cleft,
    );
    ligand::<Blood, Vasopressin, Concentration<Vasopressin>>(
        &mut laws,
        &ports,
        ["rc.avp-sites", "rc.avp-on", "rc.avp-off", "rc.avp-efficacy"],
        class,
        in_plasma,
    );
    Ok(laws.finish())
}
