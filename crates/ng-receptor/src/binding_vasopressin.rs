use crate::shared::binding;
use ng_contract::*;

/// Mass action reads a concentration (research 02), so blood that pools in a bed moves no receptor. A ligand dissolves in
/// plasma water.
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

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    route: &SiteRoute<'b, Receptor, Blood, Vasopressin>,
    total: &SiteParameter<'b, Receptor, Blood, Vasopressin>,
    class: RateClass,
) {
    binding::<Blood, Vasopressin, Concentration<Vasopressin>>(
        laws,
        route,
        total,
        "rc.avp-on",
        "rc.avp-off",
        class,
        in_plasma,
    );
}
