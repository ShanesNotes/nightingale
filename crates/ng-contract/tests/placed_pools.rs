use ng_contract::*;

fn body(class_wide: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let position = || {
        Position::new(
            Measure::new(0.0).unwrap(),
            Measure::new(0.0).unwrap(),
            Measure::new(0.0).unwrap(),
        )
    };
    let root = map.site::<TissueSite>("synthetic body", position());
    let a = map.site::<VesselSegment>("synthetic a", position());
    let b = map.site::<VesselSegment>("synthetic b", position());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    if class_wide {
        schema.carry::<Sodium, VesselSegment>(Space::Lumen);
    } else {
        schema.pool::<Sodium>(a.id(), Space::Lumen)?;
        schema.pool::<Sodium>(b.id(), Space::Lumen)?;
    }
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

#[test]
fn a_species_carried_by_class_travels_on_the_route() {
    assert_eq!(body(true).unwrap().columns().len(), 2);
}

#[test]
fn a_species_placed_pool_by_pool_travels_on_the_route_too() {
    assert_eq!(
        body(false)
            .expect("same pools and route as the class-wide control")
            .columns()
            .len(),
        2
    );
}
