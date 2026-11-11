use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn slow() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = at().expect("finite");
    AnatomyBuilder::new().site::<C>(key, origin).id()
}

fn law(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

fn circuit(liver: bool) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    let liver_bed = map.site::<CapillaryBed>("liver bed", at()?);
    let muscle_bed = map.site::<CapillaryBed>("muscle bed", at()?);
    for child in [artery.id(), vein.id(), liver_bed.id(), muscle_bed.id()] {
        map.contain(root.id(), child);
    }
    for (from, to) in [
        (artery.id(), liver_bed.id()),
        (liver_bed.id(), vein.id()),
        (artery.id(), muscle_bed.id()),
        (muscle_bed.id(), vein.id()),
        (vein.id(), artery.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    if liver {
        map.serves(&liver_bed, Tissue::Liver);
        map.serves(&muscle_bed, Tissue::Muscle);
    }
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.leave_out(space, "a bench of blood alone");
    }
    schema.carry::<Sodium, Blood>(Space::Plasma);
    schema.carry::<Sodium, Blood>(Space::Lumen);
    Ok(schema)
}

#[derive(Clone, Copy, PartialEq)]
enum Liver {
    Unstated,
    Built,
    Held,
}

fn circulate(liver: Liver) -> Result<CompiledBody, AdmissionError> {
    let mut schema = circuit(liver != Liver::Unstated)?;
    schema.pool::<Energy>(site::<TissueSite>("body"), Space::Elastic)?;
    assemble(schema, |mut body| {
        let class = slow()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let (pressure, _) = ports.blood_pressure(&mut cv);
        let held =
            cv.site_parameter::<Blood, Pressure>(ParameterKey::new("cv.pressure"), Admissible::finite());
        let value = cv.site_kernel(|mut k| {
            let value = k.site_parameter(&held);
            k.finish(value, cv.every_mode(), class)
        });
        cv.define_at(&pressure, value);
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let fallen = ports.gravity_work(&mut cv);
        let paid = cv.edge_kernel(|mut k| {
            let _flow = k.edge_flow();
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&fallen, paid);
        body.add(cv.finish());
        match liver {
            Liver::Unstated => {}
            Liver::Built => {
                let (mut hp, ports) = body.hepatic(None)?.parts();
                let route = ports.sinusoids(&mut hp);
                let pressure = ports.blood_pressure();
                let still = hp.edge_kernel(|mut k| {
                    let (_up, _down) = (k.upstream(&pressure), k.downstream(&pressure));
                    let zero = k.zero::<Rate<Water>>();
                    k.finish(zero, hp.every_mode(), class)
                });
                hp.transfer_along(&route, still);
                body.add(hp.finish());
            }
            Liver::Held => {
                let validity = "the liver bed's flow: a stand-in until the hepatic card's law";
                let mut held = body.hold(HeldDeclaration::<Hepatic>::new(vec![], vec![], validity))?;
                let (stand_ins, _) = held.stand_ins();
                let still = stand_ins.edge_kernel(|mut k| {
                    let zero = k.zero::<Rate<Water>>();
                    k.finish(zero, stand_ins.every_mode(), class)
                });
                let (mut hp, ports) = body.hepatic(Some(held))?.parts();
                let route = ports.sinusoids(&mut hp);
                hp.transfer_along(&route, still);
                body.add(hp.finish());
            }
        }
        body.compile()
    })
}

fn edge(from: SiteId, to: SiteId) -> Context {
    Context::Edge { from, to }
}

fn sorted(mut contexts: Vec<Context>) -> Vec<Context> {
    contexts.sort_unstable();
    contexts.dedup();
    contexts
}

fn columns_of(body: &CompiledBody, origin: LawName) -> Vec<Context> {
    sorted(
        (body.columns().iter())
            .filter(|column| column.origin() == origin)
            .map(|column| column.context())
            .collect(),
    )
}

#[test]
fn every_plain_vascular_edge_is_reached_by_circulation_or_the_sinusoids_and_never_both()
-> Result<(), AdmissionError> {
    let artery = site::<VesselSegment>("artery");
    let vein = site::<VesselSegment>("vein");
    let liver_bed = site::<CapillaryBed>("liver bed");
    let muscle_bed = site::<CapillaryBed>("muscle bed");
    let liver_edges = vec![edge(artery, liver_bed), edge(liver_bed, vein)];
    let other_edges = vec![
        edge(artery, muscle_bed),
        edge(muscle_bed, vein),
        edge(vein, artery),
    ];
    let body = circulate(Liver::Built)?;
    assert_eq!(
        columns_of(&body, law("CV", "circulation")),
        sorted(other_edges.clone())
    );
    assert_eq!(
        columns_of(&body, law("HP", "sinusoids")),
        sorted(liver_edges.clone())
    );
    let body = circulate(Liver::Unstated)?;
    assert_eq!(
        columns_of(&body, law("CV", "circulation")),
        sorted([liver_edges, other_edges].concat())
    );
    Ok(())
}

#[test]
fn the_flow_at_a_liver_bed_edge_is_the_sinusoids_whoever_reads_it() -> Result<(), AdmissionError> {
    let (artery, vein) = (site::<VesselSegment>("artery"), site::<VesselSegment>("vein"));
    let liver_bed = site::<CapillaryBed>("liver bed");
    let body = circulate(Liver::Built)?;
    let rows = (body.rows().iter()).filter_map(|row| match row.kind() {
        RowKind::Algebraic(Slot { law: found, at }) if found == law("HP", "sinusoids") => Some(at),
        _ => None,
    });
    assert_eq!(
        sorted(rows.collect()),
        sorted(vec![edge(artery, liver_bed), edge(liver_bed, vein)])
    );
    Ok(())
}

#[test]
fn the_sinusoids_leave_a_valved_edge_of_the_liver_bed_to_the_valves() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    let liver_bed = map.site::<CapillaryBed>("liver bed", at()?);
    for child in [artery.id(), vein.id(), liver_bed.id()] {
        map.contain(root.id(), child);
    }
    map.connect(Network::Vascular, artery.id(), liver_bed.id());
    map.valve(liver_bed.id(), vein.id());
    map.connect(Network::Vascular, vein.id(), artery.id());
    map.serves(&liver_bed, Tissue::Liver);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.leave_out(space, "a bench of blood alone");
    }
    schema.carry::<Sodium, Blood>(Space::Plasma);
    schema.carry::<Sodium, Blood>(Space::Lumen);
    let body = assemble(schema, |mut body| {
        let class = slow()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let flow = ports.valves(&mut cv).flow_memory();
        let held = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
            let zero = k.zero::<Rate<Rate<Water>>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.memory_rate(&flow, held);
        body.add(cv.finish());
        let (mut hp, ports) = body.hepatic(None)?.parts();
        let route = ports.sinusoids(&mut hp);
        let still = hp.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, hp.every_mode(), class)
        });
        hp.transfer_along(&route, still);
        body.add(hp.finish());
        body.compile()
    })?;
    let (artery, liver_bed) = (site::<VesselSegment>("artery"), site::<CapillaryBed>("liver bed"));
    assert_eq!(
        columns_of(&body, law("HP", "sinusoids")),
        [edge(artery, liver_bed)]
    );
    Ok(())
}

#[test]
fn a_held_hepatic_cards_stand_in_holds_the_liver_beds_edges() -> Result<(), AdmissionError> {
    let (artery, vein) = (site::<VesselSegment>("artery"), site::<VesselSegment>("vein"));
    let liver_bed = site::<CapillaryBed>("liver bed");
    let body = circulate(Liver::Held)?;
    assert_eq!(body.held().collect::<Vec<_>>(), ["HP"]);
    assert_eq!(
        columns_of(&body, law("HP", "sinusoids")),
        sorted(vec![edge(artery, liver_bed), edge(liver_bed, vein)])
    );
    Ok(())
}

fn drained(sinusoids: bool, lymph: Option<Tissue>) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    let liver_bed = map.site::<CapillaryBed>("liver bed", at()?);
    let muscle_bed = map.site::<CapillaryBed>("muscle bed", at()?);
    for child in [artery.id(), vein.id(), liver_bed.id(), muscle_bed.id()] {
        map.contain(root.id(), child);
    }
    for (from, to) in [
        (artery.id(), liver_bed.id()),
        (liver_bed.id(), vein.id()),
        (artery.id(), muscle_bed.id()),
        (muscle_bed.id(), vein.id()),
        (vein.id(), artery.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    match lymph {
        Some(Tissue::Liver) => map.connect(Network::Lymph, liver_bed.id(), vein.id()),
        Some(_) => map.connect(Network::Lymph, muscle_bed.id(), vein.id()),
        None => {}
    }
    map.serves(&liver_bed, Tissue::Liver);
    map.serves(&muscle_bed, Tissue::Muscle);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Cells, "a bench of blood and the beds' interstitium");
    schema.carry::<Sodium, Blood>(Space::Plasma);
    schema.carry::<Sodium, Blood>(Space::Lumen);
    schema.carry::<Sodium, CapillaryBed>(Space::Interstitium);
    assemble(schema, |mut body| {
        let class = slow()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());
        if sinusoids {
            let (mut hp, ports) = body.hepatic(None)?.parts();
            let route = ports.sinusoids(&mut hp);
            let still = hp.edge_kernel(|mut k| {
                let zero = k.zero::<Rate<Water>>();
                k.finish(zero, hp.every_mode(), class)
            });
            hp.transfer_along(&route, still);
            body.add(hp.finish());
        }
        let (mut fl, ports) = body.fluids()?.parts();
        let wall = ports.starling(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&wall, still);
        if lymph.is_some() {
            let route = ports.lymph(&mut fl);
            let read = fl.edge_kernel(|mut k| {
                let _flow = k.edge_flow();
                let zero = k.zero::<Rate<Water>>();
                k.finish(zero, fl.every_mode(), class)
            });
            fl.transfer_along(&route, read);
        }
        body.add(fl.finish());
        body.compile()
    })
}

#[test]
fn a_liver_bed_beside_circulation_owes_the_hepatic_cards_sinusoids() {
    assert_eq!(
        drained(false, None).map(|body| body.model_hash()),
        Err(AdmissionError::Compile(CompileDefect::OmittedLaw {
            law: law("HP", "sinusoids")
        }))
    );
}

#[test]
fn a_lymph_law_reads_no_blood_flow_at_the_liver_bed_as_at_any_other() {
    for bed in [Tissue::Liver, Tissue::Muscle] {
        assert_eq!(
            drained(true, Some(bed)).map(|body| body.model_hash()),
            Err(AdmissionError::Compile(CompileDefect::NoSuchRead {
                law: law("FL", "lymph")
            })),
            "lymph from the {bed:?} bed"
        );
    }
}

fn switch<'b, O: Owner>(
    laws: &mut FragmentBuilder<'b, O>,
    route: &EdgeRoute<'b, O, Water>,
) -> Result<(), AdmissionError> {
    let class = slow()?;
    let [on, off] = laws.modes_along(route, "switch", ["on", "off"]);
    let flow = laws.edge_kernel::<VascularEdge, _, _>(|mut k| {
        let flow = k.edge_flow();
        k.finish(flow, on.only(), class)
    });
    laws.event(Guard::new(flow, Crossing::Falling, on.to(&off)?));
    Ok(())
}

fn switched(hepatic: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(circuit(true)?, |mut body| {
        let class = slow()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        if !hepatic {
            switch(&mut cv, &route)?;
        }
        body.add(cv.finish());
        let (mut hp, ports) = body.hepatic(None)?.parts();
        let route = ports.sinusoids(&mut hp);
        let still = hp.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, hp.every_mode(), class)
        });
        hp.transfer_along(&route, still);
        if hepatic {
            switch(&mut hp, &route)?;
        }
        body.add(hp.finish());
        body.compile()
    })
}

#[test]
fn a_guard_on_a_switch_along_circulation_reads_the_flow_at_its_own_edge() -> Result<(), AdmissionError> {
    switched(false)?;
    Ok(())
}

#[test]
fn the_hepatic_card_reads_the_flow_along_its_own_sinusoids() -> Result<(), AdmissionError> {
    switched(true)?;
    Ok(())
}
