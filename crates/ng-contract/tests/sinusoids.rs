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
    schema.pool::<Energy>(root.id(), Space::Elastic)?;
    Ok(schema)
}

fn circulate(liver: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(circuit(liver)?, |mut body| {
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
        if liver {
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
    let body = circulate(true)?;
    assert_eq!(
        columns_of(&body, law("CV", "circulation")),
        sorted(other_edges.clone())
    );
    assert_eq!(
        columns_of(&body, law("HP", "sinusoids")),
        sorted(liver_edges.clone())
    );
    let body = circulate(false)?;
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
    let body = circulate(true)?;
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
