use super::anatomy::{at, body_anatomy};
use ng_contract::*;

macro_rules! placements {
    ($schema:ident; $([$($species:ident),+] in $places:tt;)+) => {
        $($(placements!(@one $schema, $species, $places);)+)+
    };
    (@one $schema:ident, $species:ident, [$($set:ty: $space:ident),+]) => {
        $($schema.carry::<$species, $set>(Space::$space);)+
    };
}

macro_rules! processes {
    ($schema:ident; $($site:ident in $space:ident: [$($process:ty),+ $(,)?];)+) => {
        $($($schema.process::<$process>($site, Space::$space)?;)+)+
    };
}

/// Each fuel's energy content, plainly synthetic.
pub(super) const GLUCOSE_ENERGY: f64 = 100_000.0;
pub(super) const FAT_ENERGY: f64 = 300_000.0;
pub(super) const LACTATE_ENERGY: f64 = 45_000.0;

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    Ok(AnatomyBuilder::new().site::<C>(key, at(0.0)?).id())
}

/// Which species live where in the whole body, with no boundary and no process: water's watery spaces
/// come with the schema, and a species lives in cells only at the beds whose cells some rung 0 law
/// moves it in.
pub fn body_schema() -> Result<Schema, AdmissionError> {
    let mut schema = Schema::new(body_anatomy()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    placements! {schema;
        [Sodium, Potassium, Glucose, Magnesium, Phosphate] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells, Urinary: Lumen,
            Alimentary: Lumen
        ];
        [Lactate, Ketones] in [Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells];
        [DietBase] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells, Alimentary: Lumen
        ];
        [Oxygen] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells, Airway: Gas
        ];
        [CarbonDioxide] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells, Urinary: Lumen,
            Airway: Gas
        ];
        [Water, NitrogenGas] in [Airway: Gas];
        [Chloride] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Urinary: Lumen, Alimentary: Lumen
        ];
        [Calcium] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Urinary: Lumen, Alimentary: Lumen,
            Beds<Bone>: Cells
        ];
        [AminoAcids] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Urinary: Lumen, Alimentary: Lumen,
            Beds<Muscle>: Cells, Beds<Liver>: Cells, Beds<Tubule>: Cells
        ];
        [FattyAcids] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, CapillaryBed: Cells
        ];
        [Glycerol] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Beds<Fat>: Cells, Beds<Liver>: Cells
        ];
        [Urea, FixedAcid] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Urinary: Lumen, Beds<Liver>: Cells
        ];
        [Creatinine] in [
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Urinary: Lumen, Beds<Muscle>: Cells
        ];
        [Ammonium] in [Urinary: Lumen, Beds<Tubule>: Cells];
        [CellAnion] in [CapillaryBed: Cells];
        [Glycogen, BodyProtein] in [Beds<Liver>: Cells, Beds<Muscle>: Cells];
        [Triglyceride] in [
            Blood: Plasma, Blood: Lumen, Alimentary: Lumen, Beds<Gut>: Interstitium, Beds<Fat>: Cells
        ];
        [Iron] in [
            Blood: Plasma, Blood: Lumen, Alimentary: Lumen, Beds<Liver>: Interstitium,
            Beds<Gut>: Interstitium, Beds<Liver>: Cells, Beds<Gut>: Cells
        ];
        [Hemoglobin, Renin, Vasopressin] in [Blood: Plasma, Blood: Lumen];
        [
            Albumin, Insulin, Glucagon, AngiotensinII, Aldosterone, NatriureticPeptide, ParathyroidHormone,
            Cortisol
        ] in [Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium];
        [Norepinephrine] in [Junction: Vesicle, Junction: Cleft, Junction: Bound];
        [Energy] in [CapillaryBed: Thermal];
    }
    schema.energy_content::<Glucose>(Measure::new(GLUCOSE_ENERGY)?)?;
    schema.energy_content::<FattyAcids>(Measure::new(FAT_ENERGY)?)?;
    schema.energy_content::<Lactate>(Measure::new(LACTATE_ENERGY)?)?;

    for space in [Space::Vesicle, Space::Cleft, Space::Bound] {
        schema.pool::<Acetylcholine>(site::<Junction>("sinoatrial junction")?, space)?;
        schema.pool::<Acetylcholine>(site::<Junction>("respiratory pump junction")?, space)?;
    }
    schema.pool::<Vasopressin>(site::<CapillaryBed>("peritubular bed")?, Space::Bound)?;
    let body = site::<TissueSite>("body")?;
    schema.pool::<Energy>(body, Space::Elastic)?;
    for heat in [
        body,
        site::<TissueSite>("skin heat site")?,
        site::<TissueSite>("conducting airway")?,
    ] {
        schema.pool::<Energy>(heat, Space::Thermal)?;
    }
    schema.pay_work(Work::Stroke, site::<CapillaryBed>("coronary bed")?)?;
    schema.pay_work(Work::Walls, site::<CapillaryBed>("muscle bed")?)?;
    boundaries(&mut schema)?;
    Ok(schema)
}

pub(super) const BEDS: [&str; 11] = [
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
pub(super) const GLUCOSE_FEED: [&str; 11] = [
    "pulmonary bed glucose feed",
    "gut bed glucose feed",
    "muscle bed glucose feed",
    "glomerular bed glucose feed",
    "peritubular bed glucose feed",
    "brain bed glucose feed",
    "coronary bed glucose feed",
    "liver bed glucose feed",
    "fat bed glucose feed",
    "skin bed glucose feed",
    "bone bed glucose feed",
];
pub(super) const FAT_FEED: [&str; 11] = [
    "pulmonary bed fatty acid feed",
    "gut bed fatty acid feed",
    "muscle bed fatty acid feed",
    "glomerular bed fatty acid feed",
    "peritubular bed fatty acid feed",
    "brain bed fatty acid feed",
    "coronary bed fatty acid feed",
    "liver bed fatty acid feed",
    "fat bed fatty acid feed",
    "skin bed fatty acid feed",
    "bone bed fatty acid feed",
];
pub(super) const CELL_WATER_OUT: [&str; 11] = [
    "pulmonary bed cell water out",
    "gut bed cell water out",
    "muscle bed cell water out",
    "glomerular bed cell water out",
    "peritubular bed cell water out",
    "brain bed cell water out",
    "coronary bed cell water out",
    "liver bed cell water out",
    "fat bed cell water out",
    "skin bed cell water out",
    "bone bed cell water out",
];

pub(super) const OXYGEN_LEAK: &str = "oxygen leak";
pub(super) const CARBON_LEAK: &str = "carbon dioxide leak";
pub(super) const NITROGEN_LEAK: &str = "nitrogen gas leak";
pub(super) const VASOPRESSIN_LEAK: &str = "vasopressin leak";

/// The routes of the body's four test leaks: oxygen and carbon dioxide into the arteries' blood, nitrogen gas into
/// the pulmonary bed's gas, and vasopressin into the central veins.
pub(super) fn leaks(schema: &mut Schema) -> Result<(), AdmissionError> {
    let into = BoundaryDirection::IntoBody;
    let arteries = site::<VesselSegment>("systemic arteries")?;
    schema.boundary::<Oxygen>(OXYGEN_LEAK, into, arteries, Space::Lumen)?;
    schema.boundary::<CarbonDioxide>(CARBON_LEAK, into, arteries, Space::Lumen)?;
    let lung = site::<CapillaryBed>("pulmonary bed")?;
    schema.boundary::<NitrogenGas>(NITROGEN_LEAK, into, lung, Space::Gas)?;
    let veins = site::<VesselSegment>("central veins")?;
    schema.boundary::<Vasopressin>(VASOPRESSIN_LEAK, into, veins, Space::Lumen)
}

fn boundaries(schema: &mut Schema) -> Result<(), AdmissionError> {
    let portal = site::<VesselSegment>("portal vein")?;
    let into = BoundaryDirection::IntoBody;
    schema.boundary::<Water>("gut water", into, portal, Space::Lumen)?;
    schema.boundary::<Sodium>("gut sodium", into, portal, Space::Lumen)?;
    schema.boundary::<Potassium>("gut potassium", into, portal, Space::Lumen)?;
    schema.boundary::<Chloride>("gut chloride", into, portal, Space::Lumen)?;
    schema.boundary::<Glucose>("gut glucose", into, portal, Space::Lumen)?;
    schema.boundary::<Triglyceride>("gut triglyceride", into, portal, Space::Lumen)?;
    schema.boundary::<AminoAcids>("gut amino acids", into, portal, Space::Lumen)?;
    schema.boundary::<DietBase>("gut diet base", into, portal, Space::Lumen)?;
    schema.boundary::<Calcium>("gut calcium", into, portal, Space::Lumen)?;
    schema.boundary::<Phosphate>("gut phosphate", into, portal, Space::Lumen)?;
    schema.boundary::<Magnesium>("gut magnesium", into, portal, Space::Lumen)?;
    schema.boundary::<Iron>("gut iron", into, portal, Space::Lumen)?;
    let out = BoundaryDirection::OutOfBody;
    let muscle = site::<CapillaryBed>("muscle bed")?;
    schema.boundary::<Water>("skin water", out, muscle, Space::Interstitium)?;
    schema.drain("urine", site::<VesselSegment>("bladder")?, Space::Lumen)?;
    schema.boundary::<Energy>("heat", out, site::<TissueSite>("body")?, Space::Thermal)?;
    for (((bed, glucose), fat), water) in BEDS
        .into_iter()
        .zip(GLUCOSE_FEED)
        .zip(FAT_FEED)
        .zip(CELL_WATER_OUT)
    {
        let bed = site::<CapillaryBed>(bed)?;
        schema.boundary::<Glucose>(glucose, into, bed, Space::Cells)?;
        schema.boundary::<FattyAcids>(fat, into, bed, Space::Cells)?;
        schema.boundary::<Water>(water, out, bed, Space::Cells)?;
    }
    let lung = site::<CapillaryBed>("pulmonary bed")?;

    let veins = site::<VesselSegment>("central veins")?;
    let glomerular = site::<CapillaryBed>("glomerular bed")?;
    let liver = site::<CapillaryBed>("liver bed")?;
    let gut = site::<CapillaryBed>("gut bed")?;
    let intestine = site::<VesselSegment>("small intestine")?;
    let skin = site::<CapillaryBed>("skin bed")?;
    let skin_heat = site::<TissueSite>("skin heat site")?;
    let airway = site::<TissueSite>("conducting airway")?;
    processes! {schema;
        portal in Lumen: [Secretion<Insulin>, Secretion<Glucagon>];
        glomerular in Plasma: [Secretion<Renin>];
        lung in Plasma: [AngiotensinIIMade];
        veins in Lumen: [
            Clearance<Insulin>, Clearance<Glucagon>, Clearance<Renin>, Clearance<AngiotensinII>,
            Secretion<Aldosterone>, Clearance<Aldosterone>, Secretion<NatriureticPeptide>,
            Clearance<NatriureticPeptide>, Secretion<ParathyroidHormone>, Clearance<ParathyroidHormone>,
            Secretion<Cortisol>, Clearance<Cortisol>, Secretion<Vasopressin>, Clearance<Vasopressin>,
            HemoglobinProduction
        ];
        liver in Plasma: [InsulinFirstPass, AlbuminSynthesis, AlbuminLoss];
        liver in Cells: [FixedAcidMade];
        gut in Plasma: [HemoglobinDestruction];
        intestine in Lumen: [
            Feces<Water>, Feces<Sodium>, Feces<Potassium>, Feces<Chloride>, Feces<Glucose>, Feces<Triglyceride>,
            Feces<AminoAcids>, Feces<Calcium>, Feces<Phosphate>, Feces<Magnesium>, Feces<Iron>
        ];
        skin in Interstitium: [SkinEvaporation, Sweat<Water>, Sweat<Sodium>, Sweat<Chloride>];
        skin_heat in Thermal: [DryHeatLoss, SkinEvaporativeHeat];
        airway in Gas: [
            Inspired<Oxygen>, Inspired<CarbonDioxide>, Inspired<NitrogenGas>, Expired<Oxygen>,
            Expired<CarbonDioxide>, Expired<NitrogenGas>, Expired<Water>
        ];
        airway in Thermal: [AirwayHeat];
    }
    Ok(())
}
