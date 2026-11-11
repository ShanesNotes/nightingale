//! The arena certificate's drift limits, on a synthetic body that energy and water enter at steady rates. No arena
//! history can serve: a short arena run moves energy by about 1e4 at most, so none comes near energy's limit of 4e5.

use ng_contract::*;
use ng_validate::*;

const HOUR: u64 = 3_600_000_000_000;
const DAY: u64 = 24 * HOUR;
const DAY_SECONDS: f64 = 86_400.0;
const ROOT: &str = "body";
const VESSEL: &str = "vessel";
const ENERGY_IN: &str = "energy in";
const WATER_IN: &str = "water in";
const ENERGY_RATE: ParameterKey = ParameterKey::new("bd.energy-in");
const WATER_RATE: ParameterKey = ParameterKey::new("bd.water-in");

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

fn steady<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    key: ParameterKey,
) -> Result<(), AdmissionError> {
    let route = ports.route::<Q>(name)?.route;
    let rate = laws.parameter::<Rate<Q>>(key, Admissible::nonnegative());
    let class = RateClass::new(RateBand::Hours, HOUR)?;
    let inflow = laws.kernel(|mut k| {
        let value = k.parameter(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, inflow));
    Ok(())
}

/// No other law moves either pool, so each one's drift over a day is what entered it that day.
fn steady_inflows() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>(ROOT, origin()?).id();
    let vessel = map.site::<VesselSegment>(VESSEL, origin()?).id();
    map.contain(root, vessel);
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(root, Space::Thermal)?;
    let into = BoundaryDirection::IntoBody;
    schema.boundary::<Energy>(ENERGY_IN, into, root, Space::Thermal)?;
    schema.boundary::<Water>(WATER_IN, into, vessel, Space::Lumen)?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        steady::<Energy>(&mut laws, &ports, ENERGY_IN, ENERGY_RATE)?;
        steady::<Water>(&mut laws, &ports, WATER_IN, WATER_RATE)?;
        body.add(laws.finish());
        body.compile()
    })
}

fn packet(energy_a_day: f64, water_a_day: f64) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let (mut packet, note) = (ParameterPacket::new(), "synthetic");
    let energy = Measure::<Rate<Energy>>::new(energy_a_day / DAY_SECONDS)?;
    packet.synthetic(ENERGY_RATE, Binding::Body, energy, note)?;
    let water = Measure::<Rate<Water>>::new(water_a_day / DAY_SECONDS)?;
    packet.synthetic(WATER_RATE, Binding::Body, water, note)?;
    let store = RowKind::Amount {
        site: site::<TissueSite>(ROOT)?,
        space: Space::Thermal,
        species: Species::Energy,
    };
    packet.start(store, Measure::<Energy>::new(1e6)?, note)?;
    let lumen = RowKind::Amount {
        site: site::<VesselSegment>(VESSEL)?,
        space: Space::Lumen,
        species: Species::Water,
    };
    packet.start(lumen, Measure::<Water>::new(1e6)?, note)?;
    Ok(packet)
}

fn certified(energy_a_day: f64, water_a_day: f64) -> Result<Certificate, CertificateFailure> {
    let body = steady_inflows().expect("compiled");
    let packet = packet(energy_a_day, water_a_day).expect("a packet");
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

#[test]
fn energy_drifts_up_to_4e5_over_the_arenas_day_and_no_further() {
    let under = certified(399_999.0, 0.0).expect("energy inside its limit");
    assert!((drift(&under, Species::Energy) - 399_999.0).abs() < 1e-6);
    let over = certified(400_001.0, 0.0).expect_err("energy past its limit");
    assert_eq!(
        refused(over),
        (CertificateCheck::Drift, Some(Book::Of(Species::Energy)))
    );
}

/// Water moves only its own quantity here, since no reaction joins it to another and it holds no charge, so the
/// refusal names the limit water keeps and no other.
#[test]
fn water_drifts_up_to_1e4_as_every_quantity_but_energy_does() {
    let under = certified(0.0, 9_999.0).expect("water inside its limit");
    assert!((drift(&under, Species::Water) - 9_999.0).abs() < 1e-6);
    let over = certified(0.0, 10_001.0).expect_err("water past its limit, far inside energy's");
    assert_eq!(
        refused(over),
        (CertificateCheck::Drift, Some(Book::Of(Species::Water)))
    );
}
