use ng_contract::*;

type Port<'b, U> = ReadPort<'b, Metabolism, PerSite<CapillaryBed, U>>;
type Bed<'b, 'k> = Kernel<'b, 'k, Metabolism, CapillaryBed>;
type Solubility<'b, Q> = Parameter<'b, Metabolism, Quotient<Concentration<Q>, Pressure>>;

/// A bed's plasma partial pressure of one gas, as speciation states it.
pub type PlasmaPressure<'r, 'b> = &'r dyn for<'k> Fn(&mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Pressure>;

const DIFFUSION_NS: u64 = 1_000_000_000;

pub struct GasRoutes<'b, Q> {
    pub wall: SiteRoute<'b, Metabolism, CapillaryBed, Q>,
    pub membrane: SiteRoute<'b, Metabolism, CapillaryBed, Q>,
}

impl<'b, Q: Respired> GasRoutes<'b, Q> {
    pub fn request(laws: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>) -> Self {
        Self {
            wall: ports.gas_exchange::<Q>(laws),
            membrane: ports.gas_into_cells::<Q>(laws),
        }
    }
}

/// A tissue space holds no carrier, so its partial pressure is its gas over its volume and solubility, and for carbon
/// dioxide over the stated share its buffers add: `P = C / (alpha (1 + r))`.
struct Tissue<'a, 'b, Q> {
    gas: Port<'b, Q>,
    water: Port<'b, Water>,
    solubility: &'a Solubility<'b, Q>,
    buffered: Option<Parameter<'b, Metabolism, Dimensionless>>,
}

impl<'b, Q: Substance + Plain> Tissue<'_, 'b, Q> {
    fn pressure<'k>(&self, k: &mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Pressure> {
        tissue_pressure(k, (&self.gas, &self.water), self.solubility, self.buffered.as_ref())
    }
}

fn tissue_pressure<'b, 'k, Q: Substance + Plain>(
    k: &mut Bed<'b, 'k>,
    (gas, water): (&Port<'b, Q>, &Port<'b, Water>),
    solubility: &Solubility<'b, Q>,
    buffered: Option<&Parameter<'b, Metabolism, Dimensionless>>,
) -> Expr<'b, 'k, Metabolism, Pressure> {
    let (gas, water) = (k.at_site(gas), k.at_site(water));
    let volume = k.volume(water);
    let quotient = k.divide(gas, volume);
    let content: Expr<'b, 'k, Metabolism, Concentration<Q>> = k.as_dimension(quotient);
    let mut solubility = k.parameter(solubility);
    if let Some(ratio) = buffered {
        let ratio = k.parameter(ratio);
        let buffered = k.scale(solubility, ratio);
        solubility = k.add(solubility, buffered);
    }
    let quotient = k.divide(content, solubility);
    k.as_dimension(quotient)
}

/// The oxygen partial pressure of each bed's cells, as the membrane's diffusion reads it.
pub struct CellOxygen<'b> {
    solubility: Solubility<'b, Oxygen>,
    gas: Port<'b, Oxygen>,
    water: Port<'b, Water>,
}

impl<'b> CellOxygen<'b> {
    pub(crate) fn pressure<'k>(&self, k: &mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Pressure> {
        tissue_pressure(k, (&self.gas, &self.water), &self.solubility, None)
    }
}

enum Side<'a, 'b, Q> {
    Plasma(PlasmaPressure<'a, 'b>),
    Tissue(&'a Tissue<'a, 'b, Q>),
}

impl<'b, Q: Substance + Plain> Side<'_, 'b, Q> {
    fn pressure<'k>(&self, k: &mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Pressure> {
        match self {
            Self::Plasma(pressure) => pressure(k),
            Self::Tissue(space) => space.pressure(k),
        }
    }
}

fn diffuse<'b, Q: Substance + Plain>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    route: &SiteRoute<'b, Metabolism, CapillaryBed, Q>,
    key: &'static str,
    (from, to): (Side<'_, 'b, Q>, Side<'_, 'b, Q>),
    class: RateClass,
) {
    let conductance = laws
        .site_parameter::<CapillaryBed, Conductance<Q>>(ParameterKey::new(key), Admissible::nonnegative());
    let rate = laws.site_kernel(|mut k| {
        let (high, low) = (from.pressure(&mut k), to.pressure(&mut k));
        let head = k.subtract(high, low);
        let conductance = k.site_parameter(&conductance);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), class)
    });
    laws.transfer_at(route, rate);
}

fn tissues<'a, 'b, Q: Substance + Plain>(
    routes: &GasRoutes<'b, Q>,
    solubility: &'a Solubility<'b, Q>,
    [interstitial, cellular]: [Option<Parameter<'b, Metabolism, Dimensionless>>; 2],
) -> [Tissue<'a, 'b, Q>; 2] {
    [
        Tissue {
            gas: routes.wall.to_amount::<Q>(),
            water: routes.wall.to_amount::<Water>(),
            solubility,
            buffered: interstitial,
        },
        Tissue {
            gas: routes.membrane.to_amount::<Q>(),
            water: routes.membrane.to_amount::<Water>(),
            solubility,
            buffered: cellular,
        },
    ]
}

/// Oxygen across each bed's capillary wall, from the plasma's partial pressure to the interstitium's, and across its
/// cell membrane, from the interstitium's to the cells', each `J = G (P_from - P_to)` with one conductance a bed. It
/// returns the cells' partial pressure, which the burn reads.
pub fn oxygen_diffusion<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    routes: &GasRoutes<'b, Oxygen>,
    plasma: PlasmaPressure<'_, 'b>,
) -> Result<CellOxygen<'b>, AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, DIFFUSION_NS)?;
    let solubility = laws.parameter(
        ParameterKey::new("mt.tissue-oxygen-solubility"),
        Admissible::positive(),
    );
    let [interstitium, cells] = tissues(routes, &solubility, [None, None]);
    let wall = (Side::Plasma(plasma), Side::Tissue(&interstitium));
    diffuse(laws, &routes.wall, "mt.wall-oxygen-conductance", wall, class);
    let membrane = (Side::Tissue(&interstitium), Side::Tissue(&cells));
    diffuse(
        laws,
        &routes.membrane,
        "mt.cell-oxygen-conductance",
        membrane,
        class,
    );
    Ok(CellOxygen {
        solubility,
        gas: routes.membrane.to_amount::<Oxygen>(),
        water: routes.membrane.to_amount::<Water>(),
    })
}

/// Carbon dioxide across the same two walls by the same rate. A tissue space has no acidity until ticket 18, so its
/// partial pressure sits on a stated buffer line, one ratio of bicarbonate to dissolved carbon dioxide a space.
pub fn carbon_dioxide_diffusion<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    routes: &GasRoutes<'b, CarbonDioxide>,
    plasma: PlasmaPressure<'_, 'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, DIFFUSION_NS)?;
    let key = ParameterKey::new;
    let solubility = laws.parameter(key("mt.tissue-carbon-dioxide-solubility"), Admissible::positive());
    let ratios = [
        laws.parameter(key("mt.interstitial-buffer-ratio"), Admissible::nonnegative()),
        laws.parameter(key("mt.cell-buffer-ratio"), Admissible::nonnegative()),
    ];
    let [interstitium, cells] = tissues(routes, &solubility, ratios.map(Some));
    let outward = (Side::Plasma(plasma), Side::Tissue(&interstitium));
    diffuse(
        laws,
        &routes.wall,
        "mt.wall-carbon-dioxide-conductance",
        outward,
        class,
    );
    let membrane = (Side::Tissue(&interstitium), Side::Tissue(&cells));
    diffuse(
        laws,
        &routes.membrane,
        "mt.cell-carbon-dioxide-conductance",
        membrane,
        class,
    );
    Ok(())
}
