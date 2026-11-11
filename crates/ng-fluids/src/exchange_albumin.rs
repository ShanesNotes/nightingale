use crate::shared::across_the_wall;
use ng_contract::*;

pub(crate) fn declare<'b>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    ports: &FluidsPorts<'b>,
    route: &SiteRoute<'b, Fluids, CapillaryBed, Water>,
    class: RateClass,
) {
    let leak = fl.site_parameter::<CapillaryBed, Rate<Water>>(
        ParameterKey::new("fl.albumin-leak"),
        Admissible::positive(),
    );
    across_the_wall::<Albumin>(fl, ports, route, &leak, class);
}
