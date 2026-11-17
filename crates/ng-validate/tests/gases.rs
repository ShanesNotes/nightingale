//! One bed whose plasma a held artery renews and whose cells use oxygen and make carbon dioxide at held rates, or burn
//! glucose and fatty acids by metabolism's laws from held feeds, so blood chemistry's speciation, metabolism's wall
//! diffusion and its burn meet closed forms with no body admission. Every value is synthetic.

use core::f64::consts::LN_10;
use core::num::NonZeroU64;
use ng_blood_chemistry::speciate;
use ng_contract::catalog::{PlasmaGases, Reads};
use ng_contract::*;
use ng_metabolism::{GasRoutes, bed_heat, burn, carbon_dioxide_diffusion, oxygen_diffusion};
use ng_record::{Running, Session, Step};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const NOTE: &str = "gas fixture: synthetic";
const BED: &str = "bed";
const SPACES: [Space; 3] = [Space::Plasma, Space::Interstitium, Space::Cells];
const WATER: [f64; 3] = [100.0, 300.0, 600.0];
const HEMOGLOBIN: f64 = 2.0;
const SOLUBILITY: f64 = 0.001;
const HEME: f64 = 4.0;
const P50: f64 = 25.0;
const HILL: (f64, f64, f64) = (2.5, 1.0, 10.0);
struct ByAcidity {
    standard: f64,
    cubic: [f64; 3],
}
const BY_ACIDITY: ByAcidity = ByAcidity {
    standard: 7.5,
    cubic: [-1.0, 0.5, -0.25],
};
struct ByCarbonDioxide {
    standard: f64,
    scale: f64,
    quadratic: [f64; 2],
}
const BY_CARBON_DIOXIDE: ByCarbonDioxide = ByCarbonDioxide {
    standard: 30.0,
    scale: 10.0,
    quadratic: [0.1, 0.01],
};
const CARBON_DIOXIDE_SOLUBILITY: f64 = 0.02;
const PK: f64 = 6.0;
const STRONG_ION_OFFSET: f64 = 50.0;
struct WeakAcid {
    concentration: f64,
    slope: f64,
    offset: f64,
}
const ALBUMIN: WeakAcid = WeakAcid {
    concentration: 1.0,
    slope: 10.0,
    offset: 60.0,
};
const PHOSPHATE: WeakAcid = WeakAcid {
    concentration: 1.0,
    slope: 1.0,
    offset: 6.0,
};
const HEMOGLOBIN_CHARGE: (f64, f64) = (5.0, 30.0);
struct Carbamino {
    sites: f64,
    half: f64,
    half_rise: f64,
}
const CARBAMINO: Carbamino = Carbamino {
    sites: 4.0,
    half: 10.0,
    half_rise: 10.0,
};
const BUFFER_RATIOS: (f64, f64) = (15.0, 10.0);
const IONS: [(Species, f64, f64); 9] = [
    (Species::Sodium, 103.0, 1.0),
    (Species::Potassium, 4.5, 1.0),
    (Species::Magnesium, 0.8, 2.0),
    (Species::Calcium, 1.3, 2.0),
    (Species::Chloride, 97.0, -1.0),
    (Species::Lactate, 1.7, -1.0),
    (Species::Ketones, 0.4, -1.0),
    (Species::DietBase, 2.2, -1.0),
    (Species::FixedAcid, 0.9, -1.0),
];
const RENEWAL: f64 = 1.0;
#[derive(Clone, Copy, Debug)]
struct Point {
    ln_over_p50: f64,
    carbon_dioxide_pressure: f64,
}
const ARTERIAL: Point = Point {
    ln_over_p50: 1.3,
    carbon_dioxide_pressure: 31.0,
};
const VENOUS: Point = Point {
    ln_over_p50: 0.5,
    carbon_dioxide_pressure: 35.0,
};
const HEAD: f64 = 5.0;
const CARBON_DIOXIDE_HEAD: f64 = 1.0;
const DEMAND: f64 = 100.0;
const CELL_PRESSURE_AT_DEMAND: f64 = 20.0;
const WIDTH_SHARE_OF_DEMAND: f64 = 0.02;
const GLUCOSE_SHARE: f64 = 5.0 / 28.0;
const QUOTIENT: f64 = 0.75;
const GLUCOSE_ENERGY: f64 = 1_000.0;
const FAT_ENERGY: f64 = 3_000.0;
const BED_HEAT_SPEED: f64 = 0.05;
const FEED_SPEED: f64 = 0.1;
const GLUCOSE_SET: f64 = 3_000.0;
const FAT_SET: f64 = 600.0;
const CELL_WATER_SPEED: f64 = 10.0;

fn ten_to(power: f64) -> f64 {
    exp(power * LN_10)
}

fn weak(acidity: f64) -> f64 {
    let acids: f64 = ([ALBUMIN, PHOSPHATE].iter())
        .map(|acid| acid.concentration * (acid.slope * acidity - acid.offset))
        .sum();
    acids + HEMOGLOBIN * (HEMOGLOBIN_CHARGE.0 * acidity - HEMOGLOBIN_CHARGE.1)
}

fn strong_ions() -> f64 {
    IONS.iter().map(|(_, held, charge)| held * charge).sum()
}

fn bicarbonate(acidity: f64) -> f64 {
    strong_ions() + STRONG_ION_OFFSET - weak(acidity)
}

fn half(acidity: f64, pressure: f64) -> f64 {
    let d = acidity - BY_ACIDITY.standard;
    let [c1, c2, c3] = BY_ACIDITY.cubic;
    let u = (pressure - BY_CARBON_DIOXIDE.standard) / BY_CARBON_DIOXIDE.scale;
    let [q1, q2] = BY_CARBON_DIOXIDE.quadratic;
    P50 * (1.0 + c1 * d + c2 * d * d + c3 * d * d * d) * (1.0 + q1 * u + q2 * u * u)
}

fn carbamino(dissolved: f64, saturation: f64) -> f64 {
    let reach = CARBAMINO.half + CARBAMINO.half_rise * saturation;
    CARBAMINO.sites * HEMOGLOBIN * dissolved / (dissolved + reach)
}

#[derive(Clone, Copy, Debug)]
struct Speciated {
    oxygen_pressure: f64,
    saturation: f64,
    oxygen_content: f64,
    acidity: f64,
    bicarbonate: f64,
    carbon_dioxide_pressure: f64,
    carbon_dioxide_total: f64,
}

/// The plasma at a carbon dioxide partial pressure, whose charge balance sets the acidity by bisection, and at an
/// oxygen partial pressure `P50 e^t` with P50 at that acidity and partial pressure.
fn speciated(point: Point) -> Speciated {
    let (t, pressure) = (point.ln_over_p50, point.carbon_dioxide_pressure);
    let (mut low, mut high) = (5.0, 9.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if bicarbonate(middle) > CARBON_DIOXIDE_SOLUBILITY * pressure * ten_to(middle - PK) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let acidity = 0.5 * (low + high);
    let oxygen = half(acidity, pressure) * exp(t);
    let exponent = HILL.0 - HILL.1 * exp(-oxygen / HILL.2);
    let saturation = 1.0 / (1.0 + exp(-exponent * t));
    let dissolved = CARBON_DIOXIDE_SOLUBILITY * pressure;
    Speciated {
        oxygen_pressure: oxygen,
        saturation,
        oxygen_content: SOLUBILITY * oxygen + HEME * HEMOGLOBIN * saturation,
        acidity,
        bicarbonate: bicarbonate(acidity),
        carbon_dioxide_pressure: pressure,
        carbon_dioxide_total: bicarbonate(acidity) + dissolved + carbamino(dissolved, saturation),
    }
}

fn flow(renewal: f64) -> f64 {
    renewal * WATER[0]
}

fn used() -> f64 {
    flow(RENEWAL) * (speciated(ARTERIAL).oxygen_content - speciated(VENOUS).oxygen_content)
}

fn made() -> f64 {
    flow(RENEWAL) * (speciated(VENOUS).carbon_dioxide_total - speciated(ARTERIAL).carbon_dioxide_total)
}

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    Ok(AnatomyBuilder::new().site::<C>(key, origin).id())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cells {
    HeldUse,
    Burning,
    OneMixedBurn,
    BurningUnderTwoCards,
}

fn compiled(cells: Cells) -> Result<CompiledBody, AdmissionError> {
    let origin = || -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?).id();
    let vessel = map.site::<VesselSegment>("vessel", origin()?).id();
    let bed_site = map.site::<CapillaryBed>(BED, origin()?);
    let bed = bed_site.id();
    if cells == Cells::BurningUnderTwoCards {
        map.serves(&bed_site, Tissue::Muscle);
    }
    map.contain(root, vessel);
    map.contain(root, bed);
    map.connect(Network::Vascular, vessel, bed);
    map.connect(Network::Vascular, bed, vessel);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Lumen, "no vessel blood in this fixture");
    schema.partial_volume(Measure::new(1.0)?)?;
    for space in SPACES {
        schema.carry::<Oxygen, CapillaryBed>(space);
        schema.carry::<CarbonDioxide, CapillaryBed>(space);
    }
    schema.carry::<Hemoglobin, CapillaryBed>(Space::Plasma);
    schema.carry::<Sodium, CapillaryBed>(Space::Plasma);
    schema.carry::<Potassium, CapillaryBed>(Space::Plasma);
    schema.carry::<Magnesium, CapillaryBed>(Space::Plasma);
    schema.carry::<Calcium, CapillaryBed>(Space::Plasma);
    schema.carry::<Chloride, CapillaryBed>(Space::Plasma);
    schema.carry::<Lactate, CapillaryBed>(Space::Plasma);
    schema.carry::<Ketones, CapillaryBed>(Space::Plasma);
    schema.carry::<DietBase, CapillaryBed>(Space::Plasma);
    schema.carry::<FixedAcid, CapillaryBed>(Space::Plasma);
    let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
    let burns = cells != Cells::HeldUse;
    schema.boundary::<Oxygen>("artery oxygen", into, bed, Space::Plasma)?;
    schema.boundary::<Oxygen>("vein oxygen", out, bed, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("artery carbon dioxide", into, bed, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("vein carbon dioxide", out, bed, Space::Plasma)?;
    if burns {
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
        schema.carry::<FattyAcids, CapillaryBed>(Space::Cells);
        schema.carry::<Energy, CapillaryBed>(Space::Thermal);
        schema.pool::<Energy>(root, Space::Thermal)?;
        schema.energy_content::<Glucose>(Measure::new(GLUCOSE_ENERGY)?)?;
        schema.energy_content::<FattyAcids>(Measure::new(FAT_ENERGY)?)?;
        schema.boundary::<Glucose>("glucose feed", into, bed, Space::Cells)?;
        schema.boundary::<FattyAcids>("fatty acid feed", into, bed, Space::Cells)?;
        schema.boundary::<Water>("cell water out", out, bed, Space::Cells)?;
    } else {
        schema.boundary::<Oxygen>("oxygen use", out, bed, Space::Cells)?;
        schema.boundary::<CarbonDioxide>("carbon dioxide made", into, bed, Space::Cells)?;
    }
    assemble(schema, |mut body| {
        let mut held = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), NOTE);
        for space in SPACES
            .into_iter()
            .filter(|&space| !(burns && space == Space::Cells))
        {
            held = held.pool::<Water, CapillaryBed>(space);
        }
        let plasma = Space::Plasma;
        let held = (held.pool::<Hemoglobin, CapillaryBed>(plasma))
            .pool::<Sodium, CapillaryBed>(plasma)
            .pool::<Potassium, CapillaryBed>(plasma)
            .pool::<Magnesium, CapillaryBed>(plasma)
            .pool::<Calcium, CapillaryBed>(plasma)
            .pool::<Chloride, CapillaryBed>(plasma)
            .pool::<Lactate, CapillaryBed>(plasma)
            .pool::<Ketones, CapillaryBed>(plasma)
            .pool::<DietBase, CapillaryBed>(plasma)
            .pool::<FixedAcid, CapillaryBed>(plasma);
        let held = body.hold(held)?;
        let (mut laws, ports) = body.blood_chemistry(Some(held))?.parts();
        speciate(&mut laws, &ports)?;
        body.add(laws.finish());

        let (mut laws, ports) = body.metabolism(None)?.parts();
        let oxygen = GasRoutes::<Oxygen>::request(&mut laws, &ports);
        let carbon_dioxide = GasRoutes::<CarbonDioxide>::request(&mut laws, &ports);
        let gases: PlasmaGases<Reads<'_, Metabolism>> = laws.receive();
        let cell_oxygen = oxygen_diffusion(&mut laws, &oxygen, &gases.oxygen_partial_pressure)?;
        carbon_dioxide_diffusion(&mut laws, &carbon_dioxide, &gases.carbon_dioxide_partial_pressure)?;
        match cells {
            Cells::HeldUse => {}
            Cells::Burning | Cells::BurningUnderTwoCards => burn(&mut laws, &cell_oxygen)?,
            Cells::OneMixedBurn => {
                let at = Space::Cells;
                let mixed = laws.reaction::<CapillaryBed, Oxygen, 168>(
                    "mixed burn",
                    Proportions::<Oxygen, 168>::using(at)
                        .uses::<Glucose, 5>(at)
                        .uses::<FattyAcids, 6>(at)
                        .makes::<CarbonDioxide, 126>(at)
                        .makes::<Water, 126>(at)
                        .energy_into(Space::Thermal),
                );
                let class = RateClass::new(RateBand::Reflex, SECOND)?;
                let rate = laws.site_kernel(|mut k| {
                    let zero = k.zero::<Rate<Oxygen>>();
                    k.finish(zero, laws.every_mode(), class)
                });
                laws.react_at(&mixed, rate);
            }
        }
        if burns {
            bed_heat(&mut laws, &ports)?;
        }
        body.add(laws.finish());

        if cells == Cells::BurningUnderTwoCards {
            let held = HeldDeclaration::<Musculoskeletal>::new(Vec::new(), Vec::new(), NOTE);
            let (mut laws, _) = body.hold(held)?.parts();
            let at = Space::Cells;
            let glucose = laws.reaction::<Beds<Muscle>, Glucose, 1>(
                "glucose burn",
                Proportions::<Glucose>::using(at)
                    .uses::<Oxygen, 6>(at)
                    .makes::<CarbonDioxide, 6>(at)
                    .makes::<Water, 6>(at)
                    .energy_into(Space::Thermal),
            );
            let class = RateClass::new(RateBand::Reflex, SECOND)?;
            let rate = laws.site_kernel(|mut k| {
                let zero = k.zero::<Rate<Glucose>>();
                k.finish(zero, laws.every_mode(), class)
            });
            laws.react_at(&glucose, rate);
            body.add(laws.finish());
        }

        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Reflex, SECOND)?;
        let key = ParameterKey::new;
        let renewal =
            laws.parameter::<Rate<Dimensionless>>(key("fixture.renewal"), Admissible::nonnegative());
        held_rate::<Oxygen>(
            &mut laws,
            &ports,
            "artery oxygen",
            "fixture.arterial-oxygen",
            class,
        )?;
        let delivered = ("artery carbon dioxide", "fixture.arterial-carbon-dioxide");
        held_rate::<CarbonDioxide>(&mut laws, &ports, delivered.0, delivered.1, class)?;
        if !burns {
            held_rate::<Oxygen>(&mut laws, &ports, "oxygen use", "fixture.oxygen-use", class)?;
            let made = ("carbon dioxide made", "fixture.carbon-dioxide-made");
            held_rate::<CarbonDioxide>(&mut laws, &ports, made.0, made.1, class)?;
        }
        let route = ports.route::<Oxygen>("vein oxygen")?.route;
        let held = route.amount::<Oxygen>();
        let law = laws.kernel(|mut k| {
            let (held, zero, speed) = (k.read(&held), k.zero(), k.parameter(&renewal));
            let value = k.relax(held, zero, speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, law));
        let route = ports.route::<CarbonDioxide>("vein carbon dioxide")?.route;
        let held = route.amount::<CarbonDioxide>();
        let law = laws.kernel(|mut k| {
            let (held, zero, speed) = (k.read(&held), k.zero(), k.parameter(&renewal));
            let value = k.relax(held, zero, speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, law));
        if burns {
            let speed = laws.parameter(key("fixture.feed-speed"), Admissible::nonnegative());
            let feed = (into, &speed);
            restoring::<Glucose>(
                &mut laws,
                &ports,
                "glucose feed",
                ("fixture.glucose-set", feed),
                class,
            )?;
            let fat = ("fixture.fatty-acid-set", feed);
            restoring::<FattyAcids>(&mut laws, &ports, "fatty acid feed", fat, class)?;
            let speed = laws.parameter(key("fixture.cell-water-speed"), Admissible::nonnegative());
            let water = ("fixture.cell-water-set", (out, &speed));
            restoring::<Water>(&mut laws, &ports, "cell water out", water, class)?;
        }
        body.add(laws.finish());
        body.compile()
    })
}

fn held_rate<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    key: &'static str,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let rate = laws.parameter::<Rate<Q>>(ParameterKey::new(key), Admissible::nonnegative());
    let route = ports.route::<Q>(name)?.route;
    let law = laws.kernel(|mut k| {
        let value = k.parameter(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, law));
    Ok(())
}

fn restoring<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    (set, (direction, speed)): (
        &'static str,
        (BoundaryDirection, &Parameter<'b, Boundary, Rate<Dimensionless>>),
    ),
    class: RateClass,
) -> Result<(), AdmissionError> {
    let set = laws.parameter::<Q>(ParameterKey::new(set), Admissible::nonnegative());
    let route = ports.route::<Q>(name)?.route;
    let held = route.amount::<Q>();
    let law = laws.kernel(|mut k| {
        let (set, held, speed) = (k.parameter(&set), k.read(&held), k.parameter(speed));
        let value = match direction {
            BoundaryDirection::IntoBody => k.relax(set, held, speed),
            BoundaryDirection::OutOfBody => k.relax(held, set, speed),
        };
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, law));
    Ok(())
}

fn solved(content: f64, pressure: f64) -> Speciated {
    let (mut low, mut high) = (-30.0, 15.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        let point = Point {
            ln_over_p50: middle,
            carbon_dioxide_pressure: pressure,
        };
        if speciated(point).oxygen_content < content {
            low = middle;
        } else {
            high = middle;
        }
    }
    speciated(Point {
        ln_over_p50: 0.5 * (low + high),
        carbon_dioxide_pressure: pressure,
    })
}

/// The plasma whose oxygen content and carbon dioxide content are `content` and `total`, by bisection on the carbon
/// dioxide partial pressure, which carbamino's reading of saturation couples to the oxygen.
fn solved_both(content: f64, total: f64) -> Speciated {
    let (mut low, mut high) = (1.0, 200.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if solved(content, middle).carbon_dioxide_total < total {
            low = middle;
        } else {
            high = middle;
        }
    }
    solved(content, 0.5 * (low + high))
}

#[derive(Clone, Copy, Debug)]
struct Fields {
    oxygen_pressure: f64,
    saturation: f64,
    bicarbonate: f64,
    acidity: f64,
}

#[derive(Clone, Copy, Debug)]
struct Start {
    plasma: Speciated,
    guess: Fields,
    arterial: Speciated,
    renewal: f64,
    demand: f64,
    cell_conductance: f64,
}
impl Start {
    fn at(plasma: Speciated) -> Self {
        Self {
            plasma,
            guess: Fields {
                oxygen_pressure: plasma.oxygen_pressure,
                saturation: plasma.saturation,
                bicarbonate: plasma.bicarbonate,
                acidity: plasma.acidity,
            },
            arterial: speciated(ARTERIAL),
            renewal: RENEWAL,
            demand: DEMAND,
            cell_conductance: used() / HEAD,
        }
    }
    fn cold(plasma: Speciated) -> Self {
        Self {
            guess: Fields {
                oxygen_pressure: 1.0,
                saturation: 0.01,
                bicarbonate: 1.0,
                acidity: 7.0,
            },
            ..Self::at(plasma)
        }
    }
}

fn guess(row: RowKind, start: Start) -> f64 {
    let settled = speciated(VENOUS);
    let space = |space| {
        SPACES
            .iter()
            .position(|of| *of == space)
            .expect("a fixture space")
    };
    let tissue = |ratio: f64, heads: f64| {
        CARBON_DIOXIDE_SOLUBILITY
            * (1.0 + ratio)
            * (settled.carbon_dioxide_pressure + heads * CARBON_DIOXIDE_HEAD)
    };
    match row {
        RowKind::Amount {
            species: Species::Glucose,
            ..
        } => GLUCOSE_SET,
        RowKind::Amount {
            species: Species::FattyAcids,
            ..
        } => FAT_SET,
        RowKind::Amount {
            species: Species::Energy,
            ..
        } => 0.0,
        RowKind::Amount {
            species, space: at, ..
        } => {
            let water = WATER[space(at)];
            water
                * match (species, at) {
                    (Species::Water, _) => 1.0,
                    (Species::Hemoglobin, _) => HEMOGLOBIN,
                    (Species::Oxygen, Space::Plasma) => start.plasma.oxygen_content,
                    (Species::Oxygen, Space::Interstitium) => SOLUBILITY * (settled.oxygen_pressure - HEAD),
                    (Species::Oxygen, _) => SOLUBILITY * (settled.oxygen_pressure - 2.0 * HEAD),
                    (Species::CarbonDioxide, Space::Plasma) => start.plasma.carbon_dioxide_total,
                    (Species::CarbonDioxide, Space::Interstitium) => tissue(BUFFER_RATIOS.0, 1.0),
                    (Species::CarbonDioxide, _) => tissue(BUFFER_RATIOS.1, 2.0),
                    (ion, _) => {
                        let named = IONS.iter().find(|(named, ..)| *named == ion);
                        named.unwrap_or_else(|| panic!("no start for {row:?}")).1
                    }
                }
        }
        RowKind::Algebraic(slot) => match slot.law.local {
            "oxygen partial pressure" => start.guess.oxygen_pressure,
            "oxygen saturation" => start.guess.saturation,
            "bicarbonate" => start.guess.bicarbonate,
            "acidity" => start.guess.acidity,
            _ => panic!("no start for {row:?}"),
        },
        other => panic!("no start for {other:?}"),
    }
}

fn packet(body: &CompiledBody, start: Start) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    packet_from(body, start, |row| guess(row, start))
}

fn packet_from(
    body: &CompiledBody,
    start: Start,
    guesses: impl Fn(RowKind) -> f64,
) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::new();
    fn set<U: Dimension>(
        packet: &mut ParameterPacket<SyntheticFixture>,
        key: &'static str,
        binding: Binding,
        value: f64,
    ) -> Result<(), AdmissionError> {
        packet.synthetic(ParameterKey::new(key), binding, Measure::<U>::new(value)?, NOTE)
    }
    let body_wide = Binding::Body;
    type Solubility = Quotient<Concentration<Oxygen>, Pressure>;
    set::<Solubility>(&mut packet, "bc.oxygen-solubility", body_wide, SOLUBILITY)?;
    set::<Solubility>(&mut packet, "mt.tissue-oxygen-solubility", body_wide, SOLUBILITY)?;
    set::<Quotient<Concentration<Oxygen>, Concentration<Hemoglobin>>>(
        &mut packet,
        "bc.heme-oxygen",
        body_wide,
        HEME,
    )?;
    type Dissolving = Quotient<Concentration<CarbonDioxide>, Pressure>;
    for key in [
        "bc.carbon-dioxide-solubility",
        "mt.tissue-carbon-dioxide-solubility",
    ] {
        set::<Dissolving>(&mut packet, key, body_wide, CARBON_DIOXIDE_SOLUBILITY)?;
    }
    for (key, value) in [
        ("bc.p50", P50),
        ("bc.hill-reach", HILL.2),
        ("bc.standard-carbon-dioxide", BY_CARBON_DIOXIDE.standard),
        ("bc.carbon-dioxide-scale", BY_CARBON_DIOXIDE.scale),
    ] {
        set::<Pressure>(&mut packet, key, body_wide, value)?;
    }
    let ([c1, c2, c3], [q1, q2]) = (BY_ACIDITY.cubic, BY_CARBON_DIOXIDE.quadratic);
    for (key, value) in [
        ("bc.hill-ceiling", HILL.0),
        ("bc.hill-fall", HILL.1),
        ("bc.standard-acidity", BY_ACIDITY.standard),
        ("bc.p50-acidity-1", c1),
        ("bc.p50-acidity-2", c2),
        ("bc.p50-acidity-3", c3),
        ("bc.p50-carbon-dioxide-1", q1),
        ("bc.p50-carbon-dioxide-2", q2),
        ("bc.pk", PK),
        ("bc.ln-ten", LN_10),
        ("mt.interstitial-buffer-ratio", BUFFER_RATIOS.0),
        ("mt.cell-buffer-ratio", BUFFER_RATIOS.1),
    ] {
        set::<Dimensionless>(&mut packet, key, body_wide, value)?;
    }
    set::<Concentration<Charge>>(&mut packet, "bc.strong-ion-offset", body_wide, STRONG_ION_OFFSET)?;
    type PerCharge = Quotient<Concentration<CarbonDioxide>, Concentration<Charge>>;
    set::<PerCharge>(&mut packet, "bc.bicarbonate-per-charge", body_wide, 1.0)?;
    set::<Concentration<Albumin>>(&mut packet, "bc.plasma-albumin", body_wide, ALBUMIN.concentration)?;
    set::<Concentration<Phosphate>>(
        &mut packet,
        "bc.plasma-phosphate",
        body_wide,
        PHOSPHATE.concentration,
    )?;
    type PerAcid<Q> = Quotient<Concentration<Charge>, Concentration<Q>>;
    set::<PerAcid<Albumin>>(&mut packet, "bc.albumin-slope", body_wide, ALBUMIN.slope)?;
    set::<PerAcid<Albumin>>(&mut packet, "bc.albumin-offset", body_wide, ALBUMIN.offset)?;
    set::<PerAcid<Phosphate>>(&mut packet, "bc.phosphate-slope", body_wide, PHOSPHATE.slope)?;
    set::<PerAcid<Phosphate>>(&mut packet, "bc.phosphate-offset", body_wide, PHOSPHATE.offset)?;
    set::<PerAcid<Hemoglobin>>(&mut packet, "bc.hemoglobin-slope", body_wide, HEMOGLOBIN_CHARGE.0)?;
    set::<PerAcid<Hemoglobin>>(
        &mut packet,
        "bc.hemoglobin-offset",
        body_wide,
        HEMOGLOBIN_CHARGE.1,
    )?;
    type Sites = Quotient<Concentration<CarbonDioxide>, Concentration<Hemoglobin>>;
    set::<Sites>(&mut packet, "bc.carbamino-sites", body_wide, CARBAMINO.sites)?;
    set::<Concentration<CarbonDioxide>>(&mut packet, "bc.carbamino-half", body_wide, CARBAMINO.half)?;
    set::<Concentration<CarbonDioxide>>(
        &mut packet,
        "bc.carbamino-half-rise",
        body_wide,
        CARBAMINO.half_rise,
    )?;
    let beds = Binding::Class(Class::CapillaryBed);
    set::<Conductance<Oxygen>>(&mut packet, "mt.wall-oxygen-conductance", beds, used() / HEAD)?;
    set::<Conductance<Oxygen>>(
        &mut packet,
        "mt.cell-oxygen-conductance",
        beds,
        start.cell_conductance,
    )?;
    for key in [
        "mt.wall-carbon-dioxide-conductance",
        "mt.cell-carbon-dioxide-conductance",
    ] {
        set::<Conductance<CarbonDioxide>>(&mut packet, key, beds, made() / CARBON_DIOXIDE_HEAD)?;
    }
    let flow = flow(start.renewal);
    set::<Rate<Oxygen>>(
        &mut packet,
        "fixture.arterial-oxygen",
        body_wide,
        flow * start.arterial.oxygen_content,
    )?;
    set::<Rate<CarbonDioxide>>(
        &mut packet,
        "fixture.arterial-carbon-dioxide",
        body_wide,
        flow * start.arterial.carbon_dioxide_total,
    )?;
    if declares(body, "mt.oxygen-demand") {
        set::<Rate<Oxygen>>(&mut packet, "mt.oxygen-demand", beds, start.demand)?;
        set::<Conductance<Oxygen>>(
            &mut packet,
            "mt.oxygen-reach",
            beds,
            DEMAND / CELL_PRESSURE_AT_DEMAND,
        )?;
        set::<Dimensionless>(
            &mut packet,
            "mt.oxygen-rate-width-share",
            beds,
            WIDTH_SHARE_OF_DEMAND,
        )?;
        set::<Dimensionless>(&mut packet, "mt.glucose-share", beds, GLUCOSE_SHARE)?;
        set::<Rate<Dimensionless>>(&mut packet, "mt.bed-heat-speed", beds, BED_HEAT_SPEED)?;
        set::<Rate<Dimensionless>>(&mut packet, "fixture.feed-speed", body_wide, FEED_SPEED)?;
        set::<Glucose>(&mut packet, "fixture.glucose-set", body_wide, GLUCOSE_SET)?;
        set::<FattyAcids>(&mut packet, "fixture.fatty-acid-set", body_wide, FAT_SET)?;
        set::<Water>(&mut packet, "fixture.cell-water-set", body_wide, WATER[2])?;
        set::<Rate<Dimensionless>>(
            &mut packet,
            "fixture.cell-water-speed",
            body_wide,
            CELL_WATER_SPEED,
        )?;
    } else {
        set::<Rate<Oxygen>>(&mut packet, "fixture.oxygen-use", body_wide, used())?;
        set::<Rate<CarbonDioxide>>(&mut packet, "fixture.carbon-dioxide-made", body_wide, made())?;
    }
    set::<Rate<Dimensionless>>(&mut packet, "fixture.renewal", body_wide, start.renewal)?;
    packet.start_rest(body, guesses, NOTE)?;
    packet.within(ErrorScales::uniform(Measure::new(1e-9)?, Measure::new(1e-10)?)?);
    Ok(packet)
}

fn declares(body: &CompiledBody, key: &'static str) -> bool {
    let key = ParameterKey::new(key);
    body.parameters().iter().any(|slot| slot.key() == key)
}

fn admitted<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
    start: Start,
) -> Result<Session<'m, Running>, InitializationFailure> {
    admit(body, core, store, packet(body, start)?)
}

fn admit<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
    packet: ParameterPacket<SyntheticFixture>,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let lens = body.lens(Vec::new(), NonZeroU64::MIN.saturating_add(SECOND - 1))?;
    let port = BoundaryPort::new(Vec::new())?;
    Ok(admit_solved(body, packet, port, core, &lens, store)?.session)
}

fn value(body: &CompiledBody, session: &Session<'_, Running>, kind: RowKind) -> f64 {
    let row =
        (body.rows().iter().find(|row| row.kind() == kind)).unwrap_or_else(|| panic!("no row {kind:?}"));
    f64::from_bits(session.record().accepted().value(row.id()).bits()[0])
}

fn field(local: &'static str) -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Algebraic(Slot {
        law: LawName {
            owner: BloodChemistry::CARD,
            local,
        },
        at: Context::Site(site::<CapillaryBed>(BED)?),
    }))
}

fn amount(species: Species, space: Space) -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Amount {
        site: site::<CapillaryBed>(BED)?,
        species,
        space,
    })
}

fn fields(body: &CompiledBody, session: &Session<'_, Running>) -> Result<Fields, AdmissionError> {
    Ok(Fields {
        oxygen_pressure: value(body, session, field("oxygen partial pressure")?),
        saturation: value(body, session, field("oxygen saturation")?),
        bicarbonate: value(body, session, field("bicarbonate")?),
        acidity: value(body, session, field("acidity")?),
    })
}

fn after<'m>(session: Session<'m, Running>, seconds: u64) -> Session<'m, Running> {
    run_until(session, SimTime::from_ns(seconds * SECOND))
        .unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()))
}

/// Each point is off both standards of P50's factors, so a factor's shift is read at a value.
#[test]
fn speciation_meets_its_closed_forms_at_three_points_from_a_cold_guess_and_a_warm_one()
-> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let low = Point {
        ln_over_p50: -0.5,
        carbon_dioxide_pressure: 20.0,
    };
    let high = Point {
        ln_over_p50: 0.5,
        carbon_dioxide_pressure: 45.0,
    };
    for point in [low, ARTERIAL, high] {
        let expected = speciated(point);
        for start in [Start::at(expected), Start::cold(expected)] {
            let session = admitted(&body, &core, &store, start)?;
            let solved = fields(&body, &session)?;
            let Fields {
                oxygen_pressure: pressure,
                saturation,
                bicarbonate,
                acidity,
            } = solved;
            let report = format!(
                "at {point:?} from {:?}: {solved:?} against {expected:?}",
                start.guess
            );
            assert!(
                (pressure - expected.oxygen_pressure).abs() <= 1e-12 * expected.oxygen_pressure,
                "{report}"
            );
            assert!((saturation - expected.saturation).abs() <= 1e-12, "{report}");
            assert!(
                (bicarbonate - expected.bicarbonate).abs() <= 1e-12 * expected.bicarbonate,
                "{report}"
            );
            assert!((acidity - expected.acidity).abs() <= 1e-12, "{report}");
            let unbalanced = strong_ions() + STRONG_ION_OFFSET - weak(acidity) - bicarbonate;
            assert!(
                unbalanced.abs() <= 1e-12 * STRONG_ION_OFFSET,
                "charge {unbalanced:e}: {report}"
            );
            let dissolved = bicarbonate / ten_to(acidity - PK);
            let total = bicarbonate + dissolved + carbamino(dissolved, saturation);
            let held = value(&body, &session, amount(Species::CarbonDioxide, Space::Plasma)?) / WATER[0];
            assert!(
                (total - held).abs() <= 1e-12 * held,
                "total {total} against {held}: {report}"
            );
        }
    }
    Ok(())
}

/// The tissues' buffered carbon dioxide empties only through the plasma's renewal, whose carbon dioxide content
/// moves less for each mmHg than theirs, so it is the bed's slowest store: measured, the bed settles to 1e-6 by
/// 400 s and not by 120 s.
const SETTLED_S: u64 = 400;

fn settling<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
) -> Result<Session<'m, Running>, InitializationFailure> {
    Ok(after(
        admitted(body, core, store, Start::at(speciated(ARTERIAL)))?,
        SETTLED_S,
    ))
}

fn tissue_pressure(
    body: &CompiledBody,
    session: &Session<'_, Running>,
    species: Species,
    space: Space,
) -> Result<f64, AdmissionError> {
    let at = SPACES.iter().position(|of| *of == space).expect("a tissue space");
    let content = value(body, session, amount(species, space)?) / WATER[at];
    Ok(match (species, space) {
        (Species::Oxygen, _) => content / SOLUBILITY,
        (_, Space::Interstitium) => content / (CARBON_DIOXIDE_SOLUBILITY * (1.0 + BUFFER_RATIOS.0)),
        _ => content / (CARBON_DIOXIDE_SOLUBILITY * (1.0 + BUFFER_RATIOS.1)),
    })
}

/// The settled plasma is the venous closed form, and the interstitium and cells sit one and two heads from it: under
/// for oxygen, which the cells use, and over for carbon dioxide, which they make.
#[test]
fn the_bed_settles_where_the_fick_identity_puts_it() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let (arterial, venous) = (speciated(ARTERIAL), speciated(VENOUS));
    let content = |species| -> Result<f64, AdmissionError> {
        Ok(value(&body, &session, amount(species, Space::Plasma)?) / WATER[0])
    };
    let taken = flow(RENEWAL) * (arterial.oxygen_content - content(Species::Oxygen)?);
    assert!(
        (taken - used()).abs() <= 1e-6 * used(),
        "the flow times the arterial less the venous oxygen content is {taken} against a use of {}",
        used()
    );
    let given = flow(RENEWAL) * (content(Species::CarbonDioxide)? - arterial.carbon_dioxide_total);
    assert!(
        (given - made()).abs() <= 1e-6 * made(),
        "the flow times the venous less the arterial carbon dioxide content is {given} against {} made",
        made()
    );
    let settled = fields(&body, &session)?;
    let Fields {
        oxygen_pressure: pressure,
        bicarbonate,
        acidity,
        ..
    } = settled;
    let report = format!("{settled:?} against {venous:?}");
    assert!(
        (pressure - venous.oxygen_pressure).abs() <= 1e-6 * pressure,
        "{report}"
    );
    assert!((acidity - venous.acidity).abs() <= 1e-6, "{report}");
    assert!(
        (bicarbonate - venous.bicarbonate).abs() <= 1e-6 * bicarbonate,
        "{report}"
    );
    let plasma = venous.carbon_dioxide_pressure;
    for (species, space, expected) in [
        (Species::Oxygen, Space::Interstitium, pressure - HEAD),
        (Species::Oxygen, Space::Cells, pressure - 2.0 * HEAD),
        (
            Species::CarbonDioxide,
            Space::Interstitium,
            plasma + CARBON_DIOXIDE_HEAD,
        ),
        (
            Species::CarbonDioxide,
            Space::Cells,
            plasma + 2.0 * CARBON_DIOXIDE_HEAD,
        ),
    ] {
        let held = tissue_pressure(&body, &session, species, space)?;
        assert!(
            (held - expected).abs() <= 1e-6 * expected,
            "{species:?} in the {space:?} at {held} mmHg against {expected}"
        );
    }
    Ok(())
}

/// The arena certificate's closure, a part in 10^12 of each window's gross exchange, with no bound on the drift,
/// because the settling bed's totals move as its plasma gives up the arterial blood's gases.
const CLOSURE: f64 = 1e-12;

#[test]
fn oxygen_and_carbon_dioxide_close_to_the_ledgers_floor_through_speciation()
-> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let windows = SlidingWindows::new(
        NonZeroU64::MIN.saturating_add(10 * SECOND - 1),
        SETTLED_S * SECOND,
    )?;
    struct Unbounded(QuantityId);
    impl Visit for Unbounded {
        type Output = Result<QuantityPolicy, AdmissionError>;
        fn visit<Q: Substance>(self) -> Self::Output {
            QuantityPolicy::new(
                self.0,
                Measure::<Q>::new(0.0)?,
                Measure::new(CLOSURE)?,
                Measure::new(1e9)?,
            )
        }
    }
    let basis = body.conserved_basis();
    let mut gases = Vec::new();
    let mut quantities = Vec::new();
    for quantity in basis.ids() {
        match quantity.book() {
            Book::Of(species) => {
                if matches!(species, Species::Oxygen | Species::CarbonDioxide) {
                    gases.push(quantity);
                }
                quantities.push(species.visit(Unbounded(quantity))?);
            }
            Book::Charge => quantities.push(QuantityPolicy::charge(
                quantity,
                Measure::new(0.0)?,
                Measure::new(CLOSURE)?,
            )?),
            other => panic!("no element or energy in this fixture: {other:?}"),
        }
    }
    let policy = CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)?;
    let certificate = certify(&session.conservation(), &policy).unwrap_or_else(|failed| {
        let residual = f64::from_bits(failed.residual().bits()[0]);
        panic!(
            "{:?} {:?} {:?} {residual:e}",
            failed.check(),
            failed.quantity(),
            failed.window()
        )
    });
    assert_eq!(gases.len(), 2);
    for gas in gases {
        assert!(
            certificate
                .results()
                .iter()
                .any(|result| result.quantity() == gas)
        );
    }
    Ok(())
}

fn refused_by(body: &CompiledBody, source: SourceId, local: &'static str) -> bool {
    let law = LawName {
        owner: BloodChemistry::CARD,
        local,
    };
    matches!(
        body.refusal_sources()[source.index()],
        RefusalSource::Divisor { law: named } | RefusalSource::LogarithmArgument { law: named } if named == law
    )
}

#[test]
fn a_trial_whose_bound_share_leaves_zero_to_one_is_refused_by_name() -> Result<(), AdmissionError> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let at = |content, pressure| {
        let plasma = solved(content, ARTERIAL.carbon_dioxide_pressure);
        Start {
            guess: Fields {
                oxygen_pressure: pressure,
                ..Start::at(plasma).guess
            },
            ..Start::at(plasma)
        }
    };
    for start in [at(HEME * HEMOGLOBIN + 1.0, 41.0), at(6.0, 7_000.0)] {
        match admitted(&body, &core, &store, start) {
            Err(InitializationFailure::Undefined { law }) => assert_eq!(
                law,
                LawName {
                    owner: BloodChemistry::CARD,
                    local: "oxygen partial pressure"
                }
            ),
            Err(other) => panic!("from {:?}: {other:?}", start.guess),
            Ok(_) => panic!("from {:?} the fixture admitted", start.guess),
        }
    }
    Ok(())
}

#[test]
fn a_bicarbonate_at_or_above_what_the_total_leaves_is_refused_by_name() -> Result<(), AdmissionError> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let plasma = speciated(ARTERIAL);
    for bicarbonate in [plasma.carbon_dioxide_total, plasma.carbon_dioxide_total + 1.0] {
        let start = Start {
            guess: Fields {
                bicarbonate,
                ..Start::at(plasma).guess
            },
            ..Start::at(plasma)
        };
        match admitted(&body, &core, &store, start) {
            Err(InitializationFailure::Undefined { law }) => assert_eq!(
                law,
                LawName {
                    owner: BloodChemistry::CARD,
                    local: "acidity"
                }
            ),
            Err(other) => panic!("from a bicarbonate of {bicarbonate}: {other:?}"),
            Ok(_) => panic!("from a bicarbonate of {bicarbonate} the fixture admitted"),
        }
    }
    Ok(())
}

#[test]
fn a_saturation_outside_zero_to_one_is_refused_before_carbamino_reads_it() -> Result<(), InitializationFailure>
{
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let plasma = speciated(ARTERIAL);
    let _session = admitted(&body, &core, &store, Start::at(plasma))?;
    let saturation_row = field("oxygen saturation")?;
    for saturation in [-0.25, 1.5] {
        let start = Start {
            guess: Fields {
                saturation,
                ..Start::at(plasma).guess
            },
            ..Start::at(plasma)
        };
        match body.bind(packet(&body, start)?) {
            Err(AdmissionError::StartOutsideSet { row }) => assert_eq!(row, saturation_row),
            Err(other) => panic!("from a saturation of {saturation}: {other:?}"),
            Ok(_) => panic!("a saturation of {saturation} bound outside zero to one"),
        }
    }
    Ok(())
}

fn a_fast_renewal(
    arterial: Speciated,
    renewal: f64,
    local: &'static str,
) -> Result<(usize, usize), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let start = Start {
        arterial,
        renewal,
        ..Start::at(speciated(ARTERIAL))
    };
    let mut session = admitted(&body, &core, &store, start)?;
    let (mut named, mut accepted) = (0, 0);
    let finite = |session: &Session<'_, Running>| {
        let view = session.record().accepted();
        (body.rows().iter()).all(|row| f64::from_bits(view.value(row.id()).bits()[0]).is_finite())
    };
    'run: while session.record().accepted().time().nanoseconds() < SECOND / 10 {
        let mut attempt = session.step();
        loop {
            match attempt {
                Step::Accepted(next, _) => {
                    session = next;
                    accepted += 1;
                    assert!(finite(&session), "an accepted value that is not finite");
                    break;
                }
                Step::Refused(retrying) => {
                    if let NumericalRefusal::TrialDomain { source } = retrying.refused().reason() {
                        named += usize::from(refused_by(&body, source, local));
                    }
                    attempt = retrying.retry();
                }
                Step::Faulted(_) => break 'run,
            }
        }
    }
    Ok((named, accepted))
}

fn a_fast_renewal_is_refused_by(
    arterial: Speciated,
    renewal: f64,
    local: &'static str,
) -> Result<(), InitializationFailure> {
    let (named, _) = a_fast_renewal(arterial, renewal, local)?;
    assert!(named > 0, "no step was refused by {local}");
    Ok(())
}

#[test]
fn a_step_into_an_oxygen_content_far_above_capacity_is_refused_by_oxygens_partial_pressure()
-> Result<(), InitializationFailure> {
    a_fast_renewal_is_refused_by(
        solved(30.0, ARTERIAL.carbon_dioxide_pressure),
        1_000.0,
        "oxygen partial pressure",
    )
}

#[test]
fn a_step_whose_bicarbonate_passes_what_the_total_leaves_is_refused_by_acidity()
-> Result<(), InitializationFailure> {
    let arterial = Speciated {
        carbon_dioxide_pressure: ARTERIAL.carbon_dioxide_pressure,
        carbon_dioxide_total: 1.0,
        ..speciated(ARTERIAL)
    };
    a_fast_renewal_is_refused_by(arterial, 10_000.0, "acidity")
}

#[test]
fn steps_that_bring_bicarbonate_near_what_the_total_leaves_are_accepted_finite()
-> Result<(), InitializationFailure> {
    let arterial = Speciated {
        carbon_dioxide_pressure: ARTERIAL.carbon_dioxide_pressure,
        carbon_dioxide_total: 1.0,
        ..speciated(ARTERIAL)
    };
    let (_, accepted) = a_fast_renewal(arterial, 100.0, "acidity")?;
    assert!(accepted > 0, "no step was accepted near acidity's edge");
    Ok(())
}

#[test]
fn an_oxygen_content_above_capacity_admits_at_its_partial_pressure() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let start = Start::at(solved(HEME * HEMOGLOBIN + 1.0, ARTERIAL.carbon_dioxide_pressure));
    let session = admitted(&body, &core, &store, start)?;
    let pressure = value(&body, &session, field("oxygen partial pressure")?);
    assert!(
        (pressure - start.guess.oxygen_pressure).abs() <= 1e-9 * start.guess.oxygen_pressure,
        "{pressure} against {}",
        start.guess.oxygen_pressure
    );
    Ok(())
}

/// Arterial blood above capacity carries the plasma's oxygen content across it while the bed settles over 200 s,
/// ten of the tissue's time constants at that pressure. No trial leaves the partial pressure's domain, so no step is
/// refused.
#[test]
fn a_run_carries_the_oxygen_content_across_capacity() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::HeldUse)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let content = HEME * HEMOGLOBIN + 2.0;
    let start = Start {
        arterial: Speciated {
            oxygen_pressure: 0.0,
            saturation: 0.0,
            oxygen_content: content,
            ..speciated(ARTERIAL)
        },
        ..Start::at(speciated(VENOUS))
    };
    let mut session = admitted(&body, &core, &store, start)?;
    let mut refusals: Vec<String> = Vec::new();
    while session.record().accepted().time().nanoseconds() < 200 * SECOND {
        let mut attempt = session.step();
        loop {
            match attempt {
                Step::Accepted(next, _) => {
                    session = next;
                    break;
                }
                Step::Refused(retrying) => {
                    refusals.push(format!(
                        "{:?} after {} ns",
                        retrying.refused().reason(),
                        retrying.refused().attempted_ns()
                    ));
                    attempt = retrying.retry();
                }
                Step::Faulted(faulted) => panic!("{:?} after {refusals:?}", faulted.fault()),
            }
        }
    }
    let settled = content - used() / flow(RENEWAL);
    let held = value(&body, &session, amount(Species::Oxygen, Space::Plasma)?) / WATER[0];
    let pressure = fields(&body, &session)?.oxygen_pressure;
    let plasma = solved_both(settled, speciated(VENOUS).carbon_dioxide_total);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(
        (held - settled).abs() <= 1e-6 * settled,
        "{held} against {settled}"
    );
    assert!(
        (pressure - plasma.oxygen_pressure).abs() <= 1e-6 * pressure,
        "{pressure} against {plasma:?}"
    );
    Ok(())
}

#[test]
fn only_the_whole_body_speciates() -> Result<(), AdmissionError> {
    let speciates = |body: &CompiledBody| {
        let key = ParameterKey::new("bc.p50");
        let declared = body.parameters().iter().any(|slot| slot.key() == key);
        let fields = ["oxygen partial pressure", "acidity"].map(|local| {
            (body.rows().iter())
                .any(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == local))
        });
        (declared, fields)
    };
    assert_eq!(speciates(&whole_body()?), (true, [true, true]));
    assert_eq!(speciates(&arena_body()?), (false, [false, false]));
    Ok(())
}

fn oxygen_rate(demand: f64, cell_pressure: f64) -> f64 {
    let (allowed, width) = (
        DEMAND / CELL_PRESSURE_AT_DEMAND * cell_pressure,
        WIDTH_SHARE_OF_DEMAND * demand,
    );
    let bent = |flow: f64| {
        let gap = demand - flow;
        0.5 * (demand + flow - (gap * gap + width * width).sqrt())
    };
    bent(allowed) - bent(0.0)
}

fn tissue_oxygen(
    body: &CompiledBody,
    session: &Session<'_, Running>,
    space: Space,
) -> Result<f64, AdmissionError> {
    let gas = value(body, session, amount(Species::Oxygen, space)?);
    Ok(gas / value(body, session, amount(Species::Water, space)?) / SOLUBILITY)
}

#[derive(Clone, Copy, Debug)]
struct Burned {
    oxygen: f64,
    carbon_dioxide: f64,
    heat: f64,
}

fn burned(body: &CompiledBody, session: &Session<'_, Running>, local: &'static str) -> Burned {
    let law = LawName {
        owner: Metabolism::CARD,
        local,
    };
    let column = (body.instances().iter())
        .find_map(|instance| match instance.target() {
            Target::Column(column) if body.columns()[column.index()].origin() == law => Some(column),
            Target::Column(_) | Target::Row(_) => None,
        })
        .unwrap_or_else(|| panic!("no column for {local}"));
    let view = session.conservation();
    let head = view.head().expect("a step");
    let interval = head.interval();
    let (forward, reverse) = head.exchange(column);
    let gained =
        |curve: &[Compensated]| f64::from_bits(curve[1].bits()[0]) - f64::from_bits(curve[0].bits()[0]);
    let extent = gained(&forward.on(interval).expect("the head's interval"))
        - gained(&reverse.on(interval).expect("the head's interval"));
    let per_second = extent * SECOND as f64 / interval.length_ns() as f64;
    let count = |species: Species| -> f64 {
        (body.columns()[column.index()].terms().iter())
            .filter(|(row, _)| {
                matches!(body.rows()[row.index()].kind(), RowKind::Amount { species: of, .. } if of == species)
            })
            .map(|&(_, count)| count as f64)
            .sum()
    };
    Burned {
        oxygen: -count(Species::Oxygen) * per_second,
        carbon_dioxide: count(Species::CarbonDioxide) * per_second,
        heat: count(Species::Energy) * per_second,
    }
}

fn close(measured: f64, expected: f64, relative: f64) -> bool {
    (measured - expected).abs() <= relative * expected.abs()
}

#[test]
fn each_burn_takes_its_share_of_the_oxygen_rate_the_cells_allow_and_makes_its_carbon_dioxide_and_heat()
-> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let rate = oxygen_rate(DEMAND, tissue_oxygen(&body, &session, Space::Cells)?);
    let (glucose, fat) = (GLUCOSE_SHARE * rate, (1.0 - GLUCOSE_SHARE) * rate);
    let expected = [
        (
            "glucose burn",
            Burned {
                oxygen: glucose,
                carbon_dioxide: glucose,
                heat: glucose / 6.0 * GLUCOSE_ENERGY,
            },
        ),
        (
            "fatty acid burn",
            Burned {
                oxygen: fat,
                carbon_dioxide: 16.0 / 23.0 * fat,
                heat: fat / 23.0 * FAT_ENERGY,
            },
        ),
    ];
    for (local, expected) in expected {
        let measured = burned(&body, &session, local);
        let report = format!("{local}: {measured:?} against {expected:?} at an oxygen rate of {rate}");
        assert!(close(measured.oxygen, expected.oxygen, 1e-6), "{report}");
        assert!(
            close(measured.carbon_dioxide, expected.carbon_dioxide, 1e-6),
            "{report}"
        );
        assert!(close(measured.heat, expected.heat, 1e-6), "{report}");
    }
    let plasma = value(&body, &session, amount(Species::Oxygen, Space::Plasma)?) / WATER[0];
    let taken = flow(RENEWAL) * (speciated(ARTERIAL).oxygen_content - plasma);
    assert!(
        close(taken, rate, 1e-6),
        "the plasma gave up {taken} against an oxygen rate of {rate}"
    );
    Ok(())
}

/// The rate passes the demand by at most its offset at no cell oxygen, `D (sqrt(1 + s^2) - 1) / 2`, which is 1e-4 of the
/// demand at a width share of 0.02.
const AT_DEMAND: f64 = 1e-4;

#[test]
fn the_bed_at_rest_burns_its_demand() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let rate: f64 = ["glucose burn", "fatty acid burn"]
        .map(|local| burned(&body, &session, local).oxygen)
        .iter()
        .sum();
    assert!(
        close(rate, DEMAND, AT_DEMAND),
        "the bed at rest burns {rate} against a demand of {DEMAND}"
    );
    Ok(())
}

#[test]
fn the_respiratory_quotient_is_the_one_glucoses_share_fixes() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let declared = GLUCOSE_SHARE + 16.0 * (1.0 - GLUCOSE_SHARE) / 23.0;
    assert!(close(declared, QUOTIENT, 1e-15), "{declared}");
    let [glucose, fat] = ["glucose burn", "fatty acid burn"].map(|local| burned(&body, &session, local));
    let burns = (glucose.carbon_dioxide + fat.carbon_dioxide) / (glucose.oxygen + fat.oxygen);
    assert!(close(burns, QUOTIENT, 1e-12), "the burns' quotient is {burns}");
    let content = |species| -> Result<f64, AdmissionError> {
        Ok(value(&body, &session, amount(species, Space::Plasma)?) / WATER[0])
    };
    let arterial = speciated(ARTERIAL);
    let given = content(Species::CarbonDioxide)? - arterial.carbon_dioxide_total;
    let taken = arterial.oxygen_content - content(Species::Oxygen)?;
    assert!(
        close(given / taken, QUOTIENT, 1e-6),
        "the plasma's quotient is {}",
        given / taken
    );
    Ok(())
}

#[test]
fn the_beds_heat_settles_at_the_burns_heat_over_its_speed() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let [glucose, fat] = ["glucose burn", "fatty acid burn"].map(|local| burned(&body, &session, local));
    let settled = (glucose.heat + fat.heat) / BED_HEAT_SPEED;
    let held = value(
        &body,
        &session,
        RowKind::Amount {
            site: site::<CapillaryBed>(BED)?,
            species: Species::Energy,
            space: Space::Thermal,
        },
    );
    assert!(
        close(held, settled, 1e-6),
        "the bed holds {held} against {settled}"
    );
    Ok(())
}

#[derive(Debug)]
struct Stressed {
    least: [f64; 3],
    steps: u64,
    refusals: Vec<String>,
    rate: f64,
    expected: f64,
    least_burn: f64,
    most_burn: f64,
}

fn stepped_from_rest(demand: f64, renewal: f64) -> Result<Stressed, InitializationFailure> {
    let start = Start {
        demand,
        renewal,
        ..Start::at(speciated(ARTERIAL))
    };
    stepped(start, None, 300)
}

fn stepped(
    start: Start,
    cell_pressure: Option<f64>,
    seconds: u64,
) -> Result<Stressed, InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let rest = settling(&body, &core, &store)?;
    let settled: Vec<(RowKind, f64)> = (body.rows().iter())
        .map(|row| (row.kind(), value(&body, &rest, row.kind())))
        .collect();
    let cells = amount(Species::Oxygen, Space::Cells)?;
    let held = |row| match cell_pressure {
        Some(pressure) if row == cells => SOLUBILITY * pressure * WATER[2],
        _ => {
            let found = settled.iter().find(|(kind, _)| *kind == row);
            found.unwrap_or_else(|| panic!("no settled {row:?}")).1
        }
    };
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let mut session = admit(&body, &core, &store, packet_from(&body, start, held)?)?;
    let mut seen = Stressed {
        least: [f64::INFINITY; 3],
        steps: 0,
        refusals: Vec::new(),
        rate: 0.0,
        expected: 0.0,
        least_burn: f64::INFINITY,
        most_burn: f64::NEG_INFINITY,
    };
    while session.record().accepted().time().nanoseconds() < seconds * SECOND {
        let mut attempt = session.step();
        loop {
            match attempt {
                Step::Accepted(next, _) => {
                    session = next;
                    seen.steps += 1;
                    break;
                }
                Step::Refused(retrying) => {
                    seen.refusals.push(format!(
                        "{:?} after {} ns",
                        retrying.refused().reason(),
                        retrying.refused().attempted_ns()
                    ));
                    attempt = retrying.retry();
                }
                Step::Faulted(faulted) => panic!("{:?} after {:?}", faulted.fault(), seen.refusals),
            }
        }
        let pressures = [
            value(&body, &session, field("oxygen partial pressure")?),
            tissue_oxygen(&body, &session, Space::Interstitium)?,
            tissue_oxygen(&body, &session, Space::Cells)?,
        ];
        for (least, pressure) in seen.least.iter_mut().zip(pressures) {
            if pressure < *least {
                *least = pressure;
            }
        }
        for local in ["glucose burn", "fatty acid burn"] {
            let oxygen = burned(&body, &session, local).oxygen;
            if oxygen < seen.least_burn {
                seen.least_burn = oxygen;
            }
            if oxygen > seen.most_burn {
                seen.most_burn = oxygen;
            }
        }
    }
    let [glucose, fat] = ["glucose burn", "fatty acid burn"].map(|local| burned(&body, &session, local));
    seen.rate = glucose.oxygen + fat.oxygen;
    seen.expected = oxygen_rate(start.demand, tissue_oxygen(&body, &session, Space::Cells)?);
    Ok(seen)
}

#[test]
fn the_oxygen_rate_bends_below_a_raised_demand_or_a_halved_flow_with_no_step_refused()
-> Result<(), InitializationFailure> {
    for (demand, renewal) in [
        (1.5 * DEMAND, RENEWAL),
        (3.0 * DEMAND, RENEWAL),
        (DEMAND, 0.5 * RENEWAL),
    ] {
        let seen = stepped_from_rest(demand, renewal)?;
        let report = format!("demand {demand}, renewal {renewal}: {seen:?}");
        assert!(seen.refusals.is_empty(), "{report}");
        assert!(seen.least.iter().all(|least| *least > 0.0), "{report}");
        assert!(close(seen.rate, seen.expected, 1e-6), "{report}");
        assert!(seen.rate < demand, "{report}");
    }
    Ok(())
}

/// With the cell membrane shut, the cells keep only the oxygen they start with.
#[test]
fn the_oxygen_rate_is_zero_with_no_cell_oxygen_and_never_runs_a_burn_backward()
-> Result<(), InitializationFailure> {
    for (cell_pressure, demand) in [(0.0, DEMAND), (1e-3, DEMAND), (0.0, 0.0), (1e-3, 0.0)] {
        let start = Start {
            demand,
            cell_conductance: 0.0,
            ..Start::at(speciated(ARTERIAL))
        };
        let seen = stepped(start, Some(cell_pressure), 20)?;
        let report = format!("cells at {cell_pressure} mmHg, demand {demand}: {seen:?}");
        assert!(seen.refusals.is_empty(), "{report}");
        assert!(seen.least_burn >= 0.0, "{report}");
        assert!(seen.least[2] >= 0.0, "{report}");
        if cell_pressure == 0.0 || demand == 0.0 {
            assert_eq!(seen.most_burn, 0.0, "{report}");
        } else {
            assert!(seen.least[2] < cell_pressure, "{report}");
        }
    }
    Ok(())
}

#[test]
fn zero_oxygen_demand_leaves_both_burns_idle() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let rest = settling(&body, &core, &store)?;
    let settled: Vec<(RowKind, f64)> = (body.rows().iter())
        .map(|row| (row.kind(), value(&body, &rest, row.kind())))
        .collect();
    let start = Start {
        demand: 0.0,
        ..Start::at(speciated(ARTERIAL))
    };
    let held = |row| {
        let found = settled.iter().find(|(kind, _)| *kind == row);
        found.unwrap_or_else(|| panic!("no settled {row:?}")).1
    };
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let session = admit(&body, &core, &store, packet_from(&body, start, held)?)?;
    let session = match session.step() {
        Step::Accepted(next, _) => next,
        Step::Refused(retrying) => panic!("zero demand was refused: {:?}", retrying.refused().reason()),
        Step::Faulted(faulted) => panic!("zero demand faulted: {:?}", faulted.fault()),
    };
    for local in ["glucose burn", "fatty acid burn"] {
        let measured = burned(&body, &session, local);
        assert_eq!(
            [measured.oxygen, measured.carbon_dioxide, measured.heat],
            [0.0; 3],
            "{local}: {measured:?} at zero demand"
        );
    }
    Ok(())
}

fn refused(cells: Cells) -> CompileDefect {
    match compiled(cells) {
        Err(AdmissionError::Compile(defect)) => defect,
        Err(other) => panic!("refused outside the compile: {other:?}"),
        Ok(_) => panic!("{cells:?} compiled"),
    }
}

/// One burn of both fuels at once leaves six glucose less five fatty acids, a combination that is no quantity.
#[test]
fn a_single_burn_of_both_fuels_is_refused_for_a_combination_that_is_no_quantity() {
    assert_eq!(
        refused(Cells::OneMixedBurn),
        CompileDefect::UnnamedConservedQuantity {
            law: LawName {
                owner: Metabolism::CARD,
                local: "mixed burn",
            }
        }
    );
}

#[test]
fn a_second_cards_glucose_burn_at_the_bed_is_refused_as_owned_twice() -> Result<(), AdmissionError> {
    let CompileDefect::ReactionOwnedTwice { laws, site: at } = refused(Cells::BurningUnderTwoCards) else {
        panic!("not refused as owned twice");
    };
    let local = "glucose burn";
    let [metabolism, muscle] =
        [Metabolism::CARD, Musculoskeletal::CARD].map(|owner| LawName { owner, local });
    assert!(
        laws == [metabolism, muscle] || laws == [muscle, metabolism],
        "{laws:?}"
    );
    assert_eq!(at, site::<CapillaryBed>(BED)?);
    Ok(())
}
