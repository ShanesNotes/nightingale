use ng_contract::*;

// The signature walk's step 0 (docs/architecture/solver-contract-sketch/src/signature_walk.rs), with
// only the crate path changed.
fn anatomy() -> Result<Schema, AdmissionError> {
    let at = |axial: f64| -> Result<Position, AdmissionError> {
        // synthetic heights
        Ok(Position::new(
            Measure::new(axial)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let kidney = map.site::<TissueSite>("kidney", at(0.5)?);
    let heart = map.site::<VesselSegment>("heart", at(1.0)?);
    let glomerular = map.site::<CapillaryBed>("glomerular bed", at(0.5)?);
    let peritubular = map.site::<CapillaryBed>("peritubular bed", at(0.5)?);
    let lung = map.site::<CapillaryBed>("pulmonary bed", at(1.2)?);
    let pleura = map.site::<EnclosedSpace>("pleural space", at(1.2)?);
    let thoracic = map.site::<NerveSegment>("thoracic segment", at(1.1)?);
    for site in [kidney.id(), heart.id(), lung.id(), pleura.id(), thoracic.id()] {
        map.contain(body.id(), site);
    }
    map.contain(kidney.id(), glomerular.id());
    map.contain(kidney.id(), peritubular.id());
    for (from, to) in [
        (heart.id(), glomerular.id()),
        (glomerular.id(), peritubular.id()),
        (peritubular.id(), heart.id()),
    ] {
        map.connect(Network::Vascular, from, to); // two beds in series, closing a loop
    }
    map.connect(Network::Vascular, heart.id(), lung.id());
    map.connect(Network::Vascular, lung.id(), heart.id());
    let pleural = Compliance::Elastic {
        elastance: ParameterKey::new("pleura.elastance"),
        unstressed: ParameterKey::new("pleura.unstressed"),
    };
    map.enclose(&pleura, &[lung.id()], pleural);
    map.served_by(lung.id(), &thoracic, Afferent::Nociceptive);
    let _kidney: RegionId = map.region("kidney", kidney.id());
    Schema::new(map.finish()?)
}

#[test]
fn the_walks_anatomy_passes_every_check() {
    assert!(anatomy().is_ok());
}
