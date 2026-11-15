use ng_contract::*;

type Port<'b, U> = ReadPort<'b, Metabolism, PerSite<CapillaryBed, U>>;
type Bed<'b, 'k> = Kernel<'b, 'k, Metabolism, CapillaryBed>;
type Solubility<'b> = Parameter<'b, Metabolism, Quotient<Concentration<Oxygen>, Pressure>>;

const DIFFUSION_NS: u64 = 1_000_000_000;

/// Oxygen's two routes at every bed: the capillary wall from plasma to the interstitium, and the cell membrane from
/// the interstitium to the cells.
pub struct OxygenRoutes<'b> {
    pub wall: SiteRoute<'b, Metabolism, CapillaryBed, Oxygen>,
    pub membrane: SiteRoute<'b, Metabolism, CapillaryBed, Oxygen>,
}

impl<'b> OxygenRoutes<'b> {
    pub fn request(laws: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>) -> Self {
        Self {
            wall: ports.gas_exchange::<Oxygen>(laws),
            membrane: ports.gas_into_cells::<Oxygen>(laws),
        }
    }
}

/// A tissue space holds no carrier, so its partial pressure is its oxygen over its volume and solubility.
struct Tissue<'b> {
    oxygen: Port<'b, Oxygen>,
    water: Port<'b, Water>,
}

/// One side of a wall: the plasma, whose partial pressure speciation states, or a tissue space.
enum Side<'a, 'b> {
    Plasma(&'a Port<'b, Pressure>),
    Tissue(&'a Tissue<'b>),
}

impl<'b> Side<'_, 'b> {
    fn pressure<'k>(&self, k: &mut Bed<'b, 'k>, solubility: &Solubility<'b>) -> Expr<'b, 'k, Metabolism, Pressure> {
        match self {
            Self::Plasma(pressure) => k.at_site(pressure),
            Self::Tissue(space) => {
                let (oxygen, water) = (k.at_site(&space.oxygen), k.at_site(&space.water));
                let volume = k.volume(water);
                let quotient = k.divide(oxygen, volume);
                let content: Expr<'b, 'k, Metabolism, Concentration<Oxygen>> = k.as_dimension(quotient);
                let solubility = k.parameter(solubility);
                let quotient = k.divide(content, solubility);
                k.as_dimension(quotient)
            }
        }
    }
}

fn diffuse<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    route: &SiteRoute<'b, Metabolism, CapillaryBed, Oxygen>,
    key: &'static str,
    (from, to): (Side<'_, 'b>, Side<'_, 'b>),
    solubility: &Solubility<'b>,
    class: RateClass,
) {
    let conductance =
        laws.site_parameter::<CapillaryBed, Conductance<Oxygen>>(ParameterKey::new(key), Admissible::nonnegative());
    let rate = laws.site_kernel(|mut k| {
        let (high, low) = (from.pressure(&mut k, solubility), to.pressure(&mut k, solubility));
        let head = k.subtract(high, low);
        let conductance = k.site_parameter(&conductance);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), class)
    });
    laws.transfer_at(route, rate);
}

/// Oxygen across each bed's capillary wall, from the plasma's partial pressure to the interstitium's, and across its
/// cell membrane, from the interstitium's to the cells', each `J = G (P_from - P_to)` with one conductance a bed.
pub fn wall_diffusion<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    routes: &OxygenRoutes<'b>,
    plasma: &Port<'b, Pressure>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, DIFFUSION_NS)?;
    let solubility = laws.parameter(ParameterKey::new("mt.tissue-oxygen-solubility"), Admissible::positive());
    let interstitium = Tissue {
        oxygen: routes.wall.to_amount::<Oxygen>(),
        water: routes.wall.to_amount::<Water>(),
    };
    let cells = Tissue {
        oxygen: routes.membrane.to_amount::<Oxygen>(),
        water: routes.membrane.to_amount::<Water>(),
    };
    let wall = (Side::Plasma(plasma), Side::Tissue(&interstitium));
    diffuse(laws, &routes.wall, "mt.wall-oxygen-conductance", wall, &solubility, class);
    let membrane = (Side::Tissue(&interstitium), Side::Tissue(&cells));
    diffuse(laws, &routes.membrane, "mt.cell-oxygen-conductance", membrane, &solubility, class);
    Ok(())
}
