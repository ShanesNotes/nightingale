use super::super::schema::{CELL_WATER_OUT, FAT_FEED, GLUCOSE_FEED};
use super::FIXED;
use ng_contract::catalog::{MyocardialWorkAndBedFlow, PlasmaGases, Reads, WorkOfBreathing};
use ng_contract::*;
use ng_metabolism::{GasRoutes, bed_heat, burn, carbon_dioxide_diffusion, oxygen_diffusion, tissue_heat};

constant_routes! {Metabolism;
    Sweat<Water> => "mt.sweat-water",
    Sweat<Sodium> => "mt.sweat-sodium",
    Sweat<Chloride> => "mt.sweat-chloride",
}

pub(super) fn metabolism(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let mut supplies = CONSTANT.to_vec();
    supplies.extend(GLUCOSE_FEED);
    supplies.extend(FAT_FEED);
    supplies.extend(CELL_WATER_OUT);
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
    let (mut laws, ports) = body.metabolism(Some(held))?.parts();
    let oxygen = GasRoutes::<Oxygen>::request(&mut laws, &ports);
    let carbon_dioxide = GasRoutes::<CarbonDioxide>::request(&mut laws, &ports);
    let gases: PlasmaGases<Reads<'_, Metabolism>> = laws.receive();
    let oxygen_pressure = &gases.oxygen_partial_pressure;
    let carbon_dioxide_pressure = &gases.carbon_dioxide_partial_pressure;
    let cells = oxygen_diffusion(&mut laws, &oxygen, oxygen_pressure)?;
    carbon_dioxide_diffusion(&mut laws, &carbon_dioxide, carbon_dioxide_pressure)?;
    let work: MyocardialWorkAndBedFlow<Reads<'_, Metabolism>> = laws.receive();
    let breathing: WorkOfBreathing<Reads<'_, Metabolism>> = laws.receive();
    burn(&mut laws, &cells, Some(work), Some(breathing))?;
    bed_heat(&mut laws, &ports)?;
    tissue_heat(&mut laws, &ports)?;
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
