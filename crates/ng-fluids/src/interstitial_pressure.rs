use ng_contract::*;

/// `P = E (V - V_0)` on the interstitium's own volume, water and albumin, on top of what encloses the bed. Model default:
/// linear.
pub(crate) fn interstitial_pressure<'b>(
    fl: &mut FragmentBuilder<'b, Fluids>,
    ports: &FluidsPorts<'b>,
    route: &SiteRoute<'b, Fluids, CapillaryBed, Water>,
    class: RateClass,
) -> ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>> {
    let (row, read) = fl.site_field("interstitial pressure", Admissible::finite());
    let stiffness = fl.site_parameter::<CapillaryBed, Elastance>(
        ParameterKey::new("fl.interstitial-elastance"),
        Admissible::positive(),
    );
    let resting = fl.site_parameter::<CapillaryBed, Volume>(
        ParameterKey::new("fl.interstitial-volume"),
        Admissible::positive(),
    );
    let room = fl.parameter::<Quotient<Volume, Albumin>>(
        ParameterKey::new("fl.albumin-volume"),
        Admissible::nonnegative(),
    );
    let value = fl.site_kernel(|mut k| {
        let water = k.at_site(&route.to_amount::<Water>());
        let solvent = k.volume(water);
        let held = k.at_site(&route.to_amount::<Albumin>());
        let per = k.parameter(&room);
        let product = k.multiply(per, held);
        let solute: Expr<'_, '_, _, Volume> = k.as_dimension(product);
        let volume = k.add(solvent, solute);
        let rest = k.site_parameter(&resting);
        let stretched = k.subtract(volume, rest);
        let wall = k.site_parameter(&stiffness);
        let product = k.multiply(wall, stretched);
        let inside = k.as_dimension(product);
        let outside = k.at_site(&ports.extravascular_pressure);
        let total = k.add(inside, outside);
        k.finish(total, fl.every_mode(), class)
    });
    fl.define_at(&row, value);
    read
}
