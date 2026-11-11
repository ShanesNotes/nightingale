//! The whole body's places and routes on a synthetic map: which sites each set holds, and where each route family
//! a port requests stands.

use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// The skin bed sits in the skin heat site, every other site in the body. The bare bed serves no tissue.
fn whole_body() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at()?);
    let airway = map.site::<TissueSite>("conducting airway", at()?);
    let skin = map.site::<TissueSite>("skin heat site", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let stomach = map.site::<VesselSegment>("stomach", at()?);
    let intestine = map.site::<VesselSegment>("small intestine", at()?);
    let lung = map.site::<CapillaryBed>("lung bed", at()?);
    let liver = map.site::<CapillaryBed>("liver bed", at()?);
    let gut = map.site::<CapillaryBed>("gut bed", at()?);
    let skin_bed = map.site::<CapillaryBed>("skin bed", at()?);
    let bare = map.site::<CapillaryBed>("bare bed", at()?);
    let beds = [lung.id(), liver.id(), gut.id(), skin_bed.id(), bare.id()];
    for site in [airway.id(), skin.id(), vessel.id(), stomach.id(), intestine.id()] {
        map.contain(body.id(), site);
    }
    for bed in beds {
        let holder = if bed == skin_bed.id() {
            skin.id()
        } else {
            body.id()
        };
        map.contain(holder, bed);
        map.connect(Network::Vascular, vessel.id(), bed);
        map.connect(Network::Vascular, bed, vessel.id());
    }
    for (bed, tissue) in [
        (&lung, Tissue::Lung),
        (&liver, Tissue::Liver),
        (&gut, Tissue::Gut),
        (&skin_bed, Tissue::Skin),
    ] {
        map.serves(bed, tissue);
    }
    map.connect(Network::Airway, airway.id(), lung.id());
    map.connect(Network::Gut, stomach.id(), intestine.id());
    map.connect(Network::Gut, intestine.id(), gut.id());
    map.finish()
}

fn holders(schema: &Schema, species: Species) -> Vec<(&'static str, Space)> {
    (schema.pools())
        .filter(|pool| pool.2 == species)
        .map(|(site, space, _)| (site.key(), space))
        .collect()
}

#[test]
fn each_site_set_holds_the_sites_the_map_places_on_it() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    schema.carry::<Glucose, Alimentary>(Space::Lumen);
    schema.carry::<NitrogenGas, Airway>(Space::Gas);
    schema.carry::<Glycogen, Beds<Liver>>(Space::Cells);
    schema.carry::<BodyProtein, Besides<Liver>>(Space::Cells);
    assert_eq!(
        holders(&schema, Species::Glucose),
        [("small intestine", Space::Lumen), ("stomach", Space::Lumen)]
    );
    assert_eq!(
        holders(&schema, Species::NitrogenGas),
        [("conducting airway", Space::Gas), ("lung bed", Space::Gas)]
    );
    assert_eq!(holders(&schema, Species::Glycogen), [("liver bed", Space::Cells)]);
    assert_eq!(
        holders(&schema, Species::BodyProtein),
        [
            ("bare bed", Space::Cells),
            ("gut bed", Space::Cells),
            ("lung bed", Space::Cells),
            ("skin bed", Space::Cells),
        ]
    );
    Ok(())
}
