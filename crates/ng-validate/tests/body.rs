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
            "the slice's body stores, 01:568 skin heat store, 06:31 the airway's heat store",
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

#[test]
fn the_whole_body_compiles_on_its_holds() {
    if let Err(refused) = whole_body() {
        panic!("{refused:?}");
    }
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
        RS Route("inspired oxygen") Ticket(10, "RS") 01:128
        RS Route("inspired carbon dioxide") Ticket(10, "RS") 01:149
        RS Route("inspired nitrogen gas") Ticket(10, "RS") 01:531
        RS Route("expired oxygen") Ticket(10, "RS") 01:133
        RS Route("expired carbon dioxide") Ticket(10, "RS") 01:150
        RS Route("expired nitrogen gas") Ticket(10, "RS") 01:531
        RS Route("expired water") Ticket(10, "RS") 01:63, 873
        RS Route("airway heat") Ticket(10, "RS") 01:171-172, 873
        RS Pool(Oxygen, Gas, Tissue(Lung)) Ticket(10, "RS") 01:129, 529
        RS Pool(CarbonDioxide, Gas, Tissue(Lung)) Ticket(10, "RS") 01:148, 530
        RS Pool(NitrogenGas, Gas, Tissue(Lung)) Ticket(10, "RS") 01:531
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
        BC Pool(Water, Cells, Class(CapillaryBed)) Ticket(17, "FL") 01:58, 880
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
        MT Pool(Oxygen, Interstitium, Class(CapillaryBed)) Ticket(8, "MT") 01:131, 871
        MT Pool(CarbonDioxide, Interstitium, Class(CapillaryBed)) Ticket(8, "MT") 01:146, 871
        MT Pool(Glucose, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 533
        MT Pool(FattyAcids, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 536
        MT Pool(Lactate, Interstitium, Class(CapillaryBed)) Ticket(9, "FL") 01:485, 540
        MT Pool(Oxygen, Cells, Class(CapillaryBed)) Ticket(8, "MT") 01:131, 871
        MT Pool(CarbonDioxide, Cells, Class(CapillaryBed)) Ticket(8, "MT") 01:38, 871, 08:19
        MT Pool(Glucose, Cells, Besides(Liver)) Ticket(9, "MT") 01:190
        MT Pool(FattyAcids, Cells, Besides(Liver)) Ticket(9, "MT") 01:200
        MT Pool(Lactate, Cells, Besides(Liver)) Ticket(9, "MT") 01:194-196
        MT Pool(Triglyceride, Cells, Tissue(Fat)) Ticket(16, "MT") 01:199, 879
        MT Pool(Glycerol, Cells, Tissue(Fat)) Ticket(16, "MT") 01:199
        HP Route("insulin first pass") Ticket(15, "HP") 01:408, 605
        HP Route("albumin synthesis") Ticket(21, "HP") 01:312, 407, 884
        HP Route("albumin loss") Ticket(21, "HP") 01:314, 407, 884
        HP Route("fixed acid made") Ticket(18, "HP") 01:252, 406
        HP Pool(Glucose, Cells, Tissue(Liver)) Ticket(14, "HP") 01:189, 877
        HP Pool(Lactate, Cells, Tissue(Liver)) Ticket(14, "HP") 01:196
        HP Pool(FattyAcids, Cells, Tissue(Liver)) Ticket(16, "HP") 01:202
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
        holds 136: routes 62, pools 74
        by card {"BC": 19, "EN": 25, "GI": 25, "HC": 6, "HP": 16, "MS": 15, "MT": 18, "RS": 12}
        by ticket {"08": 4, "09": 6, "10": 12, "12": 7, "13": 16, "14": 8, "15": 9, "16": 12, "17": 24, "18": 7, "19": 17, "20": 3, "21": 10, "kept": 1}
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
    let pin = format!(
        "rows {} {kinds:?}, columns {}, conserved combinations {}, held pools {}, held route columns {}\nmodel hash {}\nconserved basis {}\n",
        body.rows().len(),
        body.columns().len(),
        basis.combinations().len(),
        basis.coverage().pools().len(),
        basis.coverage().held_routes().len(),
        hex(body.model_hash().digest()),
        hex(basis.digest()),
    );
    expect![[r#"
        rows 1372 {"algebraic": 126, "amount": 1236, "memory": 10}, columns 1288, conserved combinations 480, held pools 409, held route columns 62
        model hash dfbd80cb51946e1c2e348a0485c2e6a4e1c2d56b7f647b4c7bd3175760902884
        conserved basis b709d2da159c9858a6d354b4b2145cb46845e18e809d0180814671f21d0ef03f
    "#]].assert_eq(&pin);
    Ok(())
}
