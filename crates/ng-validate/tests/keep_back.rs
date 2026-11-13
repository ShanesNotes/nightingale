use ng_contract::*;
use ng_validate::*;

const STARLING: LawName = LawName {
    owner: "FL",
    local: "starling",
};
const LYMPH: LawName = LawName {
    owner: "FL",
    local: "lymph",
};
const FILTRATION: LawName = LawName {
    owner: "RN",
    local: "filtration",
};
/// In the order compile reaches them, so a species the first route carries is refused there.
const ROUTES: [LawName; 3] = [STARLING, LYMPH, FILTRATION];

fn in_blood_alone<R: Substance>() -> Result<CompiledBody, AdmissionError> {
    let mut schema = arena_schema()?;
    schema.carry::<R, Blood>(Space::Plasma);
    schema.carry::<R, Blood>(Space::Lumen);
    assemble(schema, |mut body| {
        hold_outside_slice(&mut body)?;
        arena_organs(&mut body)?;
        body.compile()
    })
}

/// The route that strands the species, or `None` when the body compiles.
fn stranding<R: Substance>() -> Option<LawName> {
    match in_blood_alone::<R>() {
        Ok(_) => None,
        Err(AdmissionError::Compile(CompileDefect::CargoStranded { law, species })) if species == R::SPECIES => {
            Some(law)
        }
        Err(other) => panic!("{:?} in blood alone: {other:?}", R::SPECIES),
    }
}

struct Row {
    species: Species,
    observed: fn() -> Option<LawName>,
    kept_back_by: &'static [LawName],
}

fn row<R: Substance>(kept_back_by: &'static [LawName]) -> Row {
    Row {
        species: R::SPECIES,
        observed: stranding::<R>,
        kept_back_by,
    }
}

/// Body 06's plan, item 4: every water-borne species the slice does not place, and the routes that keep it back.
fn table() -> Vec<Row> {
    vec![
        row::<Oxygen>(&ROUTES),
        row::<CarbonDioxide>(&ROUTES),
        row::<Glucose>(&ROUTES),
        row::<Triglyceride>(&ROUTES),
        row::<FattyAcids>(&ROUTES),
        row::<Glycerol>(&ROUTES),
        row::<AminoAcids>(&ROUTES),
        row::<Lactate>(&ROUTES),
        row::<Ketones>(&ROUTES),
        row::<Urea>(&ROUTES),
        row::<Ammonium>(&ROUTES),
        row::<Creatinine>(&ROUTES),
        row::<FixedAcid>(&ROUTES),
        row::<DietBase>(&ROUTES),
        row::<Calcium>(&ROUTES),
        row::<Magnesium>(&ROUTES),
        row::<Phosphate>(&ROUTES),
        row::<Hemoglobin>(&ROUTES),
        row::<Iron>(&ROUTES),
        row::<Insulin>(&ROUTES),
        row::<Glucagon>(&ROUTES),
        row::<Renin>(&ROUTES),
        row::<AngiotensinII>(&ROUTES),
        row::<Aldosterone>(&ROUTES),
        row::<NatriureticPeptide>(&ROUTES),
        row::<ParathyroidHormone>(&ROUTES),
        row::<Cortisol>(&ROUTES),
    ]
}

#[test]
fn a_hormone_in_blood_alone_compiles_on_the_slices_own_fluid_routes() {
    assert_eq!(stranding::<Cortisol>(), None);
}

#[test]
fn each_route_keeps_back_every_water_borne_species_the_slice_does_not_place() {
    let table = table();
    assert_eq!(table.len(), 27);
    for row in &table {
        let expected = ROUTES.into_iter().find(|route| !row.kept_back_by.contains(route));
        assert_eq!((row.species, (row.observed)()), (row.species, expected));
    }
}
