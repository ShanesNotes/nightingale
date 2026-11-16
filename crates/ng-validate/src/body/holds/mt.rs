use super::super::schema::{LUNG_OXYGEN, OXYGEN_USE};
use super::{FIXED, hours};
use ng_blood_chemistry::{Plasma, oxygenate};
use ng_contract::*;
use ng_metabolism::{OxygenRoutes, wall_diffusion};

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
    supplies.push(LUNG_OXYGEN);
    let declaration = HeldDeclaration::<Metabolism>::new(supplies, Vec::new(), FIXED)
        .pool::<CarbonDioxide, CapillaryBed>(Space::Interstitium)
        .pool::<Glucose, CapillaryBed>(Space::Interstitium)
        .pool::<FattyAcids, CapillaryBed>(Space::Interstitium)
        .pool::<Lactate, CapillaryBed>(Space::Interstitium)
        .pool::<CarbonDioxide, CapillaryBed>(Space::Cells)
        .pool::<Glucose, Besides<Liver>>(Space::Cells)
        .pool::<FattyAcids, Besides<Liver>>(Space::Cells)
        .pool::<Lactate, Besides<Liver>>(Space::Cells)
        .pool::<Triglyceride, Beds<Fat>>(Space::Cells)
        .pool::<Glycerol, Beds<Fat>>(Space::Cells);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    constants(laws, ports)?;
    oxygen_use(laws, ports)?;
    let lung = ports.route::<Oxygen>(LUNG_OXYGEN)?.route;
    let (mut laws, ports) = body.metabolism(Some(held))?.parts();
    let routes = OxygenRoutes::request(&mut laws, &ports);
    let plasma = Plasma {
        water: routes.wall.from_amount::<Water>(),
        oxygen: routes.wall.from_amount::<Oxygen>(),
        hemoglobin: routes.wall.from_amount::<Hemoglobin>(),
    };
    let oxygenated = oxygenate(&mut laws, &plasma)?;
    wall_diffusion(&mut laws, &routes, &oxygenated.partial_pressure)?;
    lung_exchange(&mut laws, &lung, &oxygenated.partial_pressure)?;
    body.add(laws.finish());
    Ok(())
}

fn oxygen_use<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    ports: &HeldPorts<'b, Metabolism>,
) -> Result<(), AdmissionError> {
    let class = hours()?;
    for (_, name, key) in OXYGEN_USE {
        let route = ports.route::<Oxygen>(name)?.route;
        let used = laws.parameter::<Rate<Oxygen>>(ParameterKey::new(key), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&used);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
    }
    Ok(())
}

/// The held lung exchange until ticket 10's alveoli: `J = D (P_A - P)` into the pulmonary bed's plasma. The contract
/// has no read of one site of a class, so `P` is a body row, an exact sum of every bed's partial pressure times a
/// share that is one at the lung and zero elsewhere, and the head stays outside that row.
fn lung_exchange<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    lung: &Pair<'b, Metabolism, Oxygen>,
    plasma: &ReadPort<'b, Metabolism, PerSite<CapillaryBed, Pressure>>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
    let key = ParameterKey::new;
    let alveolar = laws.parameter::<Pressure>(key("mt.alveolar-oxygen"), Admissible::positive());
    let conductance =
        laws.parameter::<Conductance<Oxygen>>(key("mt.lung-oxygen-exchange"), Admissible::nonnegative());
    let share =
        laws.site_parameter::<CapillaryBed, Dimensionless>(key("mt.lung-share"), Admissible::unit_interval());
    let pulmonary = laws.algebraic::<Pressure>("pulmonary oxygen partial pressure", Admissible::finite());
    let row = laws.kernel(|mut k| {
        let value = k.algebraic(&pulmonary);
        k.finish(value, laws.every_mode(), class)
    });
    let each = laws.site_kernel::<CapillaryBed, _, _>(|mut k| {
        let (pressure, share) = (k.at_site(plasma), k.site_parameter(&share));
        let value = k.scale(pressure, share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.sum_at(&pulmonary, row, each);
    let rate = laws.kernel(|mut k| {
        let (alveolar, blood) = (k.parameter(&alveolar), k.algebraic(&pulmonary));
        let head = k.subtract(alveolar, blood);
        let conductance = k.parameter(&conductance);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(lung, rate));
    Ok(())
}
