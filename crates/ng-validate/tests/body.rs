use expect_test::expect;
use ng_contract::*;
use ng_validate::*;
use std::collections::BTreeMap;

const BEDS: &[&str] = &[
    "pulmonary bed",
    "gut bed",
    "muscle bed",
    "glomerular bed",
    "peritubular bed",
    "brain bed",
    "coronary bed",
    "liver bed",
    "fat bed",
    "skin bed",
    "bone bed",
];
const BLOOD_VESSELS: &[&str] = &[
    "right atrium",
    "right ventricle",
    "left atrium",
    "left ventricle",
    "pulmonary artery",
    "pulmonary vein",
    "systemic arteries",
    "central veins",
    "portal vein",
    "leg veins",
];
const URINARY_LUMENS: &[&str] = &["renal tubule", "collecting duct", "bladder"];
const GUT_LUMENS: &[&str] = &["stomach", "small intestine"];
const AIRWAY_SITES: &[&str] = &["conducting airway", "pulmonary bed"];
const ENCLOSURES: &[&str] = &["pleural space", "abdominal space"];
const JUNCTIONS: &[&str] = &[
    "sinoatrial junction",
    "myocardial junction",
    "arteriolar junction",
    "venous junction",
    "renal junction",
    "respiratory pump junction",
    "skin junction",
];

type Place = (&'static [&'static str], Space);

const PLASMA: Place = (BEDS, Space::Plasma);
const BLOOD_LUMEN: Place = (BLOOD_VESSELS, Space::Lumen);
const INTERSTITIUM: Place = (BEDS, Space::Interstitium);
const CELLS: Place = (BEDS, Space::Cells);
const URINE: Place = (URINARY_LUMENS, Space::Lumen);
const GUT: Place = (GUT_LUMENS, Space::Lumen);
const GAS: Place = (AIRWAY_SITES, Space::Gas);

const BLOOD: &[Place] = &[PLASMA, BLOOD_LUMEN];
const OUTSIDE_CELLS: &[Place] = &[PLASMA, BLOOD_LUMEN, INTERSTITIUM];
const EVERY_WATERY_SPACE: &[Place] = &[PLASMA, BLOOD_LUMEN, INTERSTITIUM, CELLS];

const fn cells(beds: &'static [&'static str]) -> Place {
    (beds, Space::Cells)
}

struct Row {
    source: &'static str,
    species: &'static [Species],
    places: Vec<Place>,
}

fn row(source: &'static str, species: &'static [Species], groups: &[&[Place]]) -> Row {
    Row {
        source,
        species,
        places: groups.concat(),
    }
}

/// Body 01's placements, stated apart from the schema's own table.
fn body_01() -> Vec<Row> {
    use Species::*;
    let enclosure_fluid: Place = (ENCLOSURES, Space::Fluid);
    vec![
        row(
            "01:37 and 01:68 every watery space, 01:532 vapor, plan row 14 enclosure fluid",
            &[Water],
            &[EVERY_WATERY_SPACE, &[URINE, GUT, enclosure_fluid, GAS]],
        ),
        row(
            "01:76-83 sodium, cells at 01:79",
            &[Sodium],
            &[EVERY_WATERY_SPACE, &[URINE, GUT]],
        ),
        row(
            "01:93-100 potassium, cells at 01:96-97",
            &[Potassium],
            &[EVERY_WATERY_SPACE, &[URINE, GUT]],
        ),
        row(
            "01:110-116 chloride, no cells at 01:120",
            &[Chloride],
            &[OUTSIDE_CELLS, &[URINE, GUT]],
        ),
        row("01:529 oxygen", &[Oxygen], &[EVERY_WATERY_SPACE, &[GAS]]),
        row(
            "01:530 carbon dioxide",
            &[CarbonDioxide],
            &[EVERY_WATERY_SPACE, &[URINE, GAS]],
        ),
        row("01:531 nitrogen gas", &[NitrogenGas], &[&[GAS]]),
        row("01:533 glucose", &[Glucose], &[EVERY_WATERY_SPACE, &[URINE, GUT]]),
        row(
            "01:534 glycogen",
            &[Glycogen],
            &[&[cells(&["liver bed", "muscle bed"])]],
        ),
        row(
            "01:535 triglyceride",
            &[Triglyceride],
            &[
                BLOOD,
                &[GUT, (&["gut bed"], Space::Interstitium), cells(&["fat bed"])],
            ],
        ),
        row(
            "01:536 fatty acids, plan row 7: no brain cells",
            &[FattyAcids],
            &[
                OUTSIDE_CELLS,
                &[cells(&[
                    "pulmonary bed",
                    "gut bed",
                    "muscle bed",
                    "glomerular bed",
                    "peritubular bed",
                    "coronary bed",
                    "liver bed",
                    "fat bed",
                    "skin bed",
                    "bone bed",
                ])],
            ],
        ),
        row(
            "01:537 glycerol, plan row 7: fat and liver cells",
            &[Glycerol],
            &[OUTSIDE_CELLS, &[cells(&["fat bed", "liver bed"])]],
        ),
        row(
            "01:538 amino acids, plan row 7: muscle, liver and peritubular cells",
            &[AminoAcids],
            &[
                OUTSIDE_CELLS,
                &[URINE, GUT, cells(&["muscle bed", "liver bed", "peritubular bed"])],
            ],
        ),
        row(
            "01:539 body protein",
            &[BodyProtein],
            &[&[cells(&["muscle bed", "liver bed"])]],
        ),
        row(
            "01:540-541 lactate and ketones",
            &[Lactate, Ketones],
            &[EVERY_WATERY_SPACE],
        ),
        row(
            "01:542 urea, plan row 6: liver cells",
            &[Urea],
            &[OUTSIDE_CELLS, &[URINE, cells(&["liver bed"])]],
        ),
        row(
            "01:543 ammonium",
            &[Ammonium],
            &[&[URINE, cells(&["peritubular bed"])]],
        ),
        row(
            "01:544 creatinine, plan row 6: muscle cells",
            &[Creatinine],
            &[OUTSIDE_CELLS, &[URINE, cells(&["muscle bed"])]],
        ),
        row(
            "01:545 fixed acid",
            &[FixedAcid],
            &[OUTSIDE_CELLS, &[URINE, cells(&["liver bed"])]],
        ),
        row("01:546 diet base", &[DietBase], &[EVERY_WATERY_SPACE, &[GUT]]),
        row("01:547 cell anion", &[CellAnion], &[&[CELLS]]),
        row(
            "01:548 calcium, plan row 5: bone cells",
            &[Calcium],
            &[OUTSIDE_CELLS, &[URINE, GUT, cells(&["bone bed"])]],
        ),
        row(
            "01:549-550 magnesium and phosphate",
            &[Magnesium, Phosphate],
            &[EVERY_WATERY_SPACE, &[URINE, GUT]],
        ),
        row("01:551 hemoglobin", &[Hemoglobin], &[BLOOD]),
        row(
            "01:552 iron",
            &[Iron],
            &[
                BLOOD,
                &[
                    GUT,
                    (&["liver bed", "gut bed"], Space::Interstitium),
                    cells(&["liver bed", "gut bed"]),
                ],
            ],
        ),
        row(
            "01:553-556 hormones, plan row 13: no bound pool",
            &[
                Insulin,
                Glucagon,
                AngiotensinII,
                Aldosterone,
                NatriureticPeptide,
                ParathyroidHormone,
                Cortisol,
            ],
            &[OUTSIDE_CELLS],
        ),
        row("01:554 renin in blood only", &[Renin], &[BLOOD]),
        row("01:557 albumin as the slice", &[Albumin], &[OUTSIDE_CELLS]),
        row(
            "01:611 vasopressin as the slice",
            &[Vasopressin],
            &[BLOOD, &[(&["peritubular bed"], Space::Bound)]],
        ),
        row(
            "the slice's transmitters at every junction",
            &[Norepinephrine],
            &[&[
                (JUNCTIONS, Space::Vesicle),
                (JUNCTIONS, Space::Cleft),
                (JUNCTIONS, Space::Bound),
            ]],
        ),
        row(
            "the slice's acetylcholine at the pacemaker",
            &[Acetylcholine],
            &[&[
                (&["sinoatrial junction"], Space::Vesicle),
                (&["sinoatrial junction"], Space::Cleft),
                (&["sinoatrial junction"], Space::Bound),
            ]],
        ),
        row(
            "the slice's body stores, 01:568 skin heat store, the airway's heat store",
            &[Energy],
            &[&[
                (&["body"], Space::Elastic),
                (&["body", "skin heat site", "conducting airway"], Space::Thermal),
            ]],
        ),
    ]
}

type Pool = (&'static str, Space, Species);

fn sorted(mut pools: Vec<Pool>) -> Vec<Pool> {
    pools.sort_unstable();
    pools.dedup();
    pools
}

fn schema_pools() -> Result<Vec<Pool>, AdmissionError> {
    let schema = body_schema()?;
    let pools = schema
        .pools()
        .map(|(site, space, species)| (site.key(), space, species));
    Ok(sorted(pools.collect()))
}

#[test]
fn the_whole_body_places_each_species_where_body_01_says() -> Result<(), AdmissionError> {
    let mut expected = Vec::new();
    let mut sources = BTreeMap::new();
    for Row {
        source,
        species,
        places,
    } in body_01()
    {
        for &(sites, space) in &places {
            for &site in sites {
                for &species in species {
                    expected.push((site, space, species));
                    sources.entry((site, space, species)).or_insert(source);
                }
            }
        }
    }
    let expected = sorted(expected);
    let actual = schema_pools()?;
    let missing: Vec<(Pool, &str)> = (expected.iter())
        .filter(|pool| actual.binary_search(pool).is_err())
        .map(|&pool| (pool, sources[&pool]))
        .collect();
    let extra: Vec<Pool> = (actual.iter())
        .filter(|pool| expected.binary_search(pool).is_err())
        .copied()
        .collect();
    assert_eq!((missing, extra), (Vec::new(), Vec::new()), "(missing, extra)");
    Ok(())
}

#[test]
fn iron_lives_in_cells_at_the_liver_and_gut_beds_only() -> Result<(), AdmissionError> {
    let beds: Vec<&str> = (schema_pools()?.into_iter())
        .filter(|&(_, space, species)| species == Species::Iron && space == Space::Cells)
        .map(|(site, _, _)| site)
        .collect();
    assert_eq!(beds, ["gut bed", "liver bed"]);
    Ok(())
}

fn tubule_cells(anatomy: Anatomy) -> Result<Vec<(&'static str, Space)>, AdmissionError> {
    let mut schema = Schema::new(anatomy)?;
    schema.carry::<Ammonium, Beds<Tubule>>(Space::Cells);
    let pools = schema.pools().filter(|pool| pool.2 == Species::Ammonium);
    Ok(pools.map(|(site, space, _)| (site.key(), space)).collect())
}

#[test]
fn only_the_whole_body_states_which_tissue_a_bed_serves() -> Result<(), AdmissionError> {
    assert_eq!(tubule_cells(arena_anatomy()?)?, []);
    assert_eq!(
        tubule_cells(body_anatomy()?)?,
        [("peritubular bed", Space::Cells)]
    );
    Ok(())
}

#[test]
fn the_whole_body_holds_its_sites_and_pools() -> Result<(), AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    let body = AnatomyBuilder::new().site::<TissueSite>("body", origin).id();
    let region = AnatomyBuilder::new().region("body", body);
    let anatomy = body_anatomy()?;
    let sites = anatomy.members(region).map_or(0, <[SiteId]>::len);
    let pools = schema_pools()?;
    let mut by_space = BTreeMap::new();
    for &(_, space, _) in &pools {
        *by_space.entry(space).or_insert(0) += 1;
    }
    let pooled: BTreeMap<&str, ()> = pools.iter().map(|pool| (pool.0, ())).collect();
    let pin = format!(
        "sites {sites}, sites with a pool {}, pools {}, by space {by_space:?}",
        pooled.len(),
        pools.len()
    );
    expect![[
        "sites 43, sites with a pool 38, pools 1236, by space {Plasma: 352, Interstitium: 300, Cells: 159, Lumen: 386, Fluid: 2, Vesicle: 8, Cleft: 8, Bound: 9, Elastic: 1, Thermal: 3, Gas: 8}"
    ]]
    .assert_eq(&pin);
    Ok(())
}
