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
            Blood: Plasma, Blood: Lumen, CapillaryBed: Interstitium, Besides<Brain>: Cells
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
    }

    for space in [Space::Vesicle, Space::Cleft, Space::Bound] {
        schema.pool::<Acetylcholine>(site::<Junction>("sinoatrial junction")?, space)?;
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
    boundaries(&mut schema)?;
    Ok(schema)
}

/// The meal's twelve routes into the portal vein (01:876), the slice's skin water, drain and heat exit, and each
/// typed process where 01:51-314 and 01:603-611 put it. Model defaults where body 01 names a site and no space: the
/// insulin first pass at the liver bed's plasma, and the skin's water and salt exits at the skin bed's interstitium,
/// as the slice's skin water.
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

    let veins = site::<VesselSegment>("central veins")?;
    let glomerular = site::<CapillaryBed>("glomerular bed")?;
    let lung = site::<CapillaryBed>("pulmonary bed")?;
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
