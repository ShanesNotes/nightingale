use ng_contract::catalog::{BaroreceptorAfferents, CapillaryAndVenousPressure, Defines, Reads};
use ng_contract::*;

// The signature walk (docs/architecture/solver-contract-sketch/src/signature_walk.rs): step 0 with only
// the crate path changed, then steps 1, 1b, 1c, 1d and 2, each change marked with the ticket that restores it.
fn anatomy() -> Result<Schema, AdmissionError> {
    let at = |axial: f64| -> Result<Position, AdmissionError> {
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
        map.connect(Network::Vascular, from, to);
    }
    map.connect(Network::Vascular, heart.id(), lung.id());
    map.connect(Network::Vascular, lung.id(), heart.id());
    let pleural = Compliance::Held {
        card: Respiratory::CARD,
    };
    map.enclose(&pleura, &[lung.id()], pleural);
    map.served_by(lung.id(), &thoracic, Afferent::Nociceptive);
    let _kidney: RegionId = map.region("kidney", kidney.id());
    Schema::new(map.finish()?)
}

fn baroreflex<'b>(scope: NervousScope<'b>) -> Result<Fragment<'b, Nervous>, AdmissionError> {
    let (mut laws, _transmitters) = scope.parts();
    let afferents: BaroreceptorAfferents<Reads<'b, Nervous>> = laws.receive();
    let filter = laws.state::<Dimensionless>("afferent-filter", Admissible::unit_interval());
    let set_point = laws.set_point::<Dimensionless>("stretch-set-point", Admissible::unit_interval());
    let chronic = laws.parameter(
        ParameterKey::new("ns.chronic-set-point"),
        Admissible::unit_interval(),
    );
    let inverse_tau = laws.parameter(ParameterKey::new("ns.afferent-rate"), Admissible::nonnegative());
    let resetting = laws.parameter(ParameterKey::new("ns.resetting-rate"), Admissible::nonnegative());
    let (reflex, slow) = (
        RateClass::new(RateBand::Reflex, 1_000_000_000)?,
        RateClass::new(RateBand::Hours, 3_600_000_000_000)?,
    );
    let relax = laws.kernel(|mut k| {
        let stretch = k.read(&afferents.carotid_sinus_stretch);
        let reference = k.state(&set_point);
        let error = k.deviation(stretch, reference);
        let target = k.logistic(error);
        let current = k.state(&filter);
        let speed = k.parameter(&inverse_tau);
        let rate = k.relax(target, current, speed);
        k.finish(rate, laws.every_mode(), reflex)
    });
    let drift = laws.kernel(|mut k| {
        let target = k.parameter(&chronic);
        let current = k.state(&set_point);
        let speed = k.parameter(&resetting);
        let rate = k.relax(target, current, speed);
        k.finish(rate, laws.every_mode(), slow)
    });
    let trusted = ValidityRange::new(
        Measure::new(-1.0)?,
        Measure::new(1.0)?,
        Basis::Fixture("fixture design range"),
    )?;
    laws.validity(&relax, trusted);
    laws.memory_rate(&filter, relax);
    laws.memory_rate(&set_point, drift);
    Ok(laws.finish())
}

fn valve<'b>(scope: CardiovascularScope<'b>) -> Result<Fragment<'b, Cardiovascular>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let wave = || RateClass::new(RateBand::Wave, 1_000_000);
    // 10 restores CV's chamber pressure port; until then the chamber is a local algebraic slot that a
    // fixture parameter closes, like the outflow.
    let chamber = laws.algebraic::<Pressure>("chamber", Admissible::finite());
    let outflow = laws.algebraic::<Pressure>("valve-outflow", Admissible::finite());
    let upstream = laws.parameter::<Pressure>(ParameterKey::new("fixture.upstream"), Admissible::finite());
    let downstream =
        laws.parameter::<Pressure>(ParameterKey::new("fixture.downstream"), Admissible::finite());
    let g = laws.parameter::<Conductance<Water>>(ParameterKey::new("fixture.g"), Admissible::nonnegative());
    let [shut, open] = laws.modes("valve", ["shut", "open"]);
    let head = |modes, rate_class| {
        laws.kernel(|mut k| {
            let upstream = k.algebraic(&chamber);
            let downstream = k.algebraic(&outflow);
            let head = k.subtract(upstream, downstream);
            k.finish(head, modes, rate_class)
        })
    };
    let opening = Guard::new(head(shut.only(), wave()?), Crossing::Rising, shut.to(&open)?);
    let closing = Guard::new(head(open.only(), wave()?), Crossing::Falling, open.to(&shut)?);
    laws.event(opening);
    laws.event(closing);
    ectopy(&mut laws)?;
    for (slot, parameter) in [(&chamber, &upstream), (&outflow, &downstream)] {
        let class = wave()?;
        let residual = laws.kernel(|mut k| {
            let actual = k.algebraic(slot);
            let target = k.parameter(parameter);
            let value = k.subtract(actual, target);
            k.finish(value, laws.every_mode(), class)
        });
        laws.residual(slot, residual);
    }
    // 10 computes these from the circulation; until then a fixture parameter closes each one.
    let afferents: BaroreceptorAfferents<Defines<'b, Cardiovascular>> = laws.emit();
    let pressures: CapillaryAndVenousPressure<Defines<'b, Cardiovascular>> = laws.emit();
    let stretch = laws.parameter::<Dimensionless>(ParameterKey::new("fixture.stretch"), Admissible::finite());
    let pressure = laws.site_parameter::<CapillaryBed, Pressure>(
        ParameterKey::new("fixture.bed-pressure"),
        Admissible::finite(),
    );
    for output in [&afferents.carotid_sinus_stretch, &afferents.aortic_arch_stretch] {
        let class = wave()?;
        let value = laws.kernel(|mut k| {
            let value = k.parameter(&stretch);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define(output, value);
    }
    for output in [&pressures.capillary_pressure, &pressures.venous_pressure] {
        let class = wave()?;
        let value = laws.site_kernel(|mut k| {
            let value = k.site_parameter(&pressure);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define_at(output, value);
    }
    let route = ports.circulation(&mut laws);
    let class = wave()?;
    let flow = laws.edge_kernel(|mut k| {
        let p = k.algebraic(&chamber);
        let q = k.algebraic(&outflow);
        let head = k.subtract(p, q);
        let conductance = k.parameter(&g);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, open.only(), class)
    });
    laws.transfer_along(&route, flow);
    let class = wave()?;
    let zero = laws.edge_kernel(|mut k| {
        let zero = k.zero::<Rate<Water>>();
        k.finish(zero, shut.only(), class)
    });
    laws.transfer_along(&route, zero);
    Ok(laws.finish())
}

fn ectopy<'b>(laws: &mut FragmentBuilder<'b, Cardiovascular>) -> Result<(), AdmissionError> {
    let dice = laws.stream("ectopic-beat");
    let wobble = laws.forcing(
        "hazard-variability",
        ParameterKey::new("cv.hazard-period"),
        ParameterKey::new("cv.hazard-amplitude"),
    );
    let hazard = laws.state::<Dimensionless>("ectopy-hazard", Admissible::nonnegative());
    let ceiling = laws.parameter(ParameterKey::new("cv.ectopy-ceiling"), Admissible::nonnegative());
    let pace = laws.parameter(ParameterKey::new("cv.ectopy-rate"), Admissible::nonnegative());
    let [sinus, ectopic] = laws.modes("rhythm", ["sinus", "ectopic"]);
    let (beat, every) = (RateClass::new(RateBand::Beat, 100_000_000)?, laws.every_mode());
    let accrue = laws.kernel(|mut k| {
        let limit = k.parameter(&ceiling);
        let varied = k.forcing(&wobble);
        let target = k.scale(limit, varied);
        let current = k.state(&hazard);
        let speed = k.parameter(&pace);
        let rate = k.relax(target, current, speed);
        k.finish(rate, every, beat)
    });
    laws.hazard(HazardGuard::new(&hazard, &dice, sinus.to(&ectopic)?));
    laws.memory_rate(&hazard, accrue);
    Ok(())
}

fn starling<'b>(scope: FluidsScope<'b>) -> Result<Fragment<'b, Fluids>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let route = ports.starling(&mut laws);
    let pressures: CapillaryAndVenousPressure<Reads<'b, Fluids>> = laws.receive();
    let kf = laws.site_parameter::<CapillaryBed, Conductance<Water>>(
        ParameterKey::new("fl.kf"),
        Admissible::nonnegative(),
    );
    let minutes = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let filtration = laws.site_kernel(|mut k| {
        let inside = k.at_site(&pressures.capillary_pressure);
        let outside = k.at_site(&ports.extravascular_pressure);
        let head = k.subtract(inside, outside);
        let conductance = k.site_parameter(&kf);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), minutes)
    });
    laws.transfer_at(&route, filtration);
    Ok(laws.finish())
}

// RS holds the pleural space's pressure; its fluid comes with it.
fn pleural_pressure<'b>(body: &mut Assembly<'b>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Respiratory>::new(vec![], vec![], "arena fixture: a fixed pressure");
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let held = laws.parameter::<Pressure>(ParameterKey::new("fixture.pleural"), Admissible::finite());
    let class = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let pressure = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&ports.enclosure_pressure, pressure);
    body.add(laws.finish());
    Ok(())
}

fn held<'b, S: OutsideSlice>(body: &mut Assembly<'b>, holds: Vec<Space>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<S>::new(vec![], holds, "arena fixture");
    let (laws, _ports) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}

// 10 adds the renal, receptor and boundary fragments.
fn compile() -> Result<CompiledBody, AdmissionError> {
    assemble(anatomy()?, |mut body| {
        let heart = valve(body.cardiovascular()?)?;
        body.add(heart);
        let fluids = starling(body.fluids()?)?;
        body.add(fluids);
        let nerves = baroreflex(body.nervous()?)?;
        body.add(nerves);
        pleural_pressure(&mut body)?;
        held::<BloodChemistry>(&mut body, vec![Space::Cells])?;
        held::<Metabolism>(&mut body, vec![])?;
        held::<Musculoskeletal>(&mut body, vec![])?;
        held::<Endocrine>(&mut body, vec![])?;
        held::<Hepatic>(&mut body, vec![])?;
        held::<Gastrointestinal>(&mut body, vec![])?;
        held::<Immune>(&mut body, vec![])?;
        held::<Hematology>(&mut body, vec![])?;
        body.compile()
    })
}

#[test]
fn the_walks_anatomy_passes_every_check() {
    assert!(anatomy().is_ok());
}

#[test]
fn the_adapted_walk_compiles() -> Result<(), AdmissionError> {
    let compiled = compile()?;
    let count = |kind: fn(&RowKind) -> bool| compiled.rows().iter().filter(|row| kind(&row.kind())).count();
    assert_eq!(count(|kind| matches!(kind, RowKind::Amount { .. })), 11);
    assert_eq!(count(|kind| matches!(kind, RowKind::Memory(_))), 3);
    assert_eq!(count(|kind| matches!(kind, RowKind::Algebraic(_))), 11);
    let columns = |local| {
        (compiled.columns().iter())
            .filter(|column| column.origin().local == local)
            .count()
    };
    assert_eq!((columns("circulation"), columns("starling")), (5, 3));
    assert_eq!(compiled.programs().len(), 13);
    assert_eq!(compiled.held().count(), OUTSIDE_SLICE);
    Ok(())
}
