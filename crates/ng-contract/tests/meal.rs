use core::num::NonZeroU64;
use ng_contract::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn id(name: &[u8]) -> InputId {
    InputId::new(Digest::of_canonical(name))
}

struct Declare<'a> {
    schema: &'a mut Schema,
    site: SiteId,
    name: &'static str,
}
impl Visit for Declare<'_> {
    type Output = Result<(), AdmissionError>;
    fn visit<Q: Substance>(self) -> Self::Output {
        if Q::SPECIES != Species::Water {
            self.schema.pool::<Q>(self.site, Space::Lumen)?;
        }
        (self.schema).boundary::<Q>(self.name, BoundaryDirection::IntoBody, self.site, Space::Lumen)
    }
}

struct Deliver<'l, 'b> {
    laws: &'l mut FragmentBuilder<'b, Boundary>,
    ports: &'l BoundaryPorts<'b>,
    name: &'static str,
}
impl Visit for Deliver<'_, '_> {
    type Output = Result<(), AdmissionError>;
    fn visit<Q: Substance>(self) -> Self::Output {
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let BoundaryRoute { route, drive } = self.ports.route::<Q>(self.name)?;
        let laws = &*self.laws;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), wave)
        });
        self.laws.transfer(Transfer::new(&route, rate));
        Ok(())
    }
}

struct Start<'a> {
    packet: &'a mut ParameterPacket<SyntheticFixture>,
    site: SiteId,
}
impl Visit for Start<'_> {
    type Output = Result<(), AdmissionError>;
    fn visit<Q: Substance>(self) -> Self::Output {
        let row = RowKind::Amount {
            site: self.site,
            space: Space::Lumen,
            species: Q::SPECIES,
        };
        self.packet.start(row, measure::<Q>(1.0), "synthetic")
    }
}

/// A gut that holds each of `species`, each delivered through its own boundary, whose law reads its drive.
fn gut(species: &[Species]) -> Result<(CompiledBody, SiteId), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let gut = map.site::<VesselSegment>("gut", origin());
    map.contain(root.id(), gut.id());
    let mut schema = Schema::new(map.finish()?)?;
    let site = gut.id();
    let named = |held: Species| {
        let place = Meal::SPECIES.iter().position(|known| *known == held);
        Meal::BOUNDARIES[place.expect("a meal species")]
    };
    for &held in species {
        let name = named(held);
        held.visit(Declare {
            schema: &mut schema,
            site,
            name,
        })?;
    }
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        for &held in species {
            let name = named(held);
            held.visit(Deliver {
                laws: &mut laws,
                ports: &ports,
                name,
            })?;
        }
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((body, site))
}

fn packet(site: SiteId, species: &[Species]) -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    for &held in species {
        let packet = &mut packet;
        held.visit(Start { packet, site }).expect("a start");
    }
    packet
}

fn resolution() -> DeclaredResolution {
    let scales = ErrorScales::uniform(measure(1.0), measure(0.0)).expect("scales");
    DeclaredResolution::new(
        Digest::of_canonical(b"meal test"),
        NonZeroU64::MIN,
        4,
        ArithmeticProfile::contract_v1(),
        StageScheme::backward_euler_doubled(),
        scales,
    )
}

struct Bench {
    body: CompiledBody,
    site: SiteId,
    parts: ImageParts,
    identity: RunIdentity,
}
impl Bench {
    fn admitted(species: &[Species], events: Vec<BoundaryEvent>) -> Result<Self, InitializationFailure> {
        let (body, site) = gut(species)?;
        let parameters = body.bind(packet(site, species))?;
        let request = InitializationRequest::at(at(0));
        let system = body.initialization(parameters, BoundaryPort::new(events)?, request)?;
        let solved = system.workspace();
        let history = MethodHistory::new(Digest::of_canonical(b"meal test"), at(0), Vec::new());
        let revision = system.admit(solved, history)?;
        let identity = RunIdentity::new(
            body.model_hash(),
            revision.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution(),
        );
        Ok(Self {
            parts: revision.into_parts(),
            body,
            site,
            identity,
        })
    }
    /// The rate into each species' pool over the first ticket.
    fn rates(&self) -> Vec<(Species, f64)> {
        let parts = &self.parts;
        let accepted = AcceptedStateView::new(parts.time, &parts.state, &parts.continuation, &parts.dense);
        let base = Digest::of_canonical(b"base");
        let problem = CoupledProblem::new(&self.body, parts.parameters(), &self.identity, accepted, base);
        let mut issued = None;
        let _ = (parts.pending).attempt(&problem, Attempt::First { span_ns: SECOND }, |ticket| {
            issued = Some(ticket.clone());
            Err::<(VerifiedStep, ()), _>(ticket.mismatch().into())
        });
        let ticket = issued.expect("a ticket");
        let mut workspace = problem.workspace();
        workspace.derivative_mut().fill(0.0);
        let stage = problem
            .evaluate(&workspace.point(ticket.at(0).expect("an offset")))
            .expect("evaluated");
        let rows = self.body.rows().iter().map(|row| match row.kind() {
            RowKind::Amount { site, species, .. } if site == self.site => Some(species),
            _ => None,
        });
        let residuals = stage.residuals().iter().map(|residual| 0.0 - residual);
        rows.zip(residuals)
            .filter_map(|(species, rate)| Some((species?, rate)))
            .collect()
    }
}

fn breakfast(ends: SimTime) -> Meal {
    Meal::new(ends)
        .with(measure::<Water>(10.0))
        .with(measure::<Sodium>(20.0))
        .with(measure::<Glucose>(30.0))
        .with(measure::<Iron>(40.0))
}

fn meal(name: &[u8], from: SimTime, meal: Meal) -> BoundaryEvent {
    BoundaryEvent::new(id(name), from, BoundaryInput::Meal(meal))
}

#[test]
fn the_meal_holds_body_01s_species_in_order_each_into_its_own_gut_boundary() {
    let species = [
        Species::Water,
        Species::Sodium,
        Species::Potassium,
        Species::Chloride,
        Species::Glucose,
        Species::Triglyceride,
        Species::AminoAcids,
        Species::DietBase,
        Species::Calcium,
        Species::Phosphate,
        Species::Magnesium,
        Species::Iron,
    ];
    assert_eq!(Meal::SPECIES, species);
    let boundaries = [
        "gut water",
        "gut sodium",
        "gut potassium",
        "gut chloride",
        "gut glucose",
        "gut triglyceride",
        "gut amino acids",
        "gut diet base",
        "gut calcium",
        "gut phosphate",
        "gut magnesium",
        "gut iron",
    ];
    assert_eq!(Meal::BOUNDARIES, boundaries);
}

#[test]
fn a_meal_lands_as_one_delivery_for_each_species_above_zero() -> Result<(), InitializationFailure> {
    let bench = Bench::admitted(&Meal::SPECIES, vec![meal(b"breakfast", at(0), breakfast(at(10)))])?;
    let rates = bench.rates();
    let fed = [
        (Species::Water, 1.0),
        (Species::Sodium, 2.0),
        (Species::Glucose, 3.0),
        (Species::Iron, 4.0),
    ];
    assert_eq!(rates.len(), Meal::SPECIES.len());
    for (species, rate) in rates {
        let expected = fed.iter().find(|fed| fed.0 == species).map_or(0.0, |fed| fed.1);
        assert_eq!(rate, expected, "{species:?}");
    }
    Ok(())
}

#[test]
fn a_meal_sent_twice_lands_once_and_two_meals_at_one_instant_both_land() -> Result<(), InitializationFailure>
{
    let twice = vec![
        meal(b"breakfast", at(0), breakfast(at(10))),
        meal(b"breakfast", at(0), breakfast(at(10))),
    ];
    let once = vec![meal(b"breakfast", at(0), breakfast(at(10)))];
    assert_eq!(
        BoundaryPort::new(twice)?.digest(),
        BoundaryPort::new(once)?.digest()
    );
    let both = vec![
        meal(b"breakfast", at(0), breakfast(at(10))),
        meal(b"second helping", at(0), breakfast(at(10))),
    ];
    let bench = Bench::admitted(&Meal::SPECIES, both)?;
    let water = bench.rates().into_iter().find(|rate| rate.0 == Species::Water);
    assert_eq!(water, Some((Species::Water, 2.0)));
    Ok(())
}

#[test]
fn one_id_with_another_meal_is_a_conflict_even_when_it_holds_less() -> Result<(), AdmissionError> {
    let less = Meal::new(at(10))
        .with(measure::<Water>(10.0))
        .with(measure::<Sodium>(20.0))
        .with(measure::<Glucose>(30.0));
    let more = breakfast(at(10)).with(measure::<Calcium>(1.0));
    let conflict = Some(AdmissionError::InputConflict { id: id(b"breakfast") });
    for other in [less, more, breakfast(at(11))] {
        let events = vec![
            meal(b"breakfast", at(0), breakfast(at(10))),
            meal(b"breakfast", at(0), other),
        ];
        assert_eq!(BoundaryPort::new(events).err(), conflict);
    }
    Ok(())
}

#[test]
fn a_meal_any_part_of_which_would_be_refused_is_refused_whole() {
    let breakfast_id = id(b"breakfast");
    let refusals = [
        (
            breakfast(at(10)).with(measure::<Calcium>(-1.0)),
            AdmissionError::NegativeDelivery { id: breakfast_id },
        ),
        (
            Meal::new(at(10)),
            AdmissionError::EmptyDelivery { id: breakfast_id },
        ),
        (
            breakfast(at(0)),
            AdmissionError::EmptyDelivery { id: breakfast_id },
        ),
        (
            breakfast(SimTime::from_ns(1)).with(measure::<Calcium>(f64::MAX)),
            AdmissionError::UnboundedDelivery { id: breakfast_id },
        ),
    ];
    for (refused, error) in refusals {
        let events = vec![meal(b"breakfast", at(0), refused)];
        assert_eq!(BoundaryPort::new(events).err(), Some(error));
    }
}

#[test]
fn a_body_without_a_boundary_the_meal_needs_refuses_it_at_admission() {
    let all_but_iron = &Meal::SPECIES[..11];
    let refused = Bench::admitted(all_but_iron, vec![meal(b"breakfast", at(0), breakfast(at(10)))]);
    let unknown = AdmissionError::UnknownBoundary { name: "gut iron" };
    assert_eq!(refused.err(), Some(unknown.into()));
}

#[test]
fn a_late_meal_lands_whole_or_not_at_all_and_once() -> Result<(), InitializationFailure> {
    let all_but_iron = &Meal::SPECIES[..11];
    let mut bench = Bench::admitted(all_but_iron, Vec::new())?;
    let head = bench.parts.time;
    let before = bench.parts.pending.waiting_digest();
    let late = |name: &[u8], meal: Meal| BoundaryEvent::new(id(name), at(5), BoundaryInput::Meal(meal));
    let refused = (bench.parts.pending).admit(&bench.body, head, late(b"breakfast", breakfast(at(10))));
    let unknown = AdmissionError::UnknownBoundary { name: "gut iron" };
    assert_eq!(refused, Err(unknown));
    assert_eq!(bench.parts.pending.waiting_digest(), before, "no part waits");
    let lunch = || {
        Meal::new(at(10))
            .with(measure::<Water>(10.0))
            .with(measure::<Glucose>(5.0))
    };
    let first = (bench.parts.pending).admit(&bench.body, head, late(b"lunch", lunch()));
    let waiting = bench.parts.pending.waiting_digest();
    let again = (bench.parts.pending).admit(&bench.body, head, late(b"lunch", lunch()));
    assert!(first.is_ok());
    assert_eq!(again, first);
    assert_eq!(bench.parts.pending.waiting_digest(), waiting, "lands once");
    assert_ne!(waiting, before);
    let other = (bench.parts.pending).admit(
        &bench.body,
        head,
        late(b"lunch", lunch().with(measure::<Iron>(0.0))),
    );
    assert_eq!(other, first, "a zero amount is no part");
    let other = lunch().with(measure::<Sodium>(1.0));
    let conflict = AdmissionError::InputConflict { id: id(b"lunch") };
    assert_eq!(
        (bench.parts.pending).admit(&bench.body, head, late(b"lunch", other)),
        Err(conflict)
    );
    let fence = bench.parts.pending.earliest_stamp(head);
    let early = BoundaryEvent::new(id(b"snack"), head, BoundaryInput::Meal(lunch()));
    let refused = (bench.parts.pending).admit(&bench.body, head, early);
    assert_eq!(
        refused,
        Err(AdmissionError::EventBeforeFence {
            id: id(b"snack"),
            fence
        })
    );
    Ok(())
}

/// Where a part's boundary name starts, as its length: its tag is the 8 bytes before.
fn tag_of(bytes: &[u8], name: &str) -> u64 {
    let mut text = (name.len() as u64).to_be_bytes().to_vec();
    text.extend_from_slice(name.as_bytes());
    let found: Vec<usize> = (0..bytes.len() - text.len())
        .filter(|&at| bytes[at..at + text.len()] == text[..])
        .collect();
    assert_eq!(found.len(), 1, "{name} once");
    u64::from_be_bytes(*bytes[found[0] - 8..].first_chunk().expect("a tag"))
}

#[test]
fn every_meal_species_reads_back_under_its_own_tag() -> Result<(), InitializationFailure> {
    let mut full = Meal::new(at(10));
    full = full
        .with(measure::<Water>(1.0))
        .with(measure::<Sodium>(1.0))
        .with(measure::<Potassium>(1.0))
        .with(measure::<Chloride>(1.0))
        .with(measure::<Glucose>(1.0))
        .with(measure::<Triglyceride>(1.0))
        .with(measure::<AminoAcids>(1.0))
        .with(measure::<DietBase>(1.0))
        .with(measure::<Calcium>(1.0))
        .with(measure::<Phosphate>(1.0))
        .with(measure::<Magnesium>(1.0))
        .with(measure::<Iron>(1.0));
    let bench = Bench::admitted(&Meal::SPECIES, vec![meal(b"feast", at(0), full)])?;
    let bytes = bench.parts.canonical();
    let read = ImageParts::parse(&bench.body, &bytes).expect("parsed");
    assert_eq!(read.canonical(), bytes);
    assert_eq!(
        read.pending.waiting_digest(),
        bench.parts.pending.waiting_digest()
    );
    let tags: Vec<u64> = Meal::BOUNDARIES.iter().map(|name| tag_of(&bytes, name)).collect();
    assert_eq!(tags, [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12]);
    Ok(())
}

/// A delivery of the packet's day that ran before the anchor moment and ended by it is dropped; one that runs
/// across it is refused by its id, and a meal by the meal's.
#[test]
fn a_day_delivery_across_the_anchor_moment_is_refused_and_one_that_ended_is_dropped()
-> Result<(), AdmissionError> {
    let species = [Species::Water];
    let (body, site) = gut(&species)?;
    let admitted = |event: BoundaryEvent| -> Result<(), AdmissionError> {
        let mut packet = packet(site, &species);
        packet.day(Day::new(vec![event])?)?;
        let parameters = body.bind(packet)?;
        let request = InitializationRequest::at(at(60));
        body.initialization(parameters, BoundaryPort::new(Vec::new())?, request)?;
        Ok(())
    };
    let drink = |ends| {
        let delivered = Delivered {
            boundary: "gut water",
            amount: measure(1.0),
            ends: at(ends),
        };
        BoundaryEvent::new(id(b"drink"), at(0), BoundaryInput::Water(delivered))
    };
    assert_eq!(admitted(drink(60)), Ok(()), "ended at the moment");
    let across = AdmissionError::DeliveryAcrossAdmission { id: id(b"drink") };
    assert_eq!(admitted(drink(61)), Err(across));
    let lunch = Meal::new(at(90)).with(measure::<Water>(5.0));
    let meal = BoundaryEvent::new(id(b"lunch"), at(30), BoundaryInput::Meal(lunch));
    let across = AdmissionError::DeliveryAcrossAdmission { id: id(b"lunch") };
    assert_eq!(admitted(meal), Err(across));
    Ok(())
}
