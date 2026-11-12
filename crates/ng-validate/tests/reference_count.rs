use core::num::NonZeroU64;
use expect_test::expect;
use ng_contract::*;
use ng_record::{Running, Session, Step};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const RUN_SECONDS: u64 = 600;
const SYNTHETIC: &str = "bench: synthetic";
const RATE: &str = "mt.rate";
const PARTNER_RATE: &str = "mt.partner-rate";
const REACTION: LawName = LawName {
    owner: <Metabolism as Owner>::CARD,
    local: "reaction",
};
fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn number(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn at_origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn burn() -> Proportions<Glucose> {
    Proportions::<Glucose>::using(Space::Cells)
        .uses::<Oxygen, 6>(Space::Cells)
        .makes::<CarbonDioxide, 6>(Space::Cells)
        .makes::<Water, 6>(Space::Cells)
        .energy_into(Space::Thermal)
}

fn burn_twice() -> Proportions<Glucose, 2> {
    Proportions::<Glucose, 2>::using(Space::Cells)
        .uses::<Oxygen, 12>(Space::Cells)
        .makes::<CarbonDioxide, 12>(Space::Cells)
        .makes::<Water, 12>(Space::Cells)
        .energy_into(Space::Thermal)
}

fn paced<Q: Reactant, const N: u32>(
    laws: &mut FragmentBuilder<'_, Metabolism>,
    name: &'static str,
    proportions: Proportions<Q, N>,
    key: &'static str,
    range: Option<ValidityRange<Rate<Q>>>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Minutes, 10 * SECOND)?;
    let reaction = laws.reaction::<CapillaryBed, Q, N>(name, proportions);
    let rate =
        laws.site_parameter::<CapillaryBed, Rate<Q>>(ParameterKey::new(key), Admissible::nonnegative());
    let kernel = laws.site_kernel(|mut k| {
        let rate = k.site_parameter(&rate);
        k.finish(rate, laws.every_mode(), class)
    });
    if let Some(range) = range {
        laws.validity(&kernel, range);
    }
    laws.react_at(&reaction, kernel);
    Ok(())
}

fn bed<Q: Reactant, const N: u32>(
    carry: impl FnOnce(&mut Schema),
    proportions: Proportions<Q, N>,
    range: Option<ValidityRange<Rate<Q>>>,
    partnered: bool,
) -> Result<(CompiledBody, SiteId), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at_origin()?).id();
    let vessel = map.site::<VesselSegment>("vessel", at_origin()?).id();
    let bed = map.site::<CapillaryBed>("bed", at_origin()?).id();
    map.contain(root, vessel);
    map.contain(root, bed);
    map.connect(Network::Vascular, vessel, bed);
    map.connect(Network::Vascular, bed, vessel);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Lumen] {
        schema.leave_out(space, "the bench reacts in the cells alone");
    }
    schema.carry::<Oxygen, CapillaryBed>(Space::Cells);
    schema.carry::<CarbonDioxide, CapillaryBed>(Space::Cells);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    carry(&mut schema);
    schema.energy_content::<Glucose>(Measure::new(1_000.0)?)?;
    schema.energy_content::<Glycogen>(Measure::new(900.0)?)?;
    schema.energy_content::<AminoAcids>(Measure::new(500.0)?)?;
    schema.energy_content::<Urea>(Measure::new(200.0)?)?;
    schema.energy_content::<BodyProtein>(Measure::new(450.0)?)?;
    schema.energy_content::<Creatinine>(Measure::new(700.0)?)?;
    schema.energy_content::<Lactate>(Measure::new(400.0)?)?;
    let body = assemble(schema, |mut body| {
        let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), SYNTHETIC);
        let (mut laws, _) = body.hold(held)?.parts();
        paced(&mut laws, REACTION.local, proportions, RATE, range)?;
        if partnered {
            paced(&mut laws, "partner", burn(), PARTNER_RATE, None)?;
        }
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((body, bed))
}

fn policy(body: &CompiledBody, windows: SlidingWindows) -> Result<CertificatePolicy, AdmissionError> {
    let basis = body.conserved_basis();
    let quantities = basis.ids().map(|quantity| match quantity.book() {
        Book::Element(_) => QuantityPolicy::element(
            quantity,
            Measure::new(1e-9)?,
            Measure::new(1e-12)?,
            Measure::new(1e4)?,
        ),
        Book::Energy => QuantityPolicy::new(
            quantity,
            Measure::<Energy>::new(1e-9)?,
            Measure::new(1e-12)?,
            Measure::<Energy>::new(1e4)?,
        ),
        other => panic!("a quantity this bench does not hold: {other:?}"),
    });
    let quantities = quantities.collect::<Result<Vec<_>, _>>()?;
    CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
}

fn within_rounding(got: f64, wanted: f64, read: &[f64]) -> bool {
    let scale: f64 = (read.iter().chain([&got, &wanted]))
        .map(|value| value.abs())
        .sum();
    (got - wanted).abs() <= 2.0 * f64::EPSILON * scale
}

struct Run {
    amounts: Vec<(Species, f64, f64)>,
    turnovers: f64,
    books: Vec<Book>,
}
impl Run {
    fn amount(&self, species: Species) -> (f64, f64) {
        (self.amounts.iter())
            .find(|amount| amount.0 == species)
            .map(|&(_, was, now)| (was, now))
            .expect("a pool at the bed")
    }
    fn moved(&self, species: Species) -> f64 {
        let (was, now) = self.amount(species);
        now - was
    }
    fn moves(&self, counts: &[(Species, i64)], turnovers: f64) {
        assert!(
            within_rounding(self.turnovers, turnovers, &[]),
            "the record's extent is {} turnovers, the rate's integral over the reference's count {turnovers}",
            self.turnovers
        );
        for &(species, count) in counts {
            let ((was, now), wanted) = (self.amount(species), count as f64 * turnovers);
            assert!(
                within_rounding(now - was, wanted, &[was, now]),
                "{species:?} moved {}, {count} a turnover is {wanted}",
                now - was
            );
        }
    }
}

fn packet<Q: Reactant>(body: &CompiledBody, rate: f64, partnered: bool) -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    let beds = Binding::Class(Class::CapillaryBed);
    let speed = Measure::<Rate<Q>>::new(rate).expect("finite");
    (packet.synthetic(ParameterKey::new(RATE), beds, speed, SYNTHETIC)).expect("once");
    if partnered {
        let still = Measure::<Rate<Glucose>>::new(0.0).expect("finite");
        (packet.synthetic(ParameterKey::new(PARTNER_RATE), beds, still, SYNTHETIC)).expect("once");
    }
    (packet.start_rest(body, |_| 1_000.0, SYNTHETIC)).expect("a start for each row");
    packet
}

fn run<Q: Reactant>((body, bed): &(CompiledBody, SiteId), rate: f64, partnered: bool) -> Run {
    let packet = packet::<Q>(body, rate, partnered);
    let (store, core) = (MemoryStore::default(), ReferenceIntegrator::new(10 * SECOND));
    let port = BoundaryPort::new(Vec::new()).expect("an empty port");
    let session = admit_fixture(body, packet, port, &core, &store).expect("admitted at an instant");
    let held = |session: &Session<'_, Running>| -> Vec<(Species, f64)> {
        (body.rows().iter())
            .filter_map(|row| match row.kind() {
                RowKind::Amount { site, species, .. } if site == *bed => {
                    Some((species, number(session.record().accepted().value(row.id()))))
                }
                _ => None,
            })
            .collect()
    };
    let before = held(&session);
    let end = SimTime::from_ns(RUN_SECONDS * SECOND);
    let Ok(session) = run_until(session, end) else {
        panic!("the run faulted");
    };
    let after = held(&session);
    let view = session.conservation();
    let window = NonZeroU64::new(60 * SECOND).expect("positive");
    let windows = SlidingWindows::new(window, end.nanoseconds()).expect("a coverage");
    let certificate =
        certify(&view, &policy(body, windows).expect("a policy")).expect("every quantity closes");
    let mut books: Vec<Book> = (certificate.results().iter())
        .map(|result| result.quantity().book())
        .collect();
    books.sort_unstable();
    let column = (body.instances().iter())
        .find_map(|instance| match instance.target() {
            Target::Column(column) if body.columns()[column.index()].origin() == REACTION => Some(column),
            Target::Column(_) | Target::Row(_) => None,
        })
        .expect("the reaction's column");
    let head = view.head().expect("a step");
    let (forward, reverse) = head.exchange(column);
    let at_end = |curve: &[Compensated]| number(curve[1]);
    let turnovers = at_end(&forward.on(head.interval()).expect("the head's own interval"))
        - at_end(&reverse.on(head.interval()).expect("the head's own interval"));
    let amounts = (before.iter().zip(&after))
        .map(|(&(species, was), &(_, now))| (species, was, now))
        .collect();
    Run {
        amounts,
        turnovers,
        books,
    }
}

fn integral(rate: f64) -> f64 {
    rate * RUN_SECONDS as f64
}

fn books(elements: &[Element]) -> Vec<Book> {
    let mut books: Vec<Book> = elements.iter().map(|&element| Book::Element(element)).collect();
    books.push(Book::Energy);
    books.sort_unstable();
    books
}

#[test]
fn glycogen_stored_from_twenty_glucose_moves_glucose_by_its_rate_and_every_quantity_closes() {
    let stored = Proportions::<Glucose, 20>::using(Space::Cells)
        .uses::<Oxygen, 6>(Space::Cells)
        .makes::<Glycogen, 19>(Space::Cells)
        .makes::<CarbonDioxide, 6>(Space::Cells)
        .makes::<Water, 25>(Space::Cells)
        .energy_into(Space::Thermal);
    let carry = |schema: &mut Schema| {
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
        schema.carry::<Glycogen, CapillaryBed>(Space::Cells);
    };
    let run = run::<Glucose>(&bed(carry, stored, None, true).expect("compiled"), 0.05, true);
    assert_eq!(
        run.books,
        books(&[Element::Carbon, Element::Hydrogen, Element::Oxygen])
    );
    run.moves(
        &[
            (Species::Glucose, -20),
            (Species::Oxygen, -6),
            (Species::Glycogen, 19),
            (Species::CarbonDioxide, 6),
            (Species::Water, 25),
            (Species::Energy, 2_900),
        ],
        integral(0.05) / 20.0,
    );
}

#[test]
fn urea_made_from_amino_acids_moves_by_its_rate_and_every_quantity_closes() {
    let made = Proportions::<Urea>::making(Space::Cells)
        .uses::<AminoAcids, 2>(Space::Cells)
        .uses::<Oxygen, 6>(Space::Cells)
        .makes::<CarbonDioxide, 5>(Space::Cells)
        .makes::<Water, 5>(Space::Cells)
        .energy_into(Space::Thermal);
    let carry = |schema: &mut Schema| {
        schema.carry::<AminoAcids, CapillaryBed>(Space::Cells);
        schema.carry::<Urea, CapillaryBed>(Space::Cells);
    };
    let run = run::<Urea>(&bed(carry, made, None, false).expect("compiled"), 0.01, false);
    let elements = [
        Element::Carbon,
        Element::Hydrogen,
        Element::Nitrogen,
        Element::Oxygen,
    ];
    assert_eq!(run.books, books(&elements));
    run.moves(
        &[
            (Species::Urea, 1),
            (Species::AminoAcids, -2),
            (Species::Oxygen, -6),
            (Species::CarbonDioxide, 5),
            (Species::Water, 5),
            (Species::Energy, 800),
        ],
        integral(0.01),
    );
}

#[test]
fn creatinine_made_from_body_protein_moves_by_its_rate_and_every_quantity_closes() {
    let made = Proportions::<Creatinine>::making(Space::Cells)
        .uses::<BodyProtein, 3>(Space::Cells)
        .uses::<Oxygen, 6>(Space::Cells)
        .makes::<CarbonDioxide, 5>(Space::Cells)
        .makes::<Water, 4>(Space::Cells)
        .energy_into(Space::Thermal);
    let carry = |schema: &mut Schema| {
        schema.carry::<BodyProtein, CapillaryBed>(Space::Cells);
        schema.carry::<Creatinine, CapillaryBed>(Space::Cells);
    };
    let run = run::<Creatinine>(&bed(carry, made, None, false).expect("compiled"), 0.01, false);
    let elements = [
        Element::Carbon,
        Element::Hydrogen,
        Element::Nitrogen,
        Element::Oxygen,
    ];
    assert_eq!(run.books, books(&elements));
    run.moves(
        &[
            (Species::Creatinine, 1),
            (Species::BodyProtein, -3),
            (Species::Oxygen, -6),
            (Species::CarbonDioxide, 5),
            (Species::Water, 4),
            (Species::Energy, 650),
        ],
        integral(0.01),
    );
}

#[test]
fn glucose_made_from_lactate_warms_its_bed_and_every_quantity_closes() {
    let made = Proportions::<Glucose>::making(Space::Cells)
        .uses::<Lactate, 3>(Space::Cells)
        .uses::<Oxygen, 3>(Space::Cells)
        .makes::<CarbonDioxide, 3>(Space::Cells)
        .makes::<Water, 3>(Space::Cells)
        .energy_into(Space::Thermal);
    let carry = |schema: &mut Schema| {
        schema.carry::<Lactate, CapillaryBed>(Space::Cells);
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
    };
    let run = run::<Glucose>(&bed(carry, made, None, true).expect("compiled"), 0.01, true);
    assert_eq!(
        run.books,
        books(&[Element::Carbon, Element::Hydrogen, Element::Oxygen])
    );
    assert!(
        run.moved(Species::Energy) > 0.0,
        "the uses hold more energy than the makes"
    );
    run.moves(
        &[
            (Species::Glucose, 1),
            (Species::Lactate, -3),
            (Species::Oxygen, -3),
            (Species::CarbonDioxide, 3),
            (Species::Water, 3),
            (Species::Energy, 200),
        ],
        integral(0.01),
    );
}

#[test]
fn the_reference_count_is_in_the_model_hash() {
    let carry = |schema: &mut Schema| {
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
    };
    let (twice, _) = bed(carry, burn_twice(), None, false).expect("compiled");
    expect!["1ecafbca522756f57161f9ed7203af0bb52a0be9299d2fbb822510522d37bee6"]
        .assert_eq(&hex(twice.model_hash().digest()));
}

#[test]
fn a_reaction_counting_its_reference_once_keeps_the_burning_beds_hash() {
    let burning = BurningBeds::burning().expect("compiled");
    expect!["97f56356df7c0ce48dc256fa58ac1bf29500c29a391bfc474629771cc55d6859"]
        .assert_eq(&hex(burning.body().model_hash().digest()));
}

#[test]
fn a_validity_range_judges_the_kernels_value_before_the_division() {
    let carry = |schema: &mut Schema| {
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
    };
    let (lower, upper) = (
        Measure::new(0.0).expect("finite"),
        Measure::new(0.008).expect("finite"),
    );
    let range = ValidityRange::new(lower, upper, Basis::Fixture(SYNTHETIC)).expect("ordered");
    let (body, _) = bed(carry, burn_twice(), Some(range), false).expect("compiled");
    let (store, core) = (MemoryStore::default(), ReferenceIntegrator::new(10 * SECOND));
    let port = BoundaryPort::new(Vec::new()).expect("an empty port");
    let packet = packet::<Glucose>(&body, 0.01, false);
    let session = admit_fixture(&body, packet, port, &core, &store).expect("admitted at an instant");
    let Step::Accepted(_, receipt) = session.step() else {
        panic!("the step was not accepted");
    };
    let [excursion] = receipt.excursions() else {
        panic!("one excursion: {:?}", receipt.excursions());
    };
    assert_eq!(excursion.side(), ExcursionSide::Above);
    assert_eq!(
        number(excursion.value()),
        0.01,
        "the column's rate, 0.005 turnovers a second, is inside"
    );
}
