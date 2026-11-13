use ng_contract::*;

pub(super) fn at(axial: f64) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(axial)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// The slice's map with the sites the whole body adds. Heights are synthetic and
/// run from the hip. An edge into a bed stands for its arterioles, and the gut bed drains through the
/// portal vein into the liver bed.
pub fn body_anatomy() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let heart = map.site::<TissueSite>("heart", at(0.5)?);
    let kidney = map.site::<TissueSite>("kidney", at(0.38)?);
    let carotid = map.site::<TissueSite>("carotid sinus", at(0.7)?);
    let arch = map.site::<TissueSite>("aortic arch", at(0.55)?);
    let skin_heat = map.site::<TissueSite>("skin heat site", at(0.1)?);
    let airway = map.site::<TissueSite>("conducting airway", at(0.65)?);

    let right_atrium = map.site::<VesselSegment>("right atrium", at(0.5)?);
    let right_ventricle = map.site::<VesselSegment>("right ventricle", at(0.5)?);
    let left_atrium = map.site::<VesselSegment>("left atrium", at(0.5)?);
    let left_ventricle = map.site::<VesselSegment>("left ventricle", at(0.5)?);
    let pulmonary_artery = map.site::<VesselSegment>("pulmonary artery", at(0.55)?);
    let pulmonary_vein = map.site::<VesselSegment>("pulmonary vein", at(0.55)?);
    let arteries = map.site::<VesselSegment>("systemic arteries", at(0.5)?);
    let central_veins = map.site::<VesselSegment>("central veins", at(0.5)?);
    let portal_vein = map.site::<VesselSegment>("portal vein", at(0.25)?);
    let leg_veins = map.site::<VesselSegment>("leg veins", at(-0.5)?);

    let lung = map.site::<CapillaryBed>("pulmonary bed", at(0.55)?);
    let gut = map.site::<CapillaryBed>("gut bed", at(0.25)?);
    let muscle = map.site::<CapillaryBed>("muscle bed", at(-0.5)?);
    let glomerular = map.site::<CapillaryBed>("glomerular bed", at(0.38)?);
    let peritubular = map.site::<CapillaryBed>("peritubular bed", at(0.38)?);
    let brain = map.site::<CapillaryBed>("brain bed", at(0.8)?);
    let coronary = map.site::<CapillaryBed>("coronary bed", at(0.5)?);
    let liver = map.site::<CapillaryBed>("liver bed", at(0.3)?);
    let fat = map.site::<CapillaryBed>("fat bed", at(0.1)?);
    let skin = map.site::<CapillaryBed>("skin bed", at(0.1)?);
    let bone = map.site::<CapillaryBed>("bone bed", at(0.1)?);

    let pleura = map.site::<EnclosedSpace>("pleural space", at(0.55)?);
    let abdomen = map.site::<EnclosedSpace>("abdominal space", at(0.25)?);

    let sinoatrial = map.site::<Junction>("sinoatrial junction", at(0.5)?);
    let myocardial = map.site::<Junction>("myocardial junction", at(0.5)?);
    let arteriolar = map.site::<Junction>("arteriolar junction", at(0.15)?);
    let venous = map.site::<Junction>("venous junction", at(0.15)?);
    let renal = map.site::<Junction>("renal junction", at(0.38)?);
    let pump = map.site::<Junction>("respiratory pump junction", at(0.55)?);
    let skin_junction = map.site::<Junction>("skin junction", at(0.1)?);

    let tubule = map.site::<VesselSegment>("renal tubule", at(0.38)?);
    let duct = map.site::<VesselSegment>("collecting duct", at(0.38)?);
    let bladder = map.site::<VesselSegment>("bladder", at(0.05)?);
    let stomach = map.site::<VesselSegment>("stomach", at(0.3)?);
    let intestine = map.site::<VesselSegment>("small intestine", at(0.2)?);

    let cranial = map.site::<NerveSegment>("cranial segment", at(0.8)?);

    for site in [
        heart.id(),
        kidney.id(),
        bladder.id(),
        carotid.id(),
        arch.id(),
        pulmonary_artery.id(),
        pulmonary_vein.id(),
        arteries.id(),
        central_veins.id(),
        portal_vein.id(),
        leg_veins.id(),
        lung.id(),
        gut.id(),
        muscle.id(),
        pleura.id(),
        abdomen.id(),
        arteriolar.id(),
        venous.id(),
        cranial.id(),
        skin_heat.id(),
        airway.id(),
        brain.id(),
        liver.id(),
        fat.id(),
        skin.id(),
        bone.id(),
        pump.id(),
        skin_junction.id(),
        stomach.id(),
        intestine.id(),
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
        coronary.id(),
    ] {
        map.contain(heart.id(), site);
    }
    for site in [
        glomerular.id(),
        peritubular.id(),
        renal.id(),
        tubule.id(),
        duct.id(),
    ] {
        map.contain(kidney.id(), site);
    }

    for (from, to) in [
        (right_atrium.id(), right_ventricle.id()),
        (right_ventricle.id(), pulmonary_artery.id()),
        (left_atrium.id(), left_ventricle.id()),
        (left_ventricle.id(), arteries.id()),
        (leg_veins.id(), central_veins.id()),
    ] {
        map.valve(from, to);
    }
    for (from, to) in [
        (pulmonary_artery.id(), lung.id()),
        (lung.id(), pulmonary_vein.id()),
        (pulmonary_vein.id(), left_atrium.id()),
        (arteries.id(), gut.id()),
        (gut.id(), portal_vein.id()),
        (portal_vein.id(), liver.id()),
        (arteries.id(), liver.id()),
        (liver.id(), central_veins.id()),
        (arteries.id(), muscle.id()),
        (muscle.id(), leg_veins.id()),
        (arteries.id(), glomerular.id()),
        (glomerular.id(), peritubular.id()),
        (peritubular.id(), central_veins.id()),
        (central_veins.id(), right_atrium.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    for bed in [brain.id(), coronary.id(), fat.id(), skin.id(), bone.id()] {
        map.connect(Network::Vascular, arteries.id(), bed);
        map.connect(Network::Vascular, bed, central_veins.id());
    }
    let beds = [
        lung.id(),
        gut.id(),
        muscle.id(),
        glomerular.id(),
        peritubular.id(),
        brain.id(),
        coronary.id(),
        liver.id(),
        fat.id(),
        skin.id(),
        bone.id(),
    ];
    for bed in beds {
        map.connect(Network::Lymph, bed, central_veins.id());
    }
    map.connect(Network::Urinary, glomerular.id(), tubule.id());
    map.connect(Network::Urinary, tubule.id(), duct.id());
    map.connect(Network::Urinary, duct.id(), bladder.id());
    map.connect(Network::Reclaim, tubule.id(), peritubular.id());
    map.connect(Network::Reclaim, duct.id(), peritubular.id());
    map.connect(Network::Gut, stomach.id(), intestine.id());
    map.connect(Network::Airway, airway.id(), lung.id());

    map.supply(&myocardial, &[right_ventricle.id(), left_ventricle.id()]);
    map.supply(&arteriolar, &[gut.id(), muscle.id(), fat.id(), bone.id()]);
    map.supply(&venous, &[portal_vein.id(), leg_veins.id()]);
    map.supply(&renal, &[glomerular.id()]);
    map.supply(&skin_junction, &[skin.id()]);
    map.unsupplied(&[
        right_atrium.id(),
        left_atrium.id(),
        pulmonary_artery.id(),
        pulmonary_vein.id(),
        lung.id(),
        arteries.id(),
        central_veins.id(),
        peritubular.id(),
        brain.id(),
        coronary.id(),
        liver.id(),
    ]);
    for (landmark, site) in [
        (Landmark::CarotidSinus, carotid.id()),
        (Landmark::AorticArch, arch.id()),
    ] {
        map.landmark(landmark, arteries.id(), site);
    }
    for (landmark, site) in [
        (Landmark::Pacemaker, sinoatrial.id()),
        (Landmark::MaculaDensa, tubule.id()),
        (Landmark::CentralChemoreceptor, brain.id()),
        (Landmark::PeripheralChemoreceptor, arteries.id()),
        (Landmark::SkinThermoreceptor, skin_heat.id()),
        (Landmark::Osmoreceptor, brain.id()),
        (Landmark::VolumeReceptor, central_veins.id()),
        (Landmark::Adrenal, arteries.id()),
        (Landmark::Islets, arteries.id()),
        (Landmark::Parathyroid, arteries.id()),
        (Landmark::IronStore, liver.id()),
    ] {
        map.landmark(landmark, site, site);
    }

    let pleural = Compliance::Held {
        card: Respiratory::CARD,
    };
    let abdominal = Compliance::Held {
        card: Gastrointestinal::CARD,
    };
    map.enclose(
        &pleura,
        &[
            lung.id(),
            right_atrium.id(),
            right_ventricle.id(),
            left_atrium.id(),
            left_ventricle.id(),
            pulmonary_artery.id(),
            pulmonary_vein.id(),
            central_veins.id(),
            coronary.id(),
        ],
        pleural,
    );
    map.enclose(&abdomen, &[gut.id(), portal_vein.id(), liver.id()], abdominal);

    map.served_by(carotid.id(), &cranial, Afferent::Stretch);
    map.served_by(arch.id(), &cranial, Afferent::Stretch);

    for (bed, tissue) in [
        (&lung, Tissue::Lung),
        (&gut, Tissue::Gut),
        (&muscle, Tissue::Muscle),
        (&glomerular, Tissue::Glomerulus),
        (&peritubular, Tissue::Tubule),
        (&brain, Tissue::Brain),
        (&coronary, Tissue::Heart),
        (&liver, Tissue::Liver),
        (&fat, Tissue::Fat),
        (&skin, Tissue::Skin),
        (&bone, Tissue::Bone),
    ] {
        map.serves(bed, tissue);
    }

    map.region("body", body.id());
    map.region("heart", heart.id());
    map.region("kidney", kidney.id());
    map.finish()
}
