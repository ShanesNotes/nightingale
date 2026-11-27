use ng_contract::*;

type Capacity = Quotient<Energy, Temperature>;
type HeatConductance = Quotient<Rate<Energy>, Temperature>;
type LatentHeat = Quotient<Rate<Energy>, Rate<Water>>;
type Mt = Metabolism;

const HEAT_NS: u64 = 1_000_000_000;

pub fn tissue_heat<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    ports: &MetabolismPorts<'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, HEAT_NS)?;
    let stores = ports.temperatures(laws);
    publish(laws, &stores, class);
    let published = stores.published();
    let route = ports.conduction(laws);
    conduction(laws, &route, &published.core, &published.other, class);
    let convection = Convection::declare(laws);
    let dry = ports.boundary::<DryHeatLoss>()?;
    dry_heat_loss(laws, &dry.route, published.at_exit(&dry), &convection, class);
    let (heat, water) = (
        ports.boundary::<SkinEvaporativeHeat>()?,
        ports.boundary::<SkinEvaporation>()?,
    );
    evaporation(
        laws,
        (&heat.route, &water.route),
        published.at_exit(&heat),
        &convection,
        class,
    );
    Ok(())
}

/// Each tissue heat store's temperature, `T = T_0 + H / C`, defined once where its heat and capacity are: the core
/// store's as arrow 91's scalar on the core's capacity, and every other store's at its site on its own (research 12,
/// the row 'Heat stores and their temperatures').
fn publish<'b>(laws: &mut FragmentBuilder<'b, Mt>, stores: &TissueTemperatures<'b>, class: RateClass) {
    let key = ParameterKey::new;
    let zero = laws.parameter::<Temperature>(key("mt.thermal-zero"), Admissible::finite());
    let core = laws.parameter::<Capacity>(key("mt.core-capacity"), Admissible::positive());
    let own = laws.site_parameter::<TissueSite, Capacity>(key("mt.tissue-capacity"), Admissible::positive());
    let value = laws.kernel(|mut k| {
        let (heat, capacity) = (k.read(&stores.core_heat), k.parameter(&core));
        let value = above_zero(&mut k, &zero, heat, capacity);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define(&stores.core, value);
    let value = laws.site_kernel(|mut k| {
        let (heat, capacity) = (k.at_site(&stores.tissue_heat), k.site_parameter(&own));
        let value = above_zero(&mut k, &zero, heat, capacity);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&stores.other, value);
}

fn above_zero<'b, 'k, S>(
    k: &mut Kernel<'b, 'k, Mt, S>,
    zero: &Parameter<'b, Mt, Temperature>,
    held: Expr<'b, 'k, Mt, Energy>,
    capacity: Expr<'b, 'k, Mt, Capacity>,
) -> Expr<'b, 'k, Mt, Temperature> {
    let zero = k.parameter(zero);
    let ratio = k.divide(held, capacity);
    let rise = k.as_dimension(ratio);
    k.add(zero, rise)
}

/// `Q = K (T_c - T_s)` from the core store to each store conduction reaches, with `K` the tissue's conductance at that
/// site (research 12, the row 'Core to skin', its fixed tissue term).
fn conduction<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    route: &SiteRoute<'b, Mt, TissueSite, Energy>,
    core: &ReadPort<'b, Mt, Temperature>,
    own: &ReadPort<'b, Mt, PerSite<TissueSite, Temperature>>,
    class: RateClass,
) {
    let conductance = laws.site_parameter::<TissueSite, HeatConductance>(
        ParameterKey::new("mt.heat-conductance"),
        Admissible::nonnegative(),
    );
    let rate = laws.site_kernel(|mut k| {
        let (core, here) = (k.read(core), k.at_site(own));
        let gap = k.subtract(core, here);
        let conductance = k.site_parameter(&conductance);
        let product = k.multiply(conductance, gap);
        let value = k.as_dimension(product);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer_at(route, rate);
}

/// Convection's conductance at the room's air speed, `h_c = h_c0 exp(n ln(v_e / v_0))` with `v_e = smooth_max(v, v_0,
/// w)`, so air slower than still air reads as still air, and `n` a half for Stolwijk's square root (research 12, the
/// row 'Dry heat loss').
struct Convection<'b> {
    still: Parameter<'b, Mt, HeatConductance>,
    still_speed: Parameter<'b, Mt, Dimensionless>,
    width: Parameter<'b, Mt, Dimensionless>,
    exponent: Parameter<'b, Mt, Dimensionless>,
}

impl<'b> Convection<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, Mt>) -> Self {
        let key = ParameterKey::new;
        Self {
            still: laws.parameter(key("mt.convective-conductance"), Admissible::positive()),
            still_speed: laws.parameter(key("mt.still-air-speed"), Admissible::positive()),
            width: laws.parameter(key("mt.air-speed-width"), Admissible::positive()),
            exponent: laws.parameter(key("mt.air-speed-exponent"), Admissible::positive()),
        }
    }

    fn at<'k>(&self, k: &mut Kernel<'b, 'k, Mt>) -> Expr<'b, 'k, Mt, HeatConductance> {
        let (speed, still_speed, width) = (
            k.room_air_speed(),
            k.parameter(&self.still_speed),
            k.parameter(&self.width),
        );
        let effective = k.smooth_max(speed, still_speed, width);
        let ratio = k.divide(effective, still_speed);
        let ratio = k.as_dimension::<_, Dimensionless>(ratio);
        let (log, exponent) = (k.ln(ratio), k.parameter(&self.exponent));
        let power = k.scale(log, exponent);
        let root = k.exp(power);
        let still = k.parameter(&self.still);
        k.scale(still, root)
    }
}

/// A bare rate through clothing on a conductance `h`, `J F` with `F = 1 / (1 + h I_clo / K_clo) = K_clo / (K_clo + h
/// I_clo)`, `K_clo` the conductance of one clo over the body's surface and `I_clo` the room's clothing.
fn clothed<'b, 'k>(
    k: &mut Kernel<'b, 'k, Mt>,
    clothing: &Parameter<'b, Mt, HeatConductance>,
    conductance: Expr<'b, 'k, Mt, HeatConductance>,
    bare: Expr<'b, 'k, Mt, Rate<Energy>>,
) -> Expr<'b, 'k, Mt, Rate<Energy>> {
    let (one_clo, worn) = (k.parameter(clothing), k.clothing());
    let insulated = k.scale(conductance, worn);
    let both = k.add(one_clo, insulated);
    let ratio = k.divide(bare, both);
    let across: Expr<'_, '_, Mt, Temperature> = k.as_dimension(ratio);
    let product = k.multiply(one_clo, across);
    k.as_dimension(product)
}

/// The skin's radiation and convection to the room, `R + C = (h_r (T_s - T_r) + h_c (T_s - T_a)) F_cl`, which is
/// `h (T_s - T_o) F_cl` with `h = h_r + h_c` and the operative temperature `T_o = (h_r T_r + h_c T_a) / h`, and
/// `F_cl = 1 / (1 + 0.155 h I_clo)` per square metre (research 12, the row 'Dry heat loss', Gagge's Eq 8).
fn dry_heat_loss<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    exit: &Pair<'b, Mt, Energy>,
    at_exit: ReadPort<'b, Mt, Temperature>,
    convection: &Convection<'b>,
    class: RateClass,
) {
    let key = ParameterKey::new;
    let radiant = laws.parameter::<HeatConductance>(key("mt.radiant-conductance"), Admissible::positive());
    let clothing = laws.parameter::<HeatConductance>(key("mt.clothing-conductance"), Admissible::positive());
    let rate = laws.kernel(|mut k| {
        let skin = k.read(&at_exit);
        let (walls, air) = (k.room_radiant(), k.room_air());
        let (radiant, convective) = (k.parameter(&radiant), convection.at(&mut k));
        let to_walls = k.subtract(skin, walls);
        let product = k.multiply(radiant, to_walls);
        let radiated: Expr<'_, '_, Mt, Rate<Energy>> = k.as_dimension(product);
        let to_air = k.subtract(skin, air);
        let product = k.multiply(convective, to_air);
        let convected = k.as_dimension(product);
        let bare = k.add(radiated, convected);
        let both = k.add(radiant, convective);
        let value = clothed(&mut k, &clothing, both, bare);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(exit, rate));
}

/// `P_sat(T) = P_ref exp(a - b / (T + c))`, an Antoine form (research 12, the row 'Saturated vapour pressure').
struct Vapour<'b> {
    reference: Parameter<'b, Mt, Pressure>,
    a: Parameter<'b, Mt, Dimensionless>,
    b: Parameter<'b, Mt, Temperature>,
    c: Parameter<'b, Mt, Temperature>,
}

impl<'b> Vapour<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, Mt>) -> Self {
        let key = ParameterKey::new;
        Self {
            reference: laws.parameter(key("mt.vapour-reference"), Admissible::positive()),
            a: laws.parameter(key("mt.vapour-a"), Admissible::finite()),
            b: laws.parameter(key("mt.vapour-b"), Admissible::positive()),
            c: laws.parameter(key("mt.vapour-c"), Admissible::finite()),
        }
    }

    fn saturated<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Mt>,
        temperature: Expr<'b, 'k, Mt, Temperature>,
    ) -> Expr<'b, 'k, Mt, Pressure> {
        let (a, b, c) = (k.parameter(&self.a), k.parameter(&self.b), k.parameter(&self.c));
        let shifted = k.add(temperature, c);
        let ratio = k.divide(b, shifted);
        let ratio = k.as_dimension(ratio);
        let exponent = k.subtract(a, ratio);
        let factor = k.exp(exponent);
        let reference = k.parameter(&self.reference);
        k.scale(reference, factor)
    }
}

/// The skin's evaporation at rest, its diffusion term `E = w_d E_max` times the forcing, with the evaporative capacity
/// `E_max = LR h_c (P_sat(T_s) - phi P_sat(T_a)) F_pcl` and `F_pcl = 1 / (1 + 0.143 h_c I_clo)` (research 12, the row
/// 'Evaporation from skin', and ticket 12's fourth ruling). One rate posts both routes: heat leaves the skin store at
/// `E` and water leaves the skin bed at `E / lambda`, so the two stand in one fixed proportion.
fn evaporation<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    (heat, water): (&Pair<'b, Mt, Energy>, &Pair<'b, Mt, Water>),
    at_exit: ReadPort<'b, Mt, Temperature>,
    convection: &Convection<'b>,
    class: RateClass,
) {
    let key = ParameterKey::new;
    let vapour = Vapour::declare(laws);
    let lewis =
        laws.parameter::<Quotient<Temperature, Pressure>>(key("mt.lewis-relation"), Admissible::positive());
    let clothing =
        laws.parameter::<HeatConductance>(key("mt.clothing-vapour-conductance"), Admissible::positive());
    let wettedness =
        laws.parameter::<Dimensionless>(key("mt.diffusion-wettedness"), Admissible::unit_interval());
    let latent = laws.parameter::<LatentHeat>(key("mt.latent-heat"), Admissible::positive());
    let variability = laws.forcing(
        "evaporation variability",
        key("mt.evaporation-period"),
        key("mt.evaporation-amplitude"),
    );
    let rate = laws.algebraic::<Rate<Energy>>("evaporation", Admissible::finite());
    let law = laws.kernel(|mut k| {
        let skin = k.read(&at_exit);
        let air = k.room_air();
        let (at_skin, in_air) = (vapour.saturated(&mut k, skin), vapour.saturated(&mut k, air));
        let humidity = k.room_humidity();
        let in_room = k.scale(in_air, humidity);
        let gradient = k.subtract(at_skin, in_room);
        let lewis = k.parameter(&lewis);
        let product = k.multiply(lewis, gradient);
        let per_conductance: Expr<'_, '_, Mt, Temperature> = k.as_dimension(product);
        let convective = convection.at(&mut k);
        let product = k.multiply(convective, per_conductance);
        let open = k.as_dimension(product);
        let capacity = clothed(&mut k, &clothing, convective, open);
        let (wettedness, wandering) = (k.parameter(&wettedness), k.forcing(&variability));
        let diffusing = k.scale(capacity, wettedness);
        let value = k.scale(diffusing, wandering);
        let own = k.algebraic(&rate);
        let residual = k.subtract(own, value);
        k.finish(residual, laws.every_mode(), class)
    });
    laws.residual(&rate, law);
    let heat_rate = laws.kernel(|mut k| {
        let value = k.algebraic(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(heat, heat_rate));
    let water_rate = laws.kernel(|mut k| {
        let (value, latent) = (k.algebraic(&rate), k.parameter(&latent));
        let ratio = k.divide(value, latent);
        let value = k.as_dimension(ratio);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(water, water_rate));
}
