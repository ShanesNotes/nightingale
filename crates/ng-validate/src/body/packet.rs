use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

use super::anatomy::at;
use super::schema::{BEDS, FAT_ENERGY, GLUCOSE_ENERGY};
use crate::{CertificatePolicy, DriftTable, SlidingWindows};

const SECOND: u64 = 1_000_000_000;
const POINTS: u32 = 100;
const FLOW_SIZE: f64 = 20.0;
const POWER_SIZE: f64 = 1e6;
const NOTE: &str = "body fixture: synthetic";
const METHOD: &str = "a synthetic monitor";
const NEW_BED: f64 = 0.1;
const HEMOGLOBIN: f64 = 2.0;
/// The shunt's resistance, `R_s = R_c (1 - f) / f` for a fixed shunt fraction `f` of 0.02 beside the bed's 0.09
/// (research 11, Nunn pp. 244 to 245).
const SHUNT: f64 = 0.09 * 0.98 / 0.02;

/// The viscosity the packet's edge resistances are stated at. It was read once, at 08a's admitted start from an
/// admission whose vessels did not read it, so 08a moved no pressure at rest. It is not read again when the admitted
/// hematocrit moves: the resistances then move with viscosity, as a later ticket's change asks.
const START_VISCOSITY: f64 = 7.115_102_244_801_436;

/// Each bed's oxygen demand in its cells, in `BEDS`' order, and the head of partial pressure that every wall, cell
/// membrane and the alveolar membrane take at that demand.
const USES: [f64; 11] = [1.0, 20.0, 25.0, 3.0, 10.0, 8.0, 8.0, 15.0, 2.0, 2.0, 2.0];
const HEAD: f64 = 5.0;
const SOLUBILITY: f64 = 0.001;
const HEME: f64 = 4.0;
const P50: f64 = 25.0;
const HILL: (f64, f64, f64) = (2.5, 1.0, 10.0);
/// The start's arterial and venous oxygen partial pressures as `P50 e^t`, because the host's logarithm is banned.
const REACH: (f64, f64) = (1.3, 0.5);
struct ByAcidity {
    standard: f64,
    cubic: [f64; 3],
}
const BY_ACIDITY: ByAcidity = ByAcidity {
    standard: 7.5,
    cubic: [-1.0, 0.5, -0.25],
};
struct ByCarbonDioxide {
    standard: f64,
    scale: f64,
    quadratic: [f64; 2],
}
const BY_CARBON_DIOXIDE: ByCarbonDioxide = ByCarbonDioxide {
    standard: 30.0,
    scale: 10.0,
    quadratic: [0.1, 0.01],
};
const CARBON_DIOXIDE_SOLUBILITY: f64 = 0.02;
const PK: f64 = 6.0;
const STRONG_ION_OFFSET: f64 = 50.0;
struct WeakAcid {
    concentration: f64,
    slope: f64,
    offset: f64,
}
const ALBUMIN: WeakAcid = WeakAcid {
    concentration: 1.0,
    slope: 10.0,
    offset: 60.0,
};
const PHOSPHATE: WeakAcid = WeakAcid {
    concentration: 1.0,
    slope: 1.0,
    offset: 6.0,
};
const HEMOGLOBIN_CHARGE: (f64, f64) = (5.0, 30.0);
struct Carbamino {
    sites: f64,
    half: f64,
    half_rise: f64,
}
const CARBAMINO: Carbamino = Carbamino {
    sites: 4.0,
    half: 10.0,
    half_rise: 10.0,
};
const INTERSTITIAL_BUFFER_RATIO: f64 = 15.0;
const CELL_BUFFER_RATIO: f64 = 10.0;
const GLUCOSE_SHARE: f64 = 5.0 / 28.0;
const PER_OXYGEN: f64 = GLUCOSE_SHARE + 16.0 * (1.0 - GLUCOSE_SHARE) / 23.0;
const ENERGY_PER_OXYGEN: f64 =
    GLUCOSE_SHARE * GLUCOSE_ENERGY / 6.0 + (1.0 - GLUCOSE_SHARE) * FAT_ENERGY / 23.0;
/// The coronary bed pays the heart's stroke work, so its stated demand is a basal value plus that work's oxygen. The
/// basal is its stated 8 less the oxygen of the resting work measured on this body, 18,072 a second, so its whole-beat
/// burn stays at 8. The muscle bed pays the walls' work, 2.2e-5 of oxygen a second, with no such correction.
const STROKE_PAYER: &str = "coronary bed";
const STROKE_WORK_OXYGEN: f64 = 1.32;
/// How fast line 78's work follows the stroke work: a model default with no source (research 18).
const WORK_SPEED: f64 = 0.25;
const CONTRACTILE_SIZE: f64 = 1e6;
/// The oxygen reach meets the demand at this cell pressure, and the oxygen rate falls short of the demand only below
/// twice it, 34 mmHg. Every bed's cells sit at 38 mmHg or more at rest, so none splits glucose there or admits lactate,
/// and halving the one-bed fixture's flow takes its cells under it.
const BODY_CELL_PRESSURE_AT_DEMAND: f64 = 17.0;
const DEBT_SHARE: f64 = 0.5;
const DEBT_HALF_TIME: f64 = 1e-3;
const WIDTH_SHARE_OF_DEMAND: f64 = 0.02;
const BED_HEAT_SPEED: f64 = 0.05;
const FEED_SPEED: f64 = 0.1;
const GLUCOSE_SET: f64 = 3_000.0;
const FAT_SET: f64 = 600.0;
const CELL_WATER: f64 = 1_000.0;
const WATER_SPEED: f64 = 10.0;
const GLUCOSE_PER_OXYGEN_SHORT: f64 = 1.0 / 6.0;
const LACTATE_BURN_SPEED: f64 = 0.02;
const LACTATE_HALF_PRESSURE: f64 = 2.0;
const CELL_LACTATE_AT_REST: f64 = 0.0;
const ALVEOLAR_CARBON_DIOXIDE: f64 = 30.0;
const CARBON_DIOXIDE_HEAD: f64 = 1.0;
const VENOUS_CARBON_DIOXIDE_GUESS: f64 = 35.0;

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

pub fn body_start_guess(row: RowKind) -> Result<Compensated, AdmissionError> {
    Ok(Compensated::of(&Measure::<Dimensionless>::new(start_guess(row))?))
}

fn start_guess(row: RowKind) -> f64 {
    match row {
        RowKind::Amount {
            site,
            species,
            space: Space::Gas,
        } if species != Species::Water => match (site.key(), species) {
            ("pulmonary bed", Species::Oxygen) => 8_769.0,
            ("pulmonary bed", Species::CarbonDioxide) => 2_631.0,
            ("pulmonary bed", _) => 45_600.0,
            (_, Species::Oxygen) => 618.0,
            (_, Species::CarbonDioxide) => 78.5,
            _ => 2_703.5,
        },
        RowKind::Amount { site, species, space } => {
            if let Some(rest) = burn_rest(site.key(), species, space) {
                return rest;
            }
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
                (Species::Hemoglobin, _) => HEMOGLOBIN * holds,
                (Species::Oxygen, Space::Plasma | Space::Lumen | Space::Interstitium | Space::Cells) => {
                    oxygen_guess(site.key(), space) * water
                }
                (Species::CarbonDioxide, Space::Lumen)
                    if matches!(site.key(), "renal tubule" | "collecting duct" | "bladder") =>
                {
                    0.5
                }
                (
                    Species::CarbonDioxide,
                    Space::Plasma | Space::Lumen | Space::Interstitium | Space::Cells,
                ) => carbon_dioxide_guess(site.key(), space) * water,
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
        RowKind::Memory(slot) if slot.law.local == "mixing share" => 1.0,
        RowKind::Memory(slot) if slot.law.local == "pump drive" => RHYTHM.mean_drive(),
        RowKind::Memory(_) => 0.0,
        RowKind::Algebraic(slot) => {
            let bed = match slot.at {
                Context::Site(site) => site.key(),
                _ => "pulmonary bed",
            };
            let blood = blood(bed);
            match slot.law.local {
                "oxygen partial pressure" => blood.oxygen_pressure,
                "oxygen saturation" => blood.saturation,
                "bicarbonate" => blood.bicarbonate,
                "acidity" => blood.acidity,
                "oxygen_debt_per_bed" => 0.0,
                "chest wall rest volume" => WALL_REST,
                _ => 1.0,
            }
        }
    }
}

/// A bed's rest at its demand: its cells' glucose, fatty acids and water, and its heat, each where its held feed or
/// drain or its way to the core store meets what the burn makes or uses.
fn burn_rest(site: &str, species: Species, space: Space) -> Option<f64> {
    let used = USES[BEDS.iter().position(|bed| *bed == site)?];
    let (glucose, fat) = (GLUCOSE_SHARE * used / 6.0, (1.0 - GLUCOSE_SHARE) * used / 23.0);
    Some(match (species, space) {
        (Species::Glucose, Space::Cells) => GLUCOSE_SET - glucose / FEED_SPEED,
        (Species::FattyAcids, Space::Cells) => FAT_SET - fat / FEED_SPEED,
        (Species::Water, Space::Cells) => CELL_WATER + (6.0 * glucose + 16.0 * fat) / WATER_SPEED,
        (Species::Energy, Space::Thermal) => basal(site, used) * ENERGY_PER_OXYGEN / BED_HEAT_SPEED,
        (Species::Lactate, Space::Cells) => CELL_LACTATE_AT_REST,
        _ => return None,
    })
}

/// The burn adds the work's oxygen back to the payer's demand and passes its heat to the walls.
fn basal(bed: &str, used: f64) -> f64 {
    if bed == STROKE_PAYER {
        used - STROKE_WORK_OXYGEN
    } else {
        used
    }
}

fn ten_to(power: f64) -> f64 {
    exp(power * core::f64::consts::LN_10)
}

fn weak(acidity: f64) -> f64 {
    let acids: f64 = ([ALBUMIN, PHOSPHATE].iter())
        .map(|acid| acid.concentration * (acid.slope * acidity - acid.offset))
        .sum();
    acids + HEMOGLOBIN * (HEMOGLOBIN_CHARGE.0 * acidity - HEMOGLOBIN_CHARGE.1)
}

/// The acidity whose charge balance's bicarbonate is `10^(pH - pK)` times the dissolved carbon dioxide at `pressure`,
/// by bisection, with the strong ions' own charge at zero as the packet's guesses give it.
fn acidity_at(pressure: f64) -> f64 {
    let (mut low, mut high) = (5.0, 9.0);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        let bicarbonate = STRONG_ION_OFFSET - weak(middle);
        if bicarbonate > CARBON_DIOXIDE_SOLUBILITY * pressure * ten_to(middle - PK) {
            low = middle;
        } else {
            high = middle;
        }
    }
    0.5 * (low + high)
}

struct Blood {
    oxygen_pressure: f64,
    saturation: f64,
    oxygen_content: f64,
    acidity: f64,
    bicarbonate: f64,
    carbon_dioxide_pressure: f64,
    carbon_dioxide_total: f64,
}

fn speciated(t: f64, pressure: f64) -> Blood {
    let acidity = acidity_at(pressure);
    let bicarbonate = STRONG_ION_OFFSET - weak(acidity);
    let d = acidity - BY_ACIDITY.standard;
    let [c1, c2, c3] = BY_ACIDITY.cubic;
    let u = (pressure - BY_CARBON_DIOXIDE.standard) / BY_CARBON_DIOXIDE.scale;
    let [q1, q2] = BY_CARBON_DIOXIDE.quadratic;
    let half = P50 * (1.0 + c1 * d + c2 * d * d + c3 * d * d * d) * (1.0 + q1 * u + q2 * u * u);
    let oxygen = half * exp(t);
    let exponent = HILL.0 - HILL.1 * exp(-oxygen / HILL.2);
    let saturation = 1.0 / (1.0 + exp(-exponent * t));
    let content = SOLUBILITY * oxygen + HEME * HEMOGLOBIN * saturation;
    let dissolved = CARBON_DIOXIDE_SOLUBILITY * pressure;
    let reach = CARBAMINO.half + CARBAMINO.half_rise * saturation;
    let carbamino = CARBAMINO.sites * HEMOGLOBIN * dissolved / (dissolved + reach);
    Blood {
        oxygen_pressure: oxygen,
        saturation,
        oxygen_content: content,
        acidity,
        bicarbonate,
        carbon_dioxide_pressure: pressure,
        carbon_dioxide_total: bicarbonate + dissolved + carbamino,
    }
}

fn blood(site: &str) -> Blood {
    let arterial = matches!(
        site,
        "pulmonary bed" | "pulmonary vein" | "left atrium" | "left ventricle" | "systemic arteries"
    );
    let arterial_pressure = ALVEOLAR_CARBON_DIOXIDE + CARBON_DIOXIDE_HEAD;
    if arterial {
        speciated(REACH.0, arterial_pressure)
    } else {
        speciated(REACH.1, VENOUS_CARBON_DIOXIDE_GUESS)
    }
}

fn oxygen_guess(site: &str, space: Space) -> f64 {
    let Blood {
        oxygen_pressure: pressure,
        oxygen_content: content,
        ..
    } = blood(site);
    match space {
        Space::Plasma | Space::Lumen => content,
        Space::Interstitium => SOLUBILITY * (pressure - HEAD),
        _ => SOLUBILITY * (pressure - 2.0 * HEAD),
    }
}

fn carbon_dioxide_guess(site: &str, space: Space) -> f64 {
    let Blood {
        carbon_dioxide_pressure: pressure,
        carbon_dioxide_total: total,
        ..
    } = blood(site);
    let tissue = |ratio: f64, heads: f64| {
        CARBON_DIOXIDE_SOLUBILITY * (1.0 + ratio) * (pressure + heads * CARBON_DIOXIDE_HEAD)
    };
    match space {
        Space::Plasma | Space::Lumen => total,
        Space::Interstitium => tissue(INTERSTITIAL_BUFFER_RATIO, 1.0),
        _ => tissue(CELL_BUFFER_RATIO, 2.0),
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
    body_packet_at_viscosity_exponent(body, Measure::new(-1.0)?)
}

/// The body's packet with viscosity's exponent `C` at `exponent`, so a test reads the row at a sign the body does not
/// take.
pub fn body_packet_at_viscosity_exponent(
    body: &CompiledBody,
    exponent: Measure<Dimensionless>,
) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
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
    let breath = LawName {
        owner: Respiratory::CARD,
        local: "breath",
    };
    packet.start_mode(breath, "rest", NOTE)?;

    table(&mut packet, exponent)?;
    oxygen_per_lactate(&mut packet, body)?;
    zero_stand_ins(&mut packet, body)?;
    packet.start_rest(body, start_guess, NOTE)?;
    for row in body.rows() {
        let kind = row.kind();
        if let RowKind::Memory(slot) | RowKind::Algebraic(slot) = kind {
            match slot.law.local {
                "valves" | "circulation" | "sinusoids" => {
                    packet.row_scale(kind, Measure::<Rate<Water>>::new(FLOW_SIZE)?)?
                }
                "wall power" => packet.row_scale(kind, Measure::<Rate<Energy>>::new(POWER_SIZE)?)?,
                "contractile store" => packet.row_scale(kind, Measure::<Energy>::new(CONTRACTILE_SIZE)?)?,
                _ => {}
            }
        }
    }
    Ok(packet)
}

fn table(
    packet: &mut ParameterPacket<SyntheticFixture>,
    exponent: Measure<Dimensionless>,
) -> Result<(), AdmissionError> {
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
    set::<Dimensionless>(packet, "cv.collapse-power", body, 1.5)?;
    set::<Dimensionless>(packet, "cv.collapse-share", blood, 0.05)?;
    set::<Rate<Energy>>(packet, "cv.heat-width", body, 100.0)?;
    set::<Quotient<Volume, Albumin>>(packet, "cv.albumin-volume", body, 0.05)?;
    set::<Quotient<Volume, Albumin>>(packet, "fl.albumin-volume", body, 0.05)?;
    set::<Pressure>(packet, "gi.abdominal-pressure", body, 4.0)?;
    respiratory(packet)?;
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
        (artery, pulmonary_vein, SHUNT),
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
    set::<Dimensionless>(packet, "rc.avp-efficacy", blood, 2.0)?;
    hematology(packet, exponent)?;
    gases(packet)
}

fn hematology(
    packet: &mut ParameterPacket<SyntheticFixture>,
    exponent: Measure<Dimensionless>,
) -> Result<(), AdmissionError> {
    let body = Binding::Body;
    set::<Viscosity>(packet, "hc.plasma-viscosity", body, 2.0)?;
    set::<Concentration<Hemoglobin>>(packet, "hc.red-cell-hemoglobin", body, 5.0)?;
    for (key, value) in [
        ("hc.reference-hematocrit", Measure::new(0.6)?),
        ("hc.relative-viscosity-at-reference", Measure::new(4.0)?),
        ("hc.viscosity-exponent", exponent),
        ("hc.wall-factor", Measure::new(1.25)?),
        ("hc.unit", Measure::new(1.0)?),
    ] {
        packet.synthetic(ParameterKey::new(key), body, value, NOTE)?;
    }
    set::<Quotient<Dimensionless, Viscosity>>(
        packet,
        "cv.reference-viscosity-reciprocal",
        body,
        1.0 / START_VISCOSITY,
    )
}

fn gases(packet: &mut ParameterPacket<SyntheticFixture>) -> Result<(), AdmissionError> {
    let body = Binding::Body;
    type Solubility = Quotient<Concentration<Oxygen>, Pressure>;
    set::<Solubility>(packet, "bc.oxygen-solubility", body, SOLUBILITY)?;
    set::<Solubility>(packet, "mt.tissue-oxygen-solubility", body, SOLUBILITY)?;
    set::<Quotient<Concentration<Oxygen>, Concentration<Hemoglobin>>>(packet, "bc.heme-oxygen", body, HEME)?;
    for (key, value) in [
        ("bc.p50", P50),
        ("bc.hill-reach", HILL.2),
        ("bc.standard-carbon-dioxide", BY_CARBON_DIOXIDE.standard),
        ("bc.carbon-dioxide-scale", BY_CARBON_DIOXIDE.scale),
    ] {
        set::<Pressure>(packet, key, body, value)?;
    }
    let [c1, c2, c3] = BY_ACIDITY.cubic;
    let [q1, q2] = BY_CARBON_DIOXIDE.quadratic;
    for (key, value) in [
        ("bc.hill-ceiling", HILL.0),
        ("bc.hill-fall", HILL.1),
        ("bc.standard-acidity", BY_ACIDITY.standard),
        ("bc.p50-acidity-1", c1),
        ("bc.p50-acidity-2", c2),
        ("bc.p50-acidity-3", c3),
        ("bc.p50-carbon-dioxide-1", q1),
        ("bc.p50-carbon-dioxide-2", q2),
        ("bc.pk", PK),
        ("bc.ln-ten", core::f64::consts::LN_10),
        ("mt.interstitial-buffer-ratio", INTERSTITIAL_BUFFER_RATIO),
        ("mt.cell-buffer-ratio", CELL_BUFFER_RATIO),
    ] {
        set::<Dimensionless>(packet, key, body, value)?;
    }
    carbon_dioxide(packet)?;
    let total: f64 = USES.iter().sum();
    set::<Conductance<Oxygen>>(packet, "rs.oxygen-diffusing", body, total / HEAD)?;
    let made = PER_OXYGEN * total / CARBON_DIOXIDE_HEAD;
    set::<Conductance<CarbonDioxide>>(packet, "rs.carbon-dioxide-diffusing", body, made)?;
    burn(packet)?;
    set::<Quotient<Oxygen, Energy>>(packet, "mt.oxygen-per-energy", body, 1.0 / ENERGY_PER_OXYGEN)?;
    set::<Rate<Dimensionless>>(packet, "cv.work-speed", body, WORK_SPEED)?;
    let blood = Binding::Network(Network::Vascular);
    set::<Dimensionless>(packet, "cv.debt-share", blood, 0.0)?;
    set::<Oxygen>(packet, "cv.debt-half", blood, 1.0)?;
    for (bed, used) in BEDS.into_iter().zip(USES) {
        let at = Binding::Site(site::<CapillaryBed>(bed)?);
        // The liver's inflows are the hepatic sinusoids', which read no debt; the peritubular inflow leaves
        // the glomerular bed, whose tone is the kidney's; the lung's vessels answer alveolar oxygen, not their
        // tissue's debt.
        if bed != "liver bed" {
            let silent = matches!(bed, "peritubular bed" | "pulmonary bed");
            let share = if silent { 0.0 } else { DEBT_SHARE };
            set::<Dimensionless>(packet, "cv.debt-share", at, share)?;
            set::<Oxygen>(packet, "cv.debt-half", at, used * DEBT_HALF_TIME)?;
        }
        set::<Rate<Oxygen>>(packet, "mt.oxygen-demand", at, basal(bed, used))?;
        set::<Conductance<Oxygen>>(packet, "mt.oxygen-reach", at, used / BODY_CELL_PRESSURE_AT_DEMAND)?;
        set::<Conductance<Oxygen>>(packet, "mt.wall-oxygen-conductance", at, used / HEAD)?;
        set::<Conductance<Oxygen>>(packet, "mt.cell-oxygen-conductance", at, used / HEAD)?;
        let made = PER_OXYGEN * used / CARBON_DIOXIDE_HEAD;
        type Conducted = Conductance<CarbonDioxide>;
        set::<Conducted>(packet, "mt.wall-carbon-dioxide-conductance", at, made)?;
        set::<Conducted>(packet, "mt.cell-carbon-dioxide-conductance", at, made)?;
    }
    Ok(())
}

/// The debt's oxygen to each lactate is the lactate burn's own counts, read from the compiled body.
fn oxygen_per_lactate(
    packet: &mut ParameterPacket<SyntheticFixture>,
    body: &CompiledBody,
) -> Result<(), AdmissionError> {
    let burn = LawName {
        owner: Metabolism::CARD,
        local: "lactate burn",
    };
    let column = (body.columns().iter()).find(|column| column.origin() == burn);
    let count = |species| {
        (column.into_iter().flat_map(|column| column.terms()))
            .filter(|(row, _)| {
                matches!(body.rows()[row.index()].kind(), RowKind::Amount { species: of, .. } if of == species)
            })
            .map(|&(_, count)| count as f64)
            .sum::<f64>()
    };
    let beds = Binding::Class(Class::CapillaryBed);
    let per = count(Species::Oxygen) / count(Species::Lactate);
    set::<Quotient<Oxygen, Lactate>>(packet, "mt.oxygen-per-lactate", beds, per)
}

fn burn(packet: &mut ParameterPacket<SyntheticFixture>) -> Result<(), AdmissionError> {
    let (body, beds) = (Binding::Body, Binding::Class(Class::CapillaryBed));
    set::<Dimensionless>(packet, "mt.glucose-share", beds, GLUCOSE_SHARE)?;
    set::<Dimensionless>(packet, "mt.oxygen-rate-width-share", beds, WIDTH_SHARE_OF_DEMAND)?;
    set::<Rate<Dimensionless>>(packet, "mt.bed-heat-speed", beds, BED_HEAT_SPEED)?;
    set::<Rate<Dimensionless>>(packet, "mt.feed-speed", body, FEED_SPEED)?;
    set::<Glucose>(packet, "mt.cell-glucose-set", body, GLUCOSE_SET)?;
    set::<FattyAcids>(packet, "mt.cell-fatty-acid-set", body, FAT_SET)?;
    set::<Water>(packet, "mt.cell-water-set", body, CELL_WATER)?;
    set::<Rate<Dimensionless>>(packet, "mt.cell-water-speed", body, WATER_SPEED)?;
    set::<Quotient<Glucose, Oxygen>>(
        packet,
        "mt.glucose-per-oxygen-short",
        beds,
        GLUCOSE_PER_OXYGEN_SHORT,
    )?;
    set::<Rate<Dimensionless>>(packet, "mt.lactate-burn-speed", beds, LACTATE_BURN_SPEED)?;
    set::<Pressure>(
        packet,
        "mt.lactate-burn-half-pressure",
        beds,
        LACTATE_HALF_PRESSURE,
    )
}

fn carbon_dioxide(packet: &mut ParameterPacket<SyntheticFixture>) -> Result<(), AdmissionError> {
    let body = Binding::Body;
    type Solubility = Quotient<Concentration<CarbonDioxide>, Pressure>;
    set::<Solubility>(
        packet,
        "bc.carbon-dioxide-solubility",
        body,
        CARBON_DIOXIDE_SOLUBILITY,
    )?;
    set::<Solubility>(
        packet,
        "mt.tissue-carbon-dioxide-solubility",
        body,
        CARBON_DIOXIDE_SOLUBILITY,
    )?;
    set::<Concentration<Charge>>(packet, "bc.strong-ion-offset", body, STRONG_ION_OFFSET)?;
    type PerCharge = Quotient<Concentration<CarbonDioxide>, Concentration<Charge>>;
    set::<PerCharge>(packet, "bc.bicarbonate-per-charge", body, 1.0)?;
    set::<Concentration<Albumin>>(packet, "bc.plasma-albumin", body, ALBUMIN.concentration)?;
    set::<Concentration<Phosphate>>(packet, "bc.plasma-phosphate", body, PHOSPHATE.concentration)?;
    type PerAcid<Q> = Quotient<Concentration<Charge>, Concentration<Q>>;
    set::<PerAcid<Albumin>>(packet, "bc.albumin-slope", body, ALBUMIN.slope)?;
    set::<PerAcid<Albumin>>(packet, "bc.albumin-offset", body, ALBUMIN.offset)?;
    set::<PerAcid<Phosphate>>(packet, "bc.phosphate-slope", body, PHOSPHATE.slope)?;
    set::<PerAcid<Phosphate>>(packet, "bc.phosphate-offset", body, PHOSPHATE.offset)?;
    set::<PerAcid<Hemoglobin>>(packet, "bc.hemoglobin-slope", body, HEMOGLOBIN_CHARGE.0)?;
    set::<PerAcid<Hemoglobin>>(packet, "bc.hemoglobin-offset", body, HEMOGLOBIN_CHARGE.1)?;
    type Sites = Quotient<Concentration<CarbonDioxide>, Concentration<Hemoglobin>>;
    set::<Sites>(packet, "bc.carbamino-sites", body, CARBAMINO.sites)?;
    set::<Concentration<CarbonDioxide>>(packet, "bc.carbamino-half", body, CARBAMINO.half)?;
    set::<Concentration<CarbonDioxide>>(packet, "bc.carbamino-half-rise", body, CARBAMINO.half_rise)
}

/// The body's own drift bounds for energy, for the elements its reactions join and for each species that moves on it.
/// As on the slice (solver contract, section 7), each is fixture data that stands in for 1.96 times a within-subject
/// variation: twice the quantity's worst change over a ten-second window of this body's first minute from its admitted
/// start, rounded up to one significant figure. It is a synthetic target that scores a solver and judges no body. The
/// body's measured day (ticket 24) replaces it, and a reviewed packet's bound comes only from the healthy-range set
/// (record 0004). Energy's worst change is 1,347, in the window from 47 s. Oxygen's and carbon's change is mostly the
/// blood gases' swing within a beat, read across windows that hold no whole number of beats, and hydrogen's is a bed's
/// cell fuels and water settling, under 7.5e-6 at every bed but the stroke work's payer, whose cells settle by 0.077
/// over the first ten seconds whatever their start, so they take a bound of their own. The species are the held gut's and skin's
/// water and salts, vasopressin's held secretion and clearance, and nitrogen gas, whose worst change is 0.380 in
/// the first ten seconds and which falls a steady 0.22 to 0.26 in each ten seconds after, following the gas volume as
/// the net exchange settles with the body's stores. Oxygen and carbon dioxide as species keep only held
/// pools that never move, so the slice's bound for an unmoved species holds them.
const ENERGY_DRIFT: f64 = 3e3;
const ELEMENT_DRIFTS: [(Element, f64); 3] = [
    (Element::Carbon, 0.2),
    (Element::Hydrogen, 2e-5),
    (Element::Oxygen, 0.6),
];
const PAYER_HYDROGEN_DRIFT: f64 = 0.2;
const SPECIES_DRIFTS: [(Species, f64); 7] = [
    (Species::Energy, ENERGY_DRIFT),
    (Species::Water, 0.04),
    (Species::Sodium, 9e-4),
    (Species::Chloride, 9e-4),
    (Species::Potassium, 5e-5),
    (Species::Vasopressin, 2e-3),
    (Species::NitrogenGas, 0.8),
];

struct BodyDrifts;
impl DriftTable for BodyDrifts {
    fn names(&self, species: Species) -> bool {
        SPECIES_DRIFTS.iter().any(|(named, _)| *named == species)
    }
    fn drift<Q: Substance>(&self) -> Result<Measure<Q>, AdmissionError> {
        let named = SPECIES_DRIFTS.iter().find(|(named, _)| *named == Q::SPECIES);
        Measure::new(named.map_or(f64::NAN, |(_, drift)| *drift))
    }
    fn element(&self, element: Element, at: &[SiteId]) -> Result<Option<Measure<Atoms>>, AdmissionError> {
        if element == Element::Hydrogen && at.iter().all(|site| site.key() == STROKE_PAYER) {
            return Measure::new(PAYER_HYDROGEN_DRIFT).map(Some);
        }
        let named = ELEMENT_DRIFTS.iter().find(|(named, _)| *named == element);
        named.map(|(_, drift)| Measure::new(*drift)).transpose()
    }
}

pub fn body_certificate(
    body: &CompiledBody,
    windows: SlidingWindows,
) -> Result<CertificatePolicy, AdmissionError> {
    crate::certificate_with(body, windows, &BodyDrifts)
}

fn respiratory(packet: &mut ParameterPacket<SyntheticFixture>) -> Result<(), AdmissionError> {
    let body = Binding::Body;
    for (key, value) in [
        ("rs.oxygen-inspired", 0.21),
        ("rs.carbon-dioxide-inspired", 0.0004),
    ] {
        set::<Dimensionless>(packet, key, body, value)?;
    }
    set::<Pressure>(packet, "rs.room-pressure", body, 700.0)?;
    set::<Pressure>(packet, "rs.water-vapour-pressure", body, 50.0)?;
    set::<Pressure>(packet, "rs.head-width", body, 0.01)?;
    set::<Elastance>(packet, "rs.lung-elastance", body, 1.6e-4)?;
    set::<Elastance>(packet, "rs.wall-elastance", body, 1.6e-4)?;
    set::<Volume>(packet, "rs.lung-rest-volume", body, 38_250.0)?;
    set::<Volume>(packet, "rs.wall-rest-volume", body, WALL_REST)?;
    set::<Elastance>(packet, "rs.dead-space-elastance", body, 6.4e-4)?;
    set::<Volume>(packet, "rs.dead-space-volume", body, DEAD_SPACE)?;
    set::<Quotient<Volume, Oxygen>>(packet, "rs.oxygen-volume", body, 1.0)?;
    set::<Quotient<Volume, CarbonDioxide>>(packet, "rs.carbon-dioxide-volume", body, 1.0)?;
    set::<Quotient<Volume, NitrogenGas>>(packet, "rs.nitrogen-volume", body, 1.0)?;
    set::<Conductance<Volume>>(packet, "rs.airway-conductance", body, 15_600.0)?;
    set::<Rate<Volume>>(packet, "rs.airway-mixing", body, RHYTHM.mixing())?;
    for (key, value) in [
        ("rs.breath-rate", RHYTHM.rate),
        ("rs.drive-rise", RHYTHM.rise),
        ("rs.drive-fall", RHYTHM.fall),
        ("rs.rest-speed", 1.0),
        ("rs.share-fall", 1.0),
        ("rs.breath-start", 1_000.0),
    ] {
        set::<Rate<Dimensionless>>(packet, key, body, value)?;
    }
    for (key, value) in [
        ("rs.inspiration-end", RHYTHM.inspiration_end),
        ("rs.breath-end", 1.0),
        ("rs.drive-peak", RHYTHM.peak),
        ("rs.rest-drive", RHYTHM.mean_drive()),
        ("rs.rest-share", 1.0),
    ] {
        set::<Dimensionless>(packet, key, body, value)?;
    }
    set::<Pressure>(packet, "rs.pump-pressure", body, RHYTHM.pump)?;
    set::<Volume>(packet, "rs.wall-rest-rise", body, 10_000.0)?;
    set::<Length>(packet, "rs.trunk-length", body, TRUNK)?;
    Ok(())
}

const ELASTANCE: f64 = 1.6e-4 + 1.6e-4;
const WALL_REST: f64 = 75_750.0;
const DEAD_SPACE: f64 = 3_400.0;
/// How far the carotid sinus sits up the trunk from the systemic arteries, so the rise over it is the trunk's sine.
const TRUNK: f64 = 0.2;

/// The breath's synthetic rhythm: a rate in breaths a second, the phase at which inspiration ends, and the speeds at
/// which the pump drive rises toward its peak and falls toward zero.
struct Rhythm {
    rate: f64,
    inspiration_end: f64,
    rise: f64,
    fall: f64,
    peak: f64,
    pump: f64,
}

const RHYTHM: Rhythm = Rhythm {
    rate: 0.2,
    inspiration_end: 0.4,
    rise: 1.5,
    fall: 2.0,
    peak: 1.0,
    pump: 4.0,
};

impl Rhythm {
    /// The drive at inspiration's start and end once breaths repeat: `a1 = peak (1 - e_i) + a0 e_i` and `a0 = a1 e_e`,
    /// with `e_i` and `e_e` the rise's and the fall's decay over inspiration and expiration.
    fn ends(&self) -> (f64, f64, f64, f64) {
        let (inspiring, expiring) = (
            self.inspiration_end / self.rate,
            (1.0 - self.inspiration_end) / self.rate,
        );
        let (e_i, e_e) = (exp(-self.rise * inspiring), exp(-self.fall * expiring));
        let end = self.peak * (1.0 - e_i) / (1.0 - e_i * e_e);
        (end * e_e, end, inspiring, expiring)
    }
    /// The drive's mean over a breath, so the rest mode holds the breathing body's mean pleural pressure and volume.
    fn mean_drive(&self) -> f64 {
        let (start, end, inspiring, expiring) = self.ends();
        let rising =
            self.peak * inspiring + (start - self.peak) * (1.0 - exp(-self.rise * inspiring)) / self.rise;
        let falling = end * (1.0 - exp(-self.fall * expiring)) / self.fall;
        (rising + falling) * self.rate
    }
    /// The mixing exchange whose alveolar gas matches a breath's mean: two mixed exchanges in series put the alveolar
    /// unit at `2 P / M`, and a breath of volume `V_T` through a mixed dead space `V_D` at `P T / (V_T - V_D
    /// tanh(V_T / 2 V_D))` while the alveolar gas holds through the breath. `V_T` is the drive's swing at the
    /// mechanics' static solution.
    fn mixing(&self) -> f64 {
        let (start, end, ..) = self.ends();
        let tidal = self.pump * (end - start) / ELASTANCE;
        let x = tidal / DEAD_SPACE;
        let tanh = (1.0 - exp(-x)) / (1.0 + exp(-x));
        2.0 * (tidal - DEAD_SPACE * tanh) * self.rate
    }
}
