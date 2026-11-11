use crate::shared::effect;
use ng_contract::*;

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    route: &SiteRoute<'b, Receptor, Blood, Vasopressin>,
    total: &SiteParameter<'b, Receptor, Blood, Vasopressin>,
    class: RateClass,
) {
    effect::<Blood, Vasopressin>(laws, ports, route, total, "rc.avp-efficacy", class);
}
