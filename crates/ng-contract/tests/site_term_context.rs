use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn body_memory(through_term: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    for site in [artery.id(), vein.id()] {
        map.contain(root.id(), site);
    }
    map.connect(Network::Vascular, artery.id(), vein.id());
    map.connect(Network::Vascular, vein.id(), artery.id());
    map.vein(&vein);
    let schema = Schema::new(map.finish()?)?;
    assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let memory = cv.state::<Dimensionless>("memory", Admissible::finite());
        let pace = cv.site_parameter::<Veins, Rate<Dimensionless>>(
            ParameterKey::new("cv.pace"),
            Admissible::positive(),
        );
        let term = through_term.then(|| {
            cv.site_term::<Veins, Rate<Dimensionless>, _>("site pace", |mut k| {
                let value = k.site_parameter(&pace);
                k.finish_term(value)
            })
        });
        let rate = cv.site_kernel::<Veins, Rate<Dimensionless>, _>(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            let value = match &term {
                Some(term) => k.add_from(zero, term),
                None => {
                    let value = k.site_parameter(&pace);
                    k.add(zero, value)
                }
            };
            k.finish(value, cv.every_mode(), class)
        });
        cv.memory_rate(&memory, rate);
        body.add(cv.finish());
        body.compile()
    })
}

/// A kernel that adds a term stands at sites: on a body memory's rate it is refused by its own defect, and the same
/// site read written inline is a read with no site.
#[test]
fn a_site_term_cannot_hide_a_site_read_on_a_body_memory() {
    let memory = LawName {
        owner: "CV",
        local: "memory",
    };
    let inline = Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law: memory }));
    let through = Some(AdmissionError::Compile(CompileDefect::TermOffSite {
        law: memory,
    }));
    assert_eq!(body_memory(false).err(), inline);
    assert_eq!(body_memory(true).err(), through);
}
