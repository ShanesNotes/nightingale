use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::catalog::{
    BaroreceptorAfferents, CapillaryAndVenousPressure, Defines, LungAfferents, Reads, RespiratoryDrive,
};
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const KF: f64 = 2.0;
const CAPILLARY: f64 = 11.0;
const START: f64 = 100.0;

/// An id is its key and class alone, so a throwaway builder names a site of any anatomy.
fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    Ok(AnatomyBuilder::new().site::<C>(key, origin).id())
}

fn region(key: &'static str) -> Result<RegionId, AdmissionError> {
    Ok(AnatomyBuilder::new().region(key, site::<TissueSite>("body")?))
}

fn sorted(mut sites: Vec<SiteId>) -> Vec<SiteId> {
    sites.sort_unstable();
    sites
}

#[test]
fn the_slice_map_passes_every_graph_check_and_holds_the_named_sites() -> Result<(), AdmissionError> {
    let anatomy = arena_anatomy()?;
    let named = |key| {
        anatomy
            .members(region(key).expect("a region"))
            .map(<[SiteId]>::to_vec)
    };
    let mut every = Vec::new();
    for key in ["body", "heart", "kidney", "carotid sinus", "aortic arch"] {
        every.push(site::<TissueSite>(key)?);
    }
    for key in [
        "right atrium",
        "right ventricle",
        "left atrium",
        "left ventricle",
        "pulmonary artery",
        "pulmonary vein",
        "systemic arteries",
        "central veins",
        "splanchnic veins",
        "leg veins",
        "renal tubule",
        "collecting duct",
        "bladder",
    ] {
        every.push(site::<VesselSegment>(key)?);
    }
    for key in [
        "pulmonary bed",
        "splanchnic bed",
        "leg bed",
        "glomerular bed",
        "peritubular bed",
    ] {
        every.push(site::<CapillaryBed>(key)?);
    }
    for key in ["pleural space", "abdominal space"] {
        every.push(site::<EnclosedSpace>(key)?);
    }
    for key in [
        "sinoatrial junction",
        "myocardial junction",
        "arteriolar junction",
        "venous junction",
        "renal junction",
    ] {
        every.push(site::<Junction>(key)?);
    }
    every.push(site::<NerveSegment>("cranial segment")?);
    assert_eq!(named("body").map(sorted), Some(sorted(every)));
    let kidney = vec![
        site::<TissueSite>("kidney")?,
        site::<CapillaryBed>("glomerular bed")?,
        site::<CapillaryBed>("peritubular bed")?,
        site::<Junction>("renal junction")?,
        site::<VesselSegment>("renal tubule")?,
        site::<VesselSegment>("collecting duct")?,
    ];
    assert_eq!(named("kidney").map(sorted), Some(sorted(kidney)));
    Ok(())
}

fn enclosure_law<'b, S: OutsideSlice>(
    laws: &mut FragmentBuilder<'b, S>,
    ports: &HeldPorts<'b, S>,
    key: ParameterKey,
) -> Result<(), AdmissionError> {
    let held = laws.parameter::<Pressure>(key, Admissible::finite());
    let slow = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let pressure = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), slow)
    });
    laws.define_at(&ports.enclosure_pressure, pressure);
    Ok(())
}

#[test]
fn on_the_slices_map_a_route_nothing_claims_is_refused_at_compile() -> Result<(), AdmissionError> {
    let unclaimed = "unclaimed water";
    let mut schema = arena_schema()?;
    let veins = site::<VesselSegment>("splanchnic veins")?;
    schema.boundary::<Water>(unclaimed, BoundaryDirection::IntoBody, veins, Space::Lumen)?;
    let compiled = assemble(schema, |mut body| {
        hold_outside_slice(&mut body)?;
        arena_organs(&mut body)?;
        body.compile()
    });
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::OmittedLaw {
            law: LawName {
                owner: "Boundary",
                local: unclaimed
            }
        }))
    );
    Ok(())
}

fn gauge(body: &CompiledBody, row: RowKind) -> Result<ObservationLens, AdmissionError> {
    let every = NonZeroU64::new(SECOND).expect("not zero");
    body.lens(vec![ChannelSpec::new("a synthetic gauge", row)], every)
}

fn at_admission(
    session: &ng_record::Session<'_, ng_record::Running>,
    body: &CompiledBody,
    kind: RowKind,
) -> f64 {
    let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
    f64::from_bits(session.record().accepted().value(row.id()).bits()[0])
}

/// A bed that fills until its walls balance a field and its enclosure, so a steady orbit exists, with one
/// held pool and one held enclosure's fluid beside it.
fn settling_bed() -> Result<CompiledBody, AdmissionError> {
    let origin = || -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?);
    let vessel = map.site::<VesselSegment>("vessel", origin()?);
    let bed = map.site::<CapillaryBed>("bed", origin()?);
    let belly = map.site::<EnclosedSpace>("belly", origin()?);
    for at in [vessel.id(), bed.id(), belly.id()] {
        map.contain(root.id(), at);
    }
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    let held = Compliance::Held {
        card: Gastrointestinal::CARD,
    };
    map.enclose(&belly, &[bed.id()], held);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Lumen, "no circulation in this bench");
    schema.partial_volume(Measure::new(1.0)?)?;
    assemble(schema, |mut body| {
        let fast = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let gut = HeldDeclaration::<Gastrointestinal>::new(vec![], vec![], "a synthetic bench");
        let (mut laws, ports) = body.hold(gut)?.parts();
        enclosure_law(&mut laws, &ports, ParameterKey::new("gi.abdominal-pressure"))?;
        body.add(laws.finish());
        let cells = HeldDeclaration::<BloodChemistry>::new(vec![], vec![Space::Cells], "a synthetic bench");
        let (laws, _ports) = body.hold(cells)?.parts();
        body.add(laws.finish());
        let (mut cv, _ports) = body.cardiovascular()?.parts();
        let pressures: CapillaryAndVenousPressure<Defines<'_, Cardiovascular>> = cv.emit();
        let value = cv.site_parameter::<CapillaryBed, Pressure>(
            ParameterKey::new("bench.capillary"),
            Admissible::finite(),
        );
        for output in [&pressures.capillary_pressure, &pressures.venous_pressure] {
            let defined = cv.site_kernel(|mut k| {
                let value = k.site_parameter(&value);
                k.finish(value, cv.every_mode(), fast)
            });
            cv.define_at(output, defined);
        }
        body.add(cv.finish());
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        let pressures: CapillaryAndVenousPressure<Reads<'_, Fluids>> = fl.receive();
        let e = fl
            .site_parameter::<CapillaryBed, Elastance>(ParameterKey::new("bench.e"), Admissible::positive());
        let kf = fl.site_parameter::<CapillaryBed, Conductance<Water>>(
            ParameterKey::new("bench.kf"),
            Admissible::positive(),
        );
        let flow = fl.site_kernel(|mut k| {
            let plasma = k.at_site(&route.from_amount());
            let interstitium = k.at_site(&route.to_amount());
            let (plasma, interstitium) = (k.volume(plasma), k.volume(interstitium));
            let stiffness = k.site_parameter(&e);
            let inside = k.multiply(stiffness, plasma);
            let stiffness = k.site_parameter(&e);
            let outside = k.multiply(stiffness, interstitium);
            let inside = k.as_dimension::<_, Pressure>(inside);
            let outside = k.as_dimension::<_, Pressure>(outside);
            let walls = k.subtract(inside, outside);
            let capillary = k.at_site(&pressures.capillary_pressure);
            let around = k.at_site(&ports.extravascular_pressure);
            let drive = k.subtract(capillary, around);
            let head = k.add(walls, drive);
            let conductance = k.site_parameter(&kf);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, fl.every_mode(), fast)
        });
        fl.transfer_at(&route, flow);
        body.add(fl.finish());
        body.compile()
    })
}

#[test]
fn a_cycle_solves_field_rows_at_every_point_and_leaves_held_pools_where_they_are()
-> Result<(), InitializationFailure> {
    let body = settling_bed()?;
    let bed = site::<CapillaryBed>("bed")?;
    let belly = site::<EnclosedSpace>("belly")?;
    let cycle = Cycle::new(
        NonZeroU64::new(SECOND).expect("not zero"),
        NonZeroU32::new(4).expect("not zero"),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    packet.within(admission_tolerance());
    let class = Binding::Class(Class::CapillaryBed);
    let one = Measure::<Elastance>::new(1.0)?;
    packet.synthetic(ParameterKey::new("bench.e"), class, one, "synthetic")?;
    let kf = Measure::<Conductance<Water>>::new(KF)?;
    packet.synthetic(ParameterKey::new("bench.kf"), class, kf, "synthetic")?;
    let capillary = Measure::<Pressure>::new(CAPILLARY)?;
    packet.synthetic(
        ParameterKey::new("bench.capillary"),
        class,
        capillary,
        "synthetic",
    )?;
    let abdominal = Measure::<Pressure>::new(5.0)?;
    packet.synthetic(
        ParameterKey::new("gi.abdominal-pressure"),
        Binding::Body,
        abdominal,
        "synthetic",
    )?;
    for row in body.rows() {
        match row.kind() {
            RowKind::Algebraic(_) => packet.start(row.kind(), Measure::<Pressure>::new(1.0)?, "synthetic")?,
            kind => packet.start(kind, Measure::<Water>::new(START)?, "synthetic")?,
        }
    }
    let amount = |space, site| RowKind::Amount {
        site,
        space,
        species: Species::Water,
    };
    let lens = gauge(&body, amount(Space::Plasma, bed))?;
    let core = ReferenceIntegrator::new(SECOND / 4);
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_solved(&body, packet, port, &core, &lens)?.session;
    let value = |kind| at_admission(&session, &body, kind);
    let drained = (CAPILLARY - 5.0) / 2.0;
    let close = |found: f64, wanted: f64| (found - wanted).abs() < 1e-9;
    assert!(close(value(amount(Space::Plasma, bed)), START - drained));
    assert!(close(value(amount(Space::Interstitium, bed)), START + drained));
    assert_eq!(value(amount(Space::Cells, bed)), START);
    assert_eq!(value(amount(Space::Fluid, belly)), START);
    let law = LawName {
        owner: "CV",
        local: "capillary_pressure",
    };
    assert!(close(
        value(RowKind::Algebraic(Slot {
            law,
            at: Context::Site(bed)
        })),
        CAPILLARY
    ));
    Ok(())
}

#[test]
fn an_anchor_on_a_held_pool_is_a_parameters_value() -> Result<(), AdmissionError> {
    let body = arena_body()?;
    let leg = site::<CapillaryBed>("leg bed")?;
    let id = AnchorId::new("bench.pool");
    let bound = |space| -> Result<Option<AdmissionError>, AdmissionError> {
        let row = RowKind::Amount {
            site: leg,
            space,
            species: Species::Water,
        };
        let reading = Measure::<Water>::new(START)?;
        let read = Snapshot::new(row, "a synthetic gauge", AnchorStatistic::Instant, reading);
        let mut anchored = arena_packet(&body)?;
        anchored.anchor(Synthetic::new(id, read, "synthetic"))?;
        Ok(body.bind(anchored).err())
    };
    assert_eq!(
        bound(Space::Cells)?,
        Some(AdmissionError::AnchorOnParameters { anchor: id })
    );
    assert_eq!(
        bound(Space::Interstitium)?,
        Some(AdmissionError::InstantOnPulsatile { anchor: id })
    );
    Ok(())
}

#[test]
fn the_whole_map_compiles_with_nothing_left_out_and_every_card_held() -> Result<(), AdmissionError> {
    let body = arena_body()?;
    assert_eq!(body.held().count(), OUTSIDE_SLICE);
    let columns = |local| {
        (body.columns().iter())
            .filter(|column| column.origin().local == local)
            .count()
    };
    let (beds, plain_edges, valves, lymph_edges) = (5, 12, 5, 5);
    let (blood_species, tissue_species) = (6, 5);
    assert_eq!(columns("circulation"), plain_edges * blood_species);
    assert_eq!(columns("valves"), valves * blood_species);
    assert_eq!(columns("starling"), beds * tissue_species);
    assert_eq!(columns("lymph"), lymph_edges * tissue_species);
    Ok(())
}

fn stretch_law<'b>(
    laws: &FragmentBuilder<'b, Cardiovascular>,
    stretch: &Parameter<'b, Cardiovascular, Dimensionless>,
    class: RateClass,
) -> PureKernel<'b, Cardiovascular, Dimensionless> {
    laws.kernel(|mut k| {
        let value = k.parameter(stretch);
        k.finish(value, laws.every_mode(), class)
    })
}

#[test]
fn a_validity_range_on_a_definition_judges_the_row() -> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "body",
        Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?),
    );
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut laws, _ports) = body.cardiovascular()?.parts();
        let out: BaroreceptorAfferents<Defines<'_, Cardiovascular>> = laws.emit();
        let stretch = laws.parameter::<Dimensionless>(ParameterKey::new("cv.stretch"), Admissible::finite());
        let fast = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let judged = stretch_law(&laws, &stretch, fast);
        let range = ValidityRange::new(
            Measure::new(0.0)?,
            Measure::new(1.0)?,
            Basis::Fixture("synthetic"),
        )?;
        laws.validity(&judged, range);
        laws.define(&out.carotid_sinus_stretch, judged);
        let other = stretch_law(&laws, &stretch, fast);
        laws.define(&out.aortic_arch_stretch, other);
        body.add(laws.finish());
        body.compile()
    })?;
    let mut packet = ParameterPacket::new();
    let outside = Measure::<Dimensionless>::new(4.0)?;
    packet.synthetic(
        ParameterKey::new("cv.stretch"),
        Binding::Body,
        outside,
        "synthetic",
    )?;
    for row in body.rows() {
        packet.start(row.kind(), Measure::<Dimensionless>::new(4.0)?, "synthetic")?;
    }
    let core = ReferenceIntegrator::new(SECOND);
    let session = admit_fixture(&body, packet, BoundaryPort::new(Vec::new())?, &core)?;
    let ng_record::Step::Accepted(_, receipt) = session.step() else {
        panic!("the step was not accepted");
    };
    let [excursion] = receipt.excursions() else {
        panic!("one excursion: {:?}", receipt.excursions());
    };
    assert_eq!(f64::from_bits(excursion.value().bits()[0]), 4.0);
    Ok(())
}

/// `x = a y + m` and `y = b x`, one field each way, with `m` climbing, so `x = m / (1 - a b)` at every
/// step only if the two rows are solved together.
fn coupled_loop() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    map.site::<TissueSite>("body", origin);
    assemble(Schema::new(map.finish()?)?, |mut body| {
        let fast = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut laws, _ports) = body.nervous()?.parts();
        let out: RespiratoryDrive<Defines<'_, Nervous>> = laws.emit();
        let back: LungAfferents<Reads<'_, Nervous>> = laws.receive();
        let b = laws.parameter::<Dimensionless>(ParameterKey::new("bench.b"), Admissible::finite());
        let drive = laws.kernel(|mut k| {
            let stretch = k.read(&back.pulmonary_stretch);
            let gain = k.parameter(&b);
            let value = k.scale(stretch, gain);
            k.finish(value, laws.every_mode(), fast)
        });
        laws.define(&out.respiratory_drive, drive);
        body.add(laws.finish());

        let declaration = HeldDeclaration::<Respiratory>::new(vec![], vec![], "a synthetic bench");
        let (mut laws, _ports) = body.hold(declaration)?.parts();
        let out: LungAfferents<Defines<'_, Respiratory>> = laws.emit();
        let back: RespiratoryDrive<Reads<'_, Respiratory>> = laws.receive();
        let a = laws.parameter::<Dimensionless>(ParameterKey::new("bench.a"), Admissible::finite());
        let slope =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bench.slope"), Admissible::finite());
        let climbing = laws.state::<Dimensionless>("climbing", Admissible::finite());
        let climb = laws.kernel(|mut k| {
            let rate = k.parameter(&slope);
            k.finish(rate, laws.every_mode(), fast)
        });
        laws.memory_rate(&climbing, climb);
        let stretch = laws.kernel(|mut k| {
            let drive = k.read(&back.respiratory_drive);
            let gain = k.parameter(&a);
            let scaled = k.scale(drive, gain);
            let base = k.state(&climbing);
            let value = k.add(scaled, base);
            k.finish(value, laws.every_mode(), fast)
        });
        laws.define(&out.pulmonary_stretch, stretch);
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn a_loop_of_two_couplings_is_solved_as_one_coupled_step() -> Result<(), InitializationFailure> {
    let body = coupled_loop()?;
    let named = |owner, local| LawName { owner, local };
    let climbing = RowKind::Memory(named("RS", "climbing").into());
    let stretch = RowKind::Algebraic(named("RS", "pulmonary_stretch").into());
    let drive = RowKind::Algebraic(named("NS", "respiratory_drive").into());
    let mut packet = ParameterPacket::new();
    for key in ["bench.a", "bench.b"] {
        let value = Measure::<Dimensionless>::new(0.5)?;
        packet.synthetic(ParameterKey::new(key), Binding::Body, value, "synthetic")?;
    }
    let slope = Measure::<Rate<Dimensionless>>::new(3.0)?;
    packet.synthetic(
        ParameterKey::new("bench.slope"),
        Binding::Body,
        slope,
        "synthetic",
    )?;
    for (row, start) in [(climbing, 3.0), (stretch, 4.0), (drive, 2.0)] {
        packet.start(row, Measure::<Dimensionless>::new(start)?, "synthetic")?;
    }
    let core = ReferenceIntegrator::new(SECOND);
    let session = admit_fixture(&body, packet, BoundaryPort::new(Vec::new())?, &core)?;
    let Ok(session) = run_until(session, SimTime::from_ns(SECOND)) else {
        panic!("the bench faulted");
    };
    let value = |kind| {
        let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
        f64::from_bits(session.record().accepted().value(row.id()).bits()[0])
    };
    assert!((value(climbing) - 6.0).abs() < 1e-9);
    assert!((value(stretch) - 8.0).abs() < 1e-9);
    assert!((value(drive) - 4.0).abs() < 1e-9);
    Ok(())
}
