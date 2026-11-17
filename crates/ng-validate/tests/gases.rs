//! One bed whose plasma a held artery renews and whose cells use oxygen and make carbon dioxide at held rates, or burn
//! glucose and fatty acids by metabolism's laws from held feeds, so blood chemistry's speciation, metabolism's wall
//! diffusion and its burn meet closed forms with no body admission. Every value is synthetic.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::f64::consts::LN_10;
use core::num::NonZeroU64;
use ng_blood_chemistry::speciate;
use ng_cardiovascular::resistance_share_at_debt;
use ng_contract::catalog::{PlasmaGases, Reads, SPECIATION_READERS};
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
const GLUCOSE_ENERGY: f64 = 100_000.0;
const FAT_ENERGY: f64 = 300_000.0;
const BED_HEAT_SPEED: f64 = 0.05;
const FEED_SPEED: f64 = 0.1;
const GLUCOSE_SET: f64 = 3_000.0;
const FAT_SET: f64 = 600.0;
const CELL_WATER_SPEED: f64 = 10.0;
const LACTATE_ENERGY: f64 = 45_000.0;
const GLUCOSE_PER_OXYGEN_SHORT: f64 = 1.0 / 6.0;
const OXYGEN_PER_LACTATE: f64 = 3.0;
const LACTATE_BURN_SPEED: f64 = 0.02;
const LACTATE_HALF_PRESSURE: f64 = 2.0;
const DEBT_SHARE: f64 = 0.5;
const DEBT_HALF_TIME: f64 = 1e-3;
const DEBT_HALF: f64 = DEMAND * DEBT_HALF_TIME;

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
    BurningInTheLoop,
    HeldUseAtAStatedDebt,
    SecondReader,
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
    let burns = !matches!(
        cells,
        Cells::HeldUse | Cells::HeldUseAtAStatedDebt | Cells::SecondReader
    );
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
        if cells != Cells::OneMixedBurn {
            schema.carry::<Lactate, CapillaryBed>(Space::Cells);
            schema.energy_content::<Lactate>(Measure::new(LACTATE_ENERGY)?)?;
        }
        if cells == Cells::BurningInTheLoop {
            schema.boundary::<Lactate>("lactate probe", out, bed, Space::Cells)?;
        }
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
            Cells::HeldUse | Cells::HeldUseAtAStatedDebt => {}
            Cells::SecondReader => second_reader(&mut laws, &gases)?,
            Cells::Burning | Cells::BurningUnderTwoCards | Cells::BurningInTheLoop => {
                burn(&mut laws, &cell_oxygen)?
            }
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
        let debt = match cells {
            Cells::BurningInTheLoop => {
                let probe = ports.route::<Lactate>("lactate probe")?.route;
                let none = laws.kernel(|mut k| {
                    let zero = k.zero::<Rate<Lactate>>();
                    k.finish(zero, laws.every_mode(), class)
                });
                let lactate = probe.amount::<Lactate>();
                laws.transfer(Transfer::new(&probe, none));
                let per = laws.parameter(key("fixture.oxygen-per-lactate"), Admissible::positive());
                Some(Debt::Owed(lactate, per))
            }
            Cells::HeldUseAtAStatedDebt => Some(Debt::Stated(
                laws.parameter(key("fixture.stated-debt"), Admissible::finite()),
            )),
            Cells::HeldUse
            | Cells::Burning
            | Cells::OneMixedBurn
            | Cells::BurningUnderTwoCards
            | Cells::SecondReader => None,
        };
        let answer = debt.map(|debt| Answer {
            debt,
            share: laws.parameter(key("fixture.debt-share"), Admissible::unit_interval()),
            half: laws.parameter(key("fixture.debt-half"), Admissible::positive()),
        });
        let answer = answer.as_ref();
        let arterial = ("artery oxygen", "fixture.arterial-oxygen");
        held_rate::<Oxygen>(&mut laws, &ports, arterial, answer, class)?;
        let arterial = ("artery carbon dioxide", "fixture.arterial-carbon-dioxide");
        held_rate::<CarbonDioxide>(&mut laws, &ports, arterial, answer, class)?;
        if !burns {
            let used = ("oxygen use", "fixture.oxygen-use");
            held_rate::<Oxygen>(&mut laws, &ports, used, None, class)?;
            let made = ("carbon dioxide made", "fixture.carbon-dioxide-made");
            held_rate::<CarbonDioxide>(&mut laws, &ports, made, None, class)?;
        }
        let route = ports.route::<Oxygen>("vein oxygen")?.route;
        let held = route.amount::<Oxygen>();
        let law = laws.kernel(|mut k| {
            let (held, zero, mut speed) = (k.read(&held), k.zero(), k.parameter(&renewal));
            if let Some(answer) = answer {
                speed = answer.widened(&mut k, speed);
            }
            let value = k.relax(held, zero, speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, law));
        let route = ports.route::<CarbonDioxide>("vein carbon dioxide")?.route;
        let held = route.amount::<CarbonDioxide>();
        let law = laws.kernel(|mut k| {
            let (held, zero, mut speed) = (k.read(&held), k.zero(), k.parameter(&renewal));
            if let Some(answer) = answer {
                speed = answer.widened(&mut k, speed);
            }
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

enum Debt<'b> {
    Owed(
        ReadPort<'b, Boundary, Lactate>,
        Parameter<'b, Boundary, Quotient<Oxygen, Lactate>>,
    ),
    Stated(Parameter<'b, Boundary, Oxygen>),
}

struct Answer<'b> {
    debt: Debt<'b>,
    share: Parameter<'b, Boundary, Dimensionless>,
    half: Parameter<'b, Boundary, Oxygen>,
}

impl<'b> Answer<'b> {
    fn widened<'k, U: Dimension>(
        &self,
        k: &mut Kernel<'b, 'k, Boundary>,
        reference: Expr<'b, 'k, Boundary, U>,
    ) -> Expr<'b, 'k, Boundary, U> {
        let debt = match &self.debt {
            Debt::Owed(lactate, per) => {
                let (held, per) = (k.read(lactate), k.parameter(per));
                let owed = k.multiply(per, held);
                k.as_dimension(owed)
            }
            Debt::Stated(debt) => k.parameter(debt),
        };
        let (share, half) = (k.parameter(&self.share), k.parameter(&self.half));
        let resistance = resistance_share_at_debt(k, debt, share, half);
        let widened = k.divide(reference, resistance);
        k.as_dimension(widened)
    }
}

fn held_rate<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    (name, key): (&'static str, &'static str),
    answer: Option<&Answer<'b>>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let rate = laws.parameter::<Rate<Q>>(ParameterKey::new(key), Admissible::nonnegative());
    let route = ports.route::<Q>(name)?.route;
    let law = laws.kernel(|mut k| {
        let mut value = k.parameter(&rate);
        if let Some(answer) = answer {
            value = answer.widened(&mut k, value);
        }
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
    oxygen_reach: f64,
    cell_conductance: f64,
    cell_lactate: f64,
    stated_debt: f64,
    debt_share: f64,
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
            oxygen_reach: DEMAND / CELL_PRESSURE_AT_DEMAND,
            cell_conductance: used() / HEAD,
            cell_lactate: 0.0,
            stated_debt: 0.0,
            debt_share: DEBT_SHARE,
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
            species: Species::Lactate,
            space: Space::Cells,
            ..
        } => start.cell_lactate,
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
            "gas gap" => start.guess.oxygen_pressure,
            "oxygen_debt_per_bed" => 0.0,
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
        set::<Conductance<Oxygen>>(&mut packet, "mt.oxygen-reach", beds, start.oxygen_reach)?;
        set::<Dimensionless>(
            &mut packet,
            "mt.oxygen-rate-width-share",
            beds,
            WIDTH_SHARE_OF_DEMAND,
        )?;
        set::<Dimensionless>(&mut packet, "mt.glucose-share", beds, GLUCOSE_SHARE)?;
        set::<Rate<Dimensionless>>(&mut packet, "mt.bed-heat-speed", beds, BED_HEAT_SPEED)?;
        set::<Quotient<Glucose, Oxygen>>(
            &mut packet,
            "mt.glucose-per-oxygen-short",
            beds,
            GLUCOSE_PER_OXYGEN_SHORT,
        )?;
        set::<Rate<Dimensionless>>(&mut packet, "mt.lactate-burn-speed", beds, LACTATE_BURN_SPEED)?;
        set::<Pressure>(
            &mut packet,
            "mt.lactate-burn-half-pressure",
            beds,
            LACTATE_HALF_PRESSURE,
        )?;
        set::<Quotient<Oxygen, Lactate>>(&mut packet, "mt.oxygen-per-lactate", beds, OXYGEN_PER_LACTATE)?;
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
    if declares(body, "fixture.debt-half") {
        set::<Dimensionless>(&mut packet, "fixture.debt-share", body_wide, start.debt_share)?;
        set::<Oxygen>(
            &mut packet,
            "fixture.debt-half",
            body_wide,
            start.demand * DEBT_HALF_TIME,
        )?;
    }
    if declares(body, "fixture.oxygen-per-lactate") {
        let per = OXYGEN_PER_LACTATE;
        set::<Quotient<Oxygen, Lactate>>(&mut packet, "fixture.oxygen-per-lactate", body_wide, per)?;
    }
    if declares(body, "fixture.stated-debt") {
        set::<Oxygen>(&mut packet, "fixture.stated-debt", body_wide, start.stated_debt)?;
    }
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
    let owner = if SPECIATION.contains(&local) {
        BloodChemistry::CARD
    } else {
        Metabolism::CARD
    };
    Ok(RowKind::Algebraic(Slot {
        law: LawName { owner, local },
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

fn refused_by(body: &CompiledBody, source: SourceId, law: LawName) -> bool {
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
    let start = Start {
        arterial,
        renewal,
        ..Start::at(speciated(ARTERIAL))
    };
    let law = LawName {
        owner: BloodChemistry::CARD,
        local,
    };
    let renewed = renewed(Cells::HeldUse, start, law)?;
    Ok((renewed.refused, renewed.accepted))
}

#[derive(Debug)]
struct Renewed {
    refused: usize,
    refused_again: usize,
    accepted: usize,
    others: Vec<String>,
}

fn renewed(cells: Cells, start: Start, law: LawName) -> Result<Renewed, InitializationFailure> {
    let (body, core, store) = (
        compiled(cells)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let mut session = admitted(&body, &core, &store, start)?;
    let mut renewed = Renewed {
        refused: 0,
        refused_again: 0,
        accepted: 0,
        others: Vec::new(),
    };
    let finite = |session: &Session<'_, Running>| {
        let view = session.record().accepted();
        (body.rows().iter()).all(|row| f64::from_bits(view.value(row.id()).bits()[0]).is_finite())
    };
    'run: while session.record().accepted().time().nanoseconds() < SECOND / 10 {
        let (mut attempt, mut before) = (session.step(), false);
        loop {
            match attempt {
                Step::Accepted(next, _) => {
                    session = next;
                    renewed.accepted += 1;
                    assert!(finite(&session), "an accepted value that is not finite");
                    break;
                }
                Step::Refused(retrying) => {
                    let named = matches!(
                        retrying.refused().reason(),
                        NumericalRefusal::TrialDomain { source } if refused_by(&body, source, law)
                    );
                    if !named {
                        renewed.others.push(format!("{:?}", retrying.refused().reason()));
                    }
                    renewed.refused += usize::from(named);
                    renewed.refused_again += usize::from(named && before);
                    before = named;
                    attempt = retrying.retry();
                }
                Step::Faulted(faulted) => {
                    renewed.others.push(format!("{:?}", faulted.fault()));
                    break 'run;
                }
            }
        }
    }
    Ok(renewed)
}

fn a_fast_renewal_is_refused_by(
    arterial: Speciated,
    renewal: f64,
    local: &'static str,
) -> Result<(), InitializationFailure> {
    let start = Start {
        arterial,
        renewal,
        ..Start::at(speciated(ARTERIAL))
    };
    let law = LawName {
        owner: BloodChemistry::CARD,
        local,
    };
    let renewed = renewed(Cells::HeldUse, start, law)?;
    assert!(renewed.refused > 0, "no step was refused by {local}");
    assert!(renewed.refused_again > 0, "no retry was refused by {local} again");
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

#[test]
fn a_tiny_positive_oxygen_demand_never_runs_either_burn_backward() -> Result<(), InitializationFailure> {
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
        demand: 1e-170,
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
        Step::Refused(retrying) => panic!("tiny demand was refused: {:?}", retrying.refused().reason()),
        Step::Faulted(faulted) => panic!("tiny demand faulted: {:?}", faulted.fault()),
    };
    for local in ["glucose burn", "fatty acid burn"] {
        let measured = burned(&body, &session, local);
        assert!(
            [measured.oxygen, measured.carbon_dioxide, measured.heat]
                .iter()
                .all(|rate| rate.is_finite() && *rate >= 0.0),
            "{local}: {measured:?} at a nonnegative demand of {}",
            start.demand
        );
    }
    Ok(())
}

#[test]
fn an_abundant_oxygen_reach_preserves_the_beds_demand() -> Result<(), InitializationFailure> {
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
        oxygen_reach: 1e20,
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
        Step::Refused(retrying) => panic!("abundant reach was refused: {:?}", retrying.refused().reason()),
        Step::Faulted(faulted) => panic!("abundant reach faulted: {:?}", faulted.fault()),
    };
    let pressure = tissue_oxygen(&body, &session, Space::Cells)?;
    assert!(pressure > 1.0, "the cells lack oxygen: {pressure}");
    let rate: f64 = ["glucose burn", "fatty acid burn"]
        .map(|local| burned(&body, &session, local).oxygen)
        .iter()
        .sum();
    assert!(
        close(rate, start.demand, AT_DEMAND),
        "the bed burns {rate} against a demand of {} at a reach of {} and cell pressure {pressure}",
        start.demand,
        start.oxygen_reach
    );
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

/// A second law of metabolism that reads both partial pressures, `P = (P_O2 - P_CO2) P_O2 / (P_O2 - P_CO2)`,
/// so its own divide refuses a trial that carries the oxygen partial pressure below the carbon dioxide one while its
/// value stays the oxygen partial pressure.
fn second_reader<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    gases: &PlasmaGases<Reads<'b, Metabolism>>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, SECOND)?;
    let (gap_out, _) = laws.site_field::<CapillaryBed, Pressure>("gas gap", Admissible::finite());
    let law = laws.site_kernel(|mut k| {
        let oxygen = k.at_site(&gases.oxygen_partial_pressure);
        let carbon_dioxide = k.at_site(&gases.carbon_dioxide_partial_pressure);
        let gap = k.subtract(oxygen, carbon_dioxide);
        let quotient = k.divide(oxygen, gap);
        let ratio: Expr<'b, '_, Metabolism, Dimensionless> = k.as_dimension(quotient);
        let value = k.scale(gap, ratio);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&gap_out, law);
    Ok(())
}

const SPECIATION: [&str; 5] = [
    OxygenPartialPressure::NAME,
    OxygenSaturation::NAME,
    Bicarbonate::NAME,
    Acidity::NAME,
    CarbonDioxidePartialPressure::NAME,
];

fn named(owner: &str, local: &str) -> String {
    format!("LawName {{ owner: {owner:?}, local: {local:?} }}")
}

fn items<'a>(listing: &'a str, label: &str) -> Vec<&'a str> {
    let start = listing.find(label).expect("a program lists it") + label.len();
    let (mut depth, mut from, mut items) = (0, start, Vec::new());
    for (at, c) in listing[start..].char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => {
                items.push(listing[from..start + at].trim());
                break;
            }
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(listing[from..start + at].trim());
                from = start + at + 1;
            }
            _ => {}
        }
    }
    items.retain(|item| !item.is_empty());
    items
}

/// Each card outside blood chemistry, with each speciation quantity one of its programs reads: a row it reads by its
/// own op, or the explicit field whose ops compile wrote in. A read inside those written ops is the field's own.
fn speciation_readers(body: &CompiledBody) -> BTreeSet<(String, &'static str)> {
    let field = named(BloodChemistry::CARD, CarbonDioxidePartialPressure::NAME);
    let mut readers = BTreeSet::new();
    for program in body.programs() {
        let listing = format!("{program:?}");
        let owner = (listing.split("owner: \"").nth(1))
            .and_then(|rest| rest.split('"').next())
            .expect("a program names its law");
        if owner == BloodChemistry::CARD {
            continue;
        }
        let written: Vec<&str> = (items(&listing, "origins: [").into_iter())
            .filter(|origin| origin.ends_with(&format!("{field})")))
            .filter_map(|origin| origin.trim_start_matches('(').split(',').next())
            .collect();
        if !written.is_empty() {
            readers.insert((owner.to_string(), CarbonDioxidePartialPressure::NAME));
        }
        for (at, op) in items(&listing, "ops: [").into_iter().enumerate() {
            let index = at.to_string();
            for quantity in SPECIATION {
                let read = format!("Field({})", named(BloodChemistry::CARD, quantity));
                if op.contains(&read) && !written.contains(&index.as_str()) {
                    readers.insert((owner.to_string(), quantity));
                }
            }
        }
    }
    readers
}

#[test]
fn blood_chemistry_alone_states_speciation_and_names_the_explicit_fields_ops() -> Result<(), AdmissionError> {
    for (body, beds) in [(compiled(Cells::HeldUse)?, 1), (whole_body()?, 11)] {
        for quantity in SPECIATION {
            let owners: Vec<&str> = (body.rows().iter())
                .filter_map(|row| match row.kind() {
                    RowKind::Algebraic(slot) if slot.law.local == quantity => Some(slot.law.owner),
                    _ => None,
                })
                .collect();
            let rows = if quantity == CarbonDioxidePartialPressure::NAME {
                0
            } else {
                beds
            };
            assert_eq!(owners, vec![BloodChemistry::CARD; rows], "{quantity}");
        }
        let listings: Vec<String> = body
            .programs()
            .iter()
            .map(|program| format!("{program:?}"))
            .collect();
        let field = named(BloodChemistry::CARD, CarbonDioxidePartialPressure::NAME);
        let written = listings.iter().filter(|listing| listing.contains(&field)).count();
        assert!(
            written >= 3,
            "the explicit field's ops stand in {written} programs"
        );
        assert!(
            !listings
                .iter()
                .any(|listing| listing.contains(&format!("Field({field})")))
        );
        for quantity in SPECIATION {
            let stand_in = named(Metabolism::CARD, quantity);
            assert!(
                !listings.iter().any(|listing| listing.contains(&stand_in)),
                "{stand_in}"
            );
        }
    }
    Ok(())
}

#[test]
fn only_metabolism_reads_speciation_and_only_what_its_wired_row_lists() -> Result<(), AdmissionError> {
    let wired: BTreeSet<(String, &'static str)> = (SPECIATION_READERS.iter())
        .flat_map(|row| (row.wired.iter()).map(move |read| (row.reader.to_string(), read.quantity)))
        .collect();
    assert_eq!(speciation_readers(&compiled(Cells::HeldUse)?), wired);
    assert_eq!(speciation_readers(&whole_body()?), wired);
    Ok(())
}

#[test]
fn a_second_reader_adds_no_row_and_a_refusal_in_its_own_arithmetic_names_it()
-> Result<(), InitializationFailure> {
    let (once, twice) = (compiled(Cells::HeldUse)?, compiled(Cells::SecondReader)?);
    assert_eq!(
        twice.rows().len(),
        once.rows().len() + 1,
        "the second reader's own row only"
    );
    let low = speciated(Point {
        ln_over_p50: 0.1,
        carbon_dioxide_pressure: 50.0,
    });
    let start = Start {
        arterial: low,
        renewal: 100.0,
        ..Start::at(speciated(VENOUS))
    };
    let law = LawName {
        owner: Metabolism::CARD,
        local: "gas gap",
    };
    let renewed = renewed(Cells::SecondReader, start, law)?;
    assert!(
        renewed.refused > 0,
        "no step was refused by the second reader: {renewed:?}"
    );
    assert!(
        renewed.refused_again > 0,
        "no retry was refused by the second reader again: {renewed:?}"
    );
    Ok(())
}

fn extent(body: &CompiledBody, session: &Session<'_, Running>, local: &'static str) -> f64 {
    let law = LawName {
        owner: Metabolism::CARD,
        local,
    };
    column_rate(body, session, law)
}

fn column_rate(body: &CompiledBody, session: &Session<'_, Running>, law: LawName) -> f64 {
    let column = (body.instances().iter())
        .find_map(|instance| match instance.target() {
            Target::Column(column) if body.columns()[column.index()].origin() == law => Some(column),
            Target::Column(_) | Target::Row(_) => None,
        })
        .unwrap_or_else(|| {
            let origins: Vec<LawName> = body.columns().iter().map(|column| column.origin()).collect();
            panic!("no column for {law:?} among {origins:?}")
        });
    let view = session.conservation();
    let head = view.head().expect("a step");
    let interval = head.interval();
    let (forward, reverse) = head.exchange(column);
    let gained =
        |curve: &[Compensated]| f64::from_bits(curve[1].bits()[0]) - f64::from_bits(curve[0].bits()[0]);
    let extent = gained(&forward.on(interval).expect("the head's interval"))
        - gained(&reverse.on(interval).expect("the head's interval"));
    extent * SECOND as f64 / interval.length_ns() as f64
}

struct Marched {
    steps: u64,
    refusals: Vec<String>,
    least: [f64; 3],
    debts: Vec<(f64, f64)>,
    extractions: Vec<f64>,
    flows: Vec<f64>,
    least_rates: [f64; 2],
}

fn march<'m>(
    body: &'m CompiledBody,
    mut session: Session<'m, Running>,
    seconds: u64,
) -> Result<(Session<'m, Running>, Marched), InitializationFailure> {
    let start = session.record().accepted().time().nanoseconds();
    let mut seen = Marched {
        steps: 0,
        refusals: Vec::new(),
        least: [f64::INFINITY; 3],
        debts: Vec::new(),
        extractions: Vec::new(),
        flows: Vec::new(),
        least_rates: [f64::INFINITY; 2],
    };
    while session.record().accepted().time().nanoseconds() < start + seconds * SECOND {
        let mut attempt = session.step();
        loop {
            match attempt {
                Step::Accepted(next, _) => {
                    session = next;
                    seen.steps += 1;
                    break;
                }
                Step::Refused(retrying) => {
                    seen.refusals.push(format!("{:?}", retrying.refused().reason()));
                    attempt = retrying.retry();
                }
                Step::Faulted(faulted) => panic!("{:?} after {:?}", faulted.fault(), seen.refusals),
            }
        }
        let pressures = [
            value(body, &session, field("oxygen partial pressure")?),
            tissue_oxygen(body, &session, Space::Interstitium)?,
            tissue_oxygen(body, &session, Space::Cells)?,
        ];
        for (least, pressure) in seen.least.iter_mut().zip(pressures) {
            if pressure < *least {
                *least = pressure;
            }
        }
        for (least, local) in seen.least_rates.iter_mut().zip(["glycolysis", "lactate burn"]) {
            let rate = extent(body, &session, local);
            if rate < *least {
                *least = rate;
            }
        }
        let now = (session.record().accepted().time().nanoseconds() - start) as f64 / SECOND as f64;
        seen.debts
            .push((now, value(body, &session, field("oxygen_debt_per_bed")?)));
        let arterial = speciated(ARTERIAL).oxygen_content;
        let plasma = value(body, &session, amount(Species::Oxygen, Space::Plasma)?) / WATER[0];
        seen.extractions.push(1.0 - plasma / arterial);
        let delivered = column_rate(body, &session, ARTERY_OXYGEN);
        seen.flows.push(delivered / (flow(RENEWAL) * arterial));
    }
    Ok((session, seen))
}

const ARTERY_OXYGEN: LawName = LawName {
    owner: Boundary::CARD,
    local: "artery oxygen",
};

fn rest_of(body: &CompiledBody, session: &Session<'_, Running>) -> Vec<(RowKind, f64)> {
    (body.rows().iter())
        .map(|row| (row.kind(), value(body, session, row.kind())))
        .collect()
}

fn readmitted<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
    start: Start,
    from: &[(RowKind, f64)],
) -> Result<Session<'m, Running>, InitializationFailure> {
    let held = |row| {
        let found = from.iter().find(|(kind, _)| *kind == row);
        found.unwrap_or_else(|| panic!("no settled {row:?}")).1
    };
    admit(body, core, store, packet_from(body, start, held)?)
}

fn half_time(debts: &[(f64, f64)], reference: f64, share: f64) -> Option<f64> {
    let first = debts.first()?.1 - reference;
    debts
        .iter()
        .find(|(_, debt)| debt - reference <= share * first)
        .map(|(time, _)| *time)
}

fn every_quantity_closes(
    body: &CompiledBody,
    session: &Session<'_, Running>,
    coverage: u64,
) -> Result<Certificate, String> {
    let windows = SlidingWindows::new(NonZeroU64::MIN.saturating_add(10 * SECOND - 1), coverage * SECOND)
        .map_err(|error| format!("{error:?}"))?;
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
    let quantities = (basis.ids())
        .map(|quantity| match quantity.book() {
            Book::Of(species) => species.visit(Unbounded(quantity)),
            Book::Energy => Species::Energy.visit(Unbounded(quantity)),
            Book::Element(_) => QuantityPolicy::element(
                quantity,
                Measure::new(0.0)?,
                Measure::new(CLOSURE)?,
                Measure::new(1e9)?,
            ),
            Book::Charge => QuantityPolicy::charge(quantity, Measure::new(0.0)?, Measure::new(CLOSURE)?),
        })
        .collect::<Result<Vec<_>, AdmissionError>>()
        .map_err(|error| format!("{error:?}"))?;
    let policy = CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
        .map_err(|error| format!("{error:?}"))?;
    certify(&session.conservation(), &policy).map_err(|failed| {
        format!(
            "{:?} {:?} {:?} {:e}",
            failed.check(),
            failed.quantity(),
            failed.window(),
            f64::from_bits(failed.residual().bits()[0])
        )
    })
}

fn settled(body: &CompiledBody) -> Result<Vec<(RowKind, f64)>, InitializationFailure> {
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    Ok(rest_of(body, &settling(body, &core, &store)?))
}

struct ShutBedStep {
    glycolysis: f64,
    lactate_burn: f64,
    debt: f64,
    held: f64,
}

fn split(demand: f64, cell_pressure: f64) -> f64 {
    let gap = demand - oxygen_rate(demand, cell_pressure);
    if gap > 0.0 {
        GLUCOSE_PER_OXYGEN_SHORT * gap
    } else {
        0.0
    }
}

fn lactate_burned(lactate: f64, cell_pressure: f64) -> f64 {
    LACTATE_BURN_SPEED * lactate * cell_pressure / (cell_pressure + LACTATE_HALF_PRESSURE)
}

fn stepped_at(
    body: &CompiledBody,
    settled: &[(RowKind, f64)],
    (demand, cell_pressure, lactate): (f64, f64, f64),
) -> Result<ShutBedStep, InitializationFailure> {
    let start = Start {
        demand,
        cell_conductance: 0.0,
        cell_lactate: lactate,
        ..Start::at(speciated(ARTERIAL))
    };
    let (oxygen, held) = (
        amount(Species::Oxygen, Space::Cells)?,
        amount(Species::Lactate, Space::Cells)?,
    );
    let mut from = settled.to_vec();
    for (row, value) in &mut from {
        if *row == oxygen {
            *value = SOLUBILITY * cell_pressure * WATER[2];
        } else if *row == held {
            *value = lactate;
        }
    }
    let (core, store) = (ng_solver::Core::new(SECOND / 10_000), MemoryStore::default());
    let session = readmitted(body, &core, &store, start, &from)?;
    let session = match session.step() {
        Step::Accepted(next, _) => next,
        Step::Refused(retrying) => panic!("the point was refused: {:?}", retrying.refused().reason()),
        Step::Faulted(faulted) => panic!("the point faulted: {:?}", faulted.fault()),
    };
    Ok(ShutBedStep {
        glycolysis: extent(body, &session, "glycolysis"),
        lactate_burn: extent(body, &session, "lactate burn"),
        debt: value(body, &session, field("oxygen_debt_per_bed")?),
        held: value(body, &session, held),
    })
}

fn column_terms(body: &CompiledBody, local: &'static str) -> Vec<(Species, i64)> {
    let law = LawName {
        owner: Metabolism::CARD,
        local,
    };
    let column = (body.columns().iter())
        .find(|column| column.origin() == law)
        .unwrap_or_else(|| panic!("no column for {local}"));
    let mut terms: Vec<(Species, i64)> = (column.terms().iter())
        .map(|&(row, count)| match body.rows()[row.index()].kind() {
            RowKind::Amount { species, .. } => (species, count),
            other => panic!("a term on {other:?}"),
        })
        .collect();
    terms.sort_by_key(|(species, _)| format!("{species:?}"));
    terms
}

#[test]
fn glycolysis_and_lactates_burn_carry_their_atoms_and_energy_into_the_cells_and_heat()
-> Result<(), AdmissionError> {
    let body = compiled(Cells::Burning)?;
    assert_eq!(
        column_terms(&body, "glycolysis"),
        vec![
            (Species::Energy, 10_000),
            (Species::Glucose, -1),
            (Species::Lactate, 2)
        ],
        "glycolysis gives up a glucose's energy, {GLUCOSE_ENERGY}, less two lactates', {LACTATE_ENERGY} each"
    );
    assert_eq!(
        column_terms(&body, "lactate burn"),
        vec![
            (Species::CarbonDioxide, 3),
            (Species::Energy, 45_000),
            (Species::Lactate, -1),
            (Species::Oxygen, -3),
            (Species::Water, 3)
        ]
    );
    let burn = column_terms(&body, "lactate burn");
    let per = |species| {
        burn.iter()
            .find(|(of, _)| *of == species)
            .map(|(_, count)| *count as f64)
    };
    assert_eq!(
        per(Species::Oxygen)
            .zip(per(Species::Lactate))
            .map(|(o, l)| o / l),
        Some(OXYGEN_PER_LACTATE),
        "the debt's oxygen to each lactate is the burn's own"
    );
    Ok(())
}

/// Lactate still rising at the settled pressure moves a 10 ms step's mean from its end by about 2e-6.
#[test]
fn glycolysis_lactates_burn_and_the_debt_meet_their_forms_and_their_zeros()
-> Result<(), InitializationFailure> {
    let body = compiled(Cells::Burning)?;
    let settled = settled(&body)?;
    for (point, split_rate, burn_rate) in [
        ((DEMAND, 0.0, 80.0), GLUCOSE_PER_OXYGEN_SHORT * DEMAND, 0.0),
        ((DEMAND, 60.0, 0.0), 0.0, 0.0),
    ] {
        let measured = stepped_at(&body, &settled, point)?;
        assert_eq!(
            (measured.glycolysis, measured.lactate_burn),
            (split_rate, burn_rate),
            "at demand, cell pressure and lactate {point:?}"
        );
    }
    for lactate in [0.0, 10.0, 100.0] {
        let measured = stepped_at(&body, &settled, (DEMAND, 0.0, lactate))?;
        let expected = OXYGEN_PER_LACTATE * measured.held;
        assert!(
            (measured.debt - expected).abs() <= 1e-9 * (1.0 + expected),
            "the debt at {} lactate reads {} against {expected}",
            measured.held,
            measured.debt
        );
    }
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let raised = Start {
        demand: 3.0 * DEMAND,
        ..Start::at(speciated(ARTERIAL))
    };
    let session = readmitted(&body, &core, &store, raised, &settled)?;
    let (session, _) = march(&body, session, 120)?;
    let pressure = tissue_oxygen(&body, &session, Space::Cells)?;
    let held = value(&body, &session, amount(Species::Lactate, Space::Cells)?);
    let measured = [
        extent(&body, &session, "glycolysis"),
        extent(&body, &session, "lactate burn"),
    ];
    let expected = [split(raised.demand, pressure), lactate_burned(held, pressure)];
    for (measured, expected) in measured.into_iter().zip(expected) {
        assert!(
            expected > 0.0 && close(measured, expected, 1e-5),
            "{measured} against {expected} at {pressure} mmHg and {held} lactate"
        );
    }
    Ok(())
}

#[test]
fn the_bed_at_rest_splits_no_glucose_and_owes_no_oxygen() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled(Cells::Burning)?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let held = value(&body, &session, amount(Species::Lactate, Space::Cells)?);
    let debt = value(&body, &session, field("oxygen_debt_per_bed")?);
    assert_eq!(extent(&body, &session, "glycolysis"), 0.0);
    assert!(
        (0.0..1e-5).contains(&held),
        "the settled cells hold {held} lactate"
    );
    assert!(
        close(debt, OXYGEN_PER_LACTATE * held, 1e-12),
        "the debt at rest reads {debt} for {held} lactate"
    );
    Ok(())
}

const BODY_CELL_PRESSURE_AT_DEMAND: f64 = 17.0;
const BODY_REACH: f64 = DEMAND / BODY_CELL_PRESSURE_AT_DEMAND;
const REACH_AT_TEN: f64 = DEMAND / 10.0;

struct Repaid {
    built: f64,
    tenth: Option<f64>,
    report: String,
}

fn stressed_and_repaid(
    body: &CompiledBody,
    settled: &[(RowKind, f64)],
    (demand, renewal, oxygen_reach): (f64, f64, f64),
) -> Result<Repaid, InitializationFailure> {
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let stressed = Start {
        demand,
        renewal,
        oxygen_reach,
        ..Start::at(speciated(ARTERIAL))
    };
    let session = readmitted(body, &core, &store, stressed, settled)?;
    let (end, stress) = march(body, session, 120)?;
    let built = value(body, &end, field("oxygen_debt_per_bed")?);
    let at_end = rest_of(body, &end);
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let back = Start {
        oxygen_reach,
        ..Start::at(speciated(ARTERIAL))
    };
    let session = readmitted(body, &core, &store, back, &at_end)?;
    let (_, repay) = march(body, session, 300)?;
    let (half, tenth) = (
        half_time(&repay.debts, 0.0, 0.5),
        half_time(&repay.debts, 0.0, 0.1),
    );
    let report = format!(
        "demand {demand}, renewal {renewal}, reach {oxygen_reach}: debt {built} after {} steps, least pressures {:?} and {:?}, half repaid at {half:?} s, a tenth left at {tenth:?} s, refused {:?} and {:?}",
        stress.steps, stress.least, repay.least, stress.refusals, repay.refusals
    );
    assert!(
        stress.refusals.is_empty() && repay.refusals.is_empty(),
        "{report}"
    );
    assert!(
        stress.least.iter().chain(&repay.least).all(|least| *least > 0.0),
        "{report}"
    );
    assert!(
        stress
            .least_rates
            .iter()
            .chain(&repay.least_rates)
            .all(|least| *least >= 0.0),
        "{report}"
    );
    Ok(Repaid { built, tenth, report })
}

#[test]
fn at_the_bodys_reach_half_the_flow_or_half_again_the_demand_builds_a_debt_that_the_burn_repays()
-> Result<(), InitializationFailure> {
    let body = compiled(Cells::Burning)?;
    let settled = settled(&body)?;
    for case in [
        (1.5 * DEMAND, RENEWAL, BODY_REACH),
        (2.0 * DEMAND, RENEWAL, BODY_REACH),
        (3.0 * DEMAND, RENEWAL, BODY_REACH),
        (DEMAND, 0.5 * RENEWAL, BODY_REACH),
        (DEMAND, 0.25 * RENEWAL, BODY_REACH),
    ] {
        let repaid = stressed_and_repaid(&body, &settled, case)?;
        assert!(repaid.built > 1e-3, "{}", repaid.report);
        assert!(repaid.tenth.is_some(), "{}", repaid.report);
    }
    Ok(())
}

#[test]
fn at_a_reach_meeting_the_demand_at_ten_mmhg_half_again_the_demand_or_half_the_flow_builds_no_debt()
-> Result<(), InitializationFailure> {
    let body = compiled(Cells::Burning)?;
    let settled = settled(&body)?;
    for case in [
        (1.5 * DEMAND, RENEWAL, REACH_AT_TEN),
        (DEMAND, 0.5 * RENEWAL, REACH_AT_TEN),
    ] {
        let repaid = stressed_and_repaid(&body, &settled, case)?;
        assert!(repaid.built < 1e-6, "no debt within the reach: {}", repaid.report);
    }
    Ok(())
}

/// With the cells' use held, the plasma settles at the arterial content less the use over the widened flow (Fick).
#[test]
fn the_beds_resistance_meets_its_form_at_no_debt_a_hair_under_none_and_its_half()
-> Result<(), InitializationFailure> {
    let (plain, stated) = (compiled(Cells::HeldUse)?, compiled(Cells::HeldUseAtAStatedDebt)?);
    let rest = Start::at(speciated(ARTERIAL));
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let without = after(admitted(&plain, &core, &store, rest)?, 1);
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let at_none = after(admitted(&stated, &core, &store, rest)?, 1);
    for row in plain.rows() {
        let (a, b) = (
            value(&plain, &without, row.kind()),
            value(&stated, &at_none, row.kind()),
        );
        assert!(
            a.to_bits() == b.to_bits(),
            "{:?} reads {a} with no answer and {b} at no debt",
            row.kind()
        );
    }
    for debt in [0.0, -1e-29, DEBT_HALF] {
        let share = (DEBT_HALF + (debt - DEBT_SHARE * debt)) / (debt + DEBT_HALF);
        let start = Start {
            stated_debt: debt,
            ..rest
        };
        let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
        let session = after(admitted(&stated, &core, &store, start)?, SETTLED_S);
        let held = value(&stated, &session, amount(Species::Oxygen, Space::Plasma)?);
        let expected = WATER[0] * speciated(ARTERIAL).oxygen_content - share * used() / RENEWAL;
        assert!(
            close(held, expected, 1e-6),
            "at a debt of {debt} the plasma holds {held} oxygen against {expected}, a share of {share}"
        );
    }
    Ok(())
}

#[test]
fn a_full_debt_share_keeps_a_positive_resistance_at_finite_debt() -> Result<(), InitializationFailure> {
    let (plain, stated) = (compiled(Cells::HeldUse)?, compiled(Cells::HeldUseAtAStatedDebt)?);
    let rest = Start::at(speciated(ARTERIAL));
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let without = after(admitted(&plain, &core, &store, rest)?, 1);
    let debt = 1e16;
    let retained = DEBT_HALF / (debt + DEBT_HALF);
    let start = Start {
        stated_debt: debt,
        debt_share: 1.0,
        renewal: rest.renewal * retained,
        ..rest
    };
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let with = after(admitted(&stated, &core, &store, start)?, 1);
    let expected = column_rate(&plain, &without, ARTERY_OXYGEN);
    let delivered = column_rate(&stated, &with, ARTERY_OXYGEN);
    assert!(
        expected > 0.0 && close(delivered, expected, 1e-10),
        "{delivered} against {expected}"
    );
    Ok(())
}

fn half_move(times: &[f64], series: &[f64]) -> f64 {
    let (first, last) = (series[0], series[series.len() - 1]);
    let found = (times.iter().zip(series)).find(|(_, value)| (*value - first) / (last - first) >= 0.5);
    found.map_or(f64::NAN, |(time, _)| *time)
}

const DEBT_STARTED: f64 = 1e-4;

/// The flow follows the debt with no lag, because a lag needs memory at a site, which the contract has not: the bed's
/// resistance is its form of the debt at every accepted step. The flow's half comes before the debt's because the
/// answer saturates.
#[test]
fn halving_the_flow_raises_extraction_then_the_debt_and_the_flow_with_the_debt()
-> Result<(), InitializationFailure> {
    let body = compiled(Cells::BurningInTheLoop)?;
    let settled = settled(&body)?;
    let start = Start {
        renewal: 0.5 * RENEWAL,
        oxygen_reach: BODY_REACH,
        ..Start::at(speciated(ARTERIAL))
    };
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let session = readmitted(&body, &core, &store, start, &settled)?;
    let (_, seen) = march(&body, session, 300)?;
    let times: Vec<f64> = seen.debts.iter().map(|(time, _)| *time).collect();
    let debts: Vec<f64> = seen.debts.iter().map(|(_, debt)| *debt).collect();
    let widest = (seen.flows.iter().zip(&debts))
        .map(|(flow, debt)| {
            let share = (DEBT_HALF + (debt - DEBT_SHARE * debt)) / (debt + DEBT_HALF);
            (flow * RENEWAL / start.renewal * share - 1.0).abs()
        })
        .fold(0.0, |widest, off| if off > widest { off } else { widest });
    let halves = [
        half_move(&times, &seen.extractions),
        half_move(&times, &debts),
        half_move(&times, &seen.flows),
    ];
    let started = (times.iter().zip(&debts)).find(|(_, debt)| **debt > DEBT_STARTED);
    let started = started.map_or(f64::NAN, |(time, _)| *time);
    let report = format!(
        "halves of extraction, debt and flow {halves:?} s, the debt past {DEBT_STARTED} at {started} s, the flow off its form by {widest} at most, settled at debt {} and flow {}, refused {:?}",
        debts[debts.len() - 1],
        seen.flows[seen.flows.len() - 1],
        seen.refusals
    );
    assert!(seen.refusals.is_empty() && widest < 1e-4, "{report}");
    assert!(halves[0] < started && started < halves[2], "{report}");
    assert!(halves[2] <= halves[1], "{report}");
    Ok(())
}

#[test]
fn with_the_loop_closed_each_stress_settles_with_no_refusal_and_every_pressure_in_its_set()
-> Result<(), InitializationFailure> {
    let body = compiled(Cells::BurningInTheLoop)?;
    let settled = settled(&body)?;
    for (demand, renewal) in [
        (DEMAND, 0.5 * RENEWAL),
        (DEMAND, 0.25 * RENEWAL),
        (2.0 * DEMAND, RENEWAL),
        (3.0 * DEMAND, RENEWAL),
    ] {
        let start = Start {
            demand,
            renewal,
            oxygen_reach: BODY_REACH,
            ..Start::at(speciated(ARTERIAL))
        };
        let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
        let session = readmitted(&body, &core, &store, start, &settled)?;
        let (_, seen) = march(&body, session, 900)?;
        let debts: Vec<f64> = seen.debts.iter().map(|(_, debt)| *debt).collect();
        let last_minutes = seen
            .debts
            .iter()
            .position(|(time, _)| *time >= 800.0)
            .unwrap_or(0);
        let still = [&seen.extractions, &debts, &seen.flows].map(|series| {
            let first = series[0];
            let widest = (series.iter().map(|value| (value - first).abs()))
                .fold(0.0, |widest, off| if off > widest { off } else { widest });
            ((series[series.len() - 1] - series[last_minutes]) / widest).abs()
        });
        let report = format!(
            "demand {demand}, renewal {renewal}: debt {}, flow {}, extraction {}, moved over the last 100 s by {still:?} of its widest move, least pressures {:?}, least rates {:?}, refused {:?}",
            debts[debts.len() - 1],
            seen.flows[seen.flows.len() - 1],
            seen.extractions[seen.extractions.len() - 1],
            seen.least,
            seen.least_rates,
            seen.refusals
        );
        assert!(seen.refusals.is_empty(), "{report}");
        assert!(seen.least.iter().all(|least| *least > 0.0), "{report}");
        assert!(seen.least_rates.iter().all(|least| *least >= 0.0), "{report}");
        assert!(still.iter().all(|share| *share < 1e-3), "{report}");
    }
    Ok(())
}

#[test]
fn the_closed_loops_half_response_uses_the_beds_stated_demand() -> Result<(), InitializationFailure> {
    let body = compiled(Cells::BurningInTheLoop)?;
    let settled = settled(&body)?;
    let start = Start {
        demand: 3.0 * DEMAND,
        renewal: RENEWAL,
        oxygen_reach: BODY_REACH,
        ..Start::at(speciated(ARTERIAL))
    };
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let session = readmitted(&body, &core, &store, start, &settled)?;
    let session = run_until(session, SimTime::from_ns(SECOND / 10))
        .unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()));
    let debt = value(&body, &session, field("oxygen_debt_per_bed")?);
    let seconds = session
        .conservation()
        .head()
        .expect("a step")
        .interval()
        .length_ns() as f64
        / SECOND as f64;
    let owed_before_the_burn = debt + OXYGEN_PER_LACTATE * extent(&body, &session, "lactate burn") * seconds;
    let half = start.demand * DEBT_HALF_TIME;
    let least_resistance = 1.0 - DEBT_SHARE * owed_before_the_burn / (owed_before_the_burn + half);
    let most_delivered = flow(start.renewal) * start.arterial.oxygen_content / least_resistance;
    let delivered = column_rate(&body, &session, ARTERY_OXYGEN);
    assert!(debt > 0.0 && delivered > 0.0);
    assert!(
        delivered <= most_delivered * (1.0 + 1e-3),
        "at demand {} and debt at most {owed_before_the_burn}, the artery delivers {delivered}, above {most_delivered}",
        start.demand
    );
    Ok(())
}

#[test]
fn every_conserved_quantity_closes_while_glycolysis_runs() -> Result<(), InitializationFailure> {
    let body = compiled(Cells::Burning)?;
    let settled = settled(&body)?;
    let (core, store) = (ng_solver::Core::new(SECOND / 100), MemoryStore::default());
    let stressed = Start {
        demand: 3.0 * DEMAND,
        ..Start::at(speciated(ARTERIAL))
    };
    let session = readmitted(&body, &core, &store, stressed, &settled)?;
    let (end, seen) = march(&body, session, 60)?;
    assert!(seen.least_rates[0] > 0.0, "glycolysis ran the whole minute");
    let certificate = every_quantity_closes(&body, &end, 60).unwrap_or_else(|failed| panic!("{failed}"));
    let books: Vec<Book> = (certificate.results().iter())
        .map(|result| result.quantity().book())
        .collect();
    for book in [
        Book::Element(Element::Carbon),
        Book::Element(Element::Hydrogen),
        Book::Element(Element::Oxygen),
        Book::Energy,
        Book::Charge,
    ] {
        assert!(books.contains(&book), "{book:?} among {books:?}");
    }
    Ok(())
}

#[test]
fn burning_lactate_never_makes_a_negative_oxygen_debt() -> Result<(), InitializationFailure> {
    let body = compiled(Cells::Burning)?;
    let settled = settled(&body)?;
    for lactate in [50.0, 1e-9, 0.0] {
        let measured = stepped_at(&body, &settled, (DEMAND, 60.0, lactate))?;
        assert_eq!(measured.glycolysis, 0.0);
        assert!(
            measured.held <= lactate,
            "{} lactate burned from {lactate}",
            measured.held
        );
        assert!(
            measured.debt >= 0.0 && close(measured.debt, OXYGEN_PER_LACTATE * measured.held, 1e-12),
            "the cells hold {} lactate and owe {} oxygen",
            measured.held,
            measured.debt
        );
    }
    Ok(())
}
