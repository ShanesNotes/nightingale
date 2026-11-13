//! The arena certificate's drift limits, on a synthetic body that each limited quantity enters at a steady rate. An
//! arena history cannot serve: its drift is whatever the day's physiology makes it, over a day that takes hours to
//! run, and each limit needs a drift just under it and one just over. An ion enters beside its counter-ion, so the
//! drift under test is the only one past its limit.

use ng_contract::*;
use ng_validate::*;

const HOUR: u64 = 3_600_000_000_000;
const DAY: u64 = 24 * HOUR;
const DAY_SECONDS: f64 = 86_400.0;
const ROOT: &str = "body";
const VESSEL: &str = "vessel";

fn origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    Ok(AnatomyBuilder::new().site::<C>(key, origin()?).id())
}

fn inflow(species: Species) -> &'static str {
    match species {
        Species::Energy => "energy in",
        Species::Water => "water in",
        Species::Sodium => "sodium in",
        Species::Potassium => "potassium in",
        Species::Chloride => "chloride in",
        Species::Vasopressin => "vasopressin in",
        Species::Norepinephrine => "norepinephrine in",
        _ => panic!("not a quantity this body takes in"),
    }
}

fn rate_key(species: Species) -> ParameterKey {
    ParameterKey::new(match species {
        Species::Energy => "bd.energy-in",
        Species::Water => "bd.water-in",
        Species::Sodium => "bd.sodium-in",
        Species::Potassium => "bd.potassium-in",
        Species::Chloride => "bd.chloride-in",
        Species::Vasopressin => "bd.vasopressin-in",
        Species::Norepinephrine => "bd.norepinephrine-in",
        _ => panic!("not a quantity this body takes in"),
    })
}

fn steady<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    species: Species,
) -> Result<(), AdmissionError> {
    let route = ports.route::<Q>(inflow(species))?.route;
    let rate = laws.parameter::<Rate<Q>>(rate_key(species), Admissible::nonnegative());
    let class = RateClass::new(RateBand::Hours, HOUR)?;
    let inflow = laws.kernel(|mut k| {
        let value = k.parameter(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, inflow));
    Ok(())
}

fn steady_inflows() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>(ROOT, origin()?).id();
    let vessel = map.site::<VesselSegment>(VESSEL, origin()?).id();
    map.contain(root, vessel);
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(root, Space::Thermal)?;
    let into = BoundaryDirection::IntoBody;
    schema.boundary::<Energy>(inflow(Species::Energy), into, root, Space::Thermal)?;
    schema.boundary::<Water>(inflow(Species::Water), into, vessel, Space::Lumen)?;
    schema.boundary::<Sodium>(inflow(Species::Sodium), into, vessel, Space::Lumen)?;
    schema.boundary::<Potassium>(inflow(Species::Potassium), into, vessel, Space::Lumen)?;
    schema.boundary::<Chloride>(inflow(Species::Chloride), into, vessel, Space::Lumen)?;
    schema.boundary::<Vasopressin>(inflow(Species::Vasopressin), into, vessel, Space::Lumen)?;
    schema.boundary::<Norepinephrine>(inflow(Species::Norepinephrine), into, vessel, Space::Lumen)?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        steady::<Energy>(&mut laws, &ports, Species::Energy)?;
        steady::<Water>(&mut laws, &ports, Species::Water)?;
        steady::<Sodium>(&mut laws, &ports, Species::Sodium)?;
        steady::<Potassium>(&mut laws, &ports, Species::Potassium)?;
        steady::<Chloride>(&mut laws, &ports, Species::Chloride)?;
        steady::<Vasopressin>(&mut laws, &ports, Species::Vasopressin)?;
        steady::<Norepinephrine>(&mut laws, &ports, Species::Norepinephrine)?;
        body.add(laws.finish());
        body.compile()
    })
}

fn supply<Q: Substance>(
    packet: &mut ParameterPacket<SyntheticFixture>,
    species: Species,
    a_day: f64,
    store: Result<RowKind, AdmissionError>,
    start: f64,
) -> Result<(), AdmissionError> {
    let note = "synthetic";
    let rate = Measure::<Rate<Q>>::new(a_day / DAY_SECONDS)?;
    packet.synthetic(rate_key(species), Binding::Body, rate, note)?;
    packet.start(store?, Measure::<Q>::new(start)?, note)
}

fn packet(inflows: &[(Species, f64)]) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::new();
    let a_day = |species| {
        let entry = inflows.iter().find(|(each, _)| *each == species);
        entry.map_or(0.0, |(_, amount)| *amount)
    };
    let thermal = Ok(RowKind::Amount {
        site: site::<TissueSite>(ROOT)?,
        space: Space::Thermal,
        species: Species::Energy,
    });
    let lumen = |species| -> Result<RowKind, AdmissionError> {
        Ok(RowKind::Amount {
            site: site::<VesselSegment>(VESSEL)?,
            space: Space::Lumen,
            species,
        })
    };
    let (energy, water) = (Species::Energy, Species::Water);
    supply::<Energy>(&mut packet, energy, a_day(energy), thermal, 1e6)?;
    supply::<Water>(&mut packet, water, a_day(water), lumen(water), 1e6)?;
    let (sodium, potassium, chloride) = (Species::Sodium, Species::Potassium, Species::Chloride);
    supply::<Sodium>(&mut packet, sodium, a_day(sodium), lumen(sodium), 1e5)?;
    supply::<Potassium>(&mut packet, potassium, a_day(potassium), lumen(potassium), 1e5)?;
    supply::<Chloride>(&mut packet, chloride, a_day(chloride), lumen(chloride), 2e5)?;
    let (vasopressin, norepinephrine) = (Species::Vasopressin, Species::Norepinephrine);
    supply::<Vasopressin>(
        &mut packet,
        vasopressin,
        a_day(vasopressin),
        lumen(vasopressin),
        1e3,
    )?;
    supply::<Norepinephrine>(
        &mut packet,
        norepinephrine,
        a_day(norepinephrine),
        lumen(norepinephrine),
        1e2,
    )?;
    Ok(packet)
}

fn certified(inflows: &[(Species, f64)]) -> Result<Certificate, CertificateFailure> {
    let body = steady_inflows().expect("compiled");
    let packet = packet(inflows).expect("a packet");
    let (core, store) = (ReferenceIntegrator::new(HOUR), MemoryStore::default());
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let session = admit_fixture(&body, packet, boundary, &core, &store).expect("admitted");
    let Ok(session) = run_until(session, SimTime::from_ns(DAY)) else {
        panic!("the day faulted");
    };
    let policy = arena_certificate(&body, ArenaProtocol::pick().windows).expect("a policy");
    certify(&session.conservation(), &policy)
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn drift(certificate: &Certificate, species: Species) -> f64 {
    let own = |result: &&QuantityResult| result.quantity().book() == Book::Of(species);
    let result = certificate.results().iter().find(own).expect("a quantity");
    hi(result.worst_drift())
}

fn refused(failure: CertificateFailure) -> (CertificateCheck, Option<Book>) {
    (failure.check(), failure.quantity().map(QuantityId::book))
}

/// `species` drifts a thousandth under `limit` and then a thousandth over it, beside `partner` held a thousandth
/// under, which balances an ion's charge.
fn drifts_up_to(species: Species, limit: f64, partner: Option<Species>) {
    let (under, over) = (limit * 0.999, limit * 1.001);
    let beside = |amount| {
        let mut inflows = vec![(species, amount)];
        inflows.extend(partner.map(|partner| (partner, under)));
        inflows
    };
    let passed = certified(&beside(under)).expect("inside its limit");
    assert!((drift(&passed, species) - under).abs() < 1e-6 * limit);
    let failed = certified(&beside(over)).expect_err("past its limit");
    assert_eq!(
        refused(failed),
        (CertificateCheck::Drift, Some(Book::Of(species)))
    );
}

#[test]
fn energy_drifts_up_to_2e5_over_the_arenas_day_and_no_further() {
    drifts_up_to(Species::Energy, 2e5, None);
}

#[test]
fn sodium_drifts_up_to_4e4_twice_the_slices_measured_day() {
    drifts_up_to(Species::Sodium, 4e4, Some(Species::Chloride));
}

#[test]
fn chloride_drifts_up_to_4e4_twice_the_slices_measured_day() {
    drifts_up_to(Species::Chloride, 4e4, Some(Species::Sodium));
}

#[test]
fn potassium_drifts_up_to_2e3_twice_the_slices_measured_day() {
    drifts_up_to(Species::Potassium, 2e3, Some(Species::Chloride));
}

#[test]
fn water_drifts_up_to_5e2_twice_the_slices_measured_day() {
    drifts_up_to(Species::Water, 5e2, None);
}

#[test]
fn vasopressin_drifts_up_to_8e2_twice_the_slices_measured_day() {
    drifts_up_to(Species::Vasopressin, 8e2, None);
}

#[test]
fn norepinephrine_drifts_up_to_the_floor_of_a_quantity_the_slice_does_not_move() {
    drifts_up_to(Species::Norepinephrine, 1.0, None);
}
