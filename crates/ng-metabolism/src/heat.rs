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
    let sweat = sweat_rate(laws, &ports.sweat_effect(), &published.at_bed(), class);
    let heat = ports.boundary::<SkinEvaporativeHeat>()?;
    let (water, unevaporated) = (
        ports.boundary::<SkinEvaporation>()?,
        ports.boundary::<Sweat<Water>>()?,
    );
    let (sodium, chloride) = (
        ports.boundary::<Sweat<Sodium>>()?,
        ports.boundary::<Sweat<Chloride>>()?,
    );
    let routes = SkinRoutes {
        heat: &heat.route,
        water: &water.route,
        unevaporated: &unevaporated.route,
        sodium: &sodium.route,
        chloride: &chloride.route,
    };
    evaporation_and_sweat(
        laws,
        &routes,
        published.at_exit(&heat),
        (&convection, &sweat),
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

/// The sweat rate `m_sw = m_max E_m exp(L2 e_s / b_sw)`, with `E_m` acetylcholine's effect at the junction supplying
/// the skin bed and `e_s` the temperature of the store the bed sits in against the skin reference (research 12, the row
/// 'Sweat glands, and the salt sweat carries'). One kernel at the skin bed sets one body row (record 0011, consequence
/// 24), which evaporation, the unevaporated sweat and the salts read.
struct SweatRate<'b> {
    rate: AlgebraicSlot<'b, Mt, Rate<Water>>,
    most: Parameter<'b, Mt, Rate<Water>>,
}

fn sweat_rate<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    effect: &SkinSweatEffect<'b>,
    at_bed: &ReadPort<'b, Mt, PerSite<CapillaryBed, Temperature>>,
    class: RateClass,
) -> SweatRate<'b> {
    let key = ParameterKey::new;
    let most = laws.parameter::<Rate<Water>>(key("mt.sweat-gland-maximal-rate"), Admissible::positive());
    let width = laws.parameter::<Deviation<Temperature>>(key("mt.sweat-width"), Admissible::positive());
    let reference = laws.parameter::<Reference<Temperature>>(key("ns.skin-reference"), Admissible::finite());
    let ln_two = laws.parameter::<Dimensionless>(key("mt.ln-two"), Admissible::positive());
    let rate = laws.algebraic::<Rate<Water>>("sweat rate", Admissible::finite());
    let row = laws.kernel(|mut k| {
        let secreted = k.algebraic(&rate);
        k.finish(secreted, laws.every_mode(), class)
    });
    let at_skin = laws.site_kernel::<Beds<Skin>, Rate<Water>, _>(|mut k| {
        let felt = effect.at_skin(&mut k);
        let (skin, reference) = (k.at_site_in(at_bed), k.parameter(&reference));
        let (warmth, width) = (k.deviation(skin, reference), k.parameter(&width));
        let ratio = k.divide(warmth, width);
        let widths: Expr<'_, '_, Mt, Dimensionless> = k.as_dimension(ratio);
        let ln_two = k.parameter(&ln_two);
        let doublings = k.scale(widths, ln_two);
        let local = k.exp(doublings);
        let drive = k.scale(felt, local);
        let most = k.parameter(&most);
        let value = k.scale(most, drive);
        k.finish(value, laws.every_mode(), class)
    });
    laws.only_at(&rate, row, at_skin);
    SweatRate { rate, most }
}

/// The skin's evaporative capacity `E_max = LR h_c (P_sat(T_s) - phi P_sat(T_a)) F_pcl`, with `F_pcl = 1 / (1 + 0.143
/// h_c I_clo)` and `T_s` the skin store's temperature at the evaporative heat's exit (research 12, the row 'Evaporation
/// from skin').
struct Evaporative<'b> {
    vapour: Vapour<'b>,
    lewis: Parameter<'b, Mt, Quotient<Temperature, Pressure>>,
    clothing: Parameter<'b, Mt, HeatConductance>,
}

impl<'b> Evaporative<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, Mt>) -> Self {
        let key = ParameterKey::new;
        let vapour = Vapour::declare(laws);
        Self {
            vapour,
            lewis: laws.parameter(key("mt.lewis-relation"), Admissible::positive()),
            clothing: laws.parameter(key("mt.clothing-vapour-conductance"), Admissible::positive()),
        }
    }

    fn at<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Mt>,
        at_exit: &ReadPort<'b, Mt, Temperature>,
        convection: &Convection<'b>,
    ) -> Expr<'b, 'k, Mt, Rate<Energy>> {
        let skin = k.read(at_exit);
        let air = k.room_air();
        let (at_skin, in_air) = (self.vapour.saturated(k, skin), self.vapour.saturated(k, air));
        let humidity = k.room_humidity();
        let in_room = k.scale(in_air, humidity);
        let gradient = k.subtract(at_skin, in_room);
        let lewis = k.parameter(&self.lewis);
        let product = k.multiply(lewis, gradient);
        let per_conductance: Expr<'_, '_, Mt, Temperature> = k.as_dimension(product);
        let convective = convection.at(k);
        let product = k.multiply(convective, per_conductance);
        let open = k.as_dimension(product);
        clothed(k, &self.clothing, convective, open)
    }
}

/// The secreted sweat's heat `E_rsw = lambda m_sw`, and what of it the skin can take, `smooth_min(E_rsw, E_max, w)`.
fn taken<'b, 'k>(
    k: &mut Kernel<'b, 'k, Mt>,
    sweat: &AlgebraicSlot<'b, Mt, Rate<Water>>,
    (latent, width): (&Parameter<'b, Mt, LatentHeat>, &Parameter<'b, Mt, Rate<Energy>>),
    capacity: Expr<'b, 'k, Mt, Rate<Energy>>,
) -> (Expr<'b, 'k, Mt, Rate<Energy>>, Expr<'b, 'k, Mt, Rate<Energy>>) {
    let (secreted, latent) = (k.algebraic(sweat), k.parameter(latent));
    let product = k.multiply(latent, secreted);
    let secreting = k.as_dimension(product);
    let width = k.parameter(width);
    let taken = k.smooth_min(secreting, capacity, width);
    (secreting, taken)
}

/// The routes evaporation and the sweat post on: heat out of the skin store, and out of the skin bed the evaporated
/// water, the unevaporated sweat and the two salts the sweat carries.
struct SkinRoutes<'r, 'b> {
    heat: &'r Pair<'b, Mt, Energy>,
    water: &'r Pair<'b, Mt, Water>,
    unevaporated: &'r Pair<'b, Mt, Water>,
    sodium: &'r Pair<'b, Mt, Sodium>,
    chloride: &'r Pair<'b, Mt, Chloride>,
}

/// The skin's evaporation and its sweat (research 12, the rows 'Evaporation from skin' and 'Sweat glands, and the salt
/// sweat carries', and ticket 12's fourth ruling). Evaporation is one rate, `E_sk = f w_d E_max + (1 - w_d)
/// smooth_min(E_rsw, E_max, w)`: the diffusion term with the forcing `f` on it, and what the skin can take of the
/// secreted sweat's heat. Heat leaves the skin store at `E_sk` and water leaves the skin bed at `E_sk / lambda`, so the
/// two stand in one fixed proportion. The sweat the skin cannot take leaves the skin bed as water alone, the
/// unevaporated sweat `(E_rsw - smooth_min(E_rsw, E_max, w)) / lambda`, and the salts ride on all the secreted sweat.
fn evaporation_and_sweat<'b>(
    laws: &mut FragmentBuilder<'b, Mt>,
    routes: &SkinRoutes<'_, 'b>,
    at_exit: ReadPort<'b, Mt, Temperature>,
    (convection, sweat): (&Convection<'b>, &SweatRate<'b>),
    class: RateClass,
) {
    let key = ParameterKey::new;
    let evaporative = Evaporative::declare(laws);
    let wettedness =
        laws.parameter::<Dimensionless>(key("mt.diffusion-wettedness"), Admissible::unit_interval());
    let latent = laws.parameter::<LatentHeat>(key("mt.latent-heat"), Admissible::positive());
    let width = laws.parameter::<Rate<Energy>>(key("mt.evaporation-width"), Admissible::positive());
    let variability = laws.forcing(
        "evaporation variability",
        key("mt.evaporation-period"),
        key("mt.evaporation-amplitude"),
    );
    let rate = laws.algebraic::<Rate<Energy>>("evaporation", Admissible::finite());
    let law = laws.kernel(|mut k| {
        let capacity = evaporative.at(&mut k, &at_exit, convection);
        let (wettedness, wandering) = (k.parameter(&wettedness), k.forcing(&variability));
        let diffusing = k.scale(capacity, wettedness);
        let diffusing = k.scale(diffusing, wandering);
        let (_, taken) = taken(&mut k, &sweat.rate, (&latent, &width), capacity);
        let dried = k.scale(taken, wettedness);
        let wetting = k.subtract(taken, dried);
        let value = k.add(diffusing, wetting);
        let own = k.algebraic(&rate);
        let residual = k.subtract(own, value);
        k.finish(residual, laws.every_mode(), class)
    });
    laws.residual(&rate, law);
    let heat_rate = laws.kernel(|mut k| {
        let value = k.algebraic(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(routes.heat, heat_rate));
    let water_rate = laws.kernel(|mut k| {
        let (value, latent) = (k.algebraic(&rate), k.parameter(&latent));
        let ratio = k.divide(value, latent);
        let value = k.as_dimension(ratio);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(routes.water, water_rate));
    let unevaporated = laws.kernel(|mut k| {
        let capacity = evaporative.at(&mut k, &at_exit, convection);
        let (secreting, taken) = taken(&mut k, &sweat.rate, (&latent, &width), capacity);
        let left = k.subtract(secreting, taken);
        let latent = k.parameter(&latent);
        let ratio = k.divide(left, latent);
        let value = k.as_dimension(ratio);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(routes.unevaporated, unevaporated));
    let secreted = &sweat.rate;
    let sodium = ("mt.sweat-sodium-floor", "mt.sweat-sodium-slope");
    salt(laws, routes.sodium, (secreted, &sweat.most), sodium, class);
    let chloride = ("mt.sweat-chloride-floor", "mt.sweat-chloride-slope");
    salt(laws, routes.chloride, (secreted, &sweat.most), chloride, class);
}

/// The water a salt rides on and the sweat gland's maximal rate, which its concentration's share is of.
type Carrier<'r, 'b> = (
    &'r AlgebraicSlot<'b, Mt, Rate<Water>>,
    &'r Parameter<'b, Mt, Rate<Water>>,
);

/// An ion the secreted sweat carries, `m_sw (c_0 + k_c f_sw)` with `f_sw = m_sw / m_max`, so its concentration rises
/// with the sweat rate whether the sweat evaporates or not, and evaporated water carries none (research 12, the row
/// 'Sweat glands, and the salt sweat carries').
fn salt<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Mt>,
    route: &Pair<'b, Mt, Q>,
    (secreted, most): Carrier<'_, 'b>,
    (floor, slope): (&'static str, &'static str),
    class: RateClass,
) {
    let key = ParameterKey::new;
    let floor = laws.parameter::<Quotient<Q, Water>>(key(floor), Admissible::nonnegative());
    let slope = laws.parameter::<Quotient<Q, Water>>(key(slope), Admissible::nonnegative());
    let rate = laws.kernel(|mut k| {
        let (water, most) = (k.algebraic(secreted), k.parameter(most));
        let share = k.divide(water, most);
        let share = k.as_dimension(share);
        let (floor, slope) = (k.parameter(&floor), k.parameter(&slope));
        let rising = k.scale(slope, share);
        let carried = k.add(floor, rising);
        let product = k.multiply(water, carried);
        let value = k.as_dimension(product);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(route, rate));
}
