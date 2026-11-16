//! One bed whose plasma a held artery renews and whose cells use oxygen and make carbon dioxide at held rates, so blood
//! chemistry's speciation and metabolism's wall diffusion meet closed forms with no body admission. Every value is
//! synthetic.

use core::f64::consts::LN_10;
use core::num::NonZeroU64;
use ng_blood_chemistry::{Plasma, StrongIons, speciate};
use ng_contract::*;
use ng_metabolism::{GasRoutes, PlasmaPressure, carbon_dioxide_diffusion, oxygen_diffusion};
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
const BY_ACIDITY: (f64, [f64; 3]) = (7.5, [-1.0, 0.5, -0.25]);
const BY_CARBON_DIOXIDE: (f64, f64, [f64; 2]) = (30.0, 10.0, [0.1, 0.01]);
const CARBON_DIOXIDE_SOLUBILITY: f64 = 0.02;
const PK: f64 = 6.0;
const STRONG_ION_OFFSET: f64 = 50.0;
const WEAK: [(f64, f64, f64); 2] = [(1.0, 10.0, 60.0), (1.0, 1.0, 6.0)];
const HEMOGLOBIN_CHARGE: (f64, f64) = (5.0, 30.0);
const CARBAMINO: (f64, f64, f64) = (4.0, 10.0, 10.0);
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
/// `ln(P / P50)` and the carbon dioxide partial pressure of the arterial blood and of the bed's settled plasma.
const ARTERIAL: (f64, f64) = (1.3, 31.0);
const VENOUS: (f64, f64) = (0.5, 35.0);
const HEAD: f64 = 5.0;
const CARBON_DIOXIDE_HEAD: f64 = 1.0;

fn ten_to(power: f64) -> f64 {
    exp(power * LN_10)
}

fn weak(acidity: f64) -> f64 {
    let acids: f64 = (WEAK.iter())
        .map(|(held, slope, offset)| held * (slope * acidity - offset))
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
    let d = acidity - BY_ACIDITY.0;
    let [c1, c2, c3] = BY_ACIDITY.1;
    let u = (pressure - BY_CARBON_DIOXIDE.0) / BY_CARBON_DIOXIDE.1;
    let [q1, q2] = BY_CARBON_DIOXIDE.2;
    P50 * (1.0 + c1 * d + c2 * d * d + c3 * d * d * d) * (1.0 + q1 * u + q2 * u * u)
}

fn carbamino(dissolved: f64, saturation: f64) -> f64 {
    let reach = CARBAMINO.1 + CARBAMINO.2 * saturation;
    CARBAMINO.0 * HEMOGLOBIN * dissolved / (dissolved + reach)
}

/// A plasma's closed forms: the oxygen partial pressure, saturation and oxygen content, the acidity and bicarbonate,
/// and the carbon dioxide partial pressure and total.
#[derive(Clone, Copy, Debug)]
struct Speciated {
    oxygen: (f64, f64, f64),
    acidity: f64,
    bicarbonate: f64,
    carbon_dioxide: (f64, f64),
}

/// The plasma at a carbon dioxide partial pressure, whose charge balance sets the acidity by bisection, and at an
/// oxygen partial pressure `P50 e^t` with P50 at that acidity and partial pressure.
fn speciated((t, pressure): (f64, f64)) -> Speciated {
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
        oxygen: (
            oxygen,
            saturation,
            SOLUBILITY * oxygen + HEME * HEMOGLOBIN * saturation,
        ),
        acidity,
        bicarbonate: bicarbonate(acidity),
        carbon_dioxide: (
            pressure,
            bicarbonate(acidity) + dissolved + carbamino(dissolved, saturation),
        ),
    }
}

fn flow(renewal: f64) -> f64 {
    renewal * WATER[0]
}

fn used() -> f64 {
    flow(RENEWAL) * (speciated(ARTERIAL).oxygen.2 - speciated(VENOUS).oxygen.2)
}

fn made() -> f64 {
    flow(RENEWAL) * (speciated(VENOUS).carbon_dioxide.1 - speciated(ARTERIAL).carbon_dioxide.1)
}

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    Ok(AnatomyBuilder::new().site::<C>(key, origin).id())
}

fn compiled() -> Result<CompiledBody, AdmissionError> {
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
    let bed = map.site::<CapillaryBed>(BED, origin()?).id();
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
    schema.boundary::<Oxygen>("artery oxygen", into, bed, Space::Plasma)?;
    schema.boundary::<Oxygen>("vein oxygen", out, bed, Space::Plasma)?;
    schema.boundary::<Oxygen>("oxygen use", out, bed, Space::Cells)?;
    schema.boundary::<CarbonDioxide>("artery carbon dioxide", into, bed, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("vein carbon dioxide", out, bed, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("carbon dioxide made", into, bed, Space::Cells)?;
    assemble(schema, |mut body| {
        let mut held = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), NOTE);
        for space in SPACES {
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
        let (laws, _) = body.hold(held)?.parts();
        body.add(laws.finish());

        let (mut laws, ports) = body.metabolism(None)?.parts();
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
        let carbon_dioxide_pressure: PlasmaPressure<'_, '_> =
            &|k| speciated.carbon_dioxide_partial_pressure(k);
        oxygen_diffusion(&mut laws, &oxygen, oxygen_pressure)?;
        carbon_dioxide_diffusion(&mut laws, &carbon_dioxide, carbon_dioxide_pressure)?;
        body.add(laws.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Reflex, SECOND)?;
        let key = ParameterKey::new;
        let renewal =
            laws.parameter::<Rate<Dimensionless>>(key("fixture.renewal"), Admissible::nonnegative());
        let delivered =
            laws.parameter::<Rate<Oxygen>>(key("fixture.arterial-oxygen"), Admissible::nonnegative());
        let use_rate = laws.parameter::<Rate<Oxygen>>(key("fixture.oxygen-use"), Admissible::nonnegative());
        for (name, rate) in [("artery oxygen", &delivered), ("oxygen use", &use_rate)] {
            let route = ports.route::<Oxygen>(name)?.route;
            let law = laws.kernel(|mut k| {
                let value = k.parameter(rate);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, law));
        }
        let delivered = laws.parameter::<Rate<CarbonDioxide>>(
            key("fixture.arterial-carbon-dioxide"),
            Admissible::nonnegative(),
        );
        let made_rate = laws
            .parameter::<Rate<CarbonDioxide>>(key("fixture.carbon-dioxide-made"), Admissible::nonnegative());
        for (name, rate) in [
            ("artery carbon dioxide", &delivered),
            ("carbon dioxide made", &made_rate),
        ] {
            let route = ports.route::<CarbonDioxide>(name)?.route;
            let law = laws.kernel(|mut k| {
                let value = k.parameter(rate);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, law));
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
        body.add(laws.finish());
        body.compile()
    })
}

fn solved(content: f64, pressure: f64) -> Speciated {
    let (mut low, mut high) = (-30.0, 15.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if speciated((middle, pressure)).oxygen.2 < content {
            low = middle;
        } else {
            high = middle;
        }
    }
    speciated((0.5 * (low + high), pressure))
}

/// The plasma whose oxygen content and carbon dioxide content are `content` and `total`, by bisection on the carbon
/// dioxide partial pressure, which carbamino's reading of saturation couples to the oxygen.
fn solved_both(content: f64, total: f64) -> Speciated {
    let (mut low, mut high) = (1.0, 200.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        if solved(content, middle).carbon_dioxide.1 < total {
            low = middle;
        } else {
            high = middle;
        }
    }
    solved(content, 0.5 * (low + high))
}

/// The plasma's amounts and the four fields' guesses at admission, and the blood the artery delivers.
#[derive(Clone, Copy, Debug)]
struct Start {
    plasma: Speciated,
    guess: (f64, f64, f64, f64),
    arterial: Speciated,
    renewal: f64,
}
impl Start {
    fn at(plasma: Speciated) -> Self {
        Self {
            plasma,
            guess: (
                plasma.oxygen.0,
                plasma.oxygen.1,
                plasma.bicarbonate,
                plasma.acidity,
            ),
            arterial: speciated(ARTERIAL),
            renewal: RENEWAL,
        }
    }
    fn cold(plasma: Speciated) -> Self {
        Self {
            guess: (1.0, 0.01, 1.0, 7.0),
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
        CARBON_DIOXIDE_SOLUBILITY * (1.0 + ratio) * (settled.carbon_dioxide.0 + heads * CARBON_DIOXIDE_HEAD)
    };
    match row {
        RowKind::Amount {
            species, space: at, ..
        } => {
            let water = WATER[space(at)];
            water
                * match (species, at) {
                    (Species::Water, _) => 1.0,
                    (Species::Hemoglobin, _) => HEMOGLOBIN,
                    (Species::Oxygen, Space::Plasma) => start.plasma.oxygen.2,
                    (Species::Oxygen, Space::Interstitium) => SOLUBILITY * (settled.oxygen.0 - HEAD),
                    (Species::Oxygen, _) => SOLUBILITY * (settled.oxygen.0 - 2.0 * HEAD),
                    (Species::CarbonDioxide, Space::Plasma) => start.plasma.carbon_dioxide.1,
                    (Species::CarbonDioxide, Space::Interstitium) => tissue(BUFFER_RATIOS.0, 1.0),
                    (Species::CarbonDioxide, _) => tissue(BUFFER_RATIOS.1, 2.0),
                    (ion, _) => {
                        let named = IONS.iter().find(|(named, ..)| *named == ion);
                        named.unwrap_or_else(|| panic!("no start for {row:?}")).1
                    }
                }
        }
        RowKind::Algebraic(slot) => match slot.law.local {
            "oxygen partial pressure" => start.guess.0,
            "oxygen saturation" => start.guess.1,
            "bicarbonate" => start.guess.2,
            "acidity" => start.guess.3,
            _ => panic!("no start for {row:?}"),
        },
        other => panic!("no start for {other:?}"),
    }
}

fn packet(body: &CompiledBody, start: Start) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
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
        ("bc.standard-carbon-dioxide", BY_CARBON_DIOXIDE.0),
        ("bc.carbon-dioxide-scale", BY_CARBON_DIOXIDE.1),
    ] {
        set::<Pressure>(&mut packet, key, body_wide, value)?;
    }
    let ([c1, c2, c3], [q1, q2]) = (BY_ACIDITY.1, BY_CARBON_DIOXIDE.2);
    for (key, value) in [
        ("bc.hill-ceiling", HILL.0),
        ("bc.hill-fall", HILL.1),
        ("bc.standard-acidity", BY_ACIDITY.0),
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
    let [
        (albumin, albumin_slope, albumin_offset),
        (phosphate, phosphate_slope, phosphate_offset),
    ] = WEAK;
    set::<Concentration<Albumin>>(&mut packet, "bc.plasma-albumin", body_wide, albumin)?;
    set::<Concentration<Phosphate>>(&mut packet, "bc.plasma-phosphate", body_wide, phosphate)?;
    type PerAcid<Q> = Quotient<Concentration<Charge>, Concentration<Q>>;
    set::<PerAcid<Albumin>>(&mut packet, "bc.albumin-slope", body_wide, albumin_slope)?;
    set::<PerAcid<Albumin>>(&mut packet, "bc.albumin-offset", body_wide, albumin_offset)?;
    set::<PerAcid<Phosphate>>(&mut packet, "bc.phosphate-slope", body_wide, phosphate_slope)?;
    set::<PerAcid<Phosphate>>(&mut packet, "bc.phosphate-offset", body_wide, phosphate_offset)?;
    set::<PerAcid<Hemoglobin>>(&mut packet, "bc.hemoglobin-slope", body_wide, HEMOGLOBIN_CHARGE.0)?;
    set::<PerAcid<Hemoglobin>>(
        &mut packet,
        "bc.hemoglobin-offset",
        body_wide,
        HEMOGLOBIN_CHARGE.1,
    )?;
    type Sites = Quotient<Concentration<CarbonDioxide>, Concentration<Hemoglobin>>;
    set::<Sites>(&mut packet, "bc.carbamino-sites", body_wide, CARBAMINO.0)?;
    set::<Concentration<CarbonDioxide>>(&mut packet, "bc.carbamino-half", body_wide, CARBAMINO.1)?;
    set::<Concentration<CarbonDioxide>>(&mut packet, "bc.carbamino-half-rise", body_wide, CARBAMINO.2)?;
    let beds = Binding::Class(Class::CapillaryBed);
    for key in ["mt.wall-oxygen-conductance", "mt.cell-oxygen-conductance"] {
        set::<Conductance<Oxygen>>(&mut packet, key, beds, used() / HEAD)?;
    }
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
        flow * start.arterial.oxygen.2,
    )?;
    set::<Rate<CarbonDioxide>>(
        &mut packet,
        "fixture.arterial-carbon-dioxide",
        body_wide,
        flow * start.arterial.carbon_dioxide.1,
    )?;
    set::<Rate<Oxygen>>(&mut packet, "fixture.oxygen-use", body_wide, used())?;
    set::<Rate<CarbonDioxide>>(&mut packet, "fixture.carbon-dioxide-made", body_wide, made())?;
    set::<Rate<Dimensionless>>(&mut packet, "fixture.renewal", body_wide, start.renewal)?;
    packet.start_rest(body, |row| guess(row, start), NOTE)?;
    packet.within(ErrorScales::uniform(Measure::new(1e-9)?, Measure::new(1e-10)?)?);
    Ok(packet)
}

fn admitted<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
    start: Start,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let lens = body.lens(Vec::new(), NonZeroU64::MIN.saturating_add(SECOND - 1))?;
    let port = BoundaryPort::new(Vec::new())?;
    Ok(admit_solved(body, packet(body, start)?, port, core, &lens, store)?.session)
}

fn value(body: &CompiledBody, session: &Session<'_, Running>, kind: RowKind) -> f64 {
    let row =
        (body.rows().iter().find(|row| row.kind() == kind)).unwrap_or_else(|| panic!("no row {kind:?}"));
    f64::from_bits(session.record().accepted().value(row.id()).bits()[0])
}

fn field(local: &'static str) -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Algebraic(Slot {
        law: LawName {
            owner: Metabolism::CARD,
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

fn fields(body: &CompiledBody, session: &Session<'_, Running>) -> Result<[f64; 4], AdmissionError> {
    Ok([
        value(body, session, field("oxygen partial pressure")?),
        value(body, session, field("oxygen saturation")?),
        value(body, session, field("bicarbonate")?),
        value(body, session, field("acidity")?),
    ])
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
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    for point in [(-0.5, 20.0), ARTERIAL, (0.5, 45.0)] {
        let expected = speciated(point);
        for start in [Start::at(expected), Start::cold(expected)] {
            let session = admitted(&body, &core, &store, start)?;
            let solved = fields(&body, &session)?;
            let [pressure, saturation, bicarbonate, acidity] = solved;
            let report = format!(
                "at {point:?} from {:?}: {solved:?} against {expected:?}",
                start.guess
            );
            assert!(
                (pressure - expected.oxygen.0).abs() <= 1e-12 * expected.oxygen.0,
                "{report}"
            );
            assert!((saturation - expected.oxygen.1).abs() <= 1e-12, "{report}");
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
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let (arterial, venous) = (speciated(ARTERIAL), speciated(VENOUS));
    let content = |species| -> Result<f64, AdmissionError> {
        Ok(value(&body, &session, amount(species, Space::Plasma)?) / WATER[0])
    };
    let taken = flow(RENEWAL) * (arterial.oxygen.2 - content(Species::Oxygen)?);
    assert!(
        (taken - used()).abs() <= 1e-6 * used(),
        "the flow times the arterial less the venous oxygen content is {taken} against a use of {}",
        used()
    );
    let given = flow(RENEWAL) * (content(Species::CarbonDioxide)? - arterial.carbon_dioxide.1);
    assert!(
        (given - made()).abs() <= 1e-6 * made(),
        "the flow times the venous less the arterial carbon dioxide content is {given} against {} made",
        made()
    );
    let settled = fields(&body, &session)?;
    let [pressure, _, bicarbonate, acidity] = settled;
    let report = format!("{settled:?} against {venous:?}");
    assert!((pressure - venous.oxygen.0).abs() <= 1e-6 * pressure, "{report}");
    assert!((acidity - venous.acidity).abs() <= 1e-6, "{report}");
    assert!(
        (bicarbonate - venous.bicarbonate).abs() <= 1e-6 * bicarbonate,
        "{report}"
    );
    let plasma = venous.carbon_dioxide.0;
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
        compiled()?,
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
        owner: Metabolism::CARD,
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
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let at = |content, pressure| {
        let plasma = solved(content, ARTERIAL.1);
        Start {
            guess: (pressure, plasma.oxygen.1, plasma.bicarbonate, plasma.acidity),
            ..Start::at(plasma)
        }
    };
    for start in [at(HEME * HEMOGLOBIN + 1.0, 41.0), at(6.0, 7_000.0)] {
        match admitted(&body, &core, &store, start) {
            Err(InitializationFailure::Undefined { law }) => assert_eq!(
                law,
                LawName {
                    owner: Metabolism::CARD,
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
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let plasma = speciated(ARTERIAL);
    for bicarbonate in [plasma.carbon_dioxide.1, plasma.carbon_dioxide.1 + 1.0] {
        let start = Start {
            guess: (plasma.oxygen.0, plasma.oxygen.1, bicarbonate, plasma.acidity),
            ..Start::at(plasma)
        };
        match admitted(&body, &core, &store, start) {
            Err(InitializationFailure::Undefined { law }) => assert_eq!(
                law,
                LawName {
                    owner: Metabolism::CARD,
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
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let plasma = speciated(ARTERIAL);
    let _session = admitted(&body, &core, &store, Start::at(plasma))?;
    let saturation_row = field("oxygen saturation")?;
    for saturation in [-0.25, 1.5] {
        let start = Start {
            guess: (plasma.oxygen.0, saturation, plasma.bicarbonate, plasma.acidity),
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
        compiled()?,
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
    a_fast_renewal_is_refused_by(solved(30.0, ARTERIAL.1), 1_000.0, "oxygen partial pressure")
}

#[test]
fn a_step_whose_bicarbonate_passes_what_the_total_leaves_is_refused_by_acidity()
-> Result<(), InitializationFailure> {
    let arterial = Speciated {
        carbon_dioxide: (ARTERIAL.1, 1.0),
        ..speciated(ARTERIAL)
    };
    a_fast_renewal_is_refused_by(arterial, 10_000.0, "acidity")
}

#[test]
fn steps_that_bring_bicarbonate_near_what_the_total_leaves_are_accepted_finite()
-> Result<(), InitializationFailure> {
    let arterial = Speciated {
        carbon_dioxide: (ARTERIAL.1, 1.0),
        ..speciated(ARTERIAL)
    };
    let (_, accepted) = a_fast_renewal(arterial, 100.0, "acidity")?;
    assert!(accepted > 0, "no step was accepted near acidity's edge");
    Ok(())
}

#[test]
fn an_oxygen_content_above_capacity_admits_at_its_partial_pressure() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let start = Start::at(solved(HEME * HEMOGLOBIN + 1.0, ARTERIAL.1));
    let session = admitted(&body, &core, &store, start)?;
    let pressure = value(&body, &session, field("oxygen partial pressure")?);
    assert!(
        (pressure - start.guess.0).abs() <= 1e-9 * start.guess.0,
        "{pressure} against {}",
        start.guess.0
    );
    Ok(())
}

/// Arterial blood above capacity carries the plasma's oxygen content across it while the bed settles over 200 s,
/// ten of the tissue's time constants at that pressure. No trial leaves the partial pressure's domain, so no step is
/// refused.
#[test]
fn a_run_carries_the_oxygen_content_across_capacity() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let content = HEME * HEMOGLOBIN + 2.0;
    let start = Start {
        arterial: Speciated {
            oxygen: (0.0, 0.0, content),
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
    let [pressure, ..] = fields(&body, &session)?;
    let plasma = solved_both(settled, speciated(VENOUS).carbon_dioxide.1);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(
        (held - settled).abs() <= 1e-6 * settled,
        "{held} against {settled}"
    );
    assert!(
        (pressure - plasma.oxygen.0).abs() <= 1e-6 * pressure,
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
