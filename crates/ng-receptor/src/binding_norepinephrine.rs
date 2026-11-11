use crate::shared::{binding, in_cleft};
use ng_contract::*;

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    route: &SiteRoute<'b, Receptor, Junction, Norepinephrine>,
    total: &SiteParameter<'b, Receptor, Junction, Norepinephrine>,
    class: RateClass,
) {
    binding::<Junction, Norepinephrine, Norepinephrine>(
        laws,
        route,
        total,
        "rc.ne-on",
        "rc.ne-off",
        class,
        in_cleft,
    );
}
