extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use expect_test::expect;
use ng_contract::*;
use ng_validate::*;

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
            "01:536 fatty acids, in every bed's cells: the brain burns at the body's share until ticket 16",
            &[FattyAcids],
            &[EVERY_WATERY_SPACE],
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
            "the slice's body stores, 01:568 skin heat store, 06:31 the airway's heat store, 01:165 each bed's heat",
            &[Energy],
            &[&[
                (&["body"], Space::Elastic),
                (&["body", "skin heat site", "conducting airway"], Space::Thermal),
                (BEDS, Space::Thermal),
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
    expect!["sites 43, sites with a pool 38, pools 1248, by space {Plasma: 352, Interstitium: 300, Cells: 160, Lumen: 386, Fluid: 2, Vesicle: 8, Cleft: 8, Bound: 9, Elastic: 1, Thermal: 14, Gas: 8}"]
    .assert_eq(&pin);
    Ok(())
}

#[test]
fn the_whole_body_compiles_on_its_holds() {
    if let Err(refused) = whole_body() {
        panic!("{refused:?}");
    }
}

#[test]
fn the_whole_body_marks_the_slices_four_veins_and_no_other_vessel() -> Result<(), AdmissionError> {
    let body = whole_body()?;
    let marked: BTreeSet<&str> = (body.parameters().iter())
        .filter(|slot| slot.key() == ParameterKey::new("cv.collapse-share"))
        .filter_map(|slot| match slot.binding() {
            Binding::Site(site) => Some(site.key()),
            _ => None,
        })
        .collect();
    let table = BTreeSet::from(["central veins", "leg veins", "portal vein", "pulmonary vein"]);
    let unmarked: Vec<_> = table.difference(&marked).collect();
    let beyond: Vec<_> = marked.difference(&table).collect();
    assert!(
        unmarked.is_empty() && beyond.is_empty(),
        "unmarked: {unmarked:?}, marked beyond the table: {beyond:?}"
    );
    Ok(())
}

fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn pool_of(body: &CompiledBody, row: RowId) -> (SiteId, Space, Species) {
    match body
        .rows()
        .iter()
        .find(|known| known.id() == row)
        .map(|known| known.kind())
    {
        Some(RowKind::Amount { site, space, species }) => (site, space, species),
        other => panic!("a column's term on {other:?}"),
    }
}

#[test]
fn each_hold_names_the_ticket_that_releases_it_and_its_body_01_line() {
    let rows: String = (HOLD_TABLE.iter())
        .map(|hold| format!("{} {:?} {:?} {}\n", hold.card, hold.held, hold.release, hold.line))
        .collect();
    expect![[r#"
        RS Route("expired water") Ticket(10, "RS") 01:63, 873
        RS Route("airway heat") Ticket(10, "RS") 01:171-172, 873
        RS Pool(Water, Gas, Tissue(Lung)) Ticket(10, "RS") 01:356, 532
        GI Route("gut water") Ticket(13, "GI") 01:54, 876
        GI Route("gut sodium") Ticket(13, "GI") 01:77, 876
        GI Route("gut potassium") Ticket(13, "GI") 01:94, 876
        GI Route("gut chloride") Ticket(13, "GI") 01:111, 876
        GI Route("gut glucose") Ticket(13, "GI") 01:187, 876
        GI Route("gut triglyceride") Ticket(13, "GI") 01:198, 876
        GI Route("gut amino acids") Ticket(13, "GI") 01:206, 876
        GI Route("gut diet base") Ticket(18, "GI") 01:255, 424
        GI Route("gut calcium") Ticket(19, "GI") 01:275, 425
        GI Route("gut phosphate") Ticket(19, "GI") 01:275, 425
        GI Route("gut magnesium") Ticket(19, "GI") 01:275, 425
        GI Route("gut iron") Ticket(21, "GI") 01:305, 426
        GI Route("feces water") Ticket(13, "GI") 01:64
        GI Route("feces sodium") Ticket(13, "GI") 01:83
        GI Route("feces potassium") Ticket(13, "GI") 01:100
        GI Route("feces chloride") Ticket(13, "GI") 01:116
        GI Route("feces glucose") Ticket(13, "GI") 01:211
        GI Route("feces triglyceride") Ticket(13, "GI") 01:211
        GI Route("feces amino acids") Ticket(13, "GI") 01:211, 240
        GI Route("feces calcium") Ticket(19, "GI") 01:283, 293
        GI Route("feces phosphate") Ticket(19, "GI") 01:284, 293
        GI Route("feces magnesium") Ticket(19, "GI") 01:285, 293
        GI Route("feces iron") Ticket(21, "GI") 01:311
        GI Pool(Triglyceride, Interstitium, Tissue(Gut)) Ticket(13, "GI") 01:198, 535
        GI Pool(AminoAcids, Interstitium, Class(CapillaryBed)) Ticket(13, "FL") 01:485, 538
        EN Route("insulin secretion") Ticket(15, "EN") 01:437, 605
        EN Route("insulin clearance") Ticket(15, "EN") 01:605
        EN Route("glucagon secretion") Ticket(15, "EN") 01:437, 605
        EN Route("glucagon clearance") Ticket(15, "EN") 01:605
        EN Route("renin secretion") Ticket(17, "EN") 01:439, 606
        EN Route("renin clearance") Ticket(17, "EN") 01:606
        EN Route("angiotensin II made") Ticket(17, "EN") 01:607
        EN Route("angiotensin II clearance") Ticket(17, "EN") 01:607
        EN Route("aldosterone secretion") Ticket(17, "EN") 01:439, 608
        EN Route("aldosterone clearance") Ticket(17, "EN") 01:608
        EN Route("natriuretic peptide secretion") Ticket(17, "EN") 01:440, 608
        EN Route("natriuretic peptide clearance") Ticket(17, "EN") 01:608
        EN Route("parathyroid hormone secretion") Ticket(19, "EN") 01:441, 609
        EN Route("parathyroid hormone clearance") Ticket(19, "EN") 01:609
        EN Route("cortisol secretion") Ticket(20, "EN") 01:442, 610
        EN Route("cortisol clearance") Ticket(20, "EN") 01:610
        EN Route("vasopressin secretion") Ticket(17, "EN") 01:438, 611, 880
        EN Route("vasopressin clearance") Ticket(17, "EN") 01:611, 880
        EN Pool(Insulin, Interstitium, Class(CapillaryBed)) Ticket(15, "FL") 01:485, 553
        EN Pool(Glucagon, Interstitium, Class(CapillaryBed)) Ticket(15, "FL") 01:485, 553
        EN Pool(AngiotensinII, Interstitium, Class(CapillaryBed)) Ticket(17, "FL") 01:485, 554
        EN Pool(Aldosterone, Interstitium, Class(CapillaryBed)) Ticket(17, "FL") 01:485, 554
        EN Pool(NatriureticPeptide, Interstitium, Class(CapillaryBed)) Ticket(17, "FL") 01:485, 554
        EN Pool(ParathyroidHormone, Interstitium, Class(CapillaryBed)) Ticket(19, "FL") 01:485, 555
        EN Pool(Cortisol, Interstitium, Class(CapillaryBed)) Ticket(20, "FL") 01:485, 556
        MS Route("skin water") Ticket(12, "MT") 01:651, 875
        MS Pool(Magnesium, Cells, Tissue(Bone)) Ticket(19, "MS") 01:38, 277
        MS Pool(Phosphate, Cells, Tissue(Bone)) Ticket(19, "MS") 01:38, 277
        MS Pool(Magnesium, Cells, Besides(Bone)) Ticket(19, "FL") 01:38, 278
        MS Pool(Phosphate, Cells, Besides(Bone)) Ticket(19, "FL") 01:38, 278
        MS Pool(Calcium, Cells, Tissue(Bone)) Ticket(19, "MS") 01:277, 548
        MS Pool(Creatinine, Cells, Tissue(Muscle)) Ticket(16, "MS") 01:210, 236
        MS Pool(BodyProtein, Cells, Tissue(Muscle)) Ticket(16, "MS") 01:207, 879
        MS Pool(BodyProtein, Cells, Tissue(Liver)) Ticket(16, "MS") 01:207, 879
        MS Pool(Glycogen, Cells, Tissue(Muscle)) Ticket(16, "MS") 01:191, 879
        MS Pool(AminoAcids, Cells, Tissue(Muscle)) Ticket(16, "MS") 01:207
        MS Pool(Creatinine, Interstitium, Class(CapillaryBed)) Ticket(16, "FL") 01:485, 544
        MS Pool(Calcium, Interstitium, Class(CapillaryBed)) Ticket(19, "FL") 01:276, 293
        MS Pool(Magnesium, Interstitium, Class(CapillaryBed)) Ticket(19, "FL") 01:276, 293
        MS Pool(Phosphate, Interstitium, Class(CapillaryBed)) Ticket(19, "FL") 01:276, 293
        BC Pool(Sodium, Cells, Class(CapillaryBed)) Ticket(17, "FL") 01:79
        BC Pool(Potassium, Cells, Tissue(Muscle)) Ticket(15, "FL") 01:96, 878
        BC Pool(Potassium, Cells, Tissue(Fat)) Ticket(15, "FL") 01:96, 878
        BC Pool(Potassium, Cells, Tissue(Lung)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Gut)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Glomerulus)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Tubule)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Brain)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Heart)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Liver)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Skin)) Ticket(17, "FL") 01:97
        BC Pool(Potassium, Cells, Tissue(Bone)) Ticket(17, "FL") 01:97
        BC Pool(CellAnion, Cells, Class(CapillaryBed)) Kept 01:120, 547
        BC Pool(DietBase, Cells, Class(CapillaryBed)) Ticket(18, "MT") 01:256, 881
        BC Pool(Ammonium, Cells, Tissue(Tubule)) Ticket(18, "RN") 01:209, 235, 543
        BC Pool(AminoAcids, Cells, Tissue(Tubule)) Ticket(18, "RN") 01:209
        BC Pool(DietBase, Interstitium, Class(CapillaryBed)) Ticket(18, "FL") 01:546, 881
        BC Pool(FixedAcid, Interstitium, Class(CapillaryBed)) Ticket(18, "FL") 01:253, 881
        MT Route("skin evaporation") Ticket(12, "MT") 01:61
        MT Route("sweat water") Ticket(12, "MT") 01:62
        MT Route("sweat sodium") Ticket(12, "MT") 01:82
        MT Route("sweat chloride") Ticket(12, "MT") 01:115
        MT Route("dry heat loss") Ticket(12, "MT") 01:170
        MT Route("skin evaporative heat") Ticket(12, "MT") 01:171
        MT Route("pulmonary bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("pulmonary bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("pulmonary bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("gut bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("gut bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("gut bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("muscle bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("muscle bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("muscle bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("glomerular bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("glomerular bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("glomerular bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("peritubular bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("peritubular bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("peritubular bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("brain bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("brain bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("brain bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("coronary bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("coronary bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("coronary bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("liver bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("liver bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("liver bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("fat bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("fat bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("fat bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("skin bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("skin bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("skin bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Route("bone bed glucose feed") Ticket(16, "MT") 01:190
        MT Route("bone bed fatty acid feed") Ticket(16, "MT") 01:200
        MT Route("bone bed cell water out") Ticket(17, "FL") 01:58, 880
        MT Pool(Glucose, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 533
        MT Pool(FattyAcids, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 536
        MT Pool(Lactate, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 540
        MT Pool(Triglyceride, Cells, Tissue(Fat)) Ticket(16, "MT") 01:199, 879
        MT Pool(Glycerol, Cells, Tissue(Fat)) Ticket(16, "MT") 01:199
        HP Route("insulin first pass") Ticket(15, "HP") 01:408, 605
        HP Route("albumin synthesis") Ticket(21, "HP") 01:312, 407, 884
        HP Route("albumin loss") Ticket(21, "HP") 01:314, 407, 884
        HP Route("fixed acid made") Ticket(18, "HP") 01:252, 406
        HP Pool(Glycerol, Cells, Tissue(Liver)) Ticket(14, "HP") 01:204
        HP Pool(AminoAcids, Cells, Tissue(Liver)) Ticket(14, "HP") 01:208
        HP Pool(Urea, Cells, Tissue(Liver)) Ticket(14, "HP") 01:234, 877
        HP Pool(Glycogen, Cells, Tissue(Liver)) Ticket(14, "HP") 01:188, 877
        HP Pool(Ketones, Cells, Tissue(Liver)) Ticket(16, "HP") 01:38, 201-202, 879
        HP Pool(Ketones, Cells, Besides(Liver)) Ticket(16, "MT") 01:38, 203, 879
        HP Pool(Glycerol, Interstitium, Class(CapillaryBed)) Ticket(14, "FL") 01:485, 537
        HP Pool(Ketones, Interstitium, Class(CapillaryBed)) Ticket(16, "FL") 01:485, 541
        HP Pool(Urea, Interstitium, Class(CapillaryBed)) Ticket(14, "FL") 01:485, 542
        HC Route("hemoglobin production") Ticket(21, "HC") 01:301, 884
        HC Route("hemoglobin destruction") Ticket(21, "HC") 01:303, 884
        HC Pool(Iron, Cells, Tissue(Liver)) Ticket(21, "HC") 01:307-308, 884
        HC Pool(Iron, Cells, Tissue(Gut)) Ticket(21, "HC") 01:309, 884
        HC Pool(Iron, Interstitium, Tissue(Liver)) Ticket(21, "FL") 01:306
        HC Pool(Iron, Interstitium, Tissue(Gut)) Ticket(21, "FL") 01:306
    "#]]
    .assert_eq(&rows);
}

#[test]
fn the_hold_table_is_the_compiled_bodys_coverage_manifest_row_for_row() -> Result<(), AdmissionError> {
    let body = whole_body()?;
    let (mut manifest, mut whole_spaces) = (Vec::new(), Vec::new());
    for card in body.conserved_basis().coverage().declarations() {
        let name = card.card();
        manifest.extend(card.routes().iter().map(|&route| (name, Held::Route(route))));
        manifest.extend((card.species().iter()).map(|hold| {
            let held = Held::Pool(hold.species(), hold.space(), hold.sites());
            (name, held)
        }));
        whole_spaces.extend(card.spaces().iter().map(|&space| (name, space)));
    }
    manifest.sort_unstable();
    let mut table: Vec<(&str, Held)> = HOLD_TABLE.iter().map(|hold| (hold.card, hold.held)).collect();
    table.sort_unstable();
    table.dedup();
    assert_eq!(table.len(), HOLD_TABLE.len(), "a row twice in the table");
    let only_compiled: Vec<_> = (manifest.iter())
        .filter(|row| table.binary_search(row).is_err())
        .collect();
    let only_tabled: Vec<_> = (table.iter())
        .filter(|row| manifest.binary_search(row).is_err())
        .collect();
    assert_eq!(
        (only_compiled, only_tabled, whole_spaces),
        (Vec::new(), Vec::new(), Vec::new()),
        "(held by the body only, in the table only, spaces held whole)"
    );

    let (mut by_card, mut by_ticket) = (BTreeMap::new(), BTreeMap::new());
    for hold in HOLD_TABLE {
        *by_card.entry(hold.card).or_insert(0) += 1;
        let ticket = match hold.release {
            Release::Ticket(ticket, _) => format!("{ticket:02}"),
            Release::Kept => "kept".to_string(),
        };
        *by_ticket.entry(ticket).or_insert(0) += 1;
    }
    let routes = (HOLD_TABLE.iter())
        .filter(|hold| matches!(hold.held, Held::Route(_)))
        .count();
    let pin = format!(
        "holds {}: routes {routes}, pools {}\nby card {by_card:?}\nby ticket {by_ticket:?}\n",
        HOLD_TABLE.len(),
        HOLD_TABLE.len() - routes
    );
    expect![[r#"
        holds 149: routes 89, pools 60
        by card {"BC": 18, "EN": 25, "GI": 25, "HC": 6, "HP": 13, "MS": 15, "MT": 44, "RS": 3}
        by ticket {"09": 3, "10": 3, "12": 7, "13": 16, "14": 6, "15": 9, "16": 33, "17": 34, "18": 7, "19": 17, "20": 3, "21": 10, "kept": 1}
    "#]].assert_eq(&pin);
    Ok(())
}

const PROCESSES: &[(&str, &str, Space, &str)] = &[
    ("insulin secretion", "portal vein", Space::Lumen, "01:605"),
    ("glucagon secretion", "portal vein", Space::Lumen, "01:605"),
    ("insulin clearance", "central veins", Space::Lumen, "01:605"),
    ("glucagon clearance", "central veins", Space::Lumen, "01:605"),
    (
        "insulin first pass",
        "liver bed",
        Space::Plasma,
        "01:605, plan row 20",
    ),
    ("renin secretion", "glomerular bed", Space::Plasma, "01:606"),
    ("renin clearance", "central veins", Space::Lumen, "01:606"),
    ("angiotensin II made", "pulmonary bed", Space::Plasma, "01:607"),
    (
        "angiotensin II clearance",
        "central veins",
        Space::Lumen,
        "01:607",
    ),
    ("aldosterone secretion", "central veins", Space::Lumen, "01:608"),
    ("aldosterone clearance", "central veins", Space::Lumen, "01:608"),
    (
        "natriuretic peptide secretion",
        "central veins",
        Space::Lumen,
        "01:608",
    ),
    (
        "natriuretic peptide clearance",
        "central veins",
        Space::Lumen,
        "01:608",
    ),
    (
        "parathyroid hormone secretion",
        "central veins",
        Space::Lumen,
        "01:609",
    ),
    (
        "parathyroid hormone clearance",
        "central veins",
        Space::Lumen,
        "01:609",
    ),
    ("cortisol secretion", "central veins", Space::Lumen, "01:610"),
    ("cortisol clearance", "central veins", Space::Lumen, "01:610"),
    ("vasopressin secretion", "central veins", Space::Lumen, "01:611"),
    ("vasopressin clearance", "central veins", Space::Lumen, "01:611"),
    ("albumin synthesis", "liver bed", Space::Plasma, "01:312"),
    ("albumin loss", "liver bed", Space::Plasma, "01:314"),
    ("fixed acid made", "liver bed", Space::Cells, "01:252"),
    ("hemoglobin production", "central veins", Space::Lumen, "01:301"),
    ("hemoglobin destruction", "gut bed", Space::Plasma, "01:303, 525"),
    ("feces water", "small intestine", Space::Lumen, "01:64"),
    ("feces sodium", "small intestine", Space::Lumen, "01:83"),
    ("feces potassium", "small intestine", Space::Lumen, "01:100"),
    ("feces chloride", "small intestine", Space::Lumen, "01:116"),
    ("feces glucose", "small intestine", Space::Lumen, "01:211"),
    ("feces triglyceride", "small intestine", Space::Lumen, "01:211"),
    ("feces amino acids", "small intestine", Space::Lumen, "01:211"),
    ("feces calcium", "small intestine", Space::Lumen, "01:283"),
    ("feces phosphate", "small intestine", Space::Lumen, "01:284"),
    ("feces magnesium", "small intestine", Space::Lumen, "01:285"),
    ("feces iron", "small intestine", Space::Lumen, "01:311"),
    (
        "skin evaporation",
        "skin bed",
        Space::Interstitium,
        "01:61, plan row 20",
    ),
    (
        "sweat water",
        "skin bed",
        Space::Interstitium,
        "01:62, plan row 20",
    ),
    (
        "sweat sodium",
        "skin bed",
        Space::Interstitium,
        "01:82, plan row 20",
    ),
    (
        "sweat chloride",
        "skin bed",
        Space::Interstitium,
        "01:115, plan row 20",
    ),
    ("dry heat loss", "skin heat site", Space::Thermal, "01:170, 568"),
    (
        "skin evaporative heat",
        "skin heat site",
        Space::Thermal,
        "01:171",
    ),
    ("inspired oxygen", "conducting airway", Space::Gas, "01:128"),
    (
        "inspired carbon dioxide",
        "conducting airway",
        Space::Gas,
        "01:149",
    ),
    (
        "inspired nitrogen gas",
        "conducting airway",
        Space::Gas,
        "01:531, plan row 20",
    ),
    ("expired oxygen", "conducting airway", Space::Gas, "01:133"),
    (
        "expired carbon dioxide",
        "conducting airway",
        Space::Gas,
        "01:150",
    ),
    (
        "expired nitrogen gas",
        "conducting airway",
        Space::Gas,
        "01:531, plan row 20",
    ),
    (
        "expired water",
        "conducting airway",
        Space::Gas,
        "01:63, 06's airway exit",
    ),
    (
        "airway heat",
        "conducting airway",
        Space::Thermal,
        "01:171-172, 06's airway heat store",
    ),
];

#[test]
fn each_of_the_49_processes_moves_the_pool_body_01_puts_it_at() -> Result<(), AdmissionError> {
    let body = whole_body()?;
    let named = |name: &str| PROCESSES.iter().any(|process| process.0 == name);
    let mut moved = Vec::new();
    for column in body.columns() {
        let Some(receipt) = column.receipt().filter(|receipt| named(receipt.boundary())) else {
            continue;
        };
        for &(row, _) in column.terms() {
            let (site, space, _) = pool_of(&body, row);
            moved.push((receipt.boundary(), site.key(), space));
        }
    }
    moved.sort_unstable();
    let mut placed: Vec<_> = (PROCESSES.iter())
        .map(|&(name, site, space, _)| (name, site, space))
        .collect();
    placed.sort_unstable();
    assert_eq!(moved, placed);
    assert_eq!(placed.len(), 49);
    Ok(())
}

fn fl_owns(wall: &str, species: Species, bed: &str) -> bool {
    use Species::*;
    let gas = matches!(species, Oxygen | CarbonDioxide);
    let fuel = matches!(
        species,
        Glucose | FattyAcids | Glycerol | AminoAcids | Lactate | Ketones
    );
    let iron = species == Iron && matches!(bed, "liver bed" | "gut bed");
    let mineral = matches!(species, Calcium | Phosphate | Magnesium) && bed == "bone bed";
    match wall {
        "capillary wall" => !gas,
        "cell membrane" => !(gas || fuel || iron || mineral),
        _ => true,
    }
}

type Crossed = (BTreeSet<&'static str>, BTreeSet<&'static str>);

#[test]
fn fl_crosses_each_wall_with_only_the_species_body_01_gives_it() -> Result<(), AdmissionError> {
    let body = whole_body()?;
    let mut crossings: BTreeMap<(&str, Species), Crossed> = BTreeMap::new();
    for column in (body.columns().iter()).filter(|column| column.origin().owner == Fluids::CARD) {
        let mut ends: Vec<_> = (column.terms().iter())
            .map(|&(row, _)| pool_of(&body, row))
            .collect();
        ends.sort_unstable_by_key(|end| end.1);
        let wall = match ends.iter().map(|end| end.1).collect::<Vec<_>>()[..] {
            [Space::Plasma, Space::Interstitium] => "capillary wall",
            [Space::Interstitium, Space::Cells] => "cell membrane",
            [Space::Interstitium, Space::Lumen] => "lymph",
            ref other => panic!("{:?} moves {other:?}", column.origin()),
        };
        let (bed, _, species) = ends[0];
        let (laws, beds) = crossings.entry((wall, species)).or_default();
        laws.insert(column.origin().local);
        beds.insert(bed.key());
    }
    let refused: Vec<_> = (crossings.iter())
        .flat_map(|(&(wall, species), (_, beds))| beds.iter().map(move |&bed| (wall, species, bed)))
        .filter(|&(wall, species, bed)| !fl_owns(wall, species, bed))
        .collect();
    assert_eq!(refused, [], "crossings 01:38 gives another card");
    let pin: Vec<String> = (crossings.iter())
        .map(|((wall, species), (laws, beds))| {
            format!("{wall}: {species:?} by {laws:?} at {} beds", beds.len())
        })
        .collect();
    expect![[r#"
        capillary wall: Water by {"starling"} at 11 beds
        capillary wall: Sodium by {"exchange sodium", "starling"} at 11 beds
        capillary wall: Potassium by {"exchange potassium", "starling"} at 11 beds
        capillary wall: Chloride by {"exchange chloride", "starling"} at 11 beds
        capillary wall: Albumin by {"exchange albumin", "starling"} at 11 beds
        lymph: Water by {"lymph"} at 11 beds
        lymph: Sodium by {"lymph"} at 11 beds
        lymph: Potassium by {"lymph"} at 11 beds
        lymph: Chloride by {"lymph"} at 11 beds
        lymph: Albumin by {"lymph"} at 11 beds
    "#]]
    .assert_eq(&(pin.join("\n") + "\n"));
    Ok(())
}

#[test]
fn the_coronary_bed_pays_the_stroke_work_and_the_muscle_bed_the_walls_so_none_ends_at_chemical_energy()
-> Result<(), AdmissionError> {
    let mut heart = Schema::new(body_anatomy()?)?;
    heart.carry::<Ammonium, Beds<Heart>>(Space::Cells);
    let beds: Vec<_> = (heart.pools())
        .filter(|pool| pool.2 == Species::Ammonium && pool.1 == Space::Cells)
        .map(|(site, _, _)| site.key())
        .collect();
    assert_eq!(
        beds,
        ["coronary bed"],
        "the heart's beds, where the stroke work's oxygen is read"
    );
    let body = whole_body()?;
    let mut paid = BTreeMap::new();
    for column in body.columns() {
        let origin = column.origin();
        assert!(
            column
                .receipt()
                .is_none_or(|receipt| receipt.boundary() != "chemical energy"),
            "{origin:?} ends at chemical energy"
        );
        let payer = match origin.local {
            "stroke work" => "coronary bed",
            "wall work" => "muscle bed",
            _ => continue,
        };
        let terms: BTreeSet<_> = (column.terms().iter())
            .map(|&(row, count)| {
                let (site, space, species) = pool_of(&body, row);
                (site.key(), space, species, count)
            })
            .collect();
        let expected = BTreeSet::from([
            (payer, Space::Thermal, Species::Energy, -1),
            ("body", Space::Elastic, Species::Energy, 1),
        ]);
        assert_eq!(terms, expected, "{origin:?} at {:?}", column.context());
        *paid.entry(origin.local).or_insert(0) += 1;
    }
    assert_eq!(paid, BTreeMap::from([("stroke work", 21), ("wall work", 1)]));
    Ok(())
}

#[test]
fn the_whole_body_holds_its_rows_columns_and_digests() -> Result<(), AdmissionError> {
    let body = whole_body()?;
    let mut kinds = BTreeMap::new();
    for row in body.rows() {
        let kind = match row.kind() {
            RowKind::Amount { .. } => "amount",
            RowKind::Memory(_) => "memory",
            RowKind::Algebraic(_) => "algebraic",
        };
        *kinds.entry(kind).or_insert(0) += 1;
    }
    let basis = body.conserved_basis();
    let stated = body.bind(body_packet(&body)?)?.packet_hash().digest();
    let pin = format!(
        "rows {} {kinds:?}, columns {}, jacobian nonzeros {}, blocks {}, parameter slots {}, conserved combinations {}, held pools {}, held route columns {}\nmodel hash {}\nconserved basis {}\nstated packet {}\n",
        body.rows().len(),
        body.columns().len(),
        body.sparsity().unknowns().len(),
        body.blocks().len(),
        body.parameters().len(),
        basis.combinations().len(),
        basis.coverage().pools().len(),
        basis.coverage().held_routes().len(),
        hex(body.model_hash().digest()),
        hex(basis.digest()),
        hex(stated),
    );
    expect![[r#"
        rows 1449 {"algebraic": 186, "amount": 1248, "memory": 15}, columns 1458, jacobian nonzeros 10053, blocks 429, parameter slots 722, conserved combinations 399, held pools 319, held route columns 89
        model hash f1fe55e087f7235a16a3e762a20e7164afa5dd6772aab5832a76626f7e8fd3a1
        conserved basis 46b9975bf1364b8364f03f0e9b79a5b884dd7f6a3f419bf8b905493f1e9ceb3e
        stated packet b46f98edd84e77bb649165e30da175d654d4df26397a4bd10170ee4ba7298de5
    "#]].assert_eq(&pin);
    Ok(())
}

#[test]
fn the_gut_transit_reads_both_lumens_so_admission_can_place_each_species_between_them()
-> Result<(), AdmissionError> {
    let body = whole_body()?;
    let (stomach, intestine) = ("stomach", "small intestine");
    let lumen = |row: RowId| match body.rows()[row.index()].kind() {
        RowKind::Amount {
            site,
            space: Space::Lumen,
            species,
        } => Some((site.key(), species)),
        _ => None,
    };
    let transit: BTreeSet<Species> = body
        .columns()
        .iter()
        .filter_map(|column| {
            let ends: Vec<_> = column.terms().iter().filter_map(|(row, _)| lumen(*row)).collect();
            match ends[..] {
                [(a, species), (b, other)]
                    if species == other && [a, b].contains(&stomach) && [a, b].contains(&intestine) =>
                {
                    Some(species)
                }
                _ => None,
            }
        })
        .collect();
    assert_eq!(transit.len(), 12, "the gut carries {transit:?}");
    let unknown = |key: &str, species: Species| {
        let found = body
            .rows()
            .iter()
            .find(|row| lumen(row.id()) == Some((key, species)));
        found
            .unwrap_or_else(|| panic!("no {species:?} in the {key}"))
            .unknown()
    };
    let sparsity = body.sparsity();
    for species in transit {
        for key in [stomach, intestine] {
            let row = unknown(key, species).index();
            let reads = &sparsity.unknowns()[sparsity.row_offsets()[row]..sparsity.row_offsets()[row + 1]];
            for end in [stomach, intestine] {
                assert!(
                    reads.contains(&unknown(end, species)),
                    "the {key}'s {species:?} does not read the {end}'s, so the split between them is free"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn the_bodys_own_drift_bounds_are_its_stated_ones() -> Result<(), AdmissionError> {
    #[derive(Clone, Copy)]
    struct Stated {
        species: [(Species, f64); 7],
        elements: [(Element, f64); 3],
        payer_hydrogen: f64,
    }
    impl DriftTable for Stated {
        fn names(&self, species: Species) -> bool {
            self.species.iter().any(|(named, _)| *named == species)
        }
        fn drift<Q: Substance>(&self) -> Result<Measure<Q>, AdmissionError> {
            let named = self.species.iter().find(|(named, _)| *named == Q::SPECIES);
            Measure::new(named.map_or(f64::NAN, |(_, drift)| *drift))
        }
        fn element(&self, element: Element, at: &[SiteId]) -> Result<Option<Measure<Atoms>>, AdmissionError> {
            if element == Element::Hydrogen && at.iter().all(|site| site.key() == "coronary bed") {
                return Measure::new(self.payer_hydrogen).map(Some);
            }
            let named = self.elements.iter().find(|(named, _)| *named == element);
            named.map(|(_, drift)| Measure::new(*drift)).transpose()
        }
    }
    let body = whole_body()?;
    let window = core::num::NonZeroU64::new(10_000_000_000).ok_or(AdmissionError::NonFinite)?;
    let windows = SlidingWindows::new(window, 60_000_000_000)?;
    let digest = |table: &Stated| -> Result<Digest, AdmissionError> {
        Ok(certificate_with(&body, windows, table)?.digest())
    };
    let stated = Stated {
        species: [
            (Species::Energy, 1e4),
            (Species::Water, 0.04),
            (Species::Sodium, 9e-4),
            (Species::Chloride, 9e-4),
            (Species::Potassium, 5e-5),
            (Species::Vasopressin, 7e-2),
            (Species::NitrogenGas, 2e4),
        ],
        elements: [
            (Element::Carbon, 5e2),
            (Element::Hydrogen, 2e-5),
            (Element::Oxygen, 1e4),
        ],
        payer_hydrogen: 0.2,
    };
    let policy = body_certificate(&body, windows)?.digest();
    assert_eq!(policy, digest(&stated)?);
    for by in [0.5, 2.0] {
        for index in 0..stated.species.len() {
            let mut moved = stated;
            moved.species[index].1 *= by;
            let named = moved.species[index].0;
            assert_ne!(policy, digest(&moved)?, "{named:?} times {by}");
        }
        for index in 0..stated.elements.len() {
            let mut moved = stated;
            moved.elements[index].1 *= by;
            let named = moved.elements[index].0;
            assert_ne!(policy, digest(&moved)?, "{named:?} times {by}");
        }
        let moved = Stated {
            payer_hydrogen: stated.payer_hydrogen * by,
            ..stated
        };
        assert_ne!(policy, digest(&moved)?, "the payer's hydrogen times {by}");
    }
    Ok(())
}

#[test]
fn each_beds_glycolysis_and_lactate_burn_carry_their_atoms_and_energy_into_its_cells_and_heat()
-> Result<(), AdmissionError> {
    let body = whole_body()?;
    let mut seen = [0; 2];
    for column in body.columns() {
        let (which, expected): (usize, &[(Species, Space, i64)]) = match column.origin().local {
            "glycolysis" => (
                0,
                &[
                    (Species::Glucose, Space::Cells, -1),
                    (Species::Lactate, Space::Cells, 2),
                    (Species::Energy, Space::Thermal, 10_000),
                ],
            ),
            "lactate burn" => (
                1,
                &[
                    (Species::Lactate, Space::Cells, -1),
                    (Species::Oxygen, Space::Cells, -3),
                    (Species::CarbonDioxide, Space::Cells, 3),
                    (Species::Water, Space::Cells, 3),
                    (Species::Energy, Space::Thermal, 45_000),
                ],
            ),
            _ => continue,
        };
        let Context::Site(bed) = column.context() else {
            panic!("a reaction off a bed: {:?}", column.context());
        };
        let mut terms: Vec<(RowKind, i64)> = (column.terms().iter())
            .map(|(row, count)| (body.rows()[row.index()].kind(), *count))
            .collect();
        let mut wanted: Vec<(RowKind, i64)> = (expected.iter())
            .map(|&(species, space, count)| {
                (
                    RowKind::Amount {
                        site: bed,
                        species,
                        space,
                    },
                    count,
                )
            })
            .collect();
        terms.sort_by_key(|term| format!("{term:?}"));
        wanted.sort_by_key(|term| format!("{term:?}"));
        assert_eq!(terms, wanted, "{:?} at {bed:?}", column.origin());
        seen[which] += 1;
    }
    assert_eq!(seen, [11, 11]);
    Ok(())
}

#[test]
fn each_beds_burns_carry_their_fuels_atoms_and_energy_into_its_cells_and_heat() -> Result<(), AdmissionError>
{
    let body = whole_body()?;
    let mut seen = [0; 2];
    for column in body.columns() {
        let (which, fuel, expected) = match column.origin().local {
            "glucose burn" => (0, Species::Glucose, [-1, -6, 6, 6, 100_000]),
            "fatty acid burn" => (1, Species::FattyAcids, [-1, -23, 16, 16, 300_000]),
            _ => continue,
        };
        let Context::Site(bed) = column.context() else {
            panic!("a burn off a bed: {:?}", column.context());
        };
        let at = |species, space| RowKind::Amount {
            site: bed,
            species,
            space,
        };
        let rows = [
            at(fuel, Space::Cells),
            at(Species::Oxygen, Space::Cells),
            at(Species::CarbonDioxide, Space::Cells),
            at(Species::Water, Space::Cells),
            at(Species::Energy, Space::Thermal),
        ];
        let terms: Vec<(RowKind, i64)> = (column.terms().iter())
            .map(|(row, count)| (body.rows()[row.index()].kind(), *count))
            .collect();
        let wanted: Vec<(RowKind, i64)> = rows.into_iter().zip(expected).collect();
        assert_eq!(
            terms.len(),
            wanted.len(),
            "{:?} at {bed:?}: {terms:?}",
            column.origin()
        );
        for term in &wanted {
            assert!(
                terms.contains(term),
                "{:?} at {bed:?} lacks {term:?}: {terms:?}",
                column.origin()
            );
        }
        seen[which] += 1;
    }
    assert_eq!(seen, [11, 11]);
    Ok(())
}
