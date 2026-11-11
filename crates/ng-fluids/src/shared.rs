use ng_contract::*;

/// `J = g (c_plasma - c_tissue)`. A small ion crosses a capillary wall far faster than water carries it, so tissue fluid
/// holds the plasma's ions. Albumin leaks the same way, far more slowly, so tissue that only ever hands water back still
/// holds some. Model default: linear, one `g` per bed.
pub(crate) fn across_the_wall<'b, Q: Substance>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    ports: &FluidsPorts<'b>,
    water: &SiteRoute<'b, Fluids, CapillaryBed, Water>,
    wall: &SiteParameter<'b, Fluids, CapillaryBed, Rate<Water>>,
    class: RateClass,
) where
    Product<Rate<Water>, Quotient<Q, Water>>: SameDimension<Rate<Q>>,
{
    let route = ports.exchange::<Q>(fl);
    let rate = fl.site_kernel(|mut k| {
        let (held, solvent) = (k.at_site(&route.from_amount()), k.at_site(&water.from_amount()));
        let inside = k.divide(held, solvent);
        let (held, solvent) = (k.at_site(&route.to_amount()), k.at_site(&water.to_amount()));
        let outside = k.divide(held, solvent);
        let difference = k.subtract(inside, outside);
        let open = k.site_parameter(wall);
        let product = k.multiply(open, difference);
        let rate = k.as_dimension(product);
        k.finish(rate, fl.every_mode(), class)
    });
    fl.transfer_at(&route, rate);
}
