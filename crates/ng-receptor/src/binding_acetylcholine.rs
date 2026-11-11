use crate::shared::{binding, in_cleft};
use ng_contract::*;

pub(crate) fn declare<'b>(
    laws: &mut FragmentBuilder<'b, Receptor>,
    route: &SiteRoute<'b, Receptor, Junction, Acetylcholine>,
    total: &SiteParameter<'b, Receptor, Junction, Acetylcholine>,
    class: RateClass,
) {
    binding::<Junction, Acetylcholine, Acetylcholine>(
        laws,
        route,
        total,
        "rc.ach-on",
        "rc.ach-off",
        class,
        in_cleft,
    );
}
