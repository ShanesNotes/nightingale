use ng_contract::*;

fn at_origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = at_origin().expect("finite");
    AnatomyBuilder::new().site::<C>(key, origin).id()
}

/// A tissue site that stands for the body, a vessel, and each bed on a loop out of the vessel and back.
fn anatomy(beds: &[&'static str]) -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at_origin()?);
    let vessel = map.site::<VesselSegment>("vessel", at_origin()?);
    map.contain(root.id(), vessel.id());
    for &key in beds {
        let bed = map.site::<CapillaryBed>(key, at_origin()?);
        map.contain(root.id(), bed.id());
        map.connect(Network::Vascular, vessel.id(), bed.id());
        map.connect(Network::Vascular, bed.id(), vessel.id());
    }
    Ok(map.finish()?)
}

fn wave() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, u64::MAX)
}

fn content(value: f64) -> Measure<Quotient<Energy, Glucose>> {
    Measure::new(value).expect("finite")
}

/// One vessel's water, filled from outside: the smallest body that compiles, so what the schema states shows in the
/// model hash.
fn water(stated: Option<f64>) -> Result<CompiledBody, AdmissionError> {
    let mut schema = Schema::new(anatomy(&[])?)?;
    schema.boundary::<Water>(
        "inflow",
        BoundaryDirection::IntoBody,
        site::<VesselSegment>("vessel"),
        Space::Lumen,
    )?;
    if let Some(value) = stated {
        schema.energy_content::<Glucose>(content(value))?;
    }
    let class = wave()?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, .. } = ports.route::<Water>("inflow")?;
        let supply =
            laws.parameter::<Rate<Water>>(ParameterKey::new("bench.inflow"), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&supply);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn a_fuel_states_its_energy_content_once() {
    let mut schema = Schema::new(anatomy(&["bed"]).expect("an anatomy")).expect("a schema");
    assert_eq!(schema.energy_content::<Glucose>(content(1_000.0)), Ok(()));
    assert_eq!(
        schema.energy_content::<Glucose>(content(1_000.0)),
        Ok(()),
        "the same content again"
    );
    assert_eq!(
        schema.energy_content::<Glucose>(content(1_001.0)),
        Err(AdmissionError::EnergyContent {
            species: Species::Glucose
        })
    );
}

#[test]
fn an_energy_content_is_a_whole_number_of_energy_units_up_to_two_to_the_fifty_third() {
    let state = |value: f64| {
        let mut schema = Schema::new(anatomy(&["bed"]).expect("an anatomy")).expect("a schema");
        schema.energy_content::<Glucose>(content(value))
    };
    let refused = Err(AdmissionError::EnergyContent {
        species: Species::Glucose,
    });
    for value in [0.0, -1_000.0, 2.5, 9_007_199_254_740_994.0] {
        assert_eq!(state(value), refused, "{value}");
    }
    assert_eq!(state(1.0), Ok(()));
    assert_eq!(state(9_007_199_254_740_992.0), Ok(()));
}

#[test]
fn only_a_fuel_has_an_energy_content() {
    let mut schema = Schema::new(anatomy(&["bed"]).expect("an anatomy")).expect("a schema");
    let refused = |species| Err(AdmissionError::EnergyContent { species });
    let thousand = || Measure::new(1_000.0).expect("finite");
    assert_eq!(
        schema.energy_content::<Water>(thousand()),
        refused(Species::Water)
    );
    assert_eq!(
        schema.energy_content::<Oxygen>(thousand()),
        refused(Species::Oxygen)
    );
    assert_eq!(
        schema.energy_content::<CarbonDioxide>(thousand()),
        refused(Species::CarbonDioxide)
    );
    assert_eq!(
        schema.energy_content::<Sodium>(thousand()),
        refused(Species::Sodium)
    );
    assert_eq!(
        schema.energy_content::<Energy>(thousand()),
        refused(Species::Energy)
    );
}

#[test]
fn a_stated_energy_content_moves_the_model_hash_and_none_leaves_it() {
    let hash = |stated| water(stated).expect("compiled").model_hash();
    assert_eq!(hash(None), hash(None));
    assert_ne!(hash(None), hash(Some(1_000.0)));
    assert_ne!(hash(Some(1_000.0)), hash(Some(1_001.0)));
}

#[test]
fn a_bed_holds_its_own_heat() {
    let mut schema = Schema::new(anatomy(&["bed"]).expect("an anatomy")).expect("a schema");
    assert_eq!(
        schema.pool::<Energy>(site::<CapillaryBed>("bed"), Space::Thermal),
        Ok(())
    );
}

/// A bed keyed before the body that holds heat of its own, so the first thermal pool in key order is the bed's. CV
/// dissipates the body's elastic store and the bed's heat leaves through a boundary.
fn heat_ahead_of_the_body() -> Result<CompiledBody, AdmissionError> {
    let (bed, root) = (site::<CapillaryBed>("a-bed"), site::<TissueSite>("body"));
    let mut schema = Schema::new(anatomy(&["a-bed"])?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of energy alone");
    }
    schema.pool::<Energy>(root, Space::Elastic)?;
    schema.pool::<Energy>(root, Space::Thermal)?;
    schema.pool::<Energy>(bed, Space::Thermal)?;
    schema.boundary::<Energy>("bed heat", BoundaryDirection::OutOfBody, bed, Space::Thermal)?;
    let class = wave()?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let pair = ports.dissipation(&mut laws);
        let speed =
            laws.parameter::<Rate<Energy>>(ParameterKey::new("cv.dissipation"), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&pair, rate));
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, .. } = ports.route::<Energy>("bed heat")?;
        let loss =
            laws.parameter::<Rate<Energy>>(ParameterKey::new("bench.bed-heat"), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&loss);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn dissipation_reaches_the_body_store_whatever_sorts_before_it() {
    let compiled = heat_ahead_of_the_body().expect("compiled");
    let root = site::<TissueSite>("body");
    let dissipation = (compiled.columns().iter())
        .find(|column| column.origin().local == "dissipation")
        .expect("a dissipation column");
    let ends: Vec<(SiteId, Space, i64)> = (dissipation.terms().iter())
        .map(|&(row, coefficient)| match compiled.rows()[row.index()].kind() {
            RowKind::Amount { site, space, .. } => (site, space, coefficient),
            other => panic!("an amount row, not {other:?}"),
        })
        .collect();
    assert_eq!(ends, [(root, Space::Elastic, -1), (root, Space::Thermal, 1)]);
}
