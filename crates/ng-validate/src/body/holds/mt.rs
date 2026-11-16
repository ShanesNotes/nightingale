use super::super::schema::{CARBON_DIOXIDE_MADE, LUNG_CARBON_DIOXIDE, LUNG_OXYGEN, OXYGEN_USE};
use super::{FIXED, hours};
use ng_blood_chemistry::{Plasma, StrongIons, speciate};
use ng_contract::*;
use ng_metabolism::{GasRoutes, PlasmaPressure, carbon_dioxide_diffusion, oxygen_diffusion};

constant_routes! {Metabolism;
    SkinEvaporation => "mt.skin-evaporation",
    Sweat<Water> => "mt.sweat-water",
    Sweat<Sodium> => "mt.sweat-sodium",
    Sweat<Chloride> => "mt.sweat-chloride",
    DryHeatLoss => "mt.dry-heat-loss",
    SkinEvaporativeHeat => "mt.skin-evaporative-heat",
}

pub(super) fn metabolism(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let mut supplies = CONSTANT.to_vec();
    supplies.extend(OXYGEN_USE.map(|(_, name, _)| name));
    supplies.extend(CARBON_DIOXIDE_MADE);
    supplies.extend([LUNG_OXYGEN, LUNG_CARBON_DIOXIDE]);
    let declaration = HeldDeclaration::<Metabolism>::new(supplies, Vec::new(), FIXED)
        .pool::<Glucose, CapillaryBed>(Space::Interstitium)
        .pool::<FattyAcids, CapillaryBed>(Space::Interstitium)
        .pool::<Lactate, CapillaryBed>(Space::Interstitium)
        .pool::<Glucose, Besides<Liver>>(Space::Cells)
        .pool::<FattyAcids, Besides<Liver>>(Space::Cells)
        .pool::<Lactate, Besides<Liver>>(Space::Cells)
        .pool::<Triglyceride, Beds<Fat>>(Space::Cells)
        .pool::<Glycerol, Beds<Fat>>(Space::Cells);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    constants(laws, ports)?;
    respiration(laws, ports)?;
    let lungs = (
        ports.route::<Oxygen>(LUNG_OXYGEN)?.route,
        ports.route::<CarbonDioxide>(LUNG_CARBON_DIOXIDE)?.route,
    );
    let (mut laws, ports) = body.metabolism(Some(held))?.parts();
    let oxygen = GasRoutes::<Oxygen>::request(&mut laws, &ports);
    let carbon_dioxide = GasRoutes::<CarbonDioxide>::request(&mut laws, &ports);
    let wall = &oxygen.wall;
    let plasma = Plasma {
        water: wall.from_amount::<Water>(),
        oxygen: wall.from_amount::<Oxygen>(),
        carbon_dioxide: wall.from_amount::<CarbonDioxide>(),
        hemoglobin: wall.from_amount::<Hemoglobin>(),
        ions: StrongIons {
            sodium: wall.from_amount::<Sodium>(),
            potassium: wall.from_amount::<Potassium>(),
            magnesium: wall.from_amount::<Magnesium>(),
            calcium: wall.from_amount::<Calcium>(),
            chloride: wall.from_amount::<Chloride>(),
            lactate: wall.from_amount::<Lactate>(),
            ketones: wall.from_amount::<Ketones>(),
            diet_base: wall.from_amount::<DietBase>(),
            fixed_acid: wall.from_amount::<FixedAcid>(),
        },
    };
    let speciated = speciate(&mut laws, &plasma)?;
    let oxygen_pressure: PlasmaPressure<'_, '_> = &|k| k.at_site(&speciated.oxygen_partial_pressure);
    let carbon_dioxide_pressure: PlasmaPressure<'_, '_> = &|k| speciated.carbon_dioxide_partial_pressure(k);
    oxygen_diffusion(&mut laws, &oxygen, oxygen_pressure)?;
    carbon_dioxide_diffusion(&mut laws, &carbon_dioxide, carbon_dioxide_pressure)?;
    lung_exchange(&mut laws, &lungs, [oxygen_pressure, carbon_dioxide_pressure])?;
    body.add(laws.finish());
    Ok(())
}

fn respiration<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    ports: &HeldPorts<'b, Metabolism>,
) -> Result<(), AdmissionError> {
    let class = hours()?;
    let quotient = laws.parameter::<Quotient<CarbonDioxide, Oxygen>>(
        ParameterKey::new("mt.carbon-dioxide-per-oxygen"),
        Admissible::nonnegative(),
    );
    for ((_, name, key), made) in OXYGEN_USE.into_iter().zip(CARBON_DIOXIDE_MADE) {
        let route = ports.route::<Oxygen>(name)?.route;
        let used = laws.parameter::<Rate<Oxygen>>(ParameterKey::new(key), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&used);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        let route = ports.route::<CarbonDioxide>(made)?.route;
        let rate = laws.kernel(|mut k| {
            let (used, quotient) = (k.parameter(&used), k.parameter(&quotient));
            let product = k.multiply(used, quotient);
            let value: Expr<'_, '_, Metabolism, Rate<CarbonDioxide>> = k.as_dimension(product);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
    }
    Ok(())
}

/// The held lung exchange until ticket 10's alveoli: `J = D (P_A - P)` of oxygen into the pulmonary bed's plasma and
/// `J = D (P - P_A)` of carbon dioxide out of it. The contract has no read of one site of a class, so each `P` is a
/// body row, an exact sum of every bed's partial pressure times a share that is one at the lung and zero elsewhere,
/// and the head stays outside that row.
fn lung_exchange<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    (oxygen, carbon_dioxide): &(Pair<'b, Metabolism, Oxygen>, Pair<'b, Metabolism, CarbonDioxide>),
    [oxygen_pressure, carbon_dioxide_pressure]: [PlasmaPressure<'_, 'b>; 2],
) -> Result<(), AdmissionError> {
    let share = laws.site_parameter::<CapillaryBed, Dimensionless>(
        ParameterKey::new("mt.lung-share"),
        Admissible::unit_interval(),
    );
    let oxygen_keys = [
        "pulmonary oxygen partial pressure",
        "mt.alveolar-oxygen",
        "mt.lung-oxygen-exchange",
    ];
    let pulmonary = pulmonary_row(laws, oxygen_keys, &share, oxygen_pressure)?;
    exchange(laws, oxygen, oxygen_keys, &pulmonary, Head::Into)?;
    let carbon_dioxide_keys = [
        "pulmonary carbon dioxide partial pressure",
        "mt.alveolar-carbon-dioxide",
        "mt.lung-carbon-dioxide-exchange",
    ];
    let pulmonary = pulmonary_row(laws, carbon_dioxide_keys, &share, carbon_dioxide_pressure)?;
    exchange(laws, carbon_dioxide, carbon_dioxide_keys, &pulmonary, Head::Out)
}

fn pulmonary_row<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    [name, ..]: [&'static str; 3],
    share: &SiteParameter<'b, Metabolism, CapillaryBed, Dimensionless>,
    plasma: PlasmaPressure<'_, 'b>,
) -> Result<AlgebraicSlot<'b, Metabolism, Pressure>, AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
    let pulmonary = laws.algebraic::<Pressure>(name, Admissible::positive());
    let row = laws.kernel(|mut k| {
        let value = k.algebraic(&pulmonary);
        k.finish(value, laws.every_mode(), class)
    });
    let each = laws.site_kernel::<CapillaryBed, _, _>(|mut k| {
        let (pressure, share) = (plasma(&mut k), k.site_parameter(share));
        let value = k.scale(pressure, share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.sum_at(&pulmonary, row, each);
    Ok(pulmonary)
}

enum Head {
    Into,
    Out,
}

fn exchange<'b, Q: Substance + Plain>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    lung: &Pair<'b, Metabolism, Q>,
    [_, alveolar, conductance]: [&'static str; 3],
    pulmonary: &AlgebraicSlot<'b, Metabolism, Pressure>,
    head: Head,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
    let key = ParameterKey::new;
    let alveolar = laws.parameter::<Pressure>(key(alveolar), Admissible::positive());
    let conductance = laws.parameter::<Conductance<Q>>(key(conductance), Admissible::nonnegative());
    let rate = laws.kernel(|mut k| {
        let (alveolar, blood) = (k.parameter(&alveolar), k.algebraic(pulmonary));
        let head = match head {
            Head::Into => k.subtract(alveolar, blood),
            Head::Out => k.subtract(blood, alveolar),
        };
        let conductance = k.parameter(&conductance);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(lung, rate));
    Ok(())
}
