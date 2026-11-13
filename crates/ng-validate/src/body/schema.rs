use super::anatomy::{at, body_anatomy};
use ng_contract::*;

/// Each row places every species it names in each (site set, space) it lists.
macro_rules! placements {
    ($schema:ident; $([$($species:ident),+] in $places:tt;)+) => {
        $($(placements!(@one $schema, $species, $places);)+)+
    };
    (@one $schema:ident, $species:ident, [$($set:ty: $space:ident),+]) => {
        $($schema.carry::<$species, $set>(Space::$space);)+
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
    Ok(schema)
}
