//! Fluid between the spaces: Starling exchange at every capillary bed and lymph back to the veins. No law here names a
//! space's pool, and what crosses with the water follows from what both ends hold.

use ng_contract::catalog::{CapillaryAndVenousPressure, Reads};
use ng_contract::*;

const EXCHANGE_NS: u64 = 1_000_000_000;

/// `P = E (V - V_0)` on the interstitium's own volume, water and albumin, on top of what encloses the bed. Model default:
/// linear.
fn interstitial_pressure<'b>(
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

/// `pi = slope c`, with `c` albumin over the space's own volume. Model default: linear.
fn oncotic<'b, 'k>(
    k: &mut Kernel<'b, 'k, Fluids, CapillaryBed>,
    albumin: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Albumin>>,
    water: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Water>>,
    slope: &Parameter<'b, Fluids, Quotient<Pressure, Concentration<Albumin>>>,
) -> Expr<'b, 'k, Fluids, Pressure> {
    let (amount, solvent) = (k.at_site(albumin), k.at_site(water));
    let volume = k.volume(solvent);
    let ratio = k.divide(amount, volume);
    let concentration = k.as_dimension(ratio);
    let per = k.parameter(slope);
    let product = k.multiply(per, concentration);
    k.as_dimension(product)
}

/// `J = g (c_plasma - c_tissue)`. A small ion crosses a capillary wall far faster than water carries it, so tissue fluid
/// holds the plasma's ions. Model default: linear, one `g` per bed.
fn across_the_wall<'b, Q: Substance>(
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

pub fn declare<'b>(scope: FluidsScope<'b>) -> Result<Fragment<'b, Fluids>, AdmissionError> {
    let (mut fl, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, EXCHANGE_NS)?;
    let route = ports.starling(&mut fl);
    let pressures: CapillaryAndVenousPressure<Reads<'_, Fluids>> = fl.receive();
    let interstitial = interstitial_pressure(&mut fl, &ports, &route, class);
    let filtration = fl.site_parameter::<CapillaryBed, Conductance<Water>>(
        ParameterKey::new("fl.filtration"),
        Admissible::positive(),
    );
    let slope = fl.parameter(ParameterKey::new("fl.oncotic-slope"), Admissible::positive());
    let reflected = fl.reflect::<Albumin>(&route, ParameterKey::new("fl.albumin-reflection"));
    route.keeps_back::<Vasopressin>(&mut fl);
    let exchange = fl.site_kernel(|mut k| {
        let inside = k.at_site(&pressures.capillary_pressure);
        let outside = k.at_site(&interstitial);
        let head = k.subtract(inside, outside);
        let held = oncotic(&mut k, &route.from_amount(), &route.from_amount(), &slope);
        let drawn = oncotic(&mut k, &route.to_amount(), &route.to_amount(), &slope);
        let pull = k.subtract(held, drawn);
        let sigma = k.site_parameter(&reflected);
        let pulled = k.scale(pull, sigma);
        let net = k.subtract(head, pulled);
        let conductance = k.site_parameter(&filtration);
        let flow = k.conductance_flow(conductance, net);
        k.finish(flow, fl.every_mode(), class)
    });
    fl.transfer_at(&route, exchange);

    let wall = fl.site_parameter::<CapillaryBed, Rate<Water>>(
        ParameterKey::new("fl.ion-exchange"),
        Admissible::positive(),
    );
    across_the_wall::<Sodium>(&mut fl, &ports, &route, &wall, class);
    across_the_wall::<Potassium>(&mut fl, &ports, &route, &wall, class);
    across_the_wall::<Chloride>(&mut fl, &ports, &route, &wall, class);

    // Lymph runs when tissue pressure stands past a threshold, softly. Model default.
    let lymph = ports.lymph(&mut fl);
    lymph.keeps_back::<Vasopressin>(&mut fl);
    let conductance = fl.edge_parameter::<LymphEdge, Conductance<Water>>(
        ParameterKey::new("fl.lymph-conductance"),
        Admissible::positive(),
    );
    let threshold = fl
        .edge_parameter::<LymphEdge, Pressure>(ParameterKey::new("fl.lymph-threshold"), Admissible::finite());
    let width =
        fl.edge_parameter::<LymphEdge, Pressure>(ParameterKey::new("fl.lymph-width"), Admissible::positive());
    let drain = fl.edge_kernel(|mut k| {
        let inside = k.upstream(&interstitial);
        let start = k.edge_parameter(&threshold);
        let above = k.subtract(inside, start);
        let (zero, soft) = (k.zero(), k.edge_parameter(&width));
        let head = k.smooth_max(above, zero, soft);
        let open = k.edge_parameter(&conductance);
        let flow = k.conductance_flow(open, head);
        k.finish(flow, fl.every_mode(), class)
    });
    fl.transfer_along(&lymph, drain);
    Ok(fl.finish())
}
