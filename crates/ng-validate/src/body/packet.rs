use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

use super::anatomy::at;

const SECOND: u64 = 1_000_000_000;
const POINTS: u32 = 100;
const FLOW_SIZE: f64 = 20.0;
const POWER_SIZE: f64 = 1e6;
const NOTE: &str = "body fixture: synthetic";
const METHOD: &str = "a synthetic monitor";
const NEW_BED: f64 = 0.1;

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    Ok(AnatomyBuilder::new().site::<C>(key, at(0.0)?).id())
}

fn set<U: Dimension>(
    packet: &mut ParameterPacket<SyntheticFixture>,
    key: &'static str,
    binding: Binding,
    value: f64,
) -> Result<(), AdmissionError> {
    packet.synthetic(ParameterKey::new(key), binding, Measure::<U>::new(value)?, NOTE)
}

fn edge(from: SiteId, to: SiteId) -> Binding {
    Binding::Edge { from, to }
}

fn heart_rate() -> RowKind {
    let law = LawName {
        owner: Cardiovascular::CARD,
        local: "rate",
    };
    RowKind::Algebraic(law.into())
}

fn arterial_pressure() -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Algebraic(Slot {
        law: LawName {
            owner: Cardiovascular::CARD,
            local: "blood pressure",
        },
        at: Context::Site(site::<VesselSegment>("systemic arteries")?),
    }))
}

fn filtration_flow() -> Result<RowKind, AdmissionError> {
    Ok(RowKind::Algebraic(Slot {
        law: LawName {
            owner: Renal::CARD,
            local: "filtration",
        },
        at: Context::Edge {
            from: site::<CapillaryBed>("glomerular bed")?,
            to: site::<VesselSegment>("renal tubule")?,
        },
    }))
}

fn mean_anchor<U: Dimension>(id: AnchorId, row: RowKind, value: Measure<U>) -> Anchor<SyntheticFixture, U> {
    let read = Snapshot::new(row, METHOD, AnchorStatistic::CycleMean, value);
    Synthetic::new(id, read, NOTE)
}

pub fn body_lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
    let window = NonZeroU64::MIN.saturating_add(u64::from(POINTS) - 1);
    let beat = NonZeroU64::MIN.saturating_add(SECOND - 1);
    let rate = ChannelSpec::new(METHOD, heart_rate()).time_averaged(beat);
    let channels =
        [arterial_pressure()?, filtration_flow()?].map(|row| ChannelSpec::new(METHOD, row).averaged(window));
    let channels = [rate].into_iter().chain(channels).collect();
    let grid = NonZeroU64::MIN.saturating_add(SECOND / u64::from(POINTS) - 1);
    body.lens(channels, grid)
}

fn water_guess(site: &str, space: Space) -> Option<f64> {
    let new_bed = ["brain bed", "coronary bed", "fat bed", "skin bed", "bone bed"];
    Some(match (site, space) {
        ("right atrium", Space::Lumen) => 54.0,
        ("right ventricle", Space::Lumen) => 100.0,
        ("pulmonary artery", Space::Lumen) => 126.0,
        ("pulmonary vein", Space::Lumen) => 300.0,
        ("left atrium", Space::Lumen) => 74.0,
        ("left ventricle", Space::Lumen) => 100.0,
        ("systemic arteries", Space::Lumen) => 800.0,
        ("portal vein", Space::Lumen) => 800.0,
        ("leg veins", Space::Lumen) => 500.0,
        ("central veins", Space::Lumen) => 370.0,
        ("pulmonary bed", Space::Plasma) => 180.0,
        ("gut bed" | "liver bed", Space::Plasma) => 108.0,
        ("muscle bed", Space::Plasma) => 110.0,
        ("glomerular bed", Space::Plasma) => 41.0,
        ("peritubular bed", Space::Plasma) => 32.0,
        ("pulmonary bed", Space::Interstitium) => 430.0,
        ("gut bed" | "liver bed", Space::Interstitium) => 2_200.0,
        ("muscle bed", Space::Interstitium) => 2_700.0,
        ("glomerular bed", Space::Interstitium) => 100.0,
        ("peritubular bed", Space::Interstitium) => 160.0,
        ("renal tubule", Space::Lumen) => 14.0,
        ("collecting duct", Space::Lumen) => 5.0,
        ("bladder", Space::Lumen) => 20.0,
        (bed, Space::Plasma) if new_bed.contains(&bed) => 110.0 * NEW_BED,
        (bed, Space::Interstitium) if new_bed.contains(&bed) => 2_700.0 * NEW_BED,
        _ => return None,
    })
}

pub fn body_start_guess(row: RowKind) -> f64 {
    match row {
        RowKind::Amount { site, species, space } => {
            let guess = water_guess(site.key(), space);
            let water = guess.unwrap_or(1_000.0);
            let holds = if guess.is_some() { water } else { 1.0 };
            match (species, space) {
                (Species::Water, _) => water,
                (Species::Sodium, _) => 100.0 * holds,
                (Species::Potassium, _) => 5.0 * holds,
                (Species::Chloride, _) => 105.0 * holds,
                (Species::Albumin, Space::Interstitium) => 0.1 * holds,
                (Species::Albumin, _) => holds,
                (Species::Vasopressin, Space::Bound) => 0.8,
                (Species::Vasopressin, _) => 0.4 * holds,
                (Species::Energy, Space::Thermal) => 2e7,
                (Species::Energy, _) => 16_000.0,
                (_, Space::Vesicle) => 100.0,
                _ => 0.5,
            }
        }
        RowKind::Memory(slot) if slot.law.local == "afferent filter" => 1.0,
        RowKind::Memory(slot) if slot.law.local == "baroreflex set point" => 1.0,
        RowKind::Memory(_) => 0.0,
        RowKind::Algebraic(_) => 1.0,
    }
}

type Zero = fn(&mut ParameterPacket<SyntheticFixture>, &'static str, Binding) -> Result<(), AdmissionError>;

fn zero<Q>(
    packet: &mut ParameterPacket<SyntheticFixture>,
    key: &'static str,
    at: Binding,
) -> Result<(), AdmissionError>
where
    Rate<Q>: Dimension,
{
    set::<Rate<Q>>(packet, key, at, 0.0)
}

const ZERO_STAND_INS: &[(&str, Zero)] = &[
    ("en.aldosterone-clearance", zero::<Aldosterone>),
    ("en.aldosterone-secretion", zero::<Aldosterone>),
    ("en.angiotensin-ii-clearance", zero::<AngiotensinII>),
    ("en.angiotensin-ii-made", zero::<AngiotensinII>),
    ("en.cortisol-clearance", zero::<Cortisol>),
    ("en.cortisol-secretion", zero::<Cortisol>),
    ("en.glucagon-clearance", zero::<Glucagon>),
    ("en.glucagon-secretion", zero::<Glucagon>),
    ("en.insulin-clearance", zero::<Insulin>),
    ("en.insulin-secretion", zero::<Insulin>),
    ("en.natriuretic-peptide-clearance", zero::<NatriureticPeptide>),
    ("en.natriuretic-peptide-secretion", zero::<NatriureticPeptide>),
    ("en.parathyroid-hormone-clearance", zero::<ParathyroidHormone>),
    ("en.parathyroid-hormone-secretion", zero::<ParathyroidHormone>),
    ("en.renin-clearance", zero::<Renin>),
    ("en.renin-secretion", zero::<Renin>),
    ("gi.basal-amino-acids", zero::<AminoAcids>),
    ("gi.basal-calcium", zero::<Calcium>),
    ("gi.basal-diet-base", zero::<DietBase>),
    ("gi.basal-glucose", zero::<Glucose>),
    ("gi.basal-iron", zero::<Iron>),
    ("gi.basal-magnesium", zero::<Magnesium>),
    ("gi.basal-phosphate", zero::<Phosphate>),
    ("gi.basal-triglyceride", zero::<Triglyceride>),
    ("gi.feces-amino-acids", zero::<AminoAcids>),
    ("gi.feces-calcium", zero::<Calcium>),
    ("gi.feces-chloride", zero::<Chloride>),
    ("gi.feces-glucose", zero::<Glucose>),
    ("gi.feces-iron", zero::<Iron>),
    ("gi.feces-magnesium", zero::<Magnesium>),
    ("gi.feces-phosphate", zero::<Phosphate>),
    ("gi.feces-potassium", zero::<Potassium>),
    ("gi.feces-sodium", zero::<Sodium>),
    ("gi.feces-triglyceride", zero::<Triglyceride>),
    ("gi.feces-water", zero::<Water>),
    ("hc.hemoglobin-destruction", zero::<Hemoglobin>),
    ("hc.hemoglobin-production", zero::<Hemoglobin>),
    ("hp.albumin-loss", zero::<Albumin>),
    ("hp.albumin-synthesis", zero::<Albumin>),
    ("hp.fixed-acid-made", zero::<FixedAcid>),
    ("hp.insulin-first-pass", zero::<Insulin>),
    ("mt.dry-heat-loss", zero::<Energy>),
    ("mt.skin-evaporation", zero::<Water>),
    ("mt.skin-evaporative-heat", zero::<Energy>),
    ("mt.sweat-chloride", zero::<Chloride>),
    ("mt.sweat-sodium", zero::<Sodium>),
    ("mt.sweat-water", zero::<Water>),
    ("rs.airway-heat", zero::<Energy>),
    ("rs.expired-carbon-dioxide", zero::<CarbonDioxide>),
    ("rs.expired-nitrogen-gas", zero::<NitrogenGas>),
    ("rs.expired-oxygen", zero::<Oxygen>),
    ("rs.expired-water", zero::<Water>),
    ("rs.inspired-carbon-dioxide", zero::<CarbonDioxide>),
    ("rs.inspired-nitrogen-gas", zero::<NitrogenGas>),
    ("rs.inspired-oxygen", zero::<Oxygen>),
];

fn zero_stand_ins(
    packet: &mut ParameterPacket<SyntheticFixture>,
    body: &CompiledBody,
) -> Result<(), AdmissionError> {
    for slot in body.parameters() {
        let key = slot.key();
        if let Some((name, zero)) = ZERO_STAND_INS
            .iter()
            .find(|(name, _)| ParameterKey::new(name) == key)
        {
            zero(packet, name, slot.binding())?;
        }
    }
    Ok(())
}

pub fn body_packet(body: &CompiledBody) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let rate = AnchorId::new("heart rate");
    let phase = LawName {
        owner: Cardiovascular::CARD,
        local: "phase",
    };
    let cycle = Cycle::new(
        NonZeroU64::MIN.saturating_add(SECOND - 1),
        NonZeroU32::MIN.saturating_add(POINTS - 1),
    )?
    .paced(phase.into(), rate);
    let mut packet = ParameterPacket::periodic(cycle);
    let unit = |value| Measure::<Dimensionless>::new(value);
    packet.within(ErrorScales::uniform(unit(1e-9)?, unit(1e-10)?)?);

    let (arteries, glomerular) = (
        site::<VesselSegment>("systemic arteries")?,
        site::<CapillaryBed>("glomerular bed")?,
    );
    let read = Snapshot::new(
        heart_rate(),
        METHOD,
        AnchorStatistic::CycleTimeMean,
        Measure::<Rate<Dimensionless>>::new(1.0)?,
    );
    packet.anchor(Synthetic::new(rate, read, NOTE))?;
    packet.calibrated(
        ParameterKey::new("cv.intrinsic-rate"),
        Binding::Body,
        Measure::<Rate<Dimensionless>>::new(1.35)?,
        rate,
    )?;
    let pressure = AnchorId::new("mean arterial pressure");
    packet.anchor(mean_anchor(
        pressure,
        arterial_pressure()?,
        Measure::<Pressure>::new(100.0)?,
    ))?;
    packet.calibrated(
        ParameterKey::new("cv.resistance"),
        edge(arteries, glomerular),
        Measure::<Resistance>::new(0.36)?,
        pressure,
    )?;
    let filtered = AnchorId::new("filtration");
    packet.anchor(mean_anchor(
        filtered,
        filtration_flow()?,
        Measure::<Rate<Water>>::new(2.0)?,
    ))?;
    packet.calibrated(
        ParameterKey::new("rn.sodium-capacity"),
        edge(
            site::<VesselSegment>("renal tubule")?,
            site::<CapillaryBed>("peritubular bed")?,
        ),
        Measure::<Rate<Sodium>>::new(270.0)?,
        filtered,
    )?;

    let valve = |from: &'static str, to: &'static str| -> Result<Slot, AdmissionError> {
        let law = LawName {
            owner: Cardiovascular::CARD,
            local: "valve",
        };
        let (from, to) = (site::<VesselSegment>(from)?, site::<VesselSegment>(to)?);
        Ok(Slot {
            law,
            at: Context::Edge { from, to },
        })
    };
    let rhythm = LawName {
        owner: Cardiovascular::CARD,
        local: "rhythm",
    };
    packet.start_mode(rhythm, "systole", NOTE)?;
    packet.start_mode(valve("left ventricle", "systemic arteries")?, "shut", NOTE)?;
    packet.start_mode(valve("left atrium", "left ventricle")?, "open", NOTE)?;
    packet.start_mode(valve("right ventricle", "pulmonary artery")?, "shut", NOTE)?;
    packet.start_mode(valve("right atrium", "right ventricle")?, "open", NOTE)?;
    packet.start_mode(valve("leg veins", "central veins")?, "open", NOTE)?;
    let stirring = LawName {
        owner: Musculoskeletal::CARD,
        local: "stirring",
    };
    packet.start_mode(stirring, "still", NOTE)?;

    table(&mut packet)?;
    zero_stand_ins(&mut packet, body)?;
    packet.start_rest(body, body_start_guess, NOTE)?;
    for row in body.rows() {
        let kind = row.kind();
        if let RowKind::Memory(slot) | RowKind::Algebraic(slot) = kind {
            match slot.law.local {
                "valves" | "circulation" | "sinusoids" => {
                    packet.row_scale(kind, Measure::<Rate<Water>>::new(FLOW_SIZE)?)?
                }
                "wall power" => packet.row_scale(kind, Measure::<Rate<Energy>>::new(POWER_SIZE)?)?,
                _ => {}
            }
        }
    }
    Ok(packet)
}

fn table(packet: &mut ParameterPacket<SyntheticFixture>) -> Result<(), AdmissionError> {
    let blood = Binding::Network(Network::Vascular);
    let (urinary, lymph) = (
        Binding::Network(Network::Urinary),
        Binding::Network(Network::Lymph),
    );
    let (body, beds, junctions) = (
        Binding::Body,
        Binding::Class(Class::CapillaryBed),
        Binding::Class(Class::Junction),
    );
    set::<Quotient<Pressure, Length>>(packet, "cv.head-per-length", body, 70.0)?;
    for (key, value) in [
        ("cv.beta-gain", 1.0),
        ("cv.vagal-gain", 0.7),
        ("cv.systole-end", 0.3),
        ("cv.cycle-end", 1.0),
        ("cv.activation-peak", 1.0),
        ("cv.half", 0.5),
        ("ns.carotid-share", 0.7),
        ("ns.arch-share", 0.3),
        ("ns.sympathetic-rest", -1.1),
        ("ns.vagal-rest", 0.0),
        ("rn.feedback-rest", 1.0),
        ("rn.feedback-gain", 0.6),
        ("rn.feedback-middle", 0.5),
    ] {
        set::<Dimensionless>(packet, key, body, value)?;
    }
    for (key, value) in [
        ("cv.rise-speed", 20.0),
        ("cv.fall-speed", 20.0),
        ("ns.filter-speed", 4.0),
        ("ns.set-point-speed", 1e-6),
        ("en.avp-clearance", 2e-3),
        ("bd.bladder-speed", 1e-3),
        ("bd.heat-speed", 1e-3),
        ("ms.stir-rate", 2.0),
        ("gi.transit-exchange", 1e-3),
    ] {
        set::<Rate<Dimensionless>>(packet, key, body, value)?;
    }
    set::<Pressure>(packet, "cv.stretch-reference", body, 100.0)?;
    set::<Volume>(packet, "cv.collapse-width", body, 1.0)?;
    set::<Rate<Energy>>(packet, "cv.heat-width", body, 100.0)?;
    set::<Quotient<Volume, Albumin>>(packet, "cv.albumin-volume", body, 0.05)?;
    set::<Quotient<Volume, Albumin>>(packet, "fl.albumin-volume", body, 0.05)?;
    set::<Pressure>(packet, "gi.abdominal-pressure", body, 4.0)?;
    set::<Pressure>(packet, "rs.pleural-pressure", body, -3.0)?;
    set::<Deviation<Dimensionless>>(packet, "ns.sympathetic-width", body, 0.035)?;
    set::<Deviation<Dimensionless>>(packet, "ns.vagal-width", body, 0.05)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "fl.oncotic-slope", body, 20.0)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "rn.oncotic-slope", body, 20.0)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-sodium", body, 30.0)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-width", body, 5.0)?;
    set::<Concentration<Sodium>>(packet, "en.avp-sodium", body, 100.0)?;
    set::<Concentration<Sodium>>(packet, "en.avp-width", body, 2.0)?;
    set::<Rate<Vasopressin>>(packet, "en.avp-most", body, 2.0)?;
    set::<Rate<Water>>(packet, "gi.basal-water", body, 0.03)?;
    set::<Rate<Sodium>>(packet, "gi.basal-sodium", body, 1.0)?;
    set::<Rate<Potassium>>(packet, "gi.basal-potassium", body, 0.05)?;
    set::<Rate<Chloride>>(packet, "gi.basal-chloride", body, 1.05)?;
    set::<Rate<Water>>(packet, "ms.skin-water", body, 0.01)?;
    set::<Time>(packet, "ms.skin-water-period", body, 300.0)?;
    set::<Dimensionless>(packet, "ms.skin-water-amplitude", body, 0.5)?;
    set::<Water>(packet, "bd.bladder-rest", body, 0.0)?;
    set::<Rate<Water>>(packet, "bd.bladder-width", body, 1e-3)?;

    set::<Dimensionless>(packet, "cv.inotropy", blood, 1.0)?;
    set::<Elastance>(packet, "cv.active-elastance", blood, 0.0)?;
    set::<Volume>(packet, "cv.unstressed-gain", blood, 0.0)?;
    fn wall(
        packet: &mut ParameterPacket<SyntheticFixture>,
        at: Binding,
        (elastance, active, unstressed, gain): (f64, f64, f64, f64),
    ) -> Result<(), AdmissionError> {
        set::<Elastance>(packet, "cv.elastance", at, elastance)?;
        if active > 0.0 {
            set::<Elastance>(packet, "cv.active-elastance", at, active)?;
        }
        set::<Volume>(packet, "cv.unstressed", at, unstressed)?;
        if gain > 0.0 {
            set::<Volume>(packet, "cv.unstressed-gain", at, gain)?;
        }
        Ok(())
    }
    for (key, values) in [
        ("right atrium", (0.25, 0.0, 30.0, 0.0)),
        ("right ventricle", (0.05, 0.75, 40.0, 0.0)),
        ("pulmonary artery", (0.5, 0.0, 90.0, 0.0)),
        ("pulmonary vein", (0.1, 0.0, 180.0, 0.0)),
        ("left atrium", (0.25, 0.0, 30.0, 0.0)),
        ("left ventricle", (0.1, 3.0, 20.0, 0.0)),
        ("systemic arteries", (1.0, 0.0, 700.0, 0.0)),
        ("portal vein", (0.1, 0.0, 1_064.0, 800.0)),
        ("leg veins", (0.4, 0.0, 566.0, 200.0)),
        ("central veins", (0.1, 0.0, 300.0, 0.0)),
    ] {
        wall(packet, Binding::Site(site::<VesselSegment>(key)?), values)?;
    }
    let leg = ((2.0, 0.0, 100.0, 0.0), 0.002, 0.04, 2_750.0);
    let scaled = {
        let ((elastance, active, unstressed, gain), filtration, stiffness, rest) = leg;
        (
            (elastance / NEW_BED, active, unstressed * NEW_BED, gain * NEW_BED),
            filtration * NEW_BED,
            stiffness / NEW_BED,
            rest * NEW_BED,
        )
    };
    for (key, (values, filtration, stiffness, rest)) in [
        ("pulmonary bed", ((0.25, 0.0, 120.0, 0.0), 0.002, 0.05, 420.0)),
        ("gut bed", ((2.0, 0.0, 100.0, 0.0), 0.003, 0.04, 2_250.0)),
        ("muscle bed", leg),
        ("glomerular bed", ((4.0, 0.0, 30.0, 0.0), 0.0005, 0.05, 40.0)),
        ("peritubular bed", ((4.0, 0.0, 30.0, 0.0), 0.002, 0.05, 200.0)),
        ("brain bed", scaled),
        ("coronary bed", scaled),
        ("liver bed", ((2.0, 0.0, 100.0, 0.0), 0.003, 0.04, 2_250.0)),
        ("fat bed", scaled),
        ("skin bed", scaled),
        ("bone bed", scaled),
    ] {
        let at = Binding::Site(site::<CapillaryBed>(key)?);
        wall(packet, at, values)?;
        set::<Conductance<Water>>(packet, "fl.filtration", at, filtration)?;
        set::<Elastance>(packet, "fl.interstitial-elastance", at, stiffness)?;
        set::<Volume>(packet, "fl.interstitial-volume", at, rest)?;
    }

    set::<Resistance>(packet, "cv.resistance", blood, 0.01)?;
    set::<Dimensionless>(packet, "cv.tone-gain", blood, 0.0)?;
    set::<Dimensionless>(packet, "cv.feedback-share", blood, 0.0)?;
    set::<Inertance>(packet, "cv.inertance", blood, 5e-4)?;
    set::<Orifice>(packet, "cv.orifice", blood, 5e-5)?;
    set::<Resistance>(packet, "cv.valve-reading", blood, 0.01)?;
    set::<Pressure>(packet, "cv.opening-head", blood, 0.01)?;
    let veins = site::<VesselSegment>("central veins")?;
    let bed = |key| site::<CapillaryBed>(key);
    let (lung, gut, muscle, peritubular, liver) = (
        bed("pulmonary bed")?,
        bed("gut bed")?,
        bed("muscle bed")?,
        bed("peritubular bed")?,
        bed("liver bed")?,
    );
    let (artery, pulmonary_vein, portal, leg_veins) = (
        site::<VesselSegment>("pulmonary artery")?,
        site::<VesselSegment>("pulmonary vein")?,
        site::<VesselSegment>("portal vein")?,
        site::<VesselSegment>("leg veins")?,
    );
    let arteries = site::<VesselSegment>("systemic arteries")?;
    let glomerular = bed("glomerular bed")?;
    set::<Orifice>(packet, "cv.orifice", edge(leg_veins, veins), 1e-3)?;
    for (from, to, resistance) in [
        (artery, lung, 0.06),
        (lung, pulmonary_vein, 0.03),
        (gut, portal, 0.5),
        (muscle, leg_veins, 0.5),
        (glomerular, peritubular, 2.0),
        (peritubular, veins, 0.2),
    ] {
        set::<Resistance>(packet, "cv.resistance", edge(from, to), resistance)?;
    }
    set::<Dimensionless>(packet, "cv.tone-gain", edge(arteries, glomerular), 60.0)?;
    for (to, resistance) in [(gut, 1.09), (muscle, 1.4)] {
        let at = edge(arteries, to);
        set::<Resistance>(packet, "cv.resistance", at, resistance)?;
        set::<Dimensionless>(packet, "cv.tone-gain", at, 6.0)?;
    }
    for (key, toned) in [
        ("brain bed", false),
        ("coronary bed", false),
        ("fat bed", true),
        ("skin bed", true),
        ("bone bed", true),
    ] {
        let new = bed(key)?;
        set::<Resistance>(packet, "cv.resistance", edge(arteries, new), 1.4 / NEW_BED)?;
        set::<Resistance>(packet, "cv.resistance", edge(new, veins), 0.5 / NEW_BED)?;
        if toned {
            set::<Dimensionless>(packet, "cv.tone-gain", edge(arteries, new), 6.0)?;
        }
    }
    for (from, to, resistance) in [
        (portal, liver, 0.05),
        (liver, veins, 0.05),
        (arteries, liver, 1.4 / NEW_BED),
    ] {
        set::<Resistance>(packet, "hp.sinusoid-resistance", edge(from, to), resistance)?;
    }
    set::<Dimensionless>(packet, "cv.feedback-share", edge(arteries, glomerular), 1.0)?;

    set::<Rate<Water>>(packet, "fl.ion-exchange", beds, 10.0)?;
    set::<Rate<Water>>(packet, "fl.albumin-leak", beds, 1e-3)?;
    set::<Dimensionless>(packet, "fl.albumin-reflection", beds, 0.9)?;
    set::<Dimensionless>(packet, "fl.albumin-reflection", Binding::Site(peritubular), 0.5)?;
    set::<Conductance<Water>>(packet, "fl.lymph-conductance", lymph, 0.04)?;
    set::<Conductance<Water>>(packet, "fl.lymph-conductance", edge(glomerular, veins), 0.002)?;
    set::<Conductance<Water>>(packet, "fl.lymph-conductance", edge(peritubular, veins), 0.005)?;
    for key in ["brain bed", "coronary bed", "fat bed", "skin bed", "bone bed"] {
        set::<Conductance<Water>>(
            packet,
            "fl.lymph-conductance",
            edge(bed(key)?, veins),
            0.04 * NEW_BED,
        )?;
    }
    set::<Pressure>(packet, "fl.lymph-threshold", lymph, -2.0)?;
    set::<Pressure>(packet, "fl.lymph-width", lymph, 0.5)?;

    let (tubule, duct, bladder) = (
        site::<VesselSegment>("renal tubule")?,
        site::<VesselSegment>("collecting duct")?,
        site::<VesselSegment>("bladder")?,
    );
    for (at, stiffness, rest) in [(tubule, 0.5, 10.0), (duct, 1.0, 5.0), (bladder, 1e-3, 0.0)] {
        set::<Elastance>(packet, "rn.lumen-elastance", Binding::Site(at), stiffness)?;
        set::<Volume>(packet, "rn.lumen-volume", Binding::Site(at), rest)?;
    }
    set::<Conductance<Water>>(packet, "rn.filtration", urinary, 0.12)?;
    set::<Resistance>(packet, "rn.tubule-resistance", urinary, 5.0)?;
    for (from, water_least, water_most, half) in [(tubule, 0.8, 0.8, 200.0), (duct, 0.5, 0.99, 20.0)] {
        let at = edge(from, peritubular);
        set::<Dimensionless>(packet, "rn.water-least", at, water_least)?;
        set::<Dimensionless>(packet, "rn.water-most", at, water_most)?;
        set::<Sodium>(packet, "rn.sodium-half", at, half)?;
        set::<Chloride>(packet, "rn.chloride-half", at, 1.0)?;
    }
    set::<Rate<Sodium>>(packet, "rn.sodium-capacity", edge(duct, peritubular), 12.0)?;

    for (release, reuptake, out, back) in [
        ("ns.release-ne", "ns.reuptake-ne", 0.005, 0.25),
        ("ns.release-ach", "ns.reuptake-ach", 0.04, 2.0),
    ] {
        set::<Rate<Dimensionless>>(packet, release, junctions, out)?;
        set::<Rate<Dimensionless>>(packet, reuptake, junctions, back)?;
    }
    set::<Norepinephrine>(packet, "rc.ne-sites", junctions, 1.0)?;
    set::<Affinity<Norepinephrine>>(packet, "rc.ne-on", junctions, 10.0)?;
    set::<Rate<Dimensionless>>(packet, "rc.ne-off", junctions, 10.0)?;
    let renal = Binding::Site(site::<Junction>("renal junction")?);
    set::<Rate<Dimensionless>>(packet, "rc.ne-off", renal, 40.0)?;
    set::<Dimensionless>(packet, "rc.ne-efficacy", junctions, 1.0)?;
    set::<Acetylcholine>(packet, "rc.ach-sites", junctions, 1.0)?;
    set::<Affinity<Acetylcholine>>(packet, "rc.ach-on", junctions, 10.0)?;
    set::<Rate<Dimensionless>>(packet, "rc.ach-off", junctions, 10.0)?;
    set::<Dimensionless>(packet, "rc.ach-efficacy", junctions, 2.0)?;
    set::<Vasopressin>(packet, "rc.avp-sites", blood, 1.0)?;
    set::<Affinity<Concentration<Vasopressin>>>(packet, "rc.avp-on", blood, 0.6)?;
    set::<Rate<Dimensionless>>(packet, "rc.avp-off", blood, 0.1)?;
    set::<Dimensionless>(packet, "rc.avp-efficacy", blood, 2.0)
}
