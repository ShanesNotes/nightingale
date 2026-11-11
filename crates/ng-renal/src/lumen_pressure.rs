use ng_contract::*;

/// Each segment's pressure from its own volume. Model default: linear.
pub(crate) fn lumen_pressure<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    class: RateClass,
) -> ReadPort<'b, Renal, PerSite<Urinary, Pressure>> {
    let (row, read) = rn.site_field("lumen pressure", Admissible::finite());
    let stiffness = rn.site_parameter::<Urinary, Elastance>(
        ParameterKey::new("rn.lumen-elastance"),
        Admissible::positive(),
    );
    let resting =
        rn.site_parameter::<Urinary, Volume>(ParameterKey::new("rn.lumen-volume"), Admissible::nonnegative());
    let water = ports.lumen::<Water>();
    let value = rn.site_kernel(|mut k| {
        let amount = k.at_site(&water);
        let volume = k.volume(amount);
        let rest = k.site_parameter(&resting);
        let stretched = k.subtract(volume, rest);
        let wall = k.site_parameter(&stiffness);
        let product = k.multiply(wall, stretched);
        let pressure = k.as_dimension(product);
        k.finish(pressure, rn.every_mode(), class)
    });
    rn.define_at(&row, value);
    read
}
