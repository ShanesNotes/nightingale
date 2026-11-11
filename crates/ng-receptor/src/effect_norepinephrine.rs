use crate::shared::effect;
use ng_contract::*;

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    ports: &ReceptorPorts<'b>,
    route: &SiteRoute<'b, Receptor, Junction, Norepinephrine>,
    total: &SiteParameter<'b, Receptor, Junction, Norepinephrine>,
    class: RateClass,
) {
    effect::<Junction, Norepinephrine>(laws, ports, route, total, "rc.ne-efficacy", class);
}
