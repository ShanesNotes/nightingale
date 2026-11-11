use crate::shared::effect;
use ng_contract::*;

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    route: &SiteRoute<'b, Receptor, Junction, Acetylcholine>,
    total: &SiteParameter<'b, Receptor, Junction, Acetylcholine>,
    class: RateClass,
) {
    effect::<Junction, Acetylcholine>(laws, ports, route, total, "rc.ach-efficacy", class);
}
