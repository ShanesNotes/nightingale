use ng_contract::*;
use ng_validate::*;

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

fn stranding<R: Substance>() -> (Species, Option<LawName>) {
    let law = match in_blood_alone::<R>() {
        Ok(_) => None,
        Err(AdmissionError::Compile(CompileDefect::CargoStranded { law, species }))
            if species == R::SPECIES =>
        {
            Some(law)
        }
        Err(other) => panic!("{:?} in blood alone: {other:?}", R::SPECIES),
    };
    (R::SPECIES, law)
}

type Probe = fn() -> (Species, Option<LawName>);

const KEPT_BACK_BY_STARLING_LYMPH_AND_FILTRATION: [Probe; 27] = [
    stranding::<Oxygen>,
    stranding::<CarbonDioxide>,
    stranding::<Glucose>,
    stranding::<Triglyceride>,
    stranding::<FattyAcids>,
    stranding::<Glycerol>,
    stranding::<AminoAcids>,
    stranding::<Lactate>,
    stranding::<Ketones>,
    stranding::<Urea>,
    stranding::<Ammonium>,
    stranding::<Creatinine>,
    stranding::<FixedAcid>,
    stranding::<DietBase>,
    stranding::<Calcium>,
    stranding::<Magnesium>,
    stranding::<Phosphate>,
    stranding::<Hemoglobin>,
    stranding::<Iron>,
    stranding::<Insulin>,
    stranding::<Glucagon>,
    stranding::<Renin>,
    stranding::<AngiotensinII>,
    stranding::<Aldosterone>,
    stranding::<NatriureticPeptide>,
    stranding::<ParathyroidHormone>,
    stranding::<Cortisol>,
];

#[test]
fn a_hormone_in_blood_alone_compiles_on_the_slices_own_fluid_routes() {
    assert_eq!(stranding::<Cortisol>(), (Species::Cortisol, None));
}

#[test]
fn each_route_keeps_back_every_water_borne_species_the_slice_does_not_place() {
    for probe in KEPT_BACK_BY_STARLING_LYMPH_AND_FILTRATION {
        let (species, law) = probe();
        assert_eq!((species, law), (species, None));
    }
}
