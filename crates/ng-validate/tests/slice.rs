use ng_contract::catalog::{
    BaroreceptorAfferents, CapillaryAndVenousPressure, Defines, LungAfferents, Reads, RespiratoryDrive,
};
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const KF: f64 = 2.0;
const CAPILLARY: f64 = 11.0;
const VENOUS: f64 = 4.0;
const STRETCH: f64 = 0.25;
const START: f64 = 100.0;

/// A site by key and class: every anatomy resolves an id by those two, so no list of ids is kept.
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
    ];
    assert_eq!(named("kidney").map(sorted), Some(sorted(kidney)));
    Ok(())
}

/// Every pool of the slice's map, with nothing left out: a still circulation and Starling at every bed
/// move the vascular pools, and held cards stand behind cell water and the two enclosures' fluid.
fn bench() -> Result<CompiledBody, AdmissionError> {
    assemble(Schema::new(arena_anatomy()?)?, |mut body| {
        hold_outside_slice(&mut body)?;
        let fast = RateClass::new(RateBand::Wave, 1_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let afferents: BaroreceptorAfferents<Defines<'_, Cardiovascular>> = cv.emit();
        let pressures: CapillaryAndVenousPressure<Defines<'_, Cardiovascular>> = cv.emit();
        let stretch = cv.parameter::<Dimensionless>(ParameterKey::new("bench.stretch"), Admissible::finite());
        let capillary = cv.site_parameter::<CapillaryBed, Pressure>(
            ParameterKey::new("bench.capillary"),
            Admissible::finite(),
        );
        let venous = cv.site_parameter::<CapillaryBed, Pressure>(
            ParameterKey::new("bench.venous"),
            Admissible::finite(),
        );
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), fast)
        });
        cv.transfer_along(&route, still);
        for output in [&afferents.carotid_sinus_stretch, &afferents.aortic_arch_stretch] {
            let value = cv.kernel(|mut k| {
                let value = k.parameter(&stretch);
                k.finish(value, cv.every_mode(), fast)
            });
            cv.define(output, value);
        }
        for (output, parameter) in [
            (&pressures.capillary_pressure, &capillary),
            (&pressures.venous_pressure, &venous),
        ] {
            let value = cv.site_kernel(|mut k| {
                let value = k.site_parameter(parameter);
                k.finish(value, cv.every_mode(), fast)
            });
            cv.define_at(output, value);
        }
        body.add(cv.finish());

        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        let pressures: CapillaryAndVenousPressure<Reads<'_, Fluids>> = fl.receive();
        let kf = fl.site_parameter::<CapillaryBed, Conductance<Water>>(
            ParameterKey::new("bench.kf"),
            Admissible::nonnegative(),
        );
        let filtration = fl.site_kernel(|mut k| {
            let inside = k.at_site(&pressures.capillary_pressure);
            let outside = k.at_site(&ports.extravascular_pressure);
            let head = k.subtract(inside, outside);
            let conductance = k.site_parameter(&kf);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, fl.every_mode(), fast)
        });
        fl.transfer_at(&route, filtration);
        body.add(fl.finish());

        let (mut ns, _ports) = body.nervous()?.parts();
        let afferents: BaroreceptorAfferents<Reads<'_, Nervous>> = ns.receive();
        let filter = ns.state::<Dimensionless>("afferent-filter", Admissible::finite());
        let pace = ns.parameter(ParameterKey::new("bench.pace"), Admissible::nonnegative());
        let relax = ns.kernel(|mut k| {
            let target = k.read(&afferents.carotid_sinus_stretch);
            let current = k.state(&filter);
            let speed = k.parameter(&pace);
            let rate = k.relax(target, current, speed);
            k.finish(rate, ns.every_mode(), fast)
        });
        ns.memory_rate(&filter, relax);
        body.add(ns.finish());
        body.compile()
    })
}

fn packet(
    body: &CompiledBody,
    pleural: f64,
    abdominal: f64,
) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::new();
    let class = Binding::Class(Class::CapillaryBed);
    packet.synthetic(
        ParameterKey::new("bench.kf"),
        class,
        Measure::<Conductance<Water>>::new(KF)?,
        "synthetic",
    )?;
    for (key, value) in [("bench.capillary", CAPILLARY), ("bench.venous", VENOUS)] {
        packet.synthetic(
            ParameterKey::new(key),
            class,
            Measure::<Pressure>::new(value)?,
            "synthetic",
        )?;
    }
    for (key, value) in [
        ("rs.pleural-pressure", pleural),
        ("gi.abdominal-pressure", abdominal),
    ] {
        let value = Measure::<Pressure>::new(value)?;
        packet.synthetic(ParameterKey::new(key), Binding::Body, value, "synthetic")?;
    }
    let stretch = Measure::<Dimensionless>::new(STRETCH)?;
    packet.synthetic(
        ParameterKey::new("bench.stretch"),
        Binding::Body,
        stretch,
        "synthetic",
    )?;
    let pace = Measure::<Rate<Dimensionless>>::new(1.0)?;
    packet.synthetic(ParameterKey::new("bench.pace"), Binding::Body, pace, "synthetic")?;
    let pleura = site::<EnclosedSpace>("pleural space")?;
    for row in body.rows() {
        match row.kind() {
            RowKind::Amount { .. } => packet.start(row.kind(), Measure::<Water>::new(START)?, "synthetic")?,
            RowKind::Memory(_) => {
                packet.start(row.kind(), Measure::<Dimensionless>::new(0.0)?, "synthetic")?
            }
            RowKind::Algebraic(Slot { law, site }) => match law.local {
                "enclosure_pressure" if site == Some(pleura) => {
                    packet.start(row.kind(), Measure::<Pressure>::new(pleural)?, "synthetic")?
                }
                "enclosure_pressure" => {
                    packet.start(row.kind(), Measure::<Pressure>::new(abdominal)?, "synthetic")?
                }
                "capillary_pressure" => {
                    packet.start(row.kind(), Measure::<Pressure>::new(CAPILLARY)?, "synthetic")?
                }
                "venous_pressure" => {
                    packet.start(row.kind(), Measure::<Pressure>::new(VENOUS)?, "synthetic")?
                }
                _ => packet.start(row.kind(), Measure::<Dimensionless>::new(STRETCH)?, "synthetic")?,
            },
        }
    }
    Ok(packet)
}

#[test]
fn the_whole_map_compiles_with_nothing_left_out_and_every_card_held() -> Result<(), AdmissionError> {
    let body = bench()?;
    assert_eq!(body.held().count(), OUTSIDE_SLICE);
    let count = |kind: fn(&RowKind) -> bool| body.rows().iter().filter(|row| kind(&row.kind())).count();
    let beds = 5;
    let vessels = 10;
    let enclosures = 2;
    assert_eq!(
        count(|kind| matches!(kind, RowKind::Amount { .. })),
        3 * beds + vessels + enclosures
    );
    assert_eq!(
        count(|kind| matches!(kind, RowKind::Algebraic(_))),
        2 * beds + 2 + enclosures
    );
    let columns = |local| {
        (body.columns().iter())
            .filter(|column| column.origin().local == local)
            .count()
    };
    assert_eq!((columns("circulation"), columns("starling")), (17, beds));
    Ok(())
}

/// The water in one bed's interstitium after one second, and the row's start.
fn interstitium_after_a_second(
    body: &CompiledBody,
    pleural: f64,
    abdominal: f64,
    bed: SiteId,
) -> Result<f64, InitializationFailure> {
    let core = ReferenceIntegrator::new(SECOND);
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_fixture(body, packet(body, pleural, abdominal)?, port, &core)?;
    let Ok(session) = run_until(session, SimTime::from_ns(SECOND)) else {
        panic!("the bench faulted");
    };
    let kind = RowKind::Amount {
        site: bed,
        space: Space::Interstitium,
    };
    let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
    Ok(f64::from_bits(
        session.record().accepted().value(row.id()).bits()[0],
    ))
}

#[test]
fn pleural_and_abdominal_pressure_reach_the_enclosed_beds_as_held_enclosure_pressure()
-> Result<(), InitializationFailure> {
    let body = bench()?;
    let filtered = |bed, pleural, abdominal| -> Result<f64, InitializationFailure> {
        Ok(interstitium_after_a_second(&body, pleural, abdominal, bed)? - START)
    };
    let lung = site::<CapillaryBed>("pulmonary bed")?;
    let gut = site::<CapillaryBed>("splanchnic bed")?;
    assert_eq!(filtered(lung, 3.0, 5.0)?, KF * (CAPILLARY - 3.0));
    assert_eq!(filtered(lung, 7.0, 5.0)?, KF * (CAPILLARY - 7.0));
    assert_eq!(filtered(gut, 3.0, 5.0)?, KF * (CAPILLARY - 5.0));
    assert_eq!(filtered(gut, 3.0, 9.0)?, KF * (CAPILLARY - 9.0));
    Ok(())
}

#[test]
fn a_bed_outside_both_spaces_reads_zero_gauge_and_the_kidney_is_one_of_them()
-> Result<(), InitializationFailure> {
    let body = bench()?;
    for bed in [
        site::<CapillaryBed>("leg bed")?,
        site::<CapillaryBed>("glomerular bed")?,
        site::<CapillaryBed>("peritubular bed")?,
    ] {
        let reached = interstitium_after_a_second(&body, 3.0, 5.0, bed)?;
        assert_eq!(reached - START, KF * CAPILLARY);
    }
    Ok(())
}

#[test]
fn the_baroreflex_reads_the_stretch_cv_defines() -> Result<(), InitializationFailure> {
    let body = bench()?;
    let core = ReferenceIntegrator::new(SECOND);
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_fixture(&body, packet(&body, 3.0, 5.0)?, port, &core)?;
    let Ok(session) = run_until(session, SimTime::from_ns(SECOND)) else {
        panic!("the bench faulted");
    };
    let filter = RowKind::Memory(LawName {
        owner: "NS",
        local: "afferent-filter",
    });
    let row = body
        .rows()
        .iter()
        .find(|row| row.kind() == filter)
        .expect("a row");
    let reached = f64::from_bits(session.record().accepted().value(row.id()).bits()[0]);
    assert!(reached > 0.0 && reached < STRETCH, "{reached}");
    Ok(())
}

#[test]
fn a_held_pool_stays_where_it_is_and_stays_in_the_ledger() -> Result<(), InitializationFailure> {
    let body = bench()?;
    let core = ReferenceIntegrator::new(SECOND);
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_fixture(&body, packet(&body, 3.0, 5.0)?, port, &core)?;
    let Ok(session) = run_until(session, SimTime::from_ns(2 * SECOND)) else {
        panic!("the bench faulted");
    };
    let value = |kind| {
        let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
        f64::from_bits(session.record().accepted().value(row.id()).bits()[0])
    };
    let cells = RowKind::Amount {
        site: site::<CapillaryBed>("leg bed")?,
        space: Space::Cells,
    };
    let fluid = RowKind::Amount {
        site: site::<EnclosedSpace>("pleural space")?,
        space: Space::Fluid,
    };
    assert_eq!((value(cells), value(fluid)), (START, START));
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
        let fast = RateClass::new(RateBand::Wave, 1_000_000)?;
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
        let fast = RateClass::new(RateBand::Wave, 1_000_000)?;
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
    let climbing = RowKind::Memory(named("RS", "climbing"));
    let stretch = RowKind::Algebraic(named("RS", "pulmonary_stretch").into());
    let drive = RowKind::Algebraic(named("NS", "respiratory_drive").into());
    let mut packet = ParameterPacket::new();
    for key in ["bench.a", "bench.b"] {
        let value = Measure::<Dimensionless>::new(0.5)?;
        packet.synthetic(ParameterKey::new(key), Binding::Body, value, "synthetic")?;
    }
    let slope = Measure::<Rate<Dimensionless>>::new(3.0)?;
    packet.synthetic(ParameterKey::new("bench.slope"), Binding::Body, slope, "synthetic")?;
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
