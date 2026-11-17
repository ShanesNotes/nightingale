use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{Curve, Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const SYNTHETIC: &str = "bench: synthetic";
const END_S: u64 = 1_500;
const FROM_S: u64 = 1_200;

const GLUCOSE_ENERGY: f64 = 1_000.0;
const DEMAND: f64 = 0.01;
const OXYGEN_HALF: f64 = 0.5;
const BED_HEAT_SPEED: f64 = 0.05;
const SHED_SPEED: f64 = 0.1;
const GLUCOSE_SUPPLY: f64 = 0.06;
const OXYGEN_SUPPLY: f64 = 0.2;
const OUT_SPEED: f64 = 0.01;
const HEAT_SPEED: f64 = 0.05;
const WORK: f64 = 2.0;
const PAST_THE_BURN: f64 = 40.0;

#[derive(Clone, Copy, Debug)]
struct Variant {
    paid: bool,
    work: f64,
    raised: bool,
}

struct Built {
    body: CompiledBody,
    bed: SiteId,
    root: SiteId,
}

fn origin() -> Position {
    let zero = || Measure::new(0.0).expect("finite");
    Position::new(zero(), zero(), zero())
}

fn key(local: &'static str) -> ParameterKey {
    ParameterKey::new(local)
}

fn built(variant: Variant) -> Result<Built, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let bed = map.site::<CapillaryBed>("muscle bed", origin()).id();
    map.contain(root.id(), bed);
    map.connect(Network::Vascular, vessel.id(), bed);
    map.connect(Network::Vascular, bed, vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Plasma, "the test burn draws on the interstitium alone");
    schema.leave_out(Space::Cells, "the test burn draws on the interstitium alone");
    schema.leave_out(Space::Lumen, "no blood moves in this bench");
    schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
    schema.carry::<CarbonDioxide, CapillaryBed>(Space::Interstitium);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.pool::<Energy>(root.id(), Space::Elastic)?;
    schema.pool::<Energy>(root.id(), Space::Thermal)?;
    schema.energy_content::<Glucose>(Measure::new(GLUCOSE_ENERGY)?)?;
    if variant.paid {
        schema.pay_work(Work::Walls, bed)?;
    }
    let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
    schema.boundary::<Glucose>("glucose in", into, bed, Space::Interstitium)?;
    schema.boundary::<Glucose>("glucose out", out, bed, Space::Interstitium)?;
    schema.boundary::<Oxygen>("oxygen in", into, bed, Space::Interstitium)?;
    schema.boundary::<Oxygen>("oxygen out", out, bed, Space::Interstitium)?;
    schema.boundary::<CarbonDioxide>("carbon dioxide out", out, bed, Space::Interstitium)?;
    schema.boundary::<Water>("water out", out, bed, Space::Interstitium)?;
    schema.boundary::<Energy>("heat", out, root.id(), Space::Thermal)?;
    let class = RateClass::new(RateBand::Minutes, 10 * SECOND)?;
    let body = assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        test_burn(&mut mt, class);
        let route = ports.bed_heat(&mut mt);
        let stored = route.from_amount::<Energy>();
        let speed = mt.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
            key("mt.test-bed-heat-speed"),
            Admissible::nonnegative(),
        );
        let passing = mt.site_kernel(|mut k| {
            let (held, zero, speed) = (k.at_site(&stored), k.zero(), k.site_parameter(&speed));
            let value = k.relax(held, zero, speed);
            k.finish(value, mt.every_mode(), class)
        });
        mt.transfer_at(&route, passing);
        body.add(mt.finish());

        let (mut cv, ports) = body.cardiovascular()?.parts();
        let work = cv.parameter::<Rate<Energy>>(key("cv.test-wall-work"), Admissible::nonnegative());
        let paid = ports.wall_work(&mut cv);
        let working = cv.kernel(|mut k| {
            let value = k.parameter(&work);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&paid, working));
        let shed = cv.parameter::<Rate<Dimensionless>>(key("cv.test-shed-speed"), Admissible::positive());
        let store = ports.dissipation(&mut cv);
        let shedding = cv.kernel(|mut k| {
            let (held, zero, speed) = (k.read(&store.amount::<Energy>()), k.zero(), k.parameter(&shed));
            let value = k.relax(held, zero, speed);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&store, shedding));
        body.add(cv.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let glucose = laws.parameter::<Rate<Glucose>>(key("bench.glucose-supply"), Admissible::nonnegative());
        let oxygen = laws.parameter::<Rate<Oxygen>>(key("bench.oxygen-supply"), Admissible::nonnegative());
        let out = laws.parameter::<Rate<Dimensionless>>(key("bench.out-speed"), Admissible::nonnegative());
        let heat = laws.parameter::<Rate<Dimensionless>>(key("bench.heat-speed"), Admissible::nonnegative());
        supplied(&mut laws, &ports, "glucose in", &glucose, class)?;
        supplied(&mut laws, &ports, "oxygen in", &oxygen, class)?;
        cleared::<Glucose>(&mut laws, &ports, "glucose out", &out, class)?;
        cleared::<Oxygen>(&mut laws, &ports, "oxygen out", &out, class)?;
        cleared::<CarbonDioxide>(&mut laws, &ports, "carbon dioxide out", &out, class)?;
        cleared::<Water>(&mut laws, &ports, "water out", &out, class)?;
        cleared::<Energy>(&mut laws, &ports, "heat", &heat, class)?;
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(Built {
        body,
        bed,
        root: root.id(),
    })
}

/// Not the body's burn: `mt.test-work-read` stands in for metabolism's read of the work.
fn test_burn<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, class: RateClass) {
    let burn = mt.reaction::<CapillaryBed, Glucose, _>(
        "test burn",
        Proportions::<Glucose>::using(Space::Interstitium)
            .uses::<Oxygen, 6>(Space::Interstitium)
            .makes::<CarbonDioxide, 6>(Space::Interstitium)
            .makes::<Water, 6>(Space::Interstitium)
            .energy_into(Space::Thermal),
    );
    let demand =
        mt.site_parameter::<CapillaryBed, Rate<Glucose>>(key("mt.test-demand"), Admissible::nonnegative());
    let read =
        mt.site_parameter::<CapillaryBed, Rate<Glucose>>(key("mt.test-work-read"), Admissible::nonnegative());
    let half = mt.site_parameter::<CapillaryBed, Oxygen>(key("mt.test-oxygen-half"), Admissible::positive());
    let oxygen = burn.amount::<Oxygen>();
    let rate = mt.site_kernel(|mut k| {
        let (demand, read, half) = (
            k.site_parameter(&demand),
            k.site_parameter(&read),
            k.site_parameter(&half),
        );
        let total = k.add(demand, read);
        let (oxygen, held) = (k.at_site(&oxygen), k.at_site(&oxygen));
        let reach = k.add(held, half);
        let share = k.divide(oxygen, reach);
        let share = k.as_dimension::<_, Dimensionless>(share);
        let value = k.scale(total, share);
        k.finish(value, mt.every_mode(), class)
    });
    mt.react_at(&burn, rate);
}

fn supplied<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    supply: &Parameter<'b, Boundary, Rate<Q>>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, .. } = ports.route::<Q>(name)?;
    let rate = laws.kernel(|mut k| {
        let rate = k.parameter(supply);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

fn cleared<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    speed: &Parameter<'b, Boundary, Rate<Dimensionless>>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, .. } = ports.route::<Q>(name)?;
    let held = route.amount::<Q>();
    let rate = laws.kernel(|mut k| {
        let (held, zero, speed) = (k.read(&held), k.zero(), k.parameter(speed));
        let rate = k.relax(held, zero, speed);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

fn packet(built: &Built, variant: Variant) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::new();
    let (body, beds) = (Binding::Body, Binding::Class(Class::CapillaryBed));
    let read = if variant.raised {
        variant.work / GLUCOSE_ENERGY
    } else {
        0.0
    };
    packet.synthetic(
        key("mt.test-demand"),
        beds,
        Measure::<Rate<Glucose>>::new(DEMAND)?,
        SYNTHETIC,
    )?;
    packet.synthetic(
        key("mt.test-work-read"),
        beds,
        Measure::<Rate<Glucose>>::new(read)?,
        SYNTHETIC,
    )?;
    packet.synthetic(
        key("mt.test-oxygen-half"),
        beds,
        Measure::<Oxygen>::new(OXYGEN_HALF)?,
        SYNTHETIC,
    )?;
    let speed = Measure::<Rate<Dimensionless>>::new(BED_HEAT_SPEED)?;
    packet.synthetic(key("mt.test-bed-heat-speed"), beds, speed, SYNTHETIC)?;
    packet.synthetic(
        key("cv.test-wall-work"),
        body,
        Measure::<Rate<Energy>>::new(variant.work)?,
        SYNTHETIC,
    )?;
    let shed = Measure::<Rate<Dimensionless>>::new(SHED_SPEED)?;
    packet.synthetic(key("cv.test-shed-speed"), body, shed, SYNTHETIC)?;
    let supply = Measure::<Rate<Glucose>>::new(GLUCOSE_SUPPLY)?;
    packet.synthetic(key("bench.glucose-supply"), body, supply, SYNTHETIC)?;
    let supply = Measure::<Rate<Oxygen>>::new(OXYGEN_SUPPLY)?;
    packet.synthetic(key("bench.oxygen-supply"), body, supply, SYNTHETIC)?;
    packet.synthetic(
        key("bench.out-speed"),
        body,
        Measure::<Rate<Dimensionless>>::new(OUT_SPEED)?,
        SYNTHETIC,
    )?;
    packet.synthetic(
        key("bench.heat-speed"),
        body,
        Measure::<Rate<Dimensionless>>::new(HEAT_SPEED)?,
        SYNTHETIC,
    )?;
    let (bed, root) = (built.bed, built.root);
    let amount = |site, space, species| RowKind::Amount { site, space, species };
    let at = |species| amount(bed, Space::Interstitium, species);
    packet.start(at(Species::Water), Measure::<Water>::new(1_000.0)?, SYNTHETIC)?;
    packet.start(at(Species::Glucose), Measure::<Glucose>::new(1.0)?, SYNTHETIC)?;
    packet.start(at(Species::Oxygen), Measure::<Oxygen>::new(10.0)?, SYNTHETIC)?;
    packet.start(
        at(Species::CarbonDioxide),
        Measure::<CarbonDioxide>::new(6.0)?,
        SYNTHETIC,
    )?;
    packet.start(
        amount(bed, Space::Thermal, Species::Energy),
        Measure::<Energy>::new(150.0)?,
        SYNTHETIC,
    )?;
    packet.start(
        amount(root, Space::Elastic, Species::Energy),
        Measure::<Energy>::new(10.0)?,
        SYNTHETIC,
    )?;
    packet.start(
        amount(root, Space::Thermal, Species::Energy),
        Measure::<Energy>::new(200.0)?,
        SYNTHETIC,
    )?;
    Ok(packet)
}

fn certificate(body: &CompiledBody) -> Result<CertificatePolicy, AdmissionError> {
    let windows = SlidingWindows::new(NonZeroU64::new(60 * SECOND).expect("positive"), END_S * SECOND)
        .expect("windows");
    let basis = body.conserved_basis();
    let quantities = basis.ids().map(|quantity| {
        let (floor, relative) = (Measure::new(1e-9)?, Measure::new(1e-12)?);
        match quantity.book() {
            Book::Element(_) => QuantityPolicy::element(quantity, floor, relative, Measure::new(1e4)?),
            Book::Energy | Book::Of(Species::Energy) => QuantityPolicy::new(
                quantity,
                Measure::<Energy>::new(1e-9)?,
                relative,
                Measure::<Energy>::new(1e4)?,
            ),
            Book::Of(Species::Water) => QuantityPolicy::new(
                quantity,
                Measure::<Water>::new(1e-9)?,
                relative,
                Measure::<Water>::new(1e4)?,
            ),
            Book::Of(Species::Glucose) => QuantityPolicy::new(
                quantity,
                Measure::<Glucose>::new(1e-9)?,
                relative,
                Measure::<Glucose>::new(1e4)?,
            ),
            Book::Of(Species::Oxygen) => QuantityPolicy::new(
                quantity,
                Measure::<Oxygen>::new(1e-9)?,
                relative,
                Measure::<Oxygen>::new(1e4)?,
            ),
            Book::Of(Species::CarbonDioxide) => QuantityPolicy::new(
                quantity,
                Measure::<CarbonDioxide>::new(1e-9)?,
                relative,
                Measure::<CarbonDioxide>::new(1e4)?,
            ),
            Book::Of(_) | Book::Charge => Err(AdmissionError::QuantityDimension { quantity }),
        }
    });
    let quantities = quantities.collect::<Result<Vec<_>, _>>()?;
    CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
}

fn lead(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn extents(body: &CompiledBody, session: &Session<'_, Running>, at: u64) -> Vec<f64> {
    let mut columns: Vec<ColumnId> = (body.instances().iter())
        .filter_map(|instance| match instance.target() {
            Target::Column(column) => Some(column),
            Target::Row(_) => None,
        })
        .collect();
    columns.sort();
    columns.dedup();
    for segment in session.conservation().newest_first() {
        let segment = segment.expect("a segment");
        let interval = segment.interval();
        if interval.start().nanoseconds() < at && at <= interval.end().nanoseconds() {
            let piece = Interval::new(interval.start(), SimTime::from_ns(at)).expect("a piece");
            let reached = |curve: Curve<'_>| lead(curve.on(piece).expect("inside its segment")[1]);
            let mut extent = vec![0.0; body.columns().len()];
            for column in columns {
                let (forward, reverse) = segment.exchange(column);
                extent[column.index()] = reached(forward) - reached(reverse);
            }
            return extent;
        }
    }
    panic!("no segment holds {at}")
}

fn column(body: &CompiledBody, local: &str) -> usize {
    (body.columns().iter())
        .position(|column| column.origin().local == local)
        .unwrap_or_else(|| panic!("a {local} column"))
}

#[derive(Debug)]
struct Settled {
    mean_burn_energy: f64,
    mean_to_core: f64,
    bed_heat_at_end: f64,
    receipt_over_run: f64,
}

#[derive(Debug)]
struct Faulted {
    kind: EngineFaultKind,
    refused: Option<RowKind>,
    lowest: f64,
}

fn ran(variant: Variant) -> Result<Settled, Faulted> {
    let built = built(variant).unwrap_or_else(|refused| panic!("{variant:?} refused: {refused:?}"));
    let body = &built.body;
    let (store, core) = (MemoryStore::default(), ReferenceIntegrator::new(10 * SECOND));
    let packet = packet(&built, variant).expect("a packet");
    let session = admit_fixture(
        body,
        packet,
        BoundaryPort::new(Vec::new()).expect("no event"),
        &core,
        &store,
    )
    .unwrap_or_else(|refused| panic!("{variant:?} not admitted: {refused:?}"));
    let amounts = || (body.rows().iter()).filter(|row| matches!(row.kind(), RowKind::Amount { .. }));
    let session = match run_until(session, SimTime::from_ns(END_S * SECOND)) {
        Ok(session) => session,
        Err(faulted) => {
            let refused = match faulted.fault().last_refusal() {
                Some(NumericalRefusal::TrialDomain { source }) => {
                    match body.refusal_sources()[source.index()] {
                        RefusalSource::Row { row } => Some(body.rows()[row.index()].kind()),
                        _ => None,
                    }
                }
                _ => None,
            };
            let view = faulted.record().accepted();
            let lowest = amounts()
                .map(|row| lead(view.value(row.id())))
                .fold(f64::INFINITY, |low, held| if held < low { held } else { low });
            return Err(Faulted {
                kind: faulted.fault().kind(),
                refused,
                lowest,
            });
        }
    };
    certify(&session.conservation(), &certificate(body).expect("a policy")).unwrap_or_else(|failed| {
        panic!(
            "{variant:?} uncertified: {:?} {:?}",
            failed.check(),
            failed.quantity()
        )
    });
    let (from, to) = (
        extents(body, &session, FROM_S * SECOND),
        extents(body, &session, END_S * SECOND),
    );
    let rate = |local| {
        let at = column(body, local);
        (to[at] - from[at]) / (END_S - FROM_S) as f64
    };
    let receipt_over_run = (body.columns().iter().enumerate())
        .filter(|(_, column)| {
            column
                .receipt()
                .is_some_and(|receipt| receipt.boundary() == "chemical energy")
        })
        .map(|(at, _)| to[at])
        .sum();
    let view = session.record().accepted();
    let bed_heat_at_end = amounts()
        .find(|row| {
            row.kind()
                == RowKind::Amount {
                    site: built.bed,
                    space: Space::Thermal,
                    species: Species::Energy,
                }
        })
        .map(|row| lead(view.value(row.id())))
        .expect("the bed's heat");
    Ok(Settled {
        mean_burn_energy: rate("test burn") * GLUCOSE_ENERGY,
        mean_to_core: rate("bed heat"),
        bed_heat_at_end,
        receipt_over_run,
    })
}

fn settled(variant: Variant) -> Settled {
    ran(variant).unwrap_or_else(|faulted| panic!("{variant:?} faulted: {faulted:?}"))
}

#[test]
fn with_the_burn_raised_by_the_work_the_beds_heat_passes_the_burn_less_the_work_to_the_core_store() {
    for work in [WORK, 2.0 * WORK] {
        let paid = settled(Variant {
            paid: true,
            work,
            raised: true,
        });
        let less = paid.mean_burn_energy - work;
        assert!((paid.mean_to_core - less).abs() <= 1e-6 * less, "{work}: {paid:?}");
        assert_eq!(paid.receipt_over_run, 0.0, "{work}: {paid:?}");
    }
}

#[test]
fn with_the_burn_not_raised_the_beds_heat_settles_lower_by_the_work_over_the_heats_speed() {
    let receipt = settled(Variant {
        paid: false,
        work: WORK,
        raised: false,
    });
    let taken = receipt.receipt_over_run;
    assert!((taken - WORK * END_S as f64).abs() <= 1e-9 * taken, "{receipt:?}");
    for work in [WORK, 2.0 * WORK] {
        let paid = settled(Variant {
            paid: true,
            work,
            raised: false,
        });
        let lower = receipt.bed_heat_at_end - work / BED_HEAT_SPEED;
        assert!((paid.bed_heat_at_end - lower).abs() <= 1e-6 * lower, "{work}: {paid:?} against {lower}");
        assert_eq!(paid.receipt_over_run, 0.0, "{work}: {paid:?}");
    }
}

#[test]
fn work_past_what_the_bed_can_burn_is_refused_at_the_beds_heat_with_no_store_below_zero() {
    let variant = Variant {
        paid: true,
        work: PAST_THE_BURN,
        raised: true,
    };
    let faulted = match ran(variant) {
        Err(faulted) => faulted,
        Ok(settled) => panic!("{variant:?} settled: {settled:?}"),
    };
    let bed = built(variant).expect("a body").bed;
    assert_eq!(faulted.kind, EngineFaultKind::NoAdmissibleStep, "{faulted:?}");
    let heat = RowKind::Amount {
        site: bed,
        space: Space::Thermal,
        species: Species::Energy,
    };
    assert_eq!(faulted.refused, Some(heat), "{faulted:?}");
    assert!(faulted.lowest >= 0.0, "a store at {}", faulted.lowest);
}
