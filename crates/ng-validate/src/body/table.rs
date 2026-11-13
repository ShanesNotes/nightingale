use ng_contract::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Held {
    /// A boundary route its stand-in serves.
    Route(&'static str),
    Pool(Species, Space, Sites),
}

/// What ends a hold: the ticket whose law replaces it and that law's card, which may differ from the holder's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Release {
    Ticket(u8, &'static str),
    /// Rung 0 keeps it.
    Kept,
}

#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub card: &'static str,
    pub held: Held,
    pub fixes: &'static str,
    pub release: Release,
    pub line: &'static str,
}

const RS: &str = Respiratory::CARD;
const GI: &str = Gastrointestinal::CARD;
const EN: &str = Endocrine::CARD;
const HP: &str = Hepatic::CARD;
const HC: &str = Hematology::CARD;
const MT: &str = Metabolism::CARD;
const MS: &str = Musculoskeletal::CARD;
const BC: &str = BloodChemistry::CARD;
const FL: &str = Fluids::CARD;
const RN: &str = Renal::CARD;

const CONSTANT: &str = "a constant rate from its packet parameter";
const MEAL: &str = "the slice's absorption: a basal rate and the meal, straight into the portal vein";
const KEPT_BACK: &str = "an interstitial amount the capillary wall keeps back";
const UNCROSSED: &str = "a cell amount no membrane law moves";
const ALVEOLAR: &str = "alveolar gas no exchange moves";

const fn route(
    card: &'static str,
    name: &'static str,
    fixes: &'static str,
    release: Release,
    line: &'static str,
) -> Hold {
    let held = Held::Route(name);
    Hold {
        card,
        held,
        fixes,
        release,
        line,
    }
}

const fn pool<Q: Substance, C: SiteSet>(
    card: &'static str,
    space: Space,
    fixes: &'static str,
    release: Release,
    line: &'static str,
) -> Hold {
    let held = Held::Pool(Q::SPECIES, space, C::SET);
    Hold {
        card,
        held,
        fixes,
        release,
        line,
    }
}

use Release::{Kept, Ticket};
use Space::{Cells, Gas, Interstitium};

/// Every hold of the whole body, by card. Its rows equal the compiled body's coverage manifest.
pub const HOLD_TABLE: &[Hold] = &[
    route(RS, "inspired oxygen", CONSTANT, Ticket(10, RS), "01:128"),
    route(RS, "inspired carbon dioxide", CONSTANT, Ticket(10, RS), "01:149"),
    route(RS, "inspired nitrogen gas", CONSTANT, Ticket(10, RS), "01:531"),
    route(RS, "expired oxygen", CONSTANT, Ticket(10, RS), "01:133"),
    route(RS, "expired carbon dioxide", CONSTANT, Ticket(10, RS), "01:150"),
    route(RS, "expired nitrogen gas", CONSTANT, Ticket(10, RS), "01:531"),
    route(RS, "expired water", CONSTANT, Ticket(10, RS), "01:63, 873"),
    route(RS, "airway heat", CONSTANT, Ticket(10, RS), "01:171-172, 873"),
    pool::<Oxygen, Beds<Lung>>(RS, Gas, ALVEOLAR, Ticket(10, RS), "01:129, 529"),
    pool::<CarbonDioxide, Beds<Lung>>(RS, Gas, ALVEOLAR, Ticket(10, RS), "01:148, 530"),
    pool::<NitrogenGas, Beds<Lung>>(RS, Gas, ALVEOLAR, Ticket(10, RS), "01:531"),
    pool::<Water, Beds<Lung>>(RS, Gas, ALVEOLAR, Ticket(10, RS), "01:356, 532"),
    route(GI, "gut water", MEAL, Ticket(13, GI), "01:54, 876"),
    route(GI, "gut sodium", MEAL, Ticket(13, GI), "01:77, 876"),
    route(GI, "gut potassium", MEAL, Ticket(13, GI), "01:94, 876"),
    route(GI, "gut chloride", MEAL, Ticket(13, GI), "01:111, 876"),
    route(GI, "gut glucose", MEAL, Ticket(13, GI), "01:187, 876"),
    route(GI, "gut triglyceride", MEAL, Ticket(13, GI), "01:198, 876"),
    route(GI, "gut amino acids", MEAL, Ticket(13, GI), "01:206, 876"),
    route(GI, "gut diet base", MEAL, Ticket(18, GI), "01:255, 424"),
    route(GI, "gut calcium", MEAL, Ticket(19, GI), "01:275, 425"),
    route(GI, "gut phosphate", MEAL, Ticket(19, GI), "01:275, 425"),
    route(GI, "gut magnesium", MEAL, Ticket(19, GI), "01:275, 425"),
    route(GI, "gut iron", MEAL, Ticket(21, GI), "01:305, 426"),
    route(GI, "feces water", CONSTANT, Ticket(13, GI), "01:64"),
    route(GI, "feces sodium", CONSTANT, Ticket(13, GI), "01:83"),
    route(GI, "feces potassium", CONSTANT, Ticket(13, GI), "01:100"),
    route(GI, "feces chloride", CONSTANT, Ticket(13, GI), "01:116"),
    route(GI, "feces glucose", CONSTANT, Ticket(13, GI), "01:211"),
    route(GI, "feces triglyceride", CONSTANT, Ticket(13, GI), "01:211"),
    route(GI, "feces amino acids", CONSTANT, Ticket(13, GI), "01:211, 240"),
    route(GI, "feces calcium", CONSTANT, Ticket(19, GI), "01:283, 293"),
    route(GI, "feces phosphate", CONSTANT, Ticket(19, GI), "01:284, 293"),
    route(GI, "feces magnesium", CONSTANT, Ticket(19, GI), "01:285, 293"),
    route(GI, "feces iron", CONSTANT, Ticket(21, GI), "01:311"),
    pool::<Triglyceride, Beds<Gut>>(GI, Interstitium, KEPT_BACK, Ticket(13, GI), "01:198, 535"),
    pool::<AminoAcids, CapillaryBed>(GI, Interstitium, KEPT_BACK, Ticket(13, FL), "01:485, 538"),
    route(EN, "insulin secretion", CONSTANT, Ticket(15, EN), "01:437, 605"),
    route(EN, "insulin clearance", CONSTANT, Ticket(15, EN), "01:605"),
    route(EN, "glucagon secretion", CONSTANT, Ticket(15, EN), "01:437, 605"),
    route(EN, "glucagon clearance", CONSTANT, Ticket(15, EN), "01:605"),
    route(EN, "renin secretion", CONSTANT, Ticket(17, EN), "01:439, 606"),
    route(EN, "renin clearance", CONSTANT, Ticket(17, EN), "01:606"),
    route(EN, "angiotensin II made", CONSTANT, Ticket(17, EN), "01:607"),
    route(EN, "angiotensin II clearance", CONSTANT, Ticket(17, EN), "01:607"),
    route(
        EN,
        "aldosterone secretion",
        CONSTANT,
        Ticket(17, EN),
        "01:439, 608",
    ),
    route(EN, "aldosterone clearance", CONSTANT, Ticket(17, EN), "01:608"),
    route(
        EN,
        "natriuretic peptide secretion",
        CONSTANT,
        Ticket(17, EN),
        "01:440, 608",
    ),
    route(
        EN,
        "natriuretic peptide clearance",
        CONSTANT,
        Ticket(17, EN),
        "01:608",
    ),
    route(
        EN,
        "parathyroid hormone secretion",
        CONSTANT,
        Ticket(19, EN),
        "01:441, 609",
    ),
    route(
        EN,
        "parathyroid hormone clearance",
        CONSTANT,
        Ticket(19, EN),
        "01:609",
    ),
    route(EN, "cortisol secretion", CONSTANT, Ticket(20, EN), "01:442, 610"),
    route(EN, "cortisol clearance", CONSTANT, Ticket(20, EN), "01:610"),
    route(
        EN,
        "vasopressin secretion",
        "the slice's logistic on the central veins' sodium",
        Ticket(17, EN),
        "01:438, 611, 880",
    ),
    route(
        EN,
        "vasopressin clearance",
        "the slice's first-order clearance",
        Ticket(17, EN),
        "01:611, 880",
    ),
    pool::<Insulin, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(15, FL), "01:485, 553"),
    pool::<Glucagon, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(15, FL), "01:485, 553"),
    pool::<AngiotensinII, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(17, FL), "01:485, 554"),
    pool::<Aldosterone, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(17, FL), "01:485, 554"),
    pool::<NatriureticPeptide, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(17, FL), "01:485, 554"),
    pool::<ParathyroidHormone, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(19, FL), "01:485, 555"),
    pool::<Cortisol, CapillaryBed>(EN, Interstitium, KEPT_BACK, Ticket(20, FL), "01:485, 556"),
    route(
        MS,
        "skin water",
        "the slice's skin water, its forcing and its stir",
        Ticket(12, MT),
        "01:651, 875",
    ),
    pool::<Magnesium, Beds<Bone>>(MS, Cells, UNCROSSED, Ticket(19, MS), "01:38, 277"),
    pool::<Phosphate, Beds<Bone>>(MS, Cells, UNCROSSED, Ticket(19, MS), "01:38, 277"),
    pool::<Magnesium, Besides<Bone>>(MS, Cells, UNCROSSED, Ticket(19, FL), "01:38, 278"),
    pool::<Phosphate, Besides<Bone>>(MS, Cells, UNCROSSED, Ticket(19, FL), "01:38, 278"),
    pool::<Calcium, Beds<Bone>>(MS, Cells, UNCROSSED, Ticket(19, MS), "01:277, 548"),
    pool::<Creatinine, Beds<Muscle>>(MS, Cells, UNCROSSED, Ticket(16, MS), "01:210, 236"),
    pool::<BodyProtein, Beds<Muscle>>(MS, Cells, UNCROSSED, Ticket(16, MS), "01:207, 879"),
    pool::<BodyProtein, Beds<Liver>>(MS, Cells, UNCROSSED, Ticket(16, MS), "01:207, 879"),
    pool::<Glycogen, Beds<Muscle>>(MS, Cells, UNCROSSED, Ticket(16, MS), "01:191, 879"),
    pool::<AminoAcids, Beds<Muscle>>(MS, Cells, UNCROSSED, Ticket(16, MS), "01:207"),
    pool::<Creatinine, CapillaryBed>(MS, Interstitium, KEPT_BACK, Ticket(16, FL), "01:485, 544"),
    pool::<Calcium, CapillaryBed>(MS, Interstitium, KEPT_BACK, Ticket(19, FL), "01:276, 293"),
    pool::<Magnesium, CapillaryBed>(MS, Interstitium, KEPT_BACK, Ticket(19, FL), "01:276, 293"),
    pool::<Phosphate, CapillaryBed>(MS, Interstitium, KEPT_BACK, Ticket(19, FL), "01:276, 293"),
    pool::<Water, CapillaryBed>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:58, 880"),
    pool::<Sodium, CapillaryBed>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:79"),
    pool::<Potassium, Beds<Muscle>>(BC, Cells, UNCROSSED, Ticket(15, FL), "01:96, 878"),
    pool::<Potassium, Beds<Fat>>(BC, Cells, UNCROSSED, Ticket(15, FL), "01:96, 878"),
    pool::<Potassium, Beds<Lung>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Gut>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Glomerulus>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Tubule>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Brain>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Heart>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Liver>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Skin>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<Potassium, Beds<Bone>>(BC, Cells, UNCROSSED, Ticket(17, FL), "01:97"),
    pool::<CellAnion, CapillaryBed>(BC, Cells, "a cell amount no law ever moves", Kept, "01:120, 547"),
    pool::<DietBase, CapillaryBed>(BC, Cells, UNCROSSED, Ticket(18, MT), "01:256, 881"),
    pool::<Ammonium, Beds<Tubule>>(BC, Cells, UNCROSSED, Ticket(18, RN), "01:209, 235, 543"),
    pool::<AminoAcids, Beds<Tubule>>(BC, Cells, UNCROSSED, Ticket(18, RN), "01:209"),
    pool::<DietBase, CapillaryBed>(BC, Interstitium, KEPT_BACK, Ticket(18, FL), "01:546, 881"),
    pool::<FixedAcid, CapillaryBed>(BC, Interstitium, KEPT_BACK, Ticket(18, FL), "01:253, 881"),
    route(MT, "skin evaporation", CONSTANT, Ticket(12, MT), "01:61"),
    route(MT, "sweat water", CONSTANT, Ticket(12, MT), "01:62"),
    route(MT, "sweat sodium", CONSTANT, Ticket(12, MT), "01:82"),
    route(MT, "sweat chloride", CONSTANT, Ticket(12, MT), "01:115"),
    route(MT, "dry heat loss", CONSTANT, Ticket(12, MT), "01:170"),
    route(MT, "skin evaporative heat", CONSTANT, Ticket(12, MT), "01:171"),
    pool::<Oxygen, CapillaryBed>(MT, Interstitium, KEPT_BACK, Ticket(8, MT), "01:131, 871"),
    pool::<CarbonDioxide, CapillaryBed>(MT, Interstitium, KEPT_BACK, Ticket(8, MT), "01:146, 871"),
    pool::<Glucose, CapillaryBed>(MT, Interstitium, KEPT_BACK, Ticket(9, FL), "01:485, 533"),
    pool::<FattyAcids, CapillaryBed>(MT, Interstitium, KEPT_BACK, Ticket(9, FL), "01:485, 536"),
    pool::<Lactate, CapillaryBed>(MT, Interstitium, KEPT_BACK, Ticket(9, FL), "01:485, 540"),
    pool::<Oxygen, CapillaryBed>(MT, Cells, UNCROSSED, Ticket(8, MT), "01:131, 871"),
    pool::<CarbonDioxide, CapillaryBed>(MT, Cells, UNCROSSED, Ticket(8, MT), "01:38, 871, 08:19"),
    pool::<Glucose, Besides<Liver>>(MT, Cells, UNCROSSED, Ticket(9, MT), "01:190"),
    pool::<FattyAcids, Besides<Liver>>(MT, Cells, UNCROSSED, Ticket(9, MT), "01:200"),
    pool::<Lactate, Besides<Liver>>(MT, Cells, UNCROSSED, Ticket(9, MT), "01:194-196"),
    pool::<Triglyceride, Beds<Fat>>(MT, Cells, UNCROSSED, Ticket(16, MT), "01:199, 879"),
    pool::<Glycerol, Beds<Fat>>(MT, Cells, UNCROSSED, Ticket(16, MT), "01:199"),
    route(HP, "insulin first pass", CONSTANT, Ticket(15, HP), "01:408, 605"),
    route(
        HP,
        "albumin synthesis",
        CONSTANT,
        Ticket(21, HP),
        "01:312, 407, 884",
    ),
    route(HP, "albumin loss", CONSTANT, Ticket(21, HP), "01:314, 407, 884"),
    route(HP, "fixed acid made", CONSTANT, Ticket(18, HP), "01:252, 406"),
    pool::<Glucose, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:189, 877"),
    pool::<Lactate, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:196"),
    pool::<FattyAcids, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(16, HP), "01:202"),
    pool::<Glycerol, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:204"),
    pool::<AminoAcids, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:208"),
    pool::<Urea, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:234, 877"),
    pool::<Glycogen, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(14, HP), "01:188, 877"),
    pool::<Ketones, Beds<Liver>>(HP, Cells, UNCROSSED, Ticket(16, HP), "01:38, 201-202, 879"),
    pool::<Ketones, Besides<Liver>>(HP, Cells, UNCROSSED, Ticket(16, MT), "01:38, 203, 879"),
    pool::<Glycerol, CapillaryBed>(HP, Interstitium, KEPT_BACK, Ticket(14, FL), "01:485, 537"),
    pool::<Ketones, CapillaryBed>(HP, Interstitium, KEPT_BACK, Ticket(16, FL), "01:485, 541"),
    pool::<Urea, CapillaryBed>(HP, Interstitium, KEPT_BACK, Ticket(14, FL), "01:485, 542"),
    route(
        HC,
        "hemoglobin production",
        CONSTANT,
        Ticket(21, HC),
        "01:301, 884",
    ),
    route(
        HC,
        "hemoglobin destruction",
        CONSTANT,
        Ticket(21, HC),
        "01:303, 884",
    ),
    pool::<Iron, Beds<Liver>>(HC, Cells, UNCROSSED, Ticket(21, HC), "01:307-308, 884"),
    pool::<Iron, Beds<Gut>>(HC, Cells, UNCROSSED, Ticket(21, HC), "01:309, 884"),
    pool::<Iron, Beds<Liver>>(HC, Interstitium, KEPT_BACK, Ticket(21, FL), "01:306"),
    pool::<Iron, Beds<Gut>>(HC, Interstitium, KEPT_BACK, Ticket(21, FL), "01:306"),
];
