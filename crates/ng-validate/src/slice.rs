use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn at(axial: f64) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(axial)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// Grouped as record 0002 asks. Heights are synthetic and run from the hip. An edge into a bed stands for its arterioles,
/// and the edge from the systemic arteries into the glomerular bed is the renal afferent edge.
pub fn arena_anatomy() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let heart = map.site::<TissueSite>("heart", at(0.5)?);
    let kidney = map.site::<TissueSite>("kidney", at(0.38)?);
    let carotid = map.site::<TissueSite>("carotid sinus", at(0.7)?);
    let arch = map.site::<TissueSite>("aortic arch", at(0.55)?);

    let right_atrium = map.site::<VesselSegment>("right atrium", at(0.5)?);
    let right_ventricle = map.site::<VesselSegment>("right ventricle", at(0.5)?);
    let left_atrium = map.site::<VesselSegment>("left atrium", at(0.5)?);
    let left_ventricle = map.site::<VesselSegment>("left ventricle", at(0.5)?);
    let pulmonary_artery = map.site::<VesselSegment>("pulmonary artery", at(0.55)?);
    let pulmonary_vein = map.site::<VesselSegment>("pulmonary vein", at(0.55)?);
    let arteries = map.site::<VesselSegment>("systemic arteries", at(0.5)?);
    let central_veins = map.site::<VesselSegment>("central veins", at(0.5)?);
    let splanchnic_veins = map.site::<VesselSegment>("splanchnic veins", at(0.25)?);
    let leg_veins = map.site::<VesselSegment>("leg veins", at(-0.5)?);

    let lung = map.site::<CapillaryBed>("pulmonary bed", at(0.55)?);
    let splanchnic = map.site::<CapillaryBed>("splanchnic bed", at(0.25)?);
    let leg = map.site::<CapillaryBed>("leg bed", at(-0.5)?);
    let glomerular = map.site::<CapillaryBed>("glomerular bed", at(0.38)?);
    let peritubular = map.site::<CapillaryBed>("peritubular bed", at(0.38)?);

    let pleura = map.site::<EnclosedSpace>("pleural space", at(0.55)?);
    let abdomen = map.site::<EnclosedSpace>("abdominal space", at(0.25)?);

    let sinoatrial = map.site::<Junction>("sinoatrial junction", at(0.5)?);
    let myocardial = map.site::<Junction>("myocardial junction", at(0.5)?);
    let arteriolar = map.site::<Junction>("arteriolar junction", at(0.15)?);
    let venous = map.site::<Junction>("venous junction", at(0.15)?);
    let renal = map.site::<Junction>("renal junction", at(0.38)?);

    let tubule = map.site::<VesselSegment>("renal tubule", at(0.38)?);
    let duct = map.site::<VesselSegment>("collecting duct", at(0.38)?);
    let bladder = map.site::<VesselSegment>("bladder", at(0.05)?);

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
        (arteries.id(), splanchnic.id()),
        (splanchnic.id(), splanchnic_veins.id()),
        (splanchnic_veins.id(), central_veins.id()),
        (arteries.id(), leg.id()),
        (leg.id(), leg_veins.id()),
        (arteries.id(), glomerular.id()),
        (glomerular.id(), peritubular.id()),
        (peritubular.id(), central_veins.id()),
        (central_veins.id(), right_atrium.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    for bed in [
        lung.id(),
        splanchnic.id(),
        leg.id(),
        glomerular.id(),
        peritubular.id(),
    ] {
        map.connect(Network::Lymph, bed, central_veins.id());
    }
    map.connect(Network::Urinary, glomerular.id(), tubule.id());
    map.connect(Network::Urinary, tubule.id(), duct.id());
    map.connect(Network::Urinary, duct.id(), bladder.id());
    map.connect(Network::Reclaim, tubule.id(), peritubular.id());
    map.connect(Network::Reclaim, duct.id(), peritubular.id());

    map.supply(&myocardial, &[right_ventricle.id(), left_ventricle.id()]);
    map.supply(&arteriolar, &[splanchnic.id(), leg.id()]);
    map.supply(&venous, &[splanchnic_veins.id(), leg_veins.id()]);
    map.supply(&renal, &[glomerular.id()]);
    map.unsupplied(&[
        right_atrium.id(),
        left_atrium.id(),
        pulmonary_artery.id(),
        pulmonary_vein.id(),
        lung.id(),
        arteries.id(),
        central_veins.id(),
        peritubular.id(),
    ]);
    map.landmark(Landmark::CarotidSinus, arteries.id(), carotid.id());
    map.landmark(Landmark::AorticArch, arteries.id(), arch.id());
    map.landmark(Landmark::Pacemaker, sinoatrial.id(), sinoatrial.id());
    map.landmark(Landmark::MaculaDensa, tubule.id(), tubule.id());

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
        ],
        pleural,
    );
    map.enclose(&abdomen, &[splanchnic.id(), splanchnic_veins.id()], abdominal);

    map.served_by(carotid.id(), &cranial, Afferent::Stretch);
    map.served_by(arch.id(), &cranial, Afferent::Stretch);

    map.region("body", body.id());
    map.region("heart", heart.id());
    map.region("kidney", kidney.id());
    map.finish()
}

const GUT_WATER: &str = "gut water";
const GUT_SODIUM: &str = "gut sodium";
const GUT_POTASSIUM: &str = "gut potassium";
const GUT_CHLORIDE: &str = "gut chloride";
const SKIN_WATER: &str = "skin water";
const URINE: &str = "urine";
const HEAT: &str = "heat";
const SECRETION: &str = "vasopressin secretion";
const CLEARANCE: &str = "vasopressin clearance";

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    Ok(AnatomyBuilder::new().site::<C>(key, at(0.0)?).id())
}

/// Which species live where, and where each route crosses the boundary: composition data, so a law names neither.
pub fn arena_schema() -> Result<Schema, AdmissionError> {
    let mut schema = Schema::new(arena_anatomy()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    fn ions<Q: Substance>(schema: &mut Schema) {
        schema.carry::<Q, Blood>(Space::Plasma);
        schema.carry::<Q, Blood>(Space::Lumen);
        schema.carry::<Q, CapillaryBed>(Space::Interstitium);
        schema.carry::<Q, Urinary>(Space::Lumen);
    }
    ions::<Sodium>(&mut schema);
    ions::<Potassium>(&mut schema);
    ions::<Chloride>(&mut schema);
    schema.carry::<Albumin, Blood>(Space::Plasma);
    schema.carry::<Albumin, Blood>(Space::Lumen);
    schema.carry::<Albumin, CapillaryBed>(Space::Interstitium);
    schema.carry::<Vasopressin, Blood>(Space::Plasma);
    schema.carry::<Vasopressin, Blood>(Space::Lumen);

    for space in [Space::Vesicle, Space::Cleft, Space::Bound] {
        schema.carry::<Norepinephrine, Junction>(space);
        schema.pool::<Acetylcholine>(site::<Junction>("sinoatrial junction")?, space)?;
    }
    schema.pool::<Vasopressin>(site::<CapillaryBed>("peritubular bed")?, Space::Bound)?;
    let body = site::<TissueSite>("body")?;
    schema.pool::<Energy>(body, Space::Elastic)?;
    schema.pool::<Energy>(body, Space::Thermal)?;

    let splanchnic = site::<VesselSegment>("splanchnic veins")?;
    let gut = |schema: &mut Schema| -> Result<(), AdmissionError> {
        let into = BoundaryDirection::IntoBody;
        schema.boundary::<Water>(GUT_WATER, into, splanchnic, Space::Lumen)?;
        schema.boundary::<Sodium>(GUT_SODIUM, into, splanchnic, Space::Lumen)?;
        schema.boundary::<Potassium>(GUT_POTASSIUM, into, splanchnic, Space::Lumen)?;
        schema.boundary::<Chloride>(GUT_CHLORIDE, into, splanchnic, Space::Lumen)
    };
    gut(&mut schema)?;
    let out = BoundaryDirection::OutOfBody;
    schema.boundary::<Water>(
        SKIN_WATER,
        out,
        site::<CapillaryBed>("leg bed")?,
        Space::Interstitium,
    )?;
    schema.drain(URINE, site::<VesselSegment>("bladder")?, Space::Lumen)?;
    schema.boundary::<Energy>(HEAT, out, body, Space::Thermal)?;
    let veins = site::<VesselSegment>("central veins")?;
    schema.boundary::<Vasopressin>(SECRETION, BoundaryDirection::IntoBody, veins, Space::Lumen)?;
    schema.boundary::<Vasopressin>(CLEARANCE, out, veins, Space::Lumen)?;
    Ok(schema)
}

const FIXED: &str = "arena fixture: synthetic, with no feedback from the slice";

fn hours() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

fn enclosure_law<'b, S: OutsideSlice>(
    laws: &mut FragmentBuilder<'b, S>,
    ports: &HeldPorts<'b, S>,
    key: ParameterKey,
) -> Result<(), AdmissionError> {
    let held = laws.parameter::<Pressure>(key, Admissible::finite());
    let class = hours()?;
    let pressure = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&ports.enclosure_pressure, pressure);
    Ok(())
}

fn held<S: OutsideSlice>(body: &mut Assembly<'_>, holds: Vec<Space>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<S>::new(Vec::new(), holds, FIXED);
    let (laws, _ports) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}

fn absorption<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ports: &HeldPorts<'b, Gastrointestinal>,
    name: &'static str,
    key: ParameterKey,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, drive } = ports.route::<Q>(name)?;
    let basal = laws.parameter::<Rate<Q>>(key, Admissible::nonnegative());
    let class = hours()?;
    let rate = laws.kernel(|mut k| {
        let (steady, meal) = (k.parameter(&basal), k.read(&drive));
        let total = k.add(steady, meal);
        k.finish(total, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

fn gut(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let supplies = vec![GUT_WATER, GUT_SODIUM, GUT_POTASSIUM, GUT_CHLORIDE];
    let declaration = HeldDeclaration::<Gastrointestinal>::new(supplies, Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    enclosure_law(&mut laws, &ports, ParameterKey::new("gi.abdominal-pressure"))?;
    absorption::<Water>(&mut laws, &ports, GUT_WATER, ParameterKey::new("gi.basal-water"))?;
    absorption::<Sodium>(
        &mut laws,
        &ports,
        GUT_SODIUM,
        ParameterKey::new("gi.basal-sodium"),
    )?;
    absorption::<Potassium>(
        &mut laws,
        &ports,
        GUT_POTASSIUM,
        ParameterKey::new("gi.basal-potassium"),
    )?;
    absorption::<Chloride>(
        &mut laws,
        &ports,
        GUT_CHLORIDE,
        ParameterKey::new("gi.basal-chloride"),
    )?;
    body.add(laws.finish());
    Ok(())
}

/// Secretion follows plasma sodium on a logistic, clearance is first order. Both held, because EN is outside the slice.
/// Model default.
fn vasopressin(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Endocrine>::new(vec![SECRETION, CLEARANCE], Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let secretion = ports.route::<Vasopressin>(SECRETION)?.route;
    let clearance = ports.route::<Vasopressin>(CLEARANCE)?.route;
    let most = laws.parameter::<Rate<Vasopressin>>(ParameterKey::new("en.avp-most"), Admissible::positive());
    let set =
        laws.parameter::<Concentration<Sodium>>(ParameterKey::new("en.avp-sodium"), Admissible::positive());
    let width =
        laws.parameter::<Concentration<Sodium>>(ParameterKey::new("en.avp-width"), Admissible::positive());
    let speed =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("en.avp-clearance"), Admissible::positive());
    let rate = laws.kernel(|mut k| {
        let (sodium, water) = (
            k.read(&secretion.amount::<Sodium>()),
            k.read(&secretion.amount::<Water>()),
        );
        let volume = k.volume(water);
        let ratio = k.divide(sodium, volume);
        let seen: Expr<'_, '_, _, Concentration<Sodium>> = k.as_dimension(ratio);
        let center = k.parameter(&set);
        let above = k.subtract(seen, center);
        let spread = k.parameter(&width);
        let ratio = k.divide(above, spread);
        let drive = k.as_dimension(ratio);
        let share = k.logistic(drive);
        let full = k.parameter(&most);
        let rate = k.scale(full, share);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&secretion, rate));
    let rate = laws.kernel(|mut k| {
        let held = k.read(&clearance.amount::<Vasopressin>());
        let (zero, pace) = (k.zero(), k.parameter(&speed));
        let leaving = k.relax(held, zero, pace);
        k.finish(leaving, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&clearance, rate));
    body.add(laws.finish());
    Ok(())
}

fn skin(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Musculoskeletal>::new(vec![SKIN_WATER], Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let BoundaryRoute { route, .. } = ports.route::<Water>(SKIN_WATER)?;
    let steady = laws.parameter::<Rate<Water>>(ParameterKey::new("ms.skin-water"), Admissible::nonnegative());
    let class = hours()?;
    let rate = laws.kernel(|mut k| {
        let value = k.parameter(&steady);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    body.add(laws.finish());
    Ok(())
}

/// RS and GI hold their enclosures' pressure, packet values under `rs.pleural-pressure` and
/// `gi.abdominal-pressure`. GI, EN and MS hold the routes no slice system owns. BC holds cell water, while cell-water
/// shifts stay an open conflict with FL. MT stands behind the chemical energy the heart's work draws on.
pub fn hold_outside_slice(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let lungs = HeldDeclaration::<Respiratory>::new(Vec::new(), Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(lungs)?.parts();
    enclosure_law(&mut laws, &ports, ParameterKey::new("rs.pleural-pressure"))?;
    body.add(laws.finish());
    gut(body)?;
    vasopressin(body)?;
    skin(body)?;
    held::<BloodChemistry>(body, vec![Space::Cells])?;
    held::<Metabolism>(body, Vec::new())?;
    held::<Hepatic>(body, Vec::new())?;
    held::<Immune>(body, Vec::new())?;
    held::<Hematology>(body, Vec::new())
}

/// Urine and heat leave by a law on what the pool holds, so water and energy answer their own flows: admission solves
/// their totals. Model defaults: the bladder drains in proportion to what it holds past a rest, and voiding as an
/// event is outside the grouped map.
fn exits(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let (mut laws, ports) = body.boundary()?.parts();
    let urine = ports.route::<Water>(URINE)?.route;
    let rest = laws.parameter::<Water>(ParameterKey::new("bd.bladder-rest"), Admissible::nonnegative());
    let voiding =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.bladder-speed"), Admissible::positive());
    let width = laws.parameter::<Rate<Water>>(ParameterKey::new("bd.bladder-width"), Admissible::positive());
    let heat = ports.route::<Energy>(HEAT)?.route;
    let loss =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.heat-speed"), Admissible::positive());
    let class = hours()?;
    let rate = laws.kernel(|mut k| {
        let (held, floor, pace) = (
            k.read(&urine.amount::<Water>()),
            k.parameter(&rest),
            k.parameter(&voiding),
        );
        let draining = k.relax(held, floor, pace);
        let (zero, soft) = (k.zero(), k.parameter(&width));
        let rate = k.smooth_max(draining, zero, soft);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&urine, rate));
    let rate = laws.kernel(|mut k| {
        let (held, zero, pace) = (k.read(&heat.amount::<Energy>()), k.zero(), k.parameter(&loss));
        let rate = k.relax(held, zero, pace);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&heat, rate));
    body.add(laws.finish());
    Ok(())
}

/// The composition root: the one place every system meets the schema.
pub fn arena_body() -> Result<CompiledBody, AdmissionError> {
    assemble(arena_schema()?, |mut body| {
        hold_outside_slice(&mut body)?;
        arena_organs(&mut body)?;
        body.compile()
    })
}

pub fn arena_organs(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    exits(body)?;
    let fragment = ng_cardiovascular::declare(body.cardiovascular()?)?;
    body.add(fragment);
    let fragment = ng_fluids::declare(body.fluids()?)?;
    body.add(fragment);
    let fragment = ng_renal::declare(body.renal()?)?;
    body.add(fragment);
    let fragment = ng_nervous::declare(body.nervous()?)?;
    body.add(fragment);
    let fragment = ng_receptor::declare(body.receptor()?)?;
    body.add(fragment);
    Ok(())
}

const SECOND: u64 = 1_000_000_000;
const POINTS: u32 = 100;
const FLOW_SIZE: f64 = 20.0;
/// The power that keeps the elastic store on its sum. That row rests on the store's change over a stage, so doubled
/// stages part by more than any other row.
const POWER_SIZE: f64 = 1e6;

/// 06:55, counted from the midnight the run's clock starts at, so a clock law reads the time of day from the head.
pub fn anchor_moment() -> SimTime {
    SimTime::from_ns((6 * 3_600 + 55 * 60) * SECOND)
}

/// One beat in 100 intervals: the cycle's period and the heart-rate anchor state one number twice, and `admit` fails
/// when they disagree.
pub fn arena_packet(body: &CompiledBody) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let note = "arena fixture: synthetic";
    let cycle = Cycle::new(
        NonZeroU64::MIN.saturating_add(SECOND - 1),
        NonZeroU32::MIN.saturating_add(POINTS - 1),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    let unit = |value| Measure::<Dimensionless>::new(value);
    let scales = ErrorScales::uniform(unit(1e-9)?, unit(1e-10)?)?;
    packet.within(scales);

    let (arteries, glomerular) = (
        site::<VesselSegment>("systemic arteries")?,
        site::<CapillaryBed>("glomerular bed")?,
    );
    let rate = AnchorId::new("heart rate");
    packet.anchor(mean_anchor(
        rate,
        heart_rate(),
        Measure::<Rate<Dimensionless>>::new(1.0)?,
    ))?;
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
        Binding::Edge {
            from: arteries,
            to: glomerular,
        },
        Measure::<Resistance>::new(0.36)?,
        pressure,
    )?;
    let filtered = AnchorId::new("filtration");
    packet.anchor(mean_anchor(
        filtered,
        filtration_flow()?,
        Measure::<Rate<Water>>::new(2.0)?,
    ))?;
    // Salt balance fixes filtration at what the tubule can take back, so that capacity is what this anchor fits.
    packet.calibrated(
        ParameterKey::new("rn.sodium-capacity"),
        Binding::Edge {
            from: site::<VesselSegment>("renal tubule")?,
            to: site::<CapillaryBed>("peritubular bed")?,
        },
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
    packet.start_mode(rhythm, "systole", note)?;
    packet.start_mode(valve("left ventricle", "systemic arteries")?, "shut", note)?;
    packet.start_mode(valve("left atrium", "left ventricle")?, "open", note)?;
    packet.start_mode(valve("right ventricle", "pulmonary artery")?, "shut", note)?;
    packet.start_mode(valve("right atrium", "right ventricle")?, "open", note)?;
    packet.start_mode(valve("leg veins", "central veins")?, "open", note)?;

    table(&mut packet, note)?;
    packet.start_rest(body, start_guess, note)?;
    for row in body.rows() {
        let kind = row.kind();
        match kind {
            RowKind::Memory(slot) | RowKind::Algebraic(slot) => match slot.law.local {
                "valves" | "circulation" => {
                    packet.row_scale(kind, Measure::<Rate<Water>>::new(FLOW_SIZE)?)?
                }
                "wall power" => packet.row_scale(kind, Measure::<Rate<Energy>>::new(POWER_SIZE)?)?,
                _ => {}
            },
            RowKind::Amount { .. } => {}
        }
    }
    Ok(packet)
}

/// Water near the orbit at each site, so the march starts with a heart that has something to pump.
const WATER_GUESS: [(&str, Space, f64); 23] = [
    ("right atrium", Space::Lumen, 54.0),
    ("right ventricle", Space::Lumen, 100.0),
    ("pulmonary artery", Space::Lumen, 126.0),
    ("pulmonary vein", Space::Lumen, 300.0),
    ("left atrium", Space::Lumen, 74.0),
    ("left ventricle", Space::Lumen, 100.0),
    ("systemic arteries", Space::Lumen, 800.0),
    ("splanchnic veins", Space::Lumen, 800.0),
    ("leg veins", Space::Lumen, 500.0),
    ("central veins", Space::Lumen, 370.0),
    ("pulmonary bed", Space::Plasma, 180.0),
    ("splanchnic bed", Space::Plasma, 108.0),
    ("leg bed", Space::Plasma, 110.0),
    ("glomerular bed", Space::Plasma, 41.0),
    ("peritubular bed", Space::Plasma, 32.0),
    ("pulmonary bed", Space::Interstitium, 430.0),
    ("splanchnic bed", Space::Interstitium, 2_200.0),
    ("leg bed", Space::Interstitium, 2_700.0),
    ("glomerular bed", Space::Interstitium, 100.0),
    ("peritubular bed", Space::Interstitium, 160.0),
    ("renal tubule", Space::Lumen, 14.0),
    ("collecting duct", Space::Lumen, 5.0),
    ("bladder", Space::Lumen, 20.0),
];

/// A flat guess, about right in size for each kind of row. The march and the ring do the rest.
fn start_guess(row: RowKind) -> f64 {
    match row {
        RowKind::Amount { site, species, space } => {
            let guess = WATER_GUESS
                .iter()
                .find(|guess| guess.0 == site.key() && guess.1 == space);
            let water = guess.map_or(1_000.0, |guess| guess.2);
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

/// Every parameter of the fixture: rounded numbers from the prototype, nobody's physiology. A key's default binds
/// at its class or network, and a site or an edge that differs says so.
fn table(packet: &mut ParameterPacket<SyntheticFixture>, note: &'static str) -> Result<(), AdmissionError> {
    fn set<U: Dimension>(
        packet: &mut ParameterPacket<SyntheticFixture>,
        key: &'static str,
        binding: Binding,
        value: f64,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        packet.synthetic(ParameterKey::new(key), binding, Measure::<U>::new(value)?, note)
    }
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
    set::<Quotient<Pressure, Length>>(packet, "cv.head-per-length", body, 70.0, note)?;
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
        set::<Dimensionless>(packet, key, body, value, note)?;
    }
    for (key, value) in [
        ("cv.rise-speed", 20.0),
        ("cv.fall-speed", 20.0),
        ("ns.filter-speed", 4.0),
        ("ns.set-point-speed", 1e-6),
        ("en.avp-clearance", 2e-3),
        ("bd.bladder-speed", 1e-3),
        ("bd.heat-speed", 1e-3),
    ] {
        set::<Rate<Dimensionless>>(packet, key, body, value, note)?;
    }
    set::<Pressure>(packet, "cv.stretch-reference", body, 100.0, note)?;
    set::<Volume>(packet, "cv.collapse-width", body, 1.0, note)?;
    set::<Rate<Energy>>(packet, "cv.heat-width", body, 100.0, note)?;
    set::<Quotient<Volume, Albumin>>(packet, "cv.albumin-volume", body, 0.05, note)?;
    set::<Quotient<Volume, Albumin>>(packet, "fl.albumin-volume", body, 0.05, note)?;
    set::<Pressure>(packet, "gi.abdominal-pressure", body, 4.0, note)?;
    set::<Pressure>(packet, "rs.pleural-pressure", body, -3.0, note)?;
    set::<Deviation<Dimensionless>>(packet, "ns.sympathetic-width", body, 0.035, note)?;
    set::<Deviation<Dimensionless>>(packet, "ns.vagal-width", body, 0.05, note)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "fl.oncotic-slope", body, 20.0, note)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "rn.oncotic-slope", body, 20.0, note)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-sodium", body, 30.0, note)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-width", body, 5.0, note)?;
    set::<Concentration<Sodium>>(packet, "en.avp-sodium", body, 100.0, note)?;
    set::<Concentration<Sodium>>(packet, "en.avp-width", body, 2.0, note)?;
    set::<Rate<Vasopressin>>(packet, "en.avp-most", body, 2.0, note)?;
    set::<Rate<Water>>(packet, "gi.basal-water", body, 0.03, note)?;
    set::<Rate<Sodium>>(packet, "gi.basal-sodium", body, 1.0, note)?;
    set::<Rate<Potassium>>(packet, "gi.basal-potassium", body, 0.05, note)?;
    set::<Rate<Chloride>>(packet, "gi.basal-chloride", body, 1.05, note)?;
    set::<Rate<Water>>(packet, "ms.skin-water", body, 0.01, note)?;
    set::<Water>(packet, "bd.bladder-rest", body, 0.0, note)?;
    set::<Rate<Water>>(packet, "bd.bladder-width", body, 1e-3, note)?;

    set::<Dimensionless>(packet, "cv.inotropy", blood, 1.0, note)?;
    set::<Elastance>(packet, "cv.active-elastance", blood, 0.0, note)?;
    set::<Volume>(packet, "cv.unstressed-gain", blood, 0.0, note)?;
    fn wall(
        packet: &mut ParameterPacket<SyntheticFixture>,
        at: Binding,
        (elastance, active, unstressed, gain): (f64, f64, f64, f64),
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        set::<Elastance>(packet, "cv.elastance", at, elastance, note)?;
        if active > 0.0 {
            set::<Elastance>(packet, "cv.active-elastance", at, active, note)?;
        }
        set::<Volume>(packet, "cv.unstressed", at, unstressed, note)?;
        if gain > 0.0 {
            set::<Volume>(packet, "cv.unstressed-gain", at, gain, note)?;
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
        ("splanchnic veins", (0.1, 0.0, 1_064.0, 800.0)),
        ("leg veins", (0.4, 0.0, 566.0, 200.0)),
        ("central veins", (0.1, 0.0, 300.0, 0.0)),
    ] {
        wall(packet, Binding::Site(site::<VesselSegment>(key)?), values, note)?;
    }
    for (key, values, filtration, stiffness, rest) in [
        ("pulmonary bed", (0.25, 0.0, 120.0, 0.0), 0.002, 0.05, 420.0),
        ("splanchnic bed", (2.0, 0.0, 100.0, 0.0), 0.003, 0.04, 2_250.0),
        ("leg bed", (2.0, 0.0, 100.0, 0.0), 0.002, 0.04, 2_750.0),
        ("glomerular bed", (4.0, 0.0, 30.0, 0.0), 0.0005, 0.05, 40.0),
        ("peritubular bed", (4.0, 0.0, 30.0, 0.0), 0.002, 0.05, 200.0),
    ] {
        let at = Binding::Site(site::<CapillaryBed>(key)?);
        wall(packet, at, values, note)?;
        set::<Conductance<Water>>(packet, "fl.filtration", at, filtration, note)?;
        set::<Elastance>(packet, "fl.interstitial-elastance", at, stiffness, note)?;
        set::<Volume>(packet, "fl.interstitial-volume", at, rest, note)?;
    }

    set::<Resistance>(packet, "cv.resistance", blood, 0.01, note)?;
    set::<Dimensionless>(packet, "cv.tone-gain", blood, 0.0, note)?;
    set::<Dimensionless>(packet, "cv.feedback-share", blood, 0.0, note)?;
    set::<Inertance>(packet, "cv.inertance", blood, 5e-4, note)?;
    set::<Orifice>(packet, "cv.orifice", blood, 5e-5, note)?;
    set::<Resistance>(packet, "cv.valve-reading", blood, 0.01, note)?;
    set::<Pressure>(packet, "cv.opening-head", blood, 0.01, note)?;
    fn edge(from: SiteId, to: SiteId) -> Binding {
        Binding::Edge { from, to }
    }
    let (arteries, veins) = (
        site::<VesselSegment>("systemic arteries")?,
        site::<VesselSegment>("central veins")?,
    );
    let (lung, splanchnic, leg, glomerular, peritubular) = (
        site::<CapillaryBed>("pulmonary bed")?,
        site::<CapillaryBed>("splanchnic bed")?,
        site::<CapillaryBed>("leg bed")?,
        site::<CapillaryBed>("glomerular bed")?,
        site::<CapillaryBed>("peritubular bed")?,
    );
    let (artery, pulmonary_vein, splanchnic_veins, leg_veins) = (
        site::<VesselSegment>("pulmonary artery")?,
        site::<VesselSegment>("pulmonary vein")?,
        site::<VesselSegment>("splanchnic veins")?,
        site::<VesselSegment>("leg veins")?,
    );
    set::<Orifice>(packet, "cv.orifice", edge(leg_veins, veins), 1e-3, note)?;
    for (from, to, resistance) in [
        (artery, lung, 0.06),
        (lung, pulmonary_vein, 0.03),
        (splanchnic, splanchnic_veins, 0.5),
        (splanchnic_veins, veins, 0.1),
        (leg, leg_veins, 0.5),
        (glomerular, peritubular, 2.0),
        (peritubular, veins, 0.2),
    ] {
        set::<Resistance>(packet, "cv.resistance", edge(from, to), resistance, note)?;
    }
    set::<Dimensionless>(packet, "cv.tone-gain", edge(arteries, glomerular), 60.0, note)?;
    for (to, resistance) in [(splanchnic, 1.09), (leg, 1.4)] {
        let at = edge(arteries, to);
        set::<Resistance>(packet, "cv.resistance", at, resistance, note)?;
        set::<Dimensionless>(packet, "cv.tone-gain", at, 6.0, note)?;
    }
    set::<Dimensionless>(packet, "cv.feedback-share", edge(arteries, glomerular), 1.0, note)?;

    set::<Rate<Water>>(packet, "fl.ion-exchange", beds, 10.0, note)?;
    set::<Rate<Water>>(packet, "fl.albumin-leak", beds, 1e-3, note)?;
    set::<Dimensionless>(packet, "fl.albumin-reflection", beds, 0.9, note)?;
    set::<Dimensionless>(
        packet,
        "fl.albumin-reflection",
        Binding::Site(peritubular),
        0.5,
        note,
    )?;
    set::<Conductance<Water>>(packet, "fl.lymph-conductance", lymph, 0.04, note)?;
    set::<Conductance<Water>>(
        packet,
        "fl.lymph-conductance",
        edge(glomerular, veins),
        0.002,
        note,
    )?;
    set::<Conductance<Water>>(
        packet,
        "fl.lymph-conductance",
        edge(peritubular, veins),
        0.005,
        note,
    )?;
    set::<Pressure>(packet, "fl.lymph-threshold", lymph, -2.0, note)?;
    set::<Pressure>(packet, "fl.lymph-width", lymph, 0.5, note)?;

    let (tubule, duct, bladder) = (
        site::<VesselSegment>("renal tubule")?,
        site::<VesselSegment>("collecting duct")?,
        site::<VesselSegment>("bladder")?,
    );
    for (at, stiffness, rest) in [(tubule, 0.5, 10.0), (duct, 1.0, 5.0), (bladder, 1e-3, 0.0)] {
        set::<Elastance>(packet, "rn.lumen-elastance", Binding::Site(at), stiffness, note)?;
        set::<Volume>(packet, "rn.lumen-volume", Binding::Site(at), rest, note)?;
    }
    set::<Conductance<Water>>(packet, "rn.filtration", urinary, 0.12, note)?;
    set::<Resistance>(packet, "rn.tubule-resistance", urinary, 5.0, note)?;
    for (from, water_least, water_most, half) in [(tubule, 0.8, 0.8, 200.0), (duct, 0.5, 0.99, 20.0)] {
        let at = edge(from, peritubular);
        set::<Dimensionless>(packet, "rn.water-least", at, water_least, note)?;
        set::<Dimensionless>(packet, "rn.water-most", at, water_most, note)?;
        set::<Sodium>(packet, "rn.sodium-half", at, half, note)?;
    }
    set::<Rate<Sodium>>(packet, "rn.sodium-capacity", edge(duct, peritubular), 12.0, note)?;

    // Norepinephrine clears slowly and acetylcholine fast, so the reflex's arteriolar arm is the slow one and the loop
    // does not hunt. The gain the stand needs would ring on two lags of a second.
    for (release, reuptake, out, back) in [
        ("ns.release-ne", "ns.reuptake-ne", 0.005, 0.25),
        ("ns.release-ach", "ns.reuptake-ach", 0.04, 2.0),
    ] {
        set::<Rate<Dimensionless>>(packet, release, junctions, out, note)?;
        set::<Rate<Dimensionless>>(packet, reuptake, junctions, back, note)?;
    }
    set::<Norepinephrine>(packet, "rc.ne-sites", junctions, 1.0, note)?;
    set::<Affinity<Norepinephrine>>(packet, "rc.ne-on", junctions, 10.0, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.ne-off", junctions, 10.0, note)?;
    let renal = Binding::Site(site::<Junction>("renal junction")?);
    set::<Rate<Dimensionless>>(packet, "rc.ne-off", renal, 40.0, note)?;
    set::<Dimensionless>(packet, "rc.ne-efficacy", junctions, 1.0, note)?;
    set::<Acetylcholine>(packet, "rc.ach-sites", junctions, 1.0, note)?;
    set::<Affinity<Acetylcholine>>(packet, "rc.ach-on", junctions, 10.0, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.ach-off", junctions, 10.0, note)?;
    set::<Dimensionless>(packet, "rc.ach-efficacy", junctions, 2.0, note)?;
    set::<Vasopressin>(packet, "rc.avp-sites", blood, 1.0, note)?;
    set::<Affinity<Concentration<Vasopressin>>>(packet, "rc.avp-on", blood, 0.6, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.avp-off", blood, 0.1, note)?;
    set::<Dimensionless>(packet, "rc.avp-efficacy", blood, 2.0, note)
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

const METHOD: &str = "a synthetic monitor";

fn mean_anchor<U: Dimension>(id: AnchorId, row: RowKind, value: Measure<U>) -> Anchor<SyntheticFixture, U> {
    let read = Snapshot::new(row, METHOD, AnchorStatistic::CycleMean, value);
    Synthetic::new(id, read, "arena fixture: synthetic")
}

pub fn arena_lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
    let window = NonZeroU64::MIN.saturating_add(u64::from(POINTS) - 1);
    let channels = [heart_rate(), arterial_pressure()?, filtration_flow()?]
        .map(|row| ChannelSpec::new(METHOD, row).averaged(window));
    let grid = NonZeroU64::MIN.saturating_add(SECOND / u64::from(POINTS) - 1);
    body.lens(channels.into(), grid)
}
