use ng_contract::*;

fn position() -> Position {
    Position::new(
        Measure::new(0.0).unwrap(),
        Measure::new(0.0).unwrap(),
        Measure::new(0.0).unwrap(),
    )
}

fn body(
    place: impl FnOnce(&mut Schema, [SiteId; 2]) -> Result<(), AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("synthetic body", position());
    let a = map.site::<VesselSegment>("synthetic a", position());
    let b = map.site::<VesselSegment>("synthetic b", position());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    place(&mut schema, [a.id(), b.id()])?;
    assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let speed =
            cv.edge_parameter::<_, Rate<Water>>(ParameterKey::new("synthetic flow"), Admissible::positive());
        let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
        let rate = cv.edge_kernel(|mut k| {
            let value = k.edge_parameter(&speed);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer_along(&route, rate);
        body.add(cv.finish());
        body.compile()
    })
}

fn class_wide(schema: &mut Schema, _: [SiteId; 2]) -> Result<(), AdmissionError> {
    schema.carry::<Sodium, VesselSegment>(Space::Lumen);
    Ok(())
}

#[test]
fn a_species_carried_by_class_travels_on_the_route() {
    assert_eq!(body(class_wide).unwrap().columns().len(), 2);
}

#[test]
fn a_species_placed_pool_by_pool_travels_on_the_route_too() {
    let pool_by_pool = |schema: &mut Schema, sites: [SiteId; 2]| -> Result<(), AdmissionError> {
        (sites.into_iter()).try_for_each(|site| schema.pool::<Sodium>(site, Space::Lumen))
    };
    assert_eq!(
        body(pool_by_pool)
            .expect("same pools and route as the class-wide control")
            .columns()
            .len(),
        2
    );
}

#[test]
fn a_species_in_a_space_it_may_not_hold_is_refused() {
    let misplaced = |schema: &mut Schema, sites: [SiteId; 2]| -> Result<(), AdmissionError> {
        class_wide(schema, sites)?;
        schema.pool::<Energy>(sites[0], Space::Lumen)
    };
    let a = AnatomyBuilder::new().site::<VesselSegment>("synthetic a", position());
    assert_eq!(
        body(misplaced).err(),
        Some(AdmissionError::Compile(CompileDefect::OutOfPlace {
            site: a.id(),
            space: Space::Lumen,
            species: Species::Energy,
        }))
    );
}
