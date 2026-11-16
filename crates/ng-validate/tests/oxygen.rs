//! One bed whose plasma a held artery renews and whose cells use oxygen at a held rate, so blood chemistry's speciation
//! and metabolism's wall diffusion meet closed forms with no body admission. Every value is synthetic.

use core::num::NonZeroU64;
use ng_blood_chemistry::{Plasma, oxygenate};
use ng_contract::*;
use ng_metabolism::{OxygenRoutes, wall_diffusion};
use ng_record::{Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const NOTE: &str = "oxygen fixture: synthetic";
const BED: &str = "bed";
const SPACES: [Space; 3] = [Space::Plasma, Space::Interstitium, Space::Cells];
const WATER: [f64; 3] = [100.0, 300.0, 600.0];
const HEMOGLOBIN: f64 = 2.0;
const SOLUBILITY: f64 = 0.001;
const HEME: f64 = 4.0;
const P50: f64 = 25.0;
const HILL: (f64, f64, f64) = (2.5, 1.0, 10.0);
const RENEWAL: f64 = 1.0;
/// `ln(P / P50)` of the arterial blood and of the bed's settled plasma, because the host's logarithm is banned.
const REACH: (f64, f64) = (1.3, 0.5);
const HEAD: f64 = 5.0;

fn curve(t: f64) -> (f64, f64, f64) {
    let pressure = P50 * exp(t);
    let exponent = HILL.0 - HILL.1 * exp(-pressure / HILL.2);
    let saturation = 1.0 / (1.0 + exp(-exponent * t));
    (
        pressure,
        saturation,
        SOLUBILITY * pressure + HEME * HEMOGLOBIN * saturation,
    )
}

fn flow() -> f64 {
    RENEWAL * WATER[0]
}

fn used() -> f64 {
    flow() * (curve(REACH.0).2 - curve(REACH.1).2)
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
    }
    schema.carry::<Hemoglobin, CapillaryBed>(Space::Plasma);
    let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
    schema.boundary::<Oxygen>("artery oxygen", into, bed, Space::Plasma)?;
    schema.boundary::<Oxygen>("vein oxygen", out, bed, Space::Plasma)?;
    schema.boundary::<Oxygen>("oxygen use", out, bed, Space::Cells)?;
    assemble(schema, |mut body| {
        let mut held = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), NOTE);
        for space in SPACES {
            held = held.pool::<Water, CapillaryBed>(space);
        }
        let (laws, _) = body
            .hold(held.pool::<Hemoglobin, CapillaryBed>(Space::Plasma))?
            .parts();
        body.add(laws.finish());

        let (mut laws, ports) = body.metabolism(None)?.parts();
        let routes = OxygenRoutes::request(&mut laws, &ports);
        let plasma = Plasma {
            water: routes.wall.from_amount::<Water>(),
            oxygen: routes.wall.from_amount::<Oxygen>(),
            hemoglobin: routes.wall.from_amount::<Hemoglobin>(),
        };
        let oxygenated = oxygenate(&mut laws, &plasma)?;
        wall_diffusion(&mut laws, &routes, &oxygenated.partial_pressure)?;
        body.add(laws.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Reflex, SECOND)?;
        let key = ParameterKey::new;
        let delivered =
            laws.parameter::<Rate<Oxygen>>(key("fixture.arterial-oxygen"), Admissible::nonnegative());
        let renewal =
            laws.parameter::<Rate<Dimensionless>>(key("fixture.renewal"), Admissible::nonnegative());
        let use_rate = laws.parameter::<Rate<Oxygen>>(key("fixture.oxygen-use"), Admissible::nonnegative());
        for (name, rate) in [("artery oxygen", &delivered), ("oxygen use", &use_rate)] {
            let route = ports.route::<Oxygen>(name)?.route;
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
        body.add(laws.finish());
        body.compile()
    })
}

#[derive(Clone, Copy)]
struct Start {
    content: f64,
    pressure: f64,
    saturation: f64,
}
impl Start {
    fn settled() -> Self {
        let (pressure, saturation, content) = curve(REACH.1);
        Self {
            content,
            pressure,
            saturation,
        }
    }
    fn cold(content: f64) -> Self {
        Self {
            content,
            pressure: 1.0,
            saturation: 0.01,
        }
    }
}

fn guess(row: RowKind, start: Start) -> f64 {
    let settled = curve(REACH.1).0;
    match row {
        RowKind::Amount {
            species: Species::Water,
            space,
            ..
        } => {
            WATER[SPACES
                .iter()
                .position(|of| *of == space)
                .expect("a fixture space")]
        }
        RowKind::Amount {
            species: Species::Hemoglobin,
            ..
        } => HEMOGLOBIN * WATER[0],
        RowKind::Amount {
            species: Species::Oxygen,
            space,
            ..
        } => match space {
            Space::Plasma => start.content * WATER[0],
            Space::Interstitium => SOLUBILITY * (settled - HEAD) * WATER[1],
            _ => SOLUBILITY * (settled - 2.0 * HEAD) * WATER[2],
        },
        RowKind::Algebraic(slot) if slot.law.local == "oxygen partial pressure" => start.pressure,
        RowKind::Algebraic(slot) if slot.law.local == "oxygen saturation" => start.saturation,
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
    for (key, value) in [
        ("bc.p50", P50),
        ("bc.hill-reach", HILL.2),
        ("bc.standard-carbon-dioxide", 40.0),
        ("bc.plasma-carbon-dioxide", 40.0),
        ("bc.carbon-dioxide-scale", 40.0),
    ] {
        set::<Pressure>(&mut packet, key, body_wide, value)?;
    }
    for (key, value) in [
        ("bc.hill-ceiling", HILL.0),
        ("bc.hill-fall", HILL.1),
        ("bc.standard-acidity", 7.4),
        ("bc.plasma-acidity", 7.4),
        ("bc.p50-acidity-1", -1.0),
        ("bc.p50-acidity-2", 0.5),
        ("bc.p50-acidity-3", -0.25),
        ("bc.p50-carbon-dioxide-1", 0.1),
        ("bc.p50-carbon-dioxide-2", 0.01),
    ] {
        set::<Dimensionless>(&mut packet, key, body_wide, value)?;
    }
    let beds = Binding::Class(Class::CapillaryBed);
    for key in ["mt.wall-oxygen-conductance", "mt.cell-oxygen-conductance"] {
        set::<Conductance<Oxygen>>(&mut packet, key, beds, used() / HEAD)?;
    }
    set::<Rate<Oxygen>>(
        &mut packet,
        "fixture.arterial-oxygen",
        body_wide,
        flow() * curve(REACH.0).2,
    )?;
    set::<Rate<Oxygen>>(&mut packet, "fixture.oxygen-use", body_wide, used())?;
    set::<Rate<Dimensionless>>(&mut packet, "fixture.renewal", body_wide, RENEWAL)?;
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

fn plasma_oxygen() -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Amount {
        site: site::<CapillaryBed>(BED)?,
        species: Species::Oxygen,
        space: Space::Plasma,
    })
}

fn after<'m>(session: Session<'m, Running>, seconds: u64) -> Session<'m, Running> {
    run_until(session, SimTime::from_ns(seconds * SECOND))
        .unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()))
}

#[test]
fn speciation_meets_the_curve_at_three_partial_pressures_from_a_cold_guess_and_a_warm_one()
-> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let (pressure, saturation) = (field("oxygen partial pressure")?, field("oxygen saturation")?);
    for t in [-0.5, REACH.1, REACH.0] {
        let (p, s, content) = curve(t);
        let warm = Start {
            content,
            pressure: p,
            saturation: s,
        };
        for start in [warm, Start::cold(content)] {
            let session = admitted(&body, &core, &store, start)?;
            let (solved, share) = (
                value(&body, &session, pressure),
                value(&body, &session, saturation),
            );
            assert!(
                (solved - p).abs() <= 1e-12 * p && (share - s).abs() <= 1e-12,
                "at t {t} from {} mmHg: {solved} and {share} against {p} and {s}",
                start.pressure
            );
        }
    }
    Ok(())
}

fn settling<'m>(
    body: &'m CompiledBody,
    core: &'m ng_solver::Core,
    store: &'m MemoryStore,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let (pressure, saturation, content) = curve(REACH.0);
    let start = Start {
        content,
        pressure,
        saturation,
    };
    Ok(after(admitted(body, core, store, start)?, 20))
}

#[test]
fn the_bed_settles_where_the_fick_identity_puts_it() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let venous = value(&body, &session, plasma_oxygen()?) / WATER[0];
    let taken = flow() * (curve(REACH.0).2 - venous);
    assert!(
        (taken - used()).abs() <= 1e-6 * used(),
        "the flow times the arterial less the venous oxygen content is {taken} against a use of {}",
        used()
    );
    let pressure = value(&body, &session, field("oxygen partial pressure")?);
    assert!(
        (pressure - curve(REACH.1).0).abs() <= 1e-6 * pressure,
        "{pressure}"
    );
    Ok(())
}

/// The arena certificate's closure, a part in 10^12 of the total, with no bound on the drift, because the settling bed's
/// total falls as its plasma gives up the arterial oxygen content.
const CLOSURE: f64 = 1e-12;

#[test]
fn oxygen_closes_to_the_ledgers_floor_through_speciation() -> Result<(), InitializationFailure> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    let session = settling(&body, &core, &store)?;
    let windows = SlidingWindows::new(NonZeroU64::MIN.saturating_add(10 * SECOND - 1), 20 * SECOND)?;
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
    let mut oxygen = None;
    let mut quantities = Vec::new();
    for quantity in basis.ids() {
        match quantity.book() {
            Book::Of(species) => {
                if species == Species::Oxygen {
                    oxygen = Some(quantity);
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
    let oxygen = oxygen.expect("oxygen is conserved");
    assert!(
        certificate
            .results()
            .iter()
            .any(|result| result.quantity() == oxygen)
    );
    Ok(())
}

/// At or above capacity no partial pressure leaves the bound share under one, so its logarithm has no value.
#[test]
fn oxygen_content_at_capacity_has_no_partial_pressure() -> Result<(), AdmissionError> {
    let (body, core, store) = (
        compiled()?,
        ng_solver::Core::new(SECOND / 100),
        MemoryStore::default(),
    );
    for content in [HEME * HEMOGLOBIN + 1.0, 2.0 * HEME * HEMOGLOBIN] {
        let start = Start {
            content,
            ..Start::settled()
        };
        match admitted(&body, &core, &store, start) {
            Err(InitializationFailure::Undefined { law }) => assert_eq!(
                law,
                LawName {
                    owner: Metabolism::CARD,
                    local: "oxygen partial pressure"
                }
            ),
            Err(other) => panic!("at {content} mM: {other:?}"),
            Ok(_) => panic!("at {content} mM the fixture admitted"),
        }
    }
    Ok(())
}

#[test]
fn only_the_whole_body_speciates_oxygen() -> Result<(), AdmissionError> {
    let speciates = |body: &CompiledBody| {
        let key = ParameterKey::new("bc.p50");
        let declared = body.parameters().iter().any(|slot| slot.key() == key);
        let fields = (body.rows().iter()).any(|row| {
            matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == "oxygen partial pressure")
        });
        (declared, fields)
    };
    assert_eq!(speciates(&whole_body()?), (true, true));
    assert_eq!(speciates(&arena_body()?), (false, false));
    Ok(())
}
