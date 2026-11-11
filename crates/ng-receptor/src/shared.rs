use ng_contract::*;

/// What an empty site meets, read from its binding route.
pub(crate) type Meets<'b, C, Q, M> =
    for<'k> fn(&mut Kernel<'b, 'k, Receptor, C>, &SiteRoute<'b, Receptor, C, Q>) -> Expr<'b, 'k, Receptor, M>;

/// A cleft has no volume of its own, so what a site meets there is the amount. Model default.
pub(crate) fn in_cleft<'b, 'k, Q: Substance>(
    k: &mut Kernel<'b, 'k, Receptor, Junction>,
    route: &SiteRoute<'b, Receptor, Junction, Q>,
) -> Expr<'b, 'k, Receptor, Q> {
    k.at_site(&route.from_amount())
}

pub(crate) fn sites<'b, C: SiteSet, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    total_key: &'static str,
) -> (SiteRoute<'b, Receptor, C, Q>, SiteParameter<'b, Receptor, C, Q>) {
    let route = ports.binding::<C, Q>(laws);
    let total = laws.site_parameter::<C, Q>(ParameterKey::new(total_key), Admissible::positive());
    (route, total)
}

pub(crate) fn binding<'b, C: SiteSet, Q: Substance, M: Dimension>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    route: &SiteRoute<'b, Receptor, C, Q>,
    total: &SiteParameter<'b, Receptor, C, Q>,
    on_key: &'static str,
    off_key: &'static str,
    class: RateClass,
    meets: Meets<'b, C, Q, M>,
) where
    Product<Affinity<M>, Product<M, Q>>: SameDimension<Rate<Q>>,
{
    let on = laws.site_parameter::<C, Affinity<M>>(ParameterKey::new(on_key), Admissible::positive());
    let off =
        laws.site_parameter::<C, Rate<Dimensionless>>(ParameterKey::new(off_key), Admissible::positive());

    let rate = laws.site_kernel(|mut k| {
        let free = meets(&mut k, route);
        let bound = k.at_site(&route.to_amount::<Q>());
        let sites = k.site_parameter(total);
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
    laws.transfer_at(route, rate);
}

pub(crate) fn effect<'b, C: SiteSet, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    route: &SiteRoute<'b, Receptor, C, Q>,
    total: &SiteParameter<'b, Receptor, C, Q>,
    efficacy_key: &'static str,
    class: RateClass,
) {
    let efficacy =
        laws.site_parameter::<C, Dimensionless>(ParameterKey::new(efficacy_key), Admissible::positive());
    let output = ports.effect::<C, Q>(laws);
    // E = eps theta / (1 + (eps - 1) theta) with theta = B / R (research 02), written in amounts, so it holds no literal.
    let effect = laws.site_kernel(|mut k| {
        let bound = k.at_site(&route.to_amount::<Q>());
        let sites = k.site_parameter(total);
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
