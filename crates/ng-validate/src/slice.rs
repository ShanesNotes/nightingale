use core::num::{NonZeroU32, NonZeroU64};
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

    let tubule = map.site::<VesselSegment>("renal tubule", at(0.5)?);
    let duct = map.site::<VesselSegment>("collecting duct", at(0.5)?);
    let bladder = map.site::<VesselSegment>("bladder", at(0.3)?);

    let cranial = map.site::<NerveSegment>("cranial segment", at(1.5)?);

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
        (leg_veins.id(), central_veins.id()),
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
    map.enclose(&pleura, &[lung.id()], pleural);
    map.enclose(&abdomen, &[splanchnic.id()], abdominal);

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

/// The one site key the schema needs beyond the map's own handles. A site id is its key and class.
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

/// What the gut holds reaches the blood at a steady rate, and a meal adds to it. Stability this buys shows in the
/// held route's flow, which the certificate reports per window.
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

/// Secretion rises with plasma sodium, clearance is first order. Both held, because EN is outside the slice.
fn vasopressin(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Endocrine>::new(vec![SECRETION, CLEARANCE], Vec::new(), FIXED);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let secretion = ports.route::<Vasopressin>(SECRETION)?.route;
    let clearance = ports.route::<Vasopressin>(CLEARANCE)?.route;
    let base =
        laws.parameter::<Rate<Vasopressin>>(ParameterKey::new("en.avp-basal"), Admissible::nonnegative());
    let gain = laws.parameter::<Dimensionless>(ParameterKey::new("en.avp-gain"), Admissible::nonnegative());
    let set =
        laws.parameter::<Concentration<Sodium>>(ParameterKey::new("en.avp-sodium"), Admissible::positive());
    let speed =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("en.avp-clearance"), Admissible::positive());
    let rate = laws.kernel(|mut k| {
        let (sodium, water) = (
            k.read(&secretion.amount::<Sodium>()),
            k.read(&secretion.amount::<Water>()),
        );
        let volume = k.volume(water);
        let ratio = k.divide(sodium, volume);
        let seen = k.as_dimension(ratio);
        let center = k.parameter(&set);
        let ratio = k.divide(seen, center);
        let relative: Expr<'_, '_, _, Dimensionless> = k.as_dimension(ratio);
        let basal = k.parameter(&base);
        let tuned = k.parameter(&gain);
        let raised = k.scale(basal, tuned);
        let added = k.scale(raised, relative);
        let total = k.add(basal, added);
        k.finish(total, laws.every_mode(), class)
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
        exits(&mut body)?;
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
        body.compile()
    })
}

const SECOND: u64 = 1_000_000_000;

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
        NonZeroU32::MIN.saturating_add(99),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    let unit = |value| Measure::<Dimensionless>::new(value);
    let scales = ErrorScales::uniform(unit(1e-9)?, unit(1e-10)?)?
        .of::<Water>(Measure::new(1e-6)?)?
        .of::<Pressure>(Measure::new(1e-6)?)?
        .of::<Norepinephrine>(Measure::new(1e-12)?)?
        .of::<Acetylcholine>(Measure::new(1e-12)?)?
        .of::<Vasopressin>(Measure::new(1e-12)?)?;
    packet.within(scales);

    let rate = LawName {
        owner: Cardiovascular::CARD,
        local: "rate",
    };
    let rate_row = RowKind::Algebraic(rate.into());
    let heart_rate = AnchorId::new("heart rate");
    packet.anchor(Anchor::new(
        heart_rate,
        Snapshot::new(
            rate_row,
            "synthetic monitor",
            AnchorStatistic::CycleMean,
            Measure::<Rate<Dimensionless>>::new(1.0)?,
        ),
        note,
    ))?;
    let intrinsic = ParameterKey::new("cv.intrinsic-rate");
    packet.calibrated(
        intrinsic,
        Binding::Body,
        Measure::<Rate<Dimensionless>>::new(1.0)?,
        heart_rate,
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

    table(&mut packet, note)?;
    packet.start_rest(body, |row| start_guess(row), note)?;
    Ok(packet)
}

/// A flat guess, about right in size for each kind of row. The march and the ring do the rest.
fn start_guess(row: RowKind) -> f64 {
    match row {
        RowKind::Amount { species, space, .. } => match (species, space) {
            (Species::Water, Space::Lumen | Space::Plasma) => 300.0,
            (Species::Water, _) => 1_000.0,
            (Species::Sodium | Species::Chloride, _) => 40.0,
            (Species::Potassium, _) => 1.5,
            (Species::Albumin, _) => 3.0,
            (Species::Energy, _) => 1_000.0,
            _ => 1.0,
        },
        _ => 0.5,
    }
}

/// Every parameter of the fixture: rounded numbers from prototype P1, nobody's physiology. A key's default binds at its
/// class or network, and a site or an edge that differs says so.
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
    let (urinary, lymph, reclaim) = (
        Binding::Network(Network::Urinary),
        Binding::Network(Network::Lymph),
        Binding::Network(Network::Reclaim),
    );
    let (body, beds, junctions) = (
        Binding::Body,
        Binding::Class(Class::CapillaryBed),
        Binding::Class(Class::Junction),
    );
    set::<Quotient<Pressure, Length>>(packet, "cv.head-per-length", body, 40.0, note)?;
    for (key, value) in [
        ("cv.beta-gain", 1.0),
        ("cv.vagal-gain", 0.5),
        ("cv.systole-end", 0.3),
        ("cv.cycle-end", 1.0),
        ("cv.activation-peak", 1.0),
        ("cv.half", 0.5),
        ("ns.carotid-share", 0.5),
        ("ns.arch-share", 0.5),
        ("rn.feedback-rest", 1.0),
        ("rn.feedback-gain", 0.4),
        ("rn.feedback-middle", 0.5),
        ("en.avp-gain", 1.0),
    ] {
        set::<Dimensionless>(packet, key, body, value, note)?;
    }
    for (key, value) in [
        ("cv.rise-speed", 20.0),
        ("cv.fall-speed", 20.0),
        ("ns.filter-speed", 1.0),
        ("ns.set-point-speed", 1e-3),
        ("en.avp-clearance", 1e-2),
        ("bd.bladder-speed", 1e-3),
        ("bd.heat-speed", 1e-3),
    ] {
        set::<Rate<Dimensionless>>(packet, key, body, value, note)?;
    }
    set::<Pressure>(packet, "cv.stretch-reference", body, 100.0, note)?;
    set::<Pressure>(packet, "gi.abdominal-pressure", body, 1.0, note)?;
    set::<Pressure>(packet, "rs.pleural-pressure", body, -4.0, note)?;
    set::<Deviation<Dimensionless>>(packet, "ns.sympathetic-width", body, 0.1, note)?;
    set::<Deviation<Dimensionless>>(packet, "ns.vagal-width", body, 0.1, note)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "fl.oncotic-slope", body, 500.0, note)?;
    set::<Quotient<Pressure, Concentration<Albumin>>>(packet, "rn.oncotic-slope", body, 500.0, note)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-sodium", body, 0.1, note)?;
    set::<Concentration<Sodium>>(packet, "rn.feedback-width", body, 0.02, note)?;
    set::<Concentration<Sodium>>(packet, "en.avp-sodium", body, 0.14, note)?;
    set::<Rate<Vasopressin>>(packet, "en.avp-basal", body, 1e-2, note)?;
    set::<Rate<Water>>(packet, "gi.basal-water", body, 0.03, note)?;
    set::<Rate<Sodium>>(packet, "gi.basal-sodium", body, 0.002, note)?;
    set::<Rate<Potassium>>(packet, "gi.basal-potassium", body, 0.001, note)?;
    set::<Rate<Chloride>>(packet, "gi.basal-chloride", body, 0.003, note)?;
    set::<Rate<Water>>(packet, "ms.skin-water", body, 0.01, note)?;
    set::<Water>(packet, "bd.bladder-rest", body, 100.0, note)?;
    set::<Rate<Water>>(packet, "bd.bladder-width", body, 0.01, note)?;

    set::<Elastance>(packet, "cv.elastance", blood, 0.5, note)?;
    set::<Elastance>(packet, "cv.active-elastance", blood, 0.0, note)?;
    set::<Dimensionless>(packet, "cv.inotropy", blood, 0.5, note)?;
    set::<Volume>(packet, "cv.unstressed", blood, 100.0, note)?;
    set::<Volume>(packet, "cv.unstressed-gain", blood, 0.0, note)?;
    for (key, elastance, active, unstressed) in [
        ("right ventricle", 0.05, 0.75, 40.0),
        ("left ventricle", 0.1, 3.0, 20.0),
        ("systemic arteries", 1.0, 0.0, 700.0),
    ] {
        let at = Binding::Site(site::<VesselSegment>(key)?);
        set::<Elastance>(packet, "cv.elastance", at, elastance, note)?;
        set::<Elastance>(packet, "cv.active-elastance", at, active, note)?;
        set::<Volume>(packet, "cv.unstressed", at, unstressed, note)?;
    }
    set::<Resistance>(packet, "cv.resistance", blood, 0.1, note)?;
    set::<Dimensionless>(packet, "cv.tone-gain", blood, 0.5, note)?;
    set::<Dimensionless>(packet, "cv.feedback-share", blood, 0.0, note)?;
    set::<Inertance>(packet, "cv.inertance", blood, 5e-4, note)?;
    set::<Orifice>(packet, "cv.orifice", blood, 5e-5, note)?;

    set::<Elastance>(packet, "fl.interstitial-elastance", beds, 0.05, note)?;
    set::<Volume>(packet, "fl.interstitial-volume", beds, 900.0, note)?;
    set::<Conductance<Water>>(packet, "fl.filtration", beds, 0.1, note)?;
    set::<Dimensionless>(packet, "fl.albumin-reflection", beds, 0.9, note)?;
    set::<Conductance<Water>>(packet, "fl.lymph-conductance", lymph, 0.05, note)?;
    set::<Pressure>(packet, "fl.lymph-threshold", lymph, 0.0, note)?;
    set::<Pressure>(packet, "fl.lymph-width", lymph, 0.5, note)?;

    set::<Elastance>(packet, "rn.lumen-elastance", urinary, 0.5, note)?;
    set::<Volume>(packet, "rn.lumen-volume", urinary, 100.0, note)?;
    set::<Conductance<Water>>(packet, "rn.filtration", urinary, 0.1, note)?;
    set::<Resistance>(packet, "rn.tubule-resistance", urinary, 1.0, note)?;
    for (key, value) in [
        ("rn.water-least", 0.8),
        ("rn.water-most", 0.95),
        ("rn.sodium-share", 0.9),
        ("rn.potassium-share", 0.8),
    ] {
        set::<Dimensionless>(packet, key, reclaim, value, note)?;
    }

    for (release, reuptake) in [
        ("ns.release-ne", "ns.reuptake-ne"),
        ("ns.release-ach", "ns.reuptake-ach"),
    ] {
        set::<Rate<Dimensionless>>(packet, release, junctions, 0.1, note)?;
        set::<Rate<Dimensionless>>(packet, reuptake, junctions, 1.0, note)?;
    }
    set::<Norepinephrine>(packet, "rc.ne-sites", junctions, 1.0, note)?;
    set::<Affinity<Norepinephrine>>(packet, "rc.ne-on", junctions, 1.0, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.ne-off", junctions, 1.0, note)?;
    set::<Dimensionless>(packet, "rc.ne-efficacy", junctions, 3.0, note)?;
    set::<Acetylcholine>(packet, "rc.ach-sites", junctions, 1.0, note)?;
    set::<Affinity<Acetylcholine>>(packet, "rc.ach-on", junctions, 1.0, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.ach-off", junctions, 1.0, note)?;
    set::<Dimensionless>(packet, "rc.ach-efficacy", junctions, 3.0, note)?;
    set::<Vasopressin>(packet, "rc.avp-sites", blood, 1.0, note)?;
    set::<Affinity<Vasopressin>>(packet, "rc.avp-on", blood, 1.0, note)?;
    set::<Rate<Dimensionless>>(packet, "rc.avp-off", blood, 1.0, note)?;
    set::<Dimensionless>(packet, "rc.avp-efficacy", blood, 3.0, note)
}

/// One cycle-mean channel per anchor, on the grid of the cycle's points.
pub fn arena_lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
    let rate = LawName {
        owner: Cardiovascular::CARD,
        local: "rate",
    };
    let channel = ChannelSpec::new("synthetic monitor", RowKind::Algebraic(rate.into()))
        .averaged(NonZeroU64::MIN.saturating_add(99));
    body.lens(vec![channel], NonZeroU64::MIN.saturating_add(SECOND / 100 - 1))
}
