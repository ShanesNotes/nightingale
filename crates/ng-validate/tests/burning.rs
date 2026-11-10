use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const BURN: LawName = LawName {
    owner: <Metabolism as Owner>::CARD,
    local: "burn",
};

fn number(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn refused(case: Result<BurningBeds, AdmissionError>) -> CompileDefect {
    match case {
        Err(AdmissionError::Compile(defect)) => defect,
        Err(other) => panic!("refused outside the compile: {other:?}"),
        Ok(_) => panic!("compiled"),
    }
}

#[test]
fn every_bed_closes_carbon_hydrogen_oxygen_and_energy_while_it_burns() {
    let case = BurningBeds::burning().expect("compiled");
    let (store, core) = (MemoryStore::default(), ReferenceIntegrator::new(10 * SECOND));
    let session = case.admit(&core, &store).expect("admitted at an instant");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the burn faulted");
    };
    let view = session.conservation();
    let window = NonZeroU64::new(60 * SECOND).expect("positive");
    let windows = SlidingWindows::new(window, case.end().nanoseconds()).expect("a coverage");
    let certificate =
        certify(&view, &case.certificate(windows).expect("a policy")).expect("every quantity closes");
    assert_eq!(certificate.results().len(), view.basis().combinations().len());

    let rows = case.body().rows();
    let place = |row: RowId| match rows[row.index()].kind() {
        RowKind::Amount { site, species, .. } => (site, species),
        other => panic!("an amount row, not {other:?}"),
    };
    let mut held: Vec<(&'static str, Book)> = (view.basis().combinations().iter())
        .map(|combination| {
            let site = place(combination.terms()[0].0).0;
            let alone = combination.terms().iter().all(|&(row, _)| place(row).0 == site);
            assert!(alone, "{:?} holds rows of another bed", combination.book());
            (site.key(), combination.book())
        })
        .collect();
    held.sort_unstable();
    let element = Book::Element;
    let quantities = [
        element(Element::Carbon),
        element(Element::Hydrogen),
        element(Element::Oxygen),
        Book::Energy,
    ];
    let expected: Vec<(&'static str, Book)> = (["bed a", "bed b", "bed c"].into_iter())
        .flat_map(|bed| quantities.map(|book| (bed, book)))
        .collect();
    assert_eq!(held, expected);

    let head = view.head().expect("a step");
    assert_eq!(case.burns().len(), 3);
    for column in case.burns() {
        let (forward, _) = head.exchange(column);
        let extent = number(forward.on(head.interval()).expect("the head's own interval")[1]);
        assert!(extent > 0.0, "column {} burned {extent}", column.index());
        let terms = case.body().columns()[column.index()].terms();
        let count = |species| {
            (terms.iter())
                .find(|&&(row, _)| place(row).1 == species)
                .map(|term| term.1)
        };
        let counts = [Species::Glucose, Species::CarbonDioxide, Species::Energy].map(count);
        assert_eq!(
            counts,
            [Some(-1), Some(6), Some(1_000)],
            "glucose used, carbon dioxide and heat made"
        );
    }
}

#[test]
fn a_burn_that_makes_carbon_is_refused_naming_carbon() {
    let seven = Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 7>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium)
        .energy_into(Space::Thermal);
    assert_eq!(
        refused(BurningBeds::with(seven)),
        CompileDefect::UnbalancedReaction {
            law: BURN,
            book: Book::Element(Element::Carbon)
        }
    );
}

#[test]
fn a_burn_whose_energy_goes_nowhere_is_refused() {
    let cold = Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 6>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium);
    assert_eq!(
        refused(BurningBeds::with(cold)),
        CompileDefect::UnbalancedReaction {
            law: BURN,
            book: Book::Energy
        }
    );
}

#[test]
fn a_burn_whose_fuel_has_no_stated_content_is_refused() {
    assert_eq!(
        refused(BurningBeds::unstated()),
        CompileDefect::UnstatedEnergy {
            law: BURN,
            species: Species::Glucose
        }
    );
}

#[test]
fn the_idle_twin_holds_the_same_rows_and_every_column_but_the_burns() {
    let (burning, idle) = (
        BurningBeds::burning().expect("compiled"),
        BurningBeds::idle().expect("compiled"),
    );
    assert_eq!(burning.body().rows(), idle.body().rows());
    let unburnt: Vec<&Column> = (burning.body().columns().iter())
        .filter(|column| column.origin() != BURN)
        .collect();
    assert_eq!(unburnt, idle.body().columns().iter().collect::<Vec<_>>());
    assert_eq!(
        burning.body().columns().len() - unburnt.len(),
        burning.burns().len()
    );
    assert!(idle.burns().is_empty());
}
