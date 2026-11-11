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
    map.finish()
}

fn wave() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, u64::MAX)
}

fn content(value: f64) -> Measure<Quotient<Energy, Glucose>> {
    Measure::new(value).expect("finite")
}

fn thousand<U>() -> Measure<U> {
    Measure::new(1_000.0).expect("finite")
}

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

/// A bed's store and another tissue site's sort before the body's, and the body is the containment root.
fn heat_ahead_of_the_body() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at_origin()?).id();
    let skin = map.site::<TissueSite>("a skin heat site", at_origin()?).id();
    let vessel = map.site::<VesselSegment>("vessel", at_origin()?).id();
    let bed = map.site::<CapillaryBed>("a-bed", at_origin()?).id();
    for site in [skin, vessel, bed] {
        map.contain(root, site);
    }
    map.connect(Network::Vascular, vessel, bed);
    map.connect(Network::Vascular, bed, vessel);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of energy alone");
    }
    schema.pool::<Energy>(root, Space::Elastic)?;
    schema.pool::<Energy>(root, Space::Thermal)?;
    schema.pool::<Energy>(bed, Space::Thermal)?;
    schema.pool::<Energy>(skin, Space::Thermal)?;
    schema.boundary::<Energy>("bed heat", BoundaryDirection::OutOfBody, bed, Space::Thermal)?;
    schema.boundary::<Energy>("skin heat", BoundaryDirection::OutOfBody, skin, Space::Thermal)?;
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
        for (name, key) in [("bed heat", "bench.bed-heat"), ("skin heat", "bench.skin-heat")] {
            let BoundaryRoute { route, .. } = ports.route::<Energy>(name)?;
            let loss = laws.parameter::<Rate<Energy>>(ParameterKey::new(key), Admissible::nonnegative());
            let rate = laws.kernel(|mut k| {
                let value = k.parameter(&loss);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, rate));
        }
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn the_core_store_is_the_roots_whatever_tissue_store_sorts_first() {
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

const BURN: LawName = LawName {
    owner: <Metabolism as Owner>::CARD,
    local: "burn",
};

/// Its counts are definitional chemistry.
fn burn() -> Proportions<Glucose> {
    Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 6>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium)
        .energy_into(Space::Thermal)
}

fn interstitial(beds: &[&'static str]) -> Result<Schema, AdmissionError> {
    let mut schema = Schema::new(anatomy(beds)?)?;
    for space in [Space::Plasma, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "the burn draws on the interstitium alone");
    }
    schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
    schema.carry::<CarbonDioxide, CapillaryBed>(Space::Interstitium);
    Ok(schema)
}

fn burning_schema(beds: &[&'static str]) -> Result<Schema, AdmissionError> {
    let mut schema = interstitial(beds)?;
    schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.energy_content::<Glucose>(content(1_000.0))?;
    Ok(schema)
}

fn burning<Q: Reactant>(
    schema: Schema,
    holds: Vec<Space>,
    proportions: Proportions<Q>,
    rates: impl for<'b> FnOnce(
        &mut FragmentBuilder<'b, Metabolism>,
        &Reaction<'b, Metabolism, CapillaryBed, Q>,
    ) -> Result<(), AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    assemble(schema, |mut body| {
        let held = HeldDeclaration::<Metabolism>::new(Vec::new(), holds, "bench: synthetic");
        let (mut laws, _) = body.hold(held)?.parts();
        let burn = laws.reaction::<CapillaryBed, Q>("burn", proportions);
        rates(&mut laws, &burn)?;
        body.add(laws.finish());
        body.compile()
    })
}

fn paced_in<'b, Q: Reactant>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    burn: &Reaction<'b, Metabolism, CapillaryBed, Q>,
    modes: ModeSet<'b, Metabolism>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let speed = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
        ParameterKey::new("mt.burn-speed"),
        Admissible::nonnegative(),
    );
    let reference = burn.amount::<Q>();
    let rate = laws.site_kernel(|mut k| {
        let amount = k.at_site(&reference);
        let zero = k.zero();
        let speed = k.site_parameter(&speed);
        let rate = k.relax(amount, zero, speed);
        k.finish(rate, modes, class)
    });
    laws.react_at(burn, rate);
    Ok(())
}

fn paced<'b, Q: Reactant>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    burn: &Reaction<'b, Metabolism, CapillaryBed, Q>,
) -> Result<(), AdmissionError> {
    let every = laws.every_mode();
    paced_in(laws, burn, every)
}

fn refused(compiled: Result<CompiledBody, AdmissionError>) -> CompileDefect {
    match compiled {
        Err(AdmissionError::Compile(defect)) => defect,
        Err(other) => panic!("refused outside the compile: {other:?}"),
        Ok(_) => panic!("compiled"),
    }
}

fn schema() -> Schema {
    burning_schema(&["bed"]).expect("a schema")
}

#[test]
fn a_reaction_that_names_a_species_twice_is_refused() {
    let twice = burn().makes::<Oxygen, 1>(Space::Interstitium);
    assert_eq!(
        refused(burning(schema(), Vec::new(), twice, paced)),
        CompileDefect::SpeciesTwice {
            law: BURN,
            species: Species::Oxygen
        }
    );
}

#[test]
fn a_burn_whose_fuel_has_no_stated_content_is_refused() {
    let mut unstated = interstitial(&["bed"]).expect("a schema");
    unstated.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    unstated.carry::<Energy, CapillaryBed>(Space::Thermal);
    assert_eq!(
        refused(burning(unstated, Vec::new(), burn(), paced)),
        CompileDefect::UnstatedEnergy {
            law: BURN,
            species: Species::Glucose
        }
    );
}

#[test]
fn a_burn_that_makes_carbon_is_refused_naming_carbon() {
    let seven = Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 7>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium)
        .energy_into(Space::Thermal);
    assert_eq!(
        refused(burning(schema(), Vec::new(), seven, paced)),
        CompileDefect::UnbalancedReaction {
            law: BURN,
            book: Book::Element(Element::Carbon)
        }
    );
}

#[test]
fn a_burn_whose_energy_goes_nowhere_is_refused() {
    let cold = Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 6>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium);
    assert_eq!(
        refused(burning(schema(), Vec::new(), cold, paced)),
        CompileDefect::UnbalancedReaction {
            law: BURN,
            book: Book::Energy
        }
    );
}

#[test]
fn a_count_times_an_energy_content_past_two_to_the_fifty_third_is_refused() {
    let mut rich = interstitial(&["bed"]).expect("a schema");
    rich.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    rich.carry::<Energy, CapillaryBed>(Space::Thermal);
    rich.energy_content::<Glucose>(content(9_007_199_254_740_992.0))
        .expect("the most a content may be");
    let doubled = Proportions::<Oxygen>::using(Space::Interstitium)
        .uses::<Glucose, 2>(Space::Interstitium)
        .makes::<CarbonDioxide, 12>(Space::Interstitium)
        .makes::<Water, 12>(Space::Interstitium)
        .energy_into(Space::Thermal);
    assert_eq!(
        refused(burning(rich, Vec::new(), doubled, paced)),
        CompileDefect::ConservationOverflow { law: Some(BURN) }
    );
}

#[test]
fn a_reaction_whose_pools_meet_at_no_site_is_refused() {
    let mut no_fuel = interstitial(&["bed"]).expect("a schema");
    no_fuel.carry::<Energy, CapillaryBed>(Space::Thermal);
    no_fuel
        .energy_content::<Glucose>(content(1_000.0))
        .expect("a content");
    assert_eq!(
        refused(burning(no_fuel, Vec::new(), burn(), paced)),
        CompileDefect::ReactionNowhere { law: BURN }
    );
}

#[test]
fn a_bed_where_a_reaction_stands_without_its_energy_pool_is_refused() {
    let mut no_store = interstitial(&["bed"]).expect("a schema");
    no_store.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    no_store
        .energy_content::<Glucose>(content(1_000.0))
        .expect("a content");
    assert_eq!(
        refused(burning(no_store, Vec::new(), burn(), paced)),
        CompileDefect::ReactionEndpoint {
            law: BURN,
            site: site::<CapillaryBed>("bed"),
            space: Space::Thermal
        }
    );
}

#[test]
fn a_reaction_with_no_rate_is_refused() {
    assert_eq!(
        refused(burning(schema(), Vec::new(), burn(), |_, _| Ok(()))),
        CompileDefect::OmittedLaw { law: BURN }
    );
}

type Burn<'b> = Reaction<'b, Metabolism, CapillaryBed, Glucose>;

fn twice<'b>(laws: &mut FragmentBuilder<'b, Metabolism>, burn: &Burn<'b>) -> Result<(), AdmissionError> {
    paced(laws, burn)?;
    paced(laws, burn)
}

#[test]
fn two_rates_on_a_reaction_in_one_mode_are_refused() {
    assert_eq!(
        refused(burning(schema(), Vec::new(), burn(), twice)),
        CompileDefect::SecondOwner {
            law: BURN,
            first: Predicate::EveryMode,
            second: Predicate::EveryMode
        }
    );
}

fn fast_only<'b>(laws: &mut FragmentBuilder<'b, Metabolism>, burn: &Burn<'b>) -> Result<(), AdmissionError> {
    let [fast, _slow] = laws.modes("pace", ["fast", "slow"]);
    paced_in(laws, burn, fast.only())
}

#[test]
fn a_reaction_whose_rates_leave_a_mode_uncovered_is_refused() {
    assert_eq!(
        refused(burning(schema(), Vec::new(), burn(), fast_only)),
        CompileDefect::UncoveredMode {
            law: BURN,
            mode: ModeValue {
                variable: "pace",
                value: "slow"
            }
        }
    );
}

fn shadowed<'b>(laws: &mut FragmentBuilder<'b, Metabolism>, burn: &Burn<'b>) -> Result<(), AdmissionError> {
    let _ = laws.state::<Dimensionless>("burn", Admissible::finite());
    paced(laws, burn)
}

#[test]
fn a_reaction_named_as_another_of_its_owners_laws_is_refused() {
    assert_eq!(
        refused(burning(schema(), Vec::new(), burn(), shadowed)),
        CompileDefect::NamedTwice { law: BURN }
    );
}

fn lactic<'b>(laws: &mut FragmentBuilder<'b, Metabolism>, burn: &Burn<'b>) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let speed = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
        ParameterKey::new("mt.burn-speed"),
        Admissible::nonnegative(),
    );
    let (lactate, fuel) = (burn.amount::<Lactate>(), burn.amount::<Glucose>());
    let rate = laws.site_kernel(|mut k| {
        let _ = k.at_site(&lactate);
        let fuel = k.at_site(&fuel);
        let zero = k.zero();
        let speed = k.site_parameter(&speed);
        let rate = k.relax(fuel, zero, speed);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.react_at(burn, rate);
    Ok(())
}

#[test]
fn a_read_of_a_species_the_reaction_does_not_hold_is_refused() {
    assert_eq!(
        refused(burning(schema(), Vec::new(), burn(), lactic)),
        CompileDefect::NoSuchRead { law: BURN }
    );
}

#[test]
fn a_reaction_in_a_left_out_space_is_refused() {
    let mut without = schema();
    without.leave_out(Space::Interstitium, "a bench whose burn has nowhere to stand");
    assert_eq!(
        refused(burning(without, Vec::new(), burn(), paced)),
        CompileDefect::LeftOutTouched {
            law: BURN,
            site: site::<CapillaryBed>("bed"),
            space: Space::Interstitium
        }
    );
}

#[test]
fn a_reaction_that_moves_a_pool_its_card_holds_is_refused() {
    assert_eq!(
        refused(burning(schema(), vec![Space::Interstitium], burn(), paced)),
        CompileDefect::HeldPoolMoved {
            card: <Metabolism as Owner>::CARD,
            site: site::<CapillaryBed>("bed"),
            space: Space::Interstitium
        }
    );
}

type Terms = Vec<(usize, i64)>;

fn combinations(compiled: &CompiledBody) -> Vec<(Book, Terms)> {
    (compiled.conserved_basis().combinations().iter())
        .map(|combination| {
            let terms = combination
                .terms()
                .iter()
                .map(|&(row, weight)| (row.index(), weight));
            (combination.book(), terms.collect())
        })
        .collect()
}

#[test]
fn a_burning_bed_closes_hydrogen_oxygen_carbon_and_energy_in_that_order() {
    let compiled = burning(schema(), Vec::new(), burn(), paced).expect("compiled");
    let element = Book::Element;
    assert_eq!(
        combinations(&compiled),
        [
            (element(Element::Hydrogen), vec![(0, 2), (3, 12)]),
            (element(Element::Oxygen), vec![(0, 1), (1, 2), (2, 2), (3, 6)]),
            (element(Element::Carbon), vec![(2, 1), (3, 6)]),
            (Book::Energy, vec![(3, 1_000), (4, 1)]),
        ]
    );
    let regulated = compiled
        .conserved_basis()
        .combinations()
        .iter()
        .any(Combination::regulated);
    assert!(!regulated, "no boundary reads this body");
}

#[test]
fn a_burn_is_one_column_at_each_bed_with_its_heat() {
    let compiled = burning(
        burning_schema(&["bed a", "bed b"]).expect("a schema"),
        Vec::new(),
        burn(),
        paced,
    )
    .expect("compiled");
    let burns: Vec<(Context, Terms)> = (compiled.columns().iter())
        .filter(|column| column.origin() == BURN)
        .map(|column| {
            assert_eq!(column.receipt(), None, "a reaction has no receipt");
            let terms = column.terms().iter().map(|&(row, count)| (row.index(), count));
            (column.context(), terms.collect())
        })
        .collect();
    let at = |key| Context::Site(site::<CapillaryBed>(key));
    let bed = |first: usize| {
        let row = |offset: usize| first + offset;
        vec![
            (row(0), 6),
            (row(1), -6),
            (row(2), 6),
            (row(3), -1),
            (row(4), 1_000),
        ]
    };
    assert_eq!(burns, [(at("bed a"), bed(0)), (at("bed b"), bed(5))]);
}

#[test]
fn a_reaction_and_its_heat_are_in_the_model_hash() {
    let hash = |stated: f64, name: &'static str| {
        let mut schema = interstitial(&["bed"]).expect("a schema");
        schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
        schema.carry::<Energy, CapillaryBed>(Space::Thermal);
        schema
            .energy_content::<Glucose>(content(stated))
            .expect("a content");
        let compiled = assemble(schema, |mut body| {
            let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), "bench: synthetic");
            let (mut laws, _) = body.hold(held)?.parts();
            let burn = laws.reaction::<CapillaryBed, Glucose>(name, burn());
            paced(&mut laws, &burn)?;
            body.add(laws.finish());
            body.compile()
        });
        compiled.expect("compiled").model_hash()
    };
    assert_eq!(hash(1_000.0, "burn"), hash(1_000.0, "burn"));
    assert_ne!(hash(1_000.0, "burn"), hash(2_000.0, "burn"));
    assert_ne!(hash(1_000.0, "burn"), hash(1_000.0, "oxidation"));
}

fn two_burns<Q: Reactant>() -> Result<CompiledBody, AdmissionError> {
    let mut schema = Schema::new(anatomy(&["bed"])?)?;
    for space in [Space::Cells, Space::Lumen] {
        schema.leave_out(space, "two burns in two spaces alone");
    }
    for space in [Space::Plasma, Space::Interstitium] {
        schema.carry::<Oxygen, CapillaryBed>(space);
        schema.carry::<CarbonDioxide, CapillaryBed>(space);
        schema.carry::<Glucose, CapillaryBed>(space);
    }
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.energy_content::<Glucose>(content(1_000.0))?;
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    assemble(schema, |mut body| {
        let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), "bench: synthetic");
        let (mut laws, _) = body.hold(held)?.parts();
        let interstitial = laws.reaction::<CapillaryBed, Glucose>("burn", burn());
        paced(&mut laws, &interstitial)?;
        let plasma = Proportions::<Glucose>::using(Space::Plasma)
            .uses::<Oxygen, 6>(Space::Plasma)
            .makes::<CarbonDioxide, 6>(Space::Plasma)
            .makes::<Water, 6>(Space::Plasma)
            .energy_into(Space::Thermal);
        let plasma = laws.reaction::<CapillaryBed, Glucose>("burn in plasma", plasma);
        paced(&mut laws, &plasma)?;
        body.add(laws.finish());
        let (mut laws, ports) = body.fluids()?.parts();
        let wall = ports.exchange::<Q>(&mut laws);
        let speed = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
            ParameterKey::new("fl.wall-speed"),
            Admissible::nonnegative(),
        );
        let rate = laws.site_kernel(|mut k| {
            let (inside, outside) = (
                k.at_site(&wall.from_amount::<Q>()),
                k.at_site(&wall.to_amount::<Q>()),
            );
            let speed = k.site_parameter(&speed);
            let rate = k.relax(inside, outside, speed);
            k.finish(rate, laws.every_mode(), class)
        });
        laws.transfer_at(&wall, rate);
        body.add(laws.finish());
        body.compile()
    })
}

/// Oxygen crossing the wall joins the oxygen of both burns, and their shared heat store joins their energy, but no
/// column holds the carbon or the hydrogen of both.
#[test]
fn a_quantity_splits_where_its_own_columns_do_not_join_its_rows() {
    let compiled = two_burns::<Oxygen>().expect("compiled");
    let element = Book::Element;
    let oxygen = vec![(0, 1), (1, 2), (2, 2), (3, 6), (4, 1), (5, 2), (6, 2), (7, 6)];
    assert_eq!(
        combinations(&compiled),
        [
            (element(Element::Hydrogen), vec![(0, 2), (3, 12)]),
            (element(Element::Oxygen), oxygen),
            (element(Element::Carbon), vec![(2, 1), (3, 6)]),
            (Book::Energy, vec![(3, 1_000), (7, 1_000), (8, 1)]),
            (element(Element::Hydrogen), vec![(4, 2), (7, 12)]),
            (element(Element::Carbon), vec![(6, 1), (7, 6)]),
        ]
    );
}

/// Water crossing the wall joins the hydrogen of both burns too, which leaves the oxygen less half the hydrogen in each
/// space conserved apart, and no quantity names it.
#[test]
fn two_burns_joined_only_by_water_leave_a_conserved_direction_unnamed() {
    assert_eq!(
        refused(two_burns::<Water>()),
        CompileDefect::UnnamedConservedQuantity { law: BURN }
    );
}
