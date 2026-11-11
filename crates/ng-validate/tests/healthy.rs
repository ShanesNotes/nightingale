use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const HOUR: u64 = 3_600 * SECOND;
const METHOD: &str = "a synthetic method";

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn water(value: f64) -> Measure<Water> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

fn excursion() -> TwoReservoirs {
    let delivery = Delivery {
        id: b"input",
        from: at(40),
        to: at(60),
        amount: water(100_000.0),
    };
    let case =
        TwoReservoirs::delivering([water(500_500.0), water(499_500.0)], vec![delivery]).expect("a body");
    case.until(at(600))
}

fn amount(case: &TwoReservoirs) -> RowKind {
    let [a, _] = case.rows();
    case.body().rows()[a.index()].kind()
}

fn set(case: &TwoReservoirs, high: f64) -> HealthyRangeSet<SyntheticFixture> {
    let averaging = Averaging::Sliding(count(4 * SECOND));
    let (low, high) = (water(490_000.0), water(high));
    let range = HealthyRange::synthetic(
        "xx.reservoir",
        amount(case),
        METHOD,
        Scope::Every,
        averaging,
        low,
        high,
        "synthetic",
    )
    .expect("a range");
    HealthyRangeSet::synthetic(vec![range], Vec::new()).expect("a set")
}

fn lens(case: &TwoReservoirs) -> ObservationLens {
    let channel = ChannelSpec::new(METHOD, amount(case)).averaged(count(4));
    case.body().lens(vec![channel], count(SECOND)).expect("a lens")
}

fn day() -> DayActivities {
    DayActivities::new(vec![(at(0), Activity::new("awake"))]).expect("a day")
}

fn step<'m>(session: Session<'m, Running>) -> Session<'m, Running> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, _) => return session,
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        };
    }
}

struct Checked {
    run: Digest,
    range: RangeReport,
    returns: ReturnReport,
}

fn checked(case: &TwoReservoirs, set: &HealthyRangeSet<SyntheticFixture>) -> Checked {
    let core = ReferenceIntegrator::new(8 * SECOND);
    let store = MemoryStore::default();
    let body = case.body();
    let bare = BareLens::new(body, &lens(case)).expect("this body's lens");
    let anchors = case.packet().expect("a packet").anchors();
    let mut session = case.admit(&core, &store).expect("admitted");
    let mut checks = ReadingChecks::new(body, &bare, set, &day(), anchors, &session).expect("the checks");
    while session.record().accepted().time() < case.end() {
        session = step(session);
        checks.read(&session).expect("the next revision");
    }
    let run = session.record().identity().digest();
    let (range, returns) = checks.finish();
    Checked { run, range, returns }
}

#[test]
fn a_run_pushed_past_its_healthy_range_for_a_while_fails_naming_the_reading_and_a_window_in_the_excursion() {
    let case = excursion();
    let checked = checked(&case, &set(&case, 575_000.0));
    let range = &checked.range;
    assert_eq!(range.verdict(), CheckVerdict::Fail);
    assert_eq!(range.gaps(), []);
    let [result] = range.results() else {
        panic!("one result for the one range: {:?}", range.results());
    };
    assert_eq!(
        (result.range(), result.row(), result.method()),
        ("xx.reservoir", amount(&case), METHOD)
    );
    let failure = result.failure().expect("a window above 575 000");
    let opens = SimTime::from_ns(failure.end().nanoseconds() - failure.length_ns());
    assert!(
        at(40) <= opens && failure.end() <= at(60),
        "{failure:?} lies inside the delivery that drives the excursion"
    );
    let mean = failure.mean().expect("every value present");
    assert!(!mean.at_most(Compensated::of(&water(575_000.0))), "{mean:?}");
    assert_eq!(
        range.lens(),
        lens(&case).digest(),
        "the report names the learner's lens"
    );
    assert_eq!(
        (checked.returns.verdict(), checked.returns.gaps()),
        (CheckVerdict::Blocked, [ReturnGap::NoAnchors].as_slice()),
        "the fixture has no anchor to return to, and a check that held nothing never passes"
    );
}

#[test]
fn a_corrected_set_rechecks_a_finished_replay_and_gives_a_new_result_for_the_same_run() {
    let case = excursion();
    let first = checked(&case, &set(&case, 575_000.0));
    let corrected = checked(&case, &set(&case, 700_000.0));
    assert_eq!(first.run, corrected.run, "one run identity before and after");
    assert_eq!((first.range.run(), corrected.range.run()), (first.run, first.run));
    assert_eq!(
        first.range.seal(),
        corrected.range.seal(),
        "the replay reached the same last revision"
    );
    assert_ne!(first.range.set(), corrected.range.set());
    assert_eq!(
        (first.range.verdict(), corrected.range.verdict()),
        (CheckVerdict::Fail, CheckVerdict::Pass)
    );
    assert_eq!(
        (first.returns.run(), first.returns.set()),
        (first.run, first.range.set())
    );
}

#[test]
fn a_return_report_names_the_packet_whose_anchors_it_read() {
    let case = excursion();
    let core = ReferenceIntegrator::new(8 * SECOND);
    let store = MemoryStore::default();
    let bare = BareLens::new(case.body(), &lens(&case)).expect("this body's lens");
    let set = set(&case, 575_000.0);
    let session = case.admit(&core, &store).expect("admitted");
    let packet = || case.packet().expect("a packet");
    let admitted = case.body().bind(packet()).expect("bound").packet_hash();
    let named = |anchors: Anchors| {
        let checks = ReadingChecks::new(case.body(), &bare, &set, &day(), anchors, &session);
        checks.expect("the checks").finish().1.packet()
    };
    assert_eq!(
        packet().anchors().packet(),
        admitted,
        "the view names the packet the run bound"
    );
    assert_eq!(named(packet().anchors()), admitted);
    let other = ParameterPacket::<SyntheticFixture>::new().anchors();
    assert_ne!(
        named(other),
        admitted,
        "another packet's anchors name another packet"
    );
}

#[test]
fn the_checks_read_this_bodys_lens_from_admission() {
    let case = excursion();
    let closed = TwoReservoirs::closed().expect("another body");
    let elsewhere = closed.body().lens(Vec::new(), count(SECOND)).expect("a lens");
    assert_eq!(
        BareLens::new(case.body(), &elsewhere).err(),
        Some(ReadingCheckError::OtherBody)
    );
    let core = ReferenceIntegrator::new(8 * SECOND);
    let store = MemoryStore::default();
    let bare = BareLens::new(case.body(), &lens(&case)).expect("this body's lens");
    let set = set(&case, 575_000.0);
    let session = step(case.admit(&core, &store).expect("admitted"));
    let anchors = case.packet().expect("a packet").anchors();
    let late = ReadingChecks::new(case.body(), &bare, &set, &day(), anchors, &session);
    assert_eq!(late.err(), Some(ReadingCheckError::AfterAdmission));
}

const GAIN: ParameterKey = ParameterKey::new("xx.gain");
const STORED: AnchorId = AnchorId::new("xx.stored");
const GAUGED: AnchorId = AnchorId::new("xx.gauged");
const QUARTER: u64 = HOUR / 4;
const DELIVERED: f64 = 900.0;
const STORED_WATER: f64 = 10.0;
const GAUGE_MEAN: f64 = 0.5;
/// The size admission gives a row whose scale the packet does not state.
const UNSCALED: f64 = 1.0;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

struct Gauged {
    body: CompiledBody,
    stored: RowKind,
    gauge: RowKind,
}

/// One vessel in the minutes band, so an instant anchor holds on its water, and a gauge that reads the inflow's
/// drive times a gain.
fn gauged() -> Result<Gauged, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let origin = Position::new(measure(0.0), measure(0.0), measure(0.0));
    let site = map.site::<VesselSegment>("vessel", origin).id();
    let mut schema = Schema::new(map.finish()?)?;
    schema.boundary::<Water>("in", BoundaryDirection::IntoBody, site, Space::Lumen)?;
    schema.boundary::<Water>("out", BoundaryDirection::OutOfBody, site, Space::Lumen)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Minutes, 60 * SECOND)?;
        let gain = laws.parameter::<Dimensionless>(GAIN, Admissible::finite());
        let gauge = laws.algebraic::<Rate<Water>>("gauge", Admissible::finite());
        for boundary in ["in", "out"] {
            let BoundaryRoute { route, drive } = ports.route::<Water>(boundary)?;
            let flow = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, flow));
            if boundary == "in" {
                let reads = laws.kernel(|mut k| {
                    let x = k.algebraic(&gauge);
                    let input = k.read(&drive);
                    let gain = k.parameter(&gain);
                    let target = k.scale(input, gain);
                    let residual = k.subtract(x, target);
                    k.finish(residual, laws.every_mode(), class)
                });
                laws.residual(&gauge, reads);
            }
        }
        body.add(laws.finish());
        body.compile()
    })?;
    let gauge = RowKind::Algebraic(
        LawName {
            owner: "Boundary",
            local: "gauge",
        }
        .into(),
    );
    let stored = RowKind::Amount {
        site,
        space: Space::Lumen,
        species: Species::Water,
    };
    Ok(Gauged { body, stored, gauge })
}

fn anchored(gauged: &Gauged) -> ParameterPacket<SyntheticFixture> {
    let cycle = Cycle::new(count(HOUR), NonZeroU32::new(4).expect("not zero")).expect("even");
    let mut packet = ParameterPacket::periodic(cycle);
    packet.within(admission_tolerance());
    let stored = water(STORED_WATER);
    packet.start(gauged.stored, stored, "synthetic").expect("once");
    packet
        .start(gauged.gauge, measure::<Rate<Water>>(1.0), "synthetic")
        .expect("once");
    let stored = Snapshot::new(
        gauged.stored,
        METHOD,
        AnchorStatistic::Instant,
        water(STORED_WATER),
    );
    packet
        .anchor(Synthetic::new(STORED, stored, "synthetic"))
        .expect("once");
    let gauge = measure::<Rate<Water>>(GAUGE_MEAN);
    let mean = Snapshot::new(gauged.gauge, METHOD, AnchorStatistic::CycleMean, gauge);
    packet
        .anchor(Synthetic::new(GAUGED, mean, "synthetic"))
        .expect("once");
    let guess = measure::<Dimensionless>(1.0);
    packet
        .calibrated(GAIN, Binding::Body, guess, GAUGED)
        .expect("once");
    packet
}

/// Water in over the cycle's first quarter and out over the whole cycle, so the vessel ends the cycle where it began
/// and the gauge reads only the first quarter.
fn first_cycle() -> BoundaryPort {
    let delivery = |id: &[u8], boundary, ends| {
        let delivered = WaterDelivery {
            boundary,
            amount: water(DELIVERED),
            ends: SimTime::from_ns(ends),
        };
        let id = InputId::new(Digest::of_canonical(id));
        BoundaryEvent::new(id, at(0), BoundaryInput::Water(delivered))
    };
    let events = vec![delivery(b"in", "in", QUARTER), delivery(b"out", "out", HOUR)];
    BoundaryPort::new(events).expect("parsed")
}

fn assert_baseline_meets_anchor<U>(returns: &ReturnReport, anchor: AnchorId, stated: f64) {
    let first = (returns.results().iter())
        .find(|result| (result.anchor(), result.morning()) == (anchor, NonZeroU32::MIN))
        .expect("morning 1 read");
    let baseline = first.admission().expect("every value present");
    let scale = admission_tolerance().at(stated, UNSCALED);
    let defect = baseline.minus(Compensated::of(&measure::<U>(stated))).abs();
    assert!(
        defect.at_most(Compensated::of(&measure::<U>(scale))),
        "{anchor:?}: {baseline:?} at admission against the packet's {stated}"
    );
}

#[test]
fn the_return_check_reads_each_anchor_at_admission_as_admission_met_it() {
    let gauged = gauged().expect("a body");
    let channels = vec![
        ChannelSpec::new(METHOD, gauged.stored),
        ChannelSpec::new(METHOD, gauged.gauge).averaged(count(4)),
    ];
    let lens = gauged.body.lens(channels, count(QUARTER)).expect("a lens");
    let core = ReferenceIntegrator::new(60 * SECOND);
    let store = MemoryStore::default();
    let admitted = admit_solved(
        &gauged.body,
        anchored(&gauged),
        first_cycle(),
        &core,
        &lens,
        &store,
    );
    let mut session = admitted.expect("admitted").session;
    let bare = BareLens::new(&gauged.body, &lens).expect("this body's lens");
    let variations = vec![
        WithinSubjectVariation::synthetic("xx.stored", gauged.stored, METHOD, water(1.0), "synthetic"),
        WithinSubjectVariation::synthetic(
            "xx.gauged",
            gauged.gauge,
            METHOD,
            measure::<Rate<Water>>(1.0),
            "synthetic",
        ),
    ];
    let variations = variations
        .into_iter()
        .map(|variation| variation.expect("a variation"));
    let set = HealthyRangeSet::synthetic(Vec::new(), variations.collect()).expect("a set");
    let anchors = anchored(&gauged).anchors();
    let mut checks =
        ReadingChecks::new(&gauged.body, &bare, &set, &day(), anchors, &session).expect("the checks");
    while session.record().accepted().time() <= SimTime::from_ns(24 * HOUR + HOUR) {
        session = step(session);
        checks.read(&session).expect("the next revision");
    }
    let (_, returns) = checks.finish();
    let bound = gauged.body.bind(anchored(&gauged)).expect("bound");
    assert_eq!(
        returns.packet(),
        bound.packet_hash(),
        "the report names the packet the run bound"
    );
    assert_baseline_meets_anchor::<Water>(&returns, STORED, STORED_WATER);
    assert_baseline_meets_anchor::<Rate<Water>>(&returns, GAUGED, GAUGE_MEAN);
    assert_eq!(
        returns.morning(NonZeroU32::MIN),
        CheckVerdict::Pass,
        "a variation of 1 allows each reading to move 2.77 from admission's: {:?}",
        returns.results()
    );
}
