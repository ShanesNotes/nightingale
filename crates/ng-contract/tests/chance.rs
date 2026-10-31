use ng_contract::*;

fn two_vessels() -> Result<Schema, AdmissionError> {
    let origin = || -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?);
    let a = map.site::<VesselSegment>("a", origin()?);
    let b = map.site::<VesselSegment>("b", origin()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    Schema::new(map.finish()?)
}

type Heart<'b> = FragmentBuilder<'b, Cardiovascular>;

fn declared(
    declare: impl for<'b> FnOnce(&mut Heart<'b>) -> Result<(), AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    assemble(two_vessels()?, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        declare(&mut laws)?;
        body.add(laws.finish());
        body.compile()
    })
}

fn name(local: &'static str) -> LawName {
    LawName { owner: "CV", local }
}

fn key(name: &'static str) -> ParameterKey {
    ParameterKey::new(name)
}

fn defect(outcome: Result<CompiledBody, AdmissionError>) -> Option<CompileDefect> {
    match outcome {
        Err(AdmissionError::Compile(defect)) => Some(defect),
        _ => None,
    }
}

fn beat() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Beat, 100_000_000)
}

/// A rate kernel that reads no chance, for a slot whose test is about something else.
fn plain_rate<'b>(
    laws: &Heart<'b>,
) -> Result<PureKernel<'b, Cardiovascular, Rate<Dimensionless>>, AdmissionError> {
    let class = beat()?;
    Ok(laws.kernel(|mut k| {
        let rate = k.zero::<Rate<Dimensionless>>();
        k.finish(rate, laws.every_mode(), class)
    }))
}

#[test]
fn a_stream_no_guard_draws_on_is_refused_by_name() {
    let outcome = declared(|laws| {
        let _ = laws.stream("dice");
        Ok(())
    });
    assert_eq!(
        defect(outcome),
        Some(CompileDefect::UndrawnStream { stream: name("dice") })
    );
}

#[test]
fn a_forcing_no_law_reads_is_refused_by_name() {
    let outcome = declared(|laws| {
        let _ = laws.forcing("wobble", key("fixture.t"), key("fixture.a"));
        Ok(())
    });
    assert_eq!(
        defect(outcome),
        Some(CompileDefect::UnreadForcing {
            forcing: name("wobble")
        })
    );
}

#[test]
fn a_second_guard_on_one_stream_is_refused_by_name() {
    let outcome = declared(|laws| {
        let dice = laws.stream("dice");
        let first = laws.state::<Dimensionless>("first", Admissible::nonnegative());
        let second = laws.state::<Dimensionless>("second", Admissible::nonnegative());
        let [a, b] = laws.modes("rhythm", ["a", "b"]);
        laws.hazard(HazardGuard::new(&first, &dice, a.to(&b)?));
        laws.hazard(HazardGuard::new(&second, &dice, b.to(&a)?));
        let (one, two) = (plain_rate(laws)?, plain_rate(laws)?);
        laws.memory_rate(&first, one);
        laws.memory_rate(&second, two);
        Ok(())
    });
    assert_eq!(
        defect(outcome),
        Some(CompileDefect::SecondGuardOnStream { stream: name("dice") })
    );
}

#[test]
fn a_second_guard_on_one_integral_is_refused_by_name() {
    let outcome = declared(|laws| {
        let (dice, more) = (laws.stream("dice"), laws.stream("more dice"));
        let integral = laws.state::<Dimensionless>("integral", Admissible::nonnegative());
        let [a, b] = laws.modes("rhythm", ["a", "b"]);
        laws.hazard(HazardGuard::new(&integral, &dice, a.to(&b)?));
        laws.hazard(HazardGuard::new(&integral, &more, b.to(&a)?));
        let rate = plain_rate(laws)?;
        laws.memory_rate(&integral, rate);
        Ok(())
    });
    assert_eq!(
        defect(outcome),
        Some(CompileDefect::SecondGuardOnIntegral {
            integral: name("integral")
        })
    );
}

#[test]
fn a_hazard_integral_whose_set_cannot_hold_a_reset_is_refused_by_name() {
    for domain in [
        Admissible::positive(),
        Admissible::unit_interval(),
        Admissible::finite(),
    ] {
        let outcome = declared(|laws| {
            let dice = laws.stream("dice");
            let integral = laws.state::<Dimensionless>("integral", domain);
            let [a, b] = laws.modes("rhythm", ["a", "b"]);
            laws.hazard(HazardGuard::new(&integral, &dice, a.to(&b)?));
            let rate = plain_rate(laws)?;
            laws.memory_rate(&integral, rate);
            Ok(())
        });
        assert_eq!(
            defect(outcome),
            Some(CompileDefect::HazardSet {
                integral: name("integral")
            })
        );
    }
}

#[test]
fn a_stream_and_a_slot_under_one_name_are_named_twice() {
    let outcome = declared(|laws| {
        let _ = laws.stream("twice");
        let slot = laws.state::<Dimensionless>("twice", Admissible::finite());
        let rate = plain_rate(laws)?;
        laws.memory_rate(&slot, rate);
        Ok(())
    });
    assert_eq!(
        defect(outcome),
        Some(CompileDefect::NamedTwice { law: name("twice") })
    );
}

/// A body with one memory row at this rate, which binds up to its parameters.
fn bound_rate(
    rate: impl for<'b, 'k> FnOnce(
        &mut Kernel<'b, 'k, Cardiovascular>,
        &Parameter<'b, Cardiovascular, Pressure>,
    ) -> Expr<'b, 'k, Cardiovascular, Rate<Dimensionless>>,
) -> Option<AdmissionError> {
    let body = declared(|laws| {
        let p = laws.parameter::<Pressure>(key("fixture.p"), Admissible::positive());
        let slot = laws.state::<Dimensionless>("memory", Admissible::finite());
        let class = beat()?;
        let kernel = laws.kernel(|mut k| {
            let value = rate(&mut k, &p);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&slot, kernel);
        Ok(())
    })
    .expect("compiled");
    body.bind(ParameterPacket::<SyntheticFixture>::new()).err()
}

#[test]
fn ruling_15_a_constant_built_only_from_literals_is_refused_by_name() {
    let exp = bound_rate(|k, _| {
        let zero = k.zero::<Dimensionless>();
        let one = k.exp(zero);
        let nothing = k.zero::<Rate<Dimensionless>>();
        k.scale(nothing, one)
    });
    let logistic = bound_rate(|k, _| {
        let zero = k.zero::<Dimensionless>();
        let half = k.logistic(zero);
        let nothing = k.zero::<Rate<Dimensionless>>();
        k.scale(nothing, half)
    });
    let nested = bound_rate(|k, _| {
        let (a, b) = (k.zero::<Dimensionless>(), k.zero::<Dimensionless>());
        let sum = k.add(a, b);
        let one = k.exp(sum);
        let nothing = k.zero::<Rate<Dimensionless>>();
        k.scale(nothing, one)
    });
    let refused = Some(AdmissionError::UnbackedConstant { law: name("memory") });
    assert_eq!((exp, logistic, nested), (refused, refused, refused));
}

#[test]
fn ruling_15_a_parameter_divided_by_itself_is_refused_by_name() {
    let over_itself = bound_rate(|k, p| {
        let (first, second) = (k.parameter(p), k.parameter(p));
        let ratio = k.divide(first, second);
        let one = k.as_dimension::<_, Dimensionless>(ratio);
        let nothing = k.zero::<Rate<Dimensionless>>();
        k.scale(nothing, one)
    });
    assert_eq!(
        over_itself,
        Some(AdmissionError::UnbackedConstant { law: name("memory") })
    );
}

#[test]
fn ruling_15_zero_and_a_law_on_a_parameter_are_backed() {
    let outcome = bound_rate(|k, p| {
        let (first, second) = (k.parameter(p), k.parameter(p));
        let gap = k.subtract(first, second);
        let (sum, more) = (k.parameter(p), k.parameter(p));
        let wider = k.add(sum, more);
        let ratio = k.divide(gap, wider);
        let read = k.as_dimension::<_, Dimensionless>(ratio);
        let grown = k.exp(read);
        let nothing = k.zero::<Rate<Dimensionless>>();
        k.scale(nothing, grown)
    });
    assert_eq!(
        outcome,
        Some(AdmissionError::UnboundParameter {
            key: key("fixture.p"),
            binding: Binding::Body
        })
    );
}

fn hazard_body(extra: impl for<'b> FnOnce(&mut Heart<'b>) -> Result<(), AdmissionError>) -> CompiledBody {
    declared(|laws| {
        let dice = laws.stream("dice");
        let integral = laws.state::<Dimensionless>("integral", Admissible::nonnegative());
        let [a, b] = laws.modes("rhythm", ["a", "b"]);
        laws.hazard(HazardGuard::new(&integral, &dice, a.to(&b)?));
        let rate = plain_rate(laws)?;
        laws.memory_rate(&integral, rate);
        extra(laws)
    })
    .expect("compiled")
}

#[test]
fn a_start_mode_is_refused_when_it_is_missing_unknown_or_given_twice() {
    let body = hazard_body(|_| Ok(()));
    let variable = name("rhythm");
    assert_eq!(
        body.bind(ParameterPacket::<SyntheticFixture>::new()).err(),
        Some(AdmissionError::MissingStartMode { variable })
    );
    let mut wrong = ParameterPacket::<SyntheticFixture>::new();
    wrong.start_mode(variable, "c", "synthetic").expect("one entry");
    assert_eq!(
        body.bind(wrong).err(),
        Some(AdmissionError::UnknownStartMode { variable, value: "c" })
    );
    let mut other = ParameterPacket::<SyntheticFixture>::new();
    other.start_mode(variable, "a", "synthetic").expect("one entry");
    let unknown = name("no such variable");
    other.start_mode(unknown, "a", "synthetic").expect("one entry");
    assert_eq!(
        body.bind(other).err(),
        Some(AdmissionError::UnknownStartMode {
            variable: unknown,
            value: "a"
        })
    );
    let mut twice = ParameterPacket::<SyntheticFixture>::new();
    twice.start_mode(variable, "a", "synthetic").expect("one entry");
    assert_eq!(
        twice.start_mode(variable, "b", "synthetic").err(),
        Some(AdmissionError::DuplicateStartMode { variable })
    );
}

#[test]
fn a_mode_variable_a_plain_guard_fires_stays_refused_by_name() {
    let body = hazard_body(|laws| {
        let [shut, open] = laws.modes("valve", ["shut", "open"]);
        let class = beat()?;
        let head = laws.kernel(|mut k| {
            let zero = k.zero::<Pressure>();
            k.finish(zero, shut.only(), class)
        });
        laws.event(Guard::new(head, Crossing::Rising, shut.to(&open)?));
        Ok(())
    });
    assert_eq!(
        body.bind(ParameterPacket::<SyntheticFixture>::new()).err(),
        Some(AdmissionError::ModesNotYetRun {
            variable: name("valve")
        })
    );
}

/// The judge's probe: a law that holds in one mode on an algebraic row would publish a value the new mode's law
/// does not satisfy, because a firing leaves algebraic values as they were and nothing solves them again. A
/// draw read by an algebraic law would do the same. A body with both stays refused until ticket 10.
#[test]
fn a_hazard_in_a_body_with_an_algebraic_row_stays_refused_by_name() {
    let body = hazard_body(|laws| {
        let held = laws.algebraic::<Dimensionless>("held", Admissible::finite());
        let class = beat()?;
        let residual = laws.kernel(|mut k| {
            let seen = k.algebraic(&held);
            k.finish(seen, laws.every_mode(), class)
        });
        laws.residual(&held, residual);
        Ok(())
    });
    assert_eq!(
        body.bind(ParameterPacket::<SyntheticFixture>::new()).err(),
        Some(AdmissionError::ModesNotYetRun {
            variable: name("rhythm")
        })
    );
}

fn hash_of(declare: impl for<'b> FnOnce(&mut Heart<'b>) -> Result<(), AdmissionError>) -> ModelHash {
    declared(declare).expect("compiled").model_hash()
}

fn forced<'b>(laws: &mut Heart<'b>, name: &'static str, class: RateClass) -> Result<(), AdmissionError> {
    let wobble = laws.forcing(name, key("fixture.t"), key("fixture.a"));
    let slot = laws.state::<Dimensionless>("memory", Admissible::finite());
    let rate = laws.kernel(|mut k| {
        let nothing = k.zero::<Rate<Dimensionless>>();
        let varied = k.forcing(&wobble);
        let rate = k.scale(nothing, varied);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.memory_rate(&slot, rate);
    Ok(())
}

#[test]
fn a_forcing_its_stream_and_a_varied_rates_class_enter_the_model_hash() {
    let one = RateClass::new(RateBand::Beat, 100_000_000).expect("a class");
    let other = RateClass::new(RateBand::Beat, 200_000_000).expect("a class");
    let base = hash_of(|laws| forced(laws, "wobble", one));
    assert_eq!(base, hash_of(|laws| forced(laws, "wobble", one)));
    assert_ne!(base, hash_of(|laws| forced(laws, "another wobble", one)));
    assert_ne!(base, hash_of(|laws| forced(laws, "wobble", other)));
    let dice = |name: &'static str| {
        hash_of(move |laws| {
            let dice = laws.stream(name);
            let integral = laws.state::<Dimensionless>("integral", Admissible::nonnegative());
            let [a, b] = laws.modes("rhythm", ["a", "b"]);
            laws.hazard(HazardGuard::new(&integral, &dice, a.to(&b)?));
            let rate = plain_rate(laws)?;
            laws.memory_rate(&integral, rate);
            Ok(())
        })
    };
    assert_ne!(dice("dice"), dice("other dice"));
}
