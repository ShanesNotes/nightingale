use crate::shared::across_the_wall;
use ng_contract::*;

pub(crate) fn declare<'b>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    ports: &FluidsPorts<'b>,
    route: &SiteRoute<'b, Fluids, CapillaryBed, Water>,
    wall: &SiteParameter<'b, Fluids, CapillaryBed, Rate<Water>>,
    class: RateClass,
) {
    across_the_wall::<Potassium>(fl, ports, route, wall, class);
}
