//! The arena certificate's element limit, on the burning beds, the cheapest body with a reaction and so with element
//! books of their own, one set a bed. Nothing burns and nothing leaves, and only oxygen enters, at a steady rate, so
//! each bed's oxygen element book rises by twice the oxygen let in over each window and every other book holds still.
//! Every value is synthetic.

use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const WINDOW_S: u64 = 10;
const NOTE: &str = "element drift: synthetic";
const BEDS: [&str; 3] = ["bed a", "bed b", "bed c"];

fn site(key: &'static str) -> Result<SiteId, AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    Ok(AnatomyBuilder::new().site::<CapillaryBed>(key, origin).id())
}

fn packet(oxygen: f64) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::new();
    let (body, beds) = (Binding::Body, Binding::Class(Class::CapillaryBed));
    let key = ParameterKey::new;
    packet.synthetic(
        key("bench.oxygen-supply"),
        body,
        Measure::<Rate<Oxygen>>::new(oxygen)?,
        NOTE,
    )?;
    packet.synthetic(
        key("bench.glucose-supply"),
        body,
        Measure::<Rate<Glucose>>::new(0.0)?,
        NOTE,
    )?;
    for still in [
        "bench.carbon-dioxide-clearance",
        "bench.water-clearance",
        "bench.heat-loss",
    ] {
        packet.synthetic(key(still), body, Measure::<Rate<Dimensionless>>::new(0.0)?, NOTE)?;
    }
    packet.synthetic(
        key("mt.burn-speed"),
        beds,
        Measure::<Rate<Dimensionless>>::new(0.0)?,
        NOTE,
    )?;
    packet.synthetic(key("mt.oxygen-half"), beds, Measure::<Oxygen>::new(5.0)?, NOTE)?;
    for bed in BEDS {
        let at = |space, species| RowKind::Amount {
            site: site(bed).expect("a bed"),
            space,
            species,
        };
        let interstitial = |species| at(Space::Interstitium, species);
        packet.start(
            interstitial(Species::Water),
            Measure::<Water>::new(1_000.0)?,
            NOTE,
        )?;
        packet.start(
            interstitial(Species::Glucose),
            Measure::<Glucose>::new(10.0)?,
            NOTE,
        )?;
        packet.start(interstitial(Species::Oxygen), Measure::<Oxygen>::new(50.0)?, NOTE)?;
        let carbon_dioxide = Measure::<CarbonDioxide>::new(6.0)?;
        packet.start(interstitial(Species::CarbonDioxide), carbon_dioxide, NOTE)?;
        let heat = Measure::<Energy>::new(1_000.0)?;
        packet.start(at(Space::Thermal, Species::Energy), heat, NOTE)?;
    }
    Ok(packet)
}

/// The arena's policy on ten-second windows over a run in which each bed's oxygen element book moves by `drift` a
/// window.
fn certified(drift: f64) -> Result<Certificate, CertificateFailure> {
    let case = BurningBeds::burning().expect("compiled");
    let oxygen = drift / (2.0 * WINDOW_S as f64);
    let (core, store) = (
        ReferenceIntegrator::new(WINDOW_S * SECOND),
        MemoryStore::default(),
    );
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let packet = packet(oxygen).expect("a packet");
    let session = admit_fixture(case.body(), packet, boundary, &core, &store).expect("admitted");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    let window = NonZeroU64::new(WINDOW_S * SECOND).expect("positive");
    let windows = SlidingWindows::new(window, case.end().nanoseconds()).expect("a coverage");
    let policy = arena_certificate(case.body(), windows).expect("a policy");
    certify(&session.conservation(), &policy)
}

#[test]
fn an_element_drifts_up_to_1e4_a_window_and_no_further() {
    let oxygen = Book::Element(Element::Oxygen);
    let (limit, margin) = (1e4, 10.0);
    let passed = certified(limit - margin).expect("inside its limit");
    let result = (passed.results().iter())
        .find(|result| result.quantity().book() == oxygen)
        .expect("oxygen's element book");
    let drift = f64::from_bits(result.worst_drift().bits()[0]);
    assert!((drift - (limit - margin)).abs() < 1e-6 * limit, "{drift}");
    let failed = certified(limit + margin).expect_err("past its limit");
    assert_eq!(
        (failed.check(), failed.quantity().map(QuantityId::book)),
        (CertificateCheck::Drift, Some(oxygen))
    );
}
