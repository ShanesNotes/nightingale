use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).unwrap()
}

fn admit(periodic: bool) -> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let x = ns.algebraic::<Dimensionless>("x", Admissible::finite());
        let y = ns.algebraic::<Dimensionless>("y", Admissible::finite());
        let p = ns.parameter::<Dimensionless>(ParameterKey::new("synthetic p"), Admissible::finite());
        let class = RateClass::new(RateBand::Minutes, 1_000_000_000)?;
        for (row, other) in [(&x, &y), (&y, &x)] {
            let residual = ns.kernel(|mut k| {
                let own = k.algebraic(row);
                let other = k.algebraic(other);
                let coefficient = k.parameter(&p);
                let scaled = k.scale(other, coefficient);
                let value = k.add(own, scaled);
                k.finish(value, ns.every_mode(), class)
            });
            ns.residual(row, residual);
        }
        body.add(ns.finish());
        body.compile()
    })?;
    let mut packet = if periodic {
        ParameterPacket::periodic(Cycle::new(
            NonZeroU64::new(2_000_000_000).unwrap(),
            NonZeroU32::new(2).unwrap(),
        )?)
    } else {
        ParameterPacket::new()
    };
    packet.synthetic(
        ParameterKey::new("synthetic p"),
        Binding::Body,
        measure::<Dimensionless>(0.0),
        "synthetic",
    )?;
    packet.start_rest(&body, |_| 0.0, "synthetic exact root")?;
    let system = body.initialization(
        body.bind(packet)?,
        BoundaryPort::new(vec![])?,
        InitializationRequest::at(SimTime::from_ns(0)),
    )?;
    let exact = system.workspace();
    assert!(system.evaluate(&exact)?.iter().all(|value| *value == 0.0));
    let jacobian = system.linearize(&exact)?;
    let view = jacobian.view();
    let size = view.row_offsets().len() - 1;
    let mut dense = vec![vec![0.0; size + 1]; size];
    for (row, entries) in dense.iter_mut().enumerate() {
        for index in view.row_offsets()[row]..view.row_offsets()[row + 1] {
            entries[view.columns()[index].index()] += view.values()[index];
        }
    }
    assert_eq!(
        eliminate(dense).unwrap(),
        vec![0.0; size],
        "the full Jacobian is invertible"
    );
    let history = MethodHistory::new(
        Digest::of_canonical(b"synthetic history"),
        SimTime::from_ns(0),
        vec![],
    );
    system.admit(exact, history)?;
    Ok(())
}

#[test]
fn a_body_with_a_zero_on_its_fixed_path_is_admitted_at_an_instant() {
    admit(false).expect("identity Jacobian");
}

#[test]
fn a_ring_with_a_zero_on_its_fixed_path_is_read_whole_and_admitted() {
    admit(true).expect("identity Jacobian at every point");
}

fn two_closed_rows(shared: bool, second_first: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e1 = ns.state::<Dimensionless>("e1", Admissible::finite());
        let e2 = ns.state::<Dimensionless>("e2", Admissible::finite());
        let p1 = ns.algebraic::<Rate<Dimensionless>>("p1", Admissible::finite());
        let p2 = ns.algebraic::<Rate<Dimensionless>>("p2", Admissible::finite());
        let t1 = ns.parameter::<Dimensionless>(ParameterKey::new("t1"), Admissible::finite());
        let t2 = ns.parameter::<Dimensionless>(ParameterKey::new("t2"), Admissible::finite());
        let first = ns.kernel(|mut k| {
            let (one, two, zero) = (k.algebraic(&p1), k.algebraic(&p2), k.zero());
            let both = k.add(one, two);
            let out = k.subtract(zero, both);
            k.finish(out, ns.every_mode(), class)
        });
        ns.memory_rate(&e1, first);
        let second = ns.kernel(|mut k| {
            let (two, zero) = (k.algebraic(&p2), k.zero());
            let out = k.subtract(zero, two);
            k.finish(out, ns.every_mode(), class)
        });
        ns.memory_rate(&e2, second);
        let hold_one = ns.kernel(|mut k| {
            let (held, goal) = (k.state(&e1), k.parameter(&t1));
            let gap = k.subtract(held, goal);
            k.finish(gap, ns.every_mode(), class)
        });
        let hold_two = ns.kernel(|mut k| {
            let (one, two, goal) = (k.state(&e1), k.state(&e2), k.parameter(&t2));
            let held = if shared { k.add(one, two) } else { two };
            let gap = k.subtract(held, goal);
            k.finish(gap, ns.every_mode(), class)
        });
        if second_first {
            ns.closes(&p2, hold_two);
            ns.closes(&p1, hold_one);
        } else {
            ns.closes(&p1, hold_one);
            ns.closes(&p2, hold_two);
        }
        body.add(ns.finish());
        body.compile()
    })
}

#[test]
fn two_closed_rows_that_read_one_store_factor_over_the_bodys_elimination() -> Result<(), InitializationFailure>
{
    for (shared, second_first) in [(false, false), (false, true), (true, false), (true, true)] {
        let body = two_closed_rows(shared, second_first)?;
        let declared_first = two_closed_rows(shared, false)?.model_hash();
        assert_eq!(
            body.model_hash(),
            declared_first,
            "shared {shared}, second first {second_first}"
        );
        let cycle = Cycle::new(
            NonZeroU64::new(2_000_000_000).unwrap(),
            NonZeroU32::new(2).unwrap(),
        )?;
        let mut packet = ParameterPacket::periodic(cycle);
        for key in ["t1", "t2"] {
            packet.synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Dimensionless>(0.5),
                "synthetic",
            )?;
        }
        packet.start_rest(&body, |_| 1.0, "synthetic")?;
        let port = BoundaryPort::new(Vec::new())?;
        let request = InitializationRequest::at(SimTime::from_ns(0));
        let system = body.initialization(body.bind(packet)?, port, request)?;
        let (_, block) = system.stage(&system.workspace(), 0)?;
        assert!(
            system.factor(&block).is_ok(),
            "shared {shared}, second first {second_first}"
        );
    }
    Ok(())
}

/// Whether an interval's stage block factors over the system's plan, with no pivot left to a whole read.
fn stage_factors(body: &CompiledBody, keys: &[&'static str]) -> Result<bool, InitializationFailure> {
    let cycle = Cycle::new(
        NonZeroU64::new(2_000_000_000).unwrap(),
        NonZeroU32::new(2).unwrap(),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    for &key in keys {
        packet.synthetic(
            ParameterKey::new(key),
            Binding::Body,
            measure::<Dimensionless>(0.5),
            "synthetic",
        )?;
    }
    packet.start_rest(body, |_| 1.0, "synthetic")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(body.bind(packet)?, port, request)?;
    let (_, block) = system.stage(&system.workspace(), 0)?;
    Ok(system.factor(&block).is_ok())
}

#[test]
fn two_closed_rows_only_one_store_can_hold_are_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let compiled = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e = ns.state::<Dimensionless>("e", Admissible::finite());
        let p1 = ns.algebraic::<Rate<Dimensionless>>("p1", Admissible::finite());
        let p2 = ns.algebraic::<Rate<Dimensionless>>("p2", Admissible::finite());
        let pays = ns.kernel(|mut k| {
            let (one, two, zero) = (k.algebraic(&p1), k.algebraic(&p2), k.zero());
            let both = k.add(one, two);
            let out = k.subtract(zero, both);
            k.finish(out, ns.every_mode(), class)
        });
        ns.memory_rate(&e, pays);
        for (row, key) in [(&p1, "t1"), (&p2, "t2")] {
            let goal = ns.parameter::<Dimensionless>(ParameterKey::new(key), Admissible::finite());
            let hold = ns.kernel(|mut k| {
                let (held, goal) = (k.state(&e), k.parameter(&goal));
                let gap = k.subtract(held, goal);
                k.finish(gap, ns.every_mode(), class)
            });
            ns.closes(row, hold);
        }
        body.add(ns.finish());
        body.compile()
    });
    let law = LawName {
        owner: "NS",
        local: "p2",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::StructuralSingularity {
            law
        }))
    );
    Ok(())
}

fn crossed(second_first: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e1 = ns.state::<Dimensionless>("e1", Admissible::finite());
        let e2 = ns.state::<Dimensionless>("e2", Admissible::finite());
        let p1 = ns.algebraic::<Rate<Dimensionless>>("p1", Admissible::finite());
        let p2 = ns.algebraic::<Rate<Dimensionless>>("p2", Admissible::finite());
        let t1 = ns.parameter::<Dimensionless>(ParameterKey::new("t1"), Admissible::finite());
        let t2 = ns.parameter::<Dimensionless>(ParameterKey::new("t2"), Admissible::finite());
        for (store, apart) in [(&e1, false), (&e2, true)] {
            let pays = ns.kernel(|mut k| {
                let (one, two, zero) = (k.algebraic(&p1), k.algebraic(&p2), k.zero());
                let out = if apart {
                    k.subtract(two, one)
                } else {
                    k.add(one, two)
                };
                let out = if apart { out } else { k.subtract(zero, out) };
                k.finish(out, ns.every_mode(), class)
            });
            ns.memory_rate(store, pays);
        }
        let mut holds = Vec::new();
        for (goal, apart) in [(&t1, false), (&t2, true)] {
            holds.push(ns.kernel(|mut k| {
                let (one, two, goal) = (k.state(&e1), k.state(&e2), k.parameter(goal));
                let held = if apart {
                    k.subtract(one, two)
                } else {
                    k.add(one, two)
                };
                let gap = k.subtract(held, goal);
                k.finish(gap, ns.every_mode(), class)
            }));
        }
        let [first, second] = <[_; 2]>::try_from(holds).ok().expect("two holds");
        if second_first {
            ns.closes(&p2, second);
            ns.closes(&p1, first);
        } else {
            ns.closes(&p1, first);
            ns.closes(&p2, second);
        }
        body.add(ns.finish());
        body.compile()
    })
}

#[test]
fn two_closed_rows_either_store_could_hold_take_the_same_stores_in_either_order()
-> Result<(), InitializationFailure> {
    let (forward, backward) = (crossed(false)?, crossed(true)?);
    assert_eq!(forward.model_hash(), backward.model_hash());
    for body in [&forward, &backward] {
        assert!(stage_factors(body, &["t1", "t2"])?);
    }
    Ok(())
}

#[test]
fn a_closed_row_that_reads_no_store_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let compiled = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let p = ns.algebraic::<Dimensionless>("p", Admissible::finite());
        let q = ns.algebraic::<Dimensionless>("q", Admissible::finite());
        let both = ns.kernel(|mut k| {
            let (one, two) = (k.algebraic(&p), k.algebraic(&q));
            let sum = k.add(one, two);
            k.finish(sum, ns.every_mode(), class)
        });
        ns.closes(&p, both);
        let one = ns.kernel(|mut k| {
            let value = k.algebraic(&p);
            k.finish(value, ns.every_mode(), class)
        });
        ns.residual(&q, one);
        body.add(ns.finish());
        body.compile()
    });
    let law = LawName {
        owner: "NS",
        local: "p",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::StructuralSingularity {
            law
        }))
    );
    Ok(())
}

#[test]
fn a_closed_row_keeps_its_own_unknown_when_another_row_could_take_it() -> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e = ns.state::<Dimensionless>("e", Admissible::finite());
        let p = ns.algebraic::<Dimensionless>("p", Admissible::finite());
        let q = ns.algebraic::<Dimensionless>("q", Admissible::finite());
        let s = ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("s"), Admissible::positive());
        let pays = ns.kernel(|mut k| {
            let (speed, held) = (k.parameter(&s), k.algebraic(&p));
            let rate = k.scale(speed, held);
            k.finish(rate, ns.every_mode(), class)
        });
        ns.memory_rate(&e, pays);
        let hold = ns.kernel(|mut k| {
            let (store, other) = (k.state(&e), k.algebraic(&q));
            let sum = k.add(store, other);
            k.finish(sum, ns.every_mode(), class)
        });
        ns.closes(&p, hold);
        let follows = ns.kernel(|mut k| {
            let (own, held) = (k.algebraic(&q), k.algebraic(&p));
            let gap = k.subtract(own, held);
            k.finish(gap, ns.every_mode(), class)
        });
        ns.residual(&q, follows);
        body.add(ns.finish());
        body.compile()
    })?;
    let cycle = Cycle::new(
        NonZeroU64::new(2_000_000_000).unwrap(),
        NonZeroU32::new(2).unwrap(),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    let speed = measure::<Rate<Dimensionless>>(0.5);
    packet.synthetic(ParameterKey::new("s"), Binding::Body, speed, "synthetic")?;
    packet.start_rest(&body, |_| 0.0, "synthetic")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(body.bind(packet)?, port, request)?;
    let (_, block) = system.stage(&system.workspace(), 0)?;
    assert!(system.factor(&block).is_ok());
    Ok(())
}

#[test]
fn a_closed_row_whose_store_reads_its_unknown_through_an_algebraic_row_is_admitted()
-> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e = ns.state::<Dimensionless>("e", Admissible::finite());
        let p = ns.algebraic::<Rate<Dimensionless>>("p", Admissible::finite());
        let q = ns.algebraic::<Rate<Dimensionless>>("q", Admissible::finite());
        let pays = ns.kernel(|mut k| {
            let flow = k.algebraic(&q);
            k.finish(flow, ns.every_mode(), class)
        });
        ns.memory_rate(&e, pays);
        let follows = ns.kernel(|mut k| {
            let (own, held) = (k.algebraic(&q), k.algebraic(&p));
            let gap = k.subtract(own, held);
            k.finish(gap, ns.every_mode(), class)
        });
        ns.residual(&q, follows);
        let hold = ns.kernel(|mut k| {
            let store = k.state(&e);
            k.finish(store, ns.every_mode(), class)
        });
        ns.closes(&p, hold);
        body.add(ns.finish());
        body.compile()
    })?;
    let cycle = Cycle::new(
        NonZeroU64::new(2_000_000_000).unwrap(),
        NonZeroU32::new(2).unwrap(),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    packet.start_rest(&body, |_| 0.0, "synthetic exact root")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(body.bind(packet)?, port, request)?;
    let exact = system.workspace();
    assert!(system.evaluate(&exact)?.iter().all(|value| *value == 0.0));
    let history = MethodHistory::new(
        Digest::of_canonical(b"synthetic history"),
        SimTime::from_ns(0),
        vec![],
    );
    system.admit(exact, history)?;
    Ok(())
}

#[test]
fn a_closed_row_whose_unknown_moves_no_store_it_reads_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let compiled = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e = ns.state::<Dimensionless>("e", Admissible::finite());
        let p = ns.algebraic::<Rate<Dimensionless>>("p", Admissible::finite());
        let q = ns.algebraic::<Rate<Dimensionless>>("q", Admissible::finite());
        let pays = ns.kernel(|mut k| {
            let flow = k.algebraic(&q);
            k.finish(flow, ns.every_mode(), class)
        });
        ns.memory_rate(&e, pays);
        let rests = ns.kernel(|mut k| {
            let own = k.algebraic(&q);
            k.finish(own, ns.every_mode(), class)
        });
        ns.residual(&q, rests);
        let hold = ns.kernel(|mut k| {
            let store = k.state(&e);
            k.finish(store, ns.every_mode(), class)
        });
        ns.closes(&p, hold);
        body.add(ns.finish());
        body.compile()
    });
    let law = LawName {
        owner: "NS",
        local: "p",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::StructuralSingularity {
            law
        }))
    );
    Ok(())
}

#[test]
fn a_closed_row_whose_unknown_reaches_its_store_only_through_another_store_is_refused()
-> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>(
        "synthetic body",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let compiled = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let e = ns.state::<Dimensionless>("e", Admissible::finite());
        let f = ns.state::<Dimensionless>("f", Admissible::finite());
        let p = ns.algebraic::<Rate<Dimensionless>>("p", Admissible::finite());
        let s = ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("s"), Admissible::positive());
        let follows = ns.kernel(|mut k| {
            let (speed, other) = (k.parameter(&s), k.state(&f));
            let rate = k.scale(speed, other);
            k.finish(rate, ns.every_mode(), class)
        });
        ns.memory_rate(&e, follows);
        let pays = ns.kernel(|mut k| {
            let flow = k.algebraic(&p);
            k.finish(flow, ns.every_mode(), class)
        });
        ns.memory_rate(&f, pays);
        let hold = ns.kernel(|mut k| {
            let store = k.state(&e);
            k.finish(store, ns.every_mode(), class)
        });
        ns.closes(&p, hold);
        body.add(ns.finish());
        body.compile()
    });
    let law = LawName {
        owner: "NS",
        local: "p",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::StructuralSingularity {
            law
        }))
    );
    Ok(())
}
