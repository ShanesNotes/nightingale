use super::super::schema::{CELL_WATER_OUT, FAT_FEED, GLUCOSE_FEED, LUNG_CARBON_DIOXIDE, LUNG_OXYGEN};
use super::FIXED;
use ng_blood_chemistry::{Plasma, StrongIons, speciate};
use ng_contract::*;
use ng_metabolism::{GasRoutes, PlasmaPressure, bed_heat, burn, carbon_dioxide_diffusion, oxygen_diffusion};

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
    supplies.extend(GLUCOSE_FEED);
    supplies.extend(FAT_FEED);
    supplies.extend(CELL_WATER_OUT);
    supplies.extend([LUNG_OXYGEN, LUNG_CARBON_DIOXIDE]);
    let declaration = HeldDeclaration::<Metabolism>::new(supplies, Vec::new(), FIXED)
        .pool::<Glucose, CapillaryBed>(Space::Interstitium)
        .pool::<FattyAcids, CapillaryBed>(Space::Interstitium)
        .pool::<Lactate, CapillaryBed>(Space::Interstitium)
        .pool::<Triglyceride, Beds<Fat>>(Space::Cells)
        .pool::<Glycerol, Beds<Fat>>(Space::Cells);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    constants(laws, ports)?;
    feeds(laws, ports)?;
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
    let cells = oxygen_diffusion(&mut laws, &oxygen, oxygen_pressure)?;
    carbon_dioxide_diffusion(&mut laws, &carbon_dioxide, carbon_dioxide_pressure)?;
    burn(&mut laws, &cells)?;
    bed_heat(&mut laws, &ports)?;
    lung_exchange(&mut laws, &lungs, [oxygen_pressure, carbon_dioxide_pressure])?;
    body.add(laws.finish());
    Ok(())
}

/// The held feeds of glucose and fatty acids into each bed's cells, `k (N_set - N)`, and the held drain of their
/// water, `k_w (W - W_set)`, until the cells take fuel across their membranes (ticket 16) and the fluids move cell
/// water (ticket 17).
fn feeds<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    ports: &HeldPorts<'b, Metabolism>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
    let key = ParameterKey::new;
    let speed = laws.parameter::<Rate<Dimensionless>>(key("mt.feed-speed"), Admissible::nonnegative());
    let water_speed =
        laws.parameter::<Rate<Dimensionless>>(key("mt.cell-water-speed"), Admissible::nonnegative());
    let glucose = laws.parameter::<Glucose>(key("mt.cell-glucose-set"), Admissible::nonnegative());
    let fat = laws.parameter::<FattyAcids>(key("mt.cell-fatty-acid-set"), Admissible::nonnegative());
    let water = laws.parameter::<Water>(key("mt.cell-water-set"), Admissible::positive());
    for ((glucose_feed, fat_feed), water_out) in GLUCOSE_FEED.into_iter().zip(FAT_FEED).zip(CELL_WATER_OUT) {
        restore(laws, ports, glucose_feed, (&glucose, &speed), class, Toward::Into)?;
        restore(laws, ports, fat_feed, (&fat, &speed), class, Toward::Into)?;
        restore(laws, ports, water_out, (&water, &water_speed), class, Toward::Out)?;
    }
    Ok(())
}

enum Toward {
    Into,
    Out,
}

fn restore<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    ports: &HeldPorts<'b, Metabolism>,
    name: &'static str,
    (set, speed): (
        &Parameter<'b, Metabolism, Q>,
        &Parameter<'b, Metabolism, Rate<Dimensionless>>,
    ),
    class: RateClass,
    toward: Toward,
) -> Result<(), AdmissionError> {
    let route = ports.route::<Q>(name)?.route;
    let held = route.amount::<Q>();
    let rate = laws.kernel(|mut k| {
        let (set, held, speed) = (k.parameter(set), k.read(&held), k.parameter(speed));
        let value = match toward {
            Toward::Into => k.relax(set, held, speed),
            Toward::Out => k.relax(held, set, speed),
        };
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
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
