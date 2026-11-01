use ng_contract::*;

fn at(axial: f64) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(axial)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// Grouped as record 0002 asks. Heights are synthetic. An edge into a bed stands for its arterioles, and the
/// edge from the systemic arteries into the glomerular bed is the renal afferent edge.
pub fn arena_anatomy() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let heart = map.site::<TissueSite>("heart", at(1.0)?);
    let kidney = map.site::<TissueSite>("kidney", at(0.5)?);
    let carotid = map.site::<TissueSite>("carotid sinus", at(1.4)?);
    let arch = map.site::<TissueSite>("aortic arch", at(1.1)?);

    let right_atrium = map.site::<VesselSegment>("right atrium", at(1.0)?);
    let right_ventricle = map.site::<VesselSegment>("right ventricle", at(1.0)?);
    let left_atrium = map.site::<VesselSegment>("left atrium", at(1.0)?);
    let left_ventricle = map.site::<VesselSegment>("left ventricle", at(1.0)?);
    let pulmonary_artery = map.site::<VesselSegment>("pulmonary artery", at(1.1)?);
    let pulmonary_vein = map.site::<VesselSegment>("pulmonary vein", at(1.1)?);
    let arteries = map.site::<VesselSegment>("systemic arteries", at(1.0)?);
    let central_veins = map.site::<VesselSegment>("central veins", at(1.0)?);
    let splanchnic_veins = map.site::<VesselSegment>("splanchnic veins", at(0.6)?);
    let leg_veins = map.site::<VesselSegment>("leg veins", at(0.2)?);

    let lung = map.site::<CapillaryBed>("pulmonary bed", at(1.2)?);
    let splanchnic = map.site::<CapillaryBed>("splanchnic bed", at(0.6)?);
    let leg = map.site::<CapillaryBed>("leg bed", at(0.2)?);
    let glomerular = map.site::<CapillaryBed>("glomerular bed", at(0.5)?);
    let peritubular = map.site::<CapillaryBed>("peritubular bed", at(0.5)?);

    let pleura = map.site::<EnclosedSpace>("pleural space", at(1.2)?);
    let abdomen = map.site::<EnclosedSpace>("abdominal space", at(0.6)?);

    let sinoatrial = map.site::<Junction>("sinoatrial junction", at(1.0)?);
    let myocardial = map.site::<Junction>("myocardial junction", at(1.0)?);
    let arteriolar = map.site::<Junction>("arteriolar junction", at(0.6)?);
    let venous = map.site::<Junction>("venous junction", at(0.6)?);
    let renal = map.site::<Junction>("renal junction", at(0.5)?);

    let cranial = map.site::<NerveSegment>("cranial segment", at(1.5)?);

    for site in [
        heart.id(),
        kidney.id(),
        carotid.id(),
        arch.id(),
        pulmonary_artery.id(),
        pulmonary_vein.id(),
        arteries.id(),
        central_veins.id(),
        splanchnic_veins.id(),
        leg_veins.id(),
        lung.id(),
        splanchnic.id(),
        leg.id(),
        pleura.id(),
        abdomen.id(),
        arteriolar.id(),
        venous.id(),
        cranial.id(),
    ] {
        map.contain(body.id(), site);
    }
    for site in [
        right_atrium.id(),
        right_ventricle.id(),
        left_atrium.id(),
        left_ventricle.id(),
        sinoatrial.id(),
        myocardial.id(),
    ] {
        map.contain(heart.id(), site);
    }
    for site in [glomerular.id(), peritubular.id(), renal.id()] {
        map.contain(kidney.id(), site);
    }

    for (from, to) in [
        (right_atrium.id(), right_ventricle.id()),
        (right_ventricle.id(), pulmonary_artery.id()),
        (pulmonary_artery.id(), lung.id()),
        (lung.id(), pulmonary_vein.id()),
        (pulmonary_vein.id(), left_atrium.id()),
        (left_atrium.id(), left_ventricle.id()),
        (left_ventricle.id(), arteries.id()),
        (arteries.id(), splanchnic.id()),
        (splanchnic.id(), splanchnic_veins.id()),
        (splanchnic_veins.id(), central_veins.id()),
        (arteries.id(), leg.id()),
        (leg.id(), leg_veins.id()),
        (leg_veins.id(), central_veins.id()),
        (arteries.id(), glomerular.id()),
        (glomerular.id(), peritubular.id()),
        (peritubular.id(), central_veins.id()),
        (central_veins.id(), right_atrium.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }

    let pleural = Compliance::Held {
        card: Respiratory::CARD,
    };
    let abdominal = Compliance::Held {
        card: Gastrointestinal::CARD,
    };
    map.enclose(&pleura, &[lung.id()], pleural);
    map.enclose(&abdomen, &[splanchnic.id()], abdominal);

    map.served_by(carotid.id(), &cranial, Afferent::Stretch);
    map.served_by(arch.id(), &cranial, Afferent::Stretch);

    map.region("body", body.id());
    map.region("heart", heart.id());
    map.region("kidney", kidney.id());
    map.finish()
}

const FIXED: &str = "arena fixture: synthetic, with no feedback from the slice";

fn hours() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

fn enclosure<S: OutsideSlice>(body: &mut Assembly<'_>, key: ParameterKey) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<S>::new(Vec::new(), Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let held = laws.parameter::<Pressure>(key, Admissible::finite());
    let class = hours()?;
    let pressure = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&ports.enclosure_pressure, pressure);
    body.add(laws.finish());
    Ok(())
}

fn held<S: OutsideSlice>(body: &mut Assembly<'_>, holds: Vec<Space>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<S>::new(Vec::new(), holds, FIXED);
    let (laws, _ports) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}

/// RS and GI hold their enclosures' pressure, packet values under `rs.pleural-pressure` and
/// `gi.abdominal-pressure`. BC holds cell water, while cell-water shifts stay an open conflict with FL.
pub fn hold_outside_slice(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    enclosure::<Respiratory>(body, ParameterKey::new("rs.pleural-pressure"))?;
    enclosure::<Gastrointestinal>(body, ParameterKey::new("gi.abdominal-pressure"))?;
    held::<BloodChemistry>(body, vec![Space::Cells])?;
    held::<Metabolism>(body, Vec::new())?;
    held::<Musculoskeletal>(body, Vec::new())?;
    held::<Endocrine>(body, Vec::new())?;
    held::<Hepatic>(body, Vec::new())?;
    held::<Immune>(body, Vec::new())?;
    held::<Hematology>(body, Vec::new())
}
