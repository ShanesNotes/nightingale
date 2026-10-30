use ng_contract::*;

/// The same declarations made in two orders.
#[derive(Clone, Copy, PartialEq)]
enum Order {
    Forward,
    Backward,
}

/// One change to the body that must change its model hash.
#[derive(Clone, Copy, PartialEq)]
enum Change {
    None,
    RateClass,
    Law,
    LeftOut,
    LeftOutReason,
    Anatomy,
    Validity,
}

fn position(axial: f64) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(axial)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// A vessel and two beds on one loop with water flowing in at the vessel: circulation, Starling
/// filtration, the inflow and a nervous fragment with memory, a set point, modes, a guard and an
/// algebraic row. Every value is synthetic.
fn schema(order: Order, change: Change) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let shift = if change == Change::Anatomy { 1.0 } else { 0.0 };
    let (root, vessel, a, b) = if order == Order::Forward {
        let root = map.site::<TissueSite>("body", position(0.0)?);
        let vessel = map.site::<VesselSegment>("vessel", position(0.0)?);
        let a = map.site::<CapillaryBed>("a", position(shift)?);
        let b = map.site::<CapillaryBed>("b", position(0.0)?);
        (root, vessel, a, b)
    } else {
        let b = map.site::<CapillaryBed>("b", position(0.0)?);
        let a = map.site::<CapillaryBed>("a", position(shift)?);
        let vessel = map.site::<VesselSegment>("vessel", position(0.0)?);
        let root = map.site::<TissueSite>("body", position(0.0)?);
        (root, vessel, a, b)
    };
    for site in [vessel.id(), a.id(), b.id()] {
        map.contain(root.id(), site);
    }
    for (from, to) in [(vessel.id(), a.id()), (a.id(), b.id()), (b.id(), vessel.id())] {
        map.connect(Network::Vascular, from, to);
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.water_boundary("inflow", BoundaryDirection::IntoBody, vessel.id(), Space::Lumen)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    let reason = if change == Change::LeftOutReason {
        "cell water waits for its law"
    } else {
        "no cell-water law in this bench"
    };
    schema.leave_out(Space::Cells, reason);
    if order == Order::Backward {
        schema.leave_out(Space::Cells, reason);
    }
    if change == Change::LeftOut {
        schema.leave_out(Space::Fluid, "no enclosure in this bench");
    }
    Ok(schema)
}

#[derive(Clone, Copy, PartialEq)]
enum End {
    Upstream,
    Downstream,
}

/// One end's pressure: its own elastance times the volume of the water it holds.
fn pressure<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular, VascularEdge>,
    route: &EdgeRoute<'b, Cardiovascular, Water>,
    e: &EndParameter<'b, Cardiovascular, Elastance>,
    end: End,
) -> Expr<'b, 'k, Cardiovascular, Pressure> {
    let port = if end == End::Upstream {
        route.from_amount()
    } else {
        route.to_amount()
    };
    let amount = k.at_edge(&port);
    let volume = k.volume(amount);
    let elastance = if end == End::Upstream {
        k.upstream_parameter(e)
    } else {
        k.downstream_parameter(e)
    };
    let product = k.multiply(elastance, volume);
    k.as_dimension(product)
}

fn circulation<'b>(
    scope: CardiovascularScope<'b>,
    order: Order,
    change: Change,
) -> Result<Fragment<'b, Cardiovascular>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let route = ports.circulation(&mut laws);
    let e_key = ParameterKey::new("cv.e");
    let g_key = ParameterKey::new("cv.g");
    let (e, g) = if order == Order::Forward {
        let e = laws.end_parameter::<Elastance>(e_key, Admissible::positive());
        let g = laws.edge_parameter::<Conductance<Water>>(g_key, Admissible::nonnegative());
        (e, g)
    } else {
        let g = laws.edge_parameter::<Conductance<Water>>(g_key, Admissible::nonnegative());
        let e = laws.end_parameter::<Elastance>(e_key, Admissible::positive());
        (e, g)
    };
    let interval = if change == Change::RateClass {
        2_000_000
    } else {
        1_000_000
    };
    let wave = RateClass::new(RateBand::Wave, interval)?;
    let flow = laws.edge_kernel(|mut k| {
        let (upstream, downstream) = if order == Order::Forward {
            let upstream = pressure(&mut k, &route, &e, End::Upstream);
            (upstream, pressure(&mut k, &route, &e, End::Downstream))
        } else {
            let downstream = pressure(&mut k, &route, &e, End::Downstream);
            let _unused = k.edge_parameter(&g);
            (pressure(&mut k, &route, &e, End::Upstream), downstream)
        };
        let head = if change == Change::Law {
            k.subtract(downstream, upstream)
        } else {
            k.subtract(upstream, downstream)
        };
        let conductance = k.edge_parameter(&g);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), wave)
    });
    laws.transfer_along(&route, flow);
    Ok(laws.finish())
}

fn starling(scope: FluidsScope<'_>, order: Order) -> Result<Fragment<'_, Fluids>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let e_key = ParameterKey::new("fl.e");
    let kf_key = ParameterKey::new("fl.kf");
    let (route, e, kf) = if order == Order::Forward {
        let route = ports.starling(&mut laws);
        let e = laws.site_parameter::<CapillaryBed, Elastance>(e_key, Admissible::positive());
        let kf = laws.site_parameter::<CapillaryBed, Conductance<Water>>(kf_key, Admissible::nonnegative());
        (route, e, kf)
    } else {
        let kf = laws.site_parameter::<CapillaryBed, Conductance<Water>>(kf_key, Admissible::nonnegative());
        let e = laws.site_parameter::<CapillaryBed, Elastance>(e_key, Admissible::positive());
        (ports.starling(&mut laws), e, kf)
    };
    let minutes = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let filtration = laws.site_kernel(|mut k| {
        let plasma = k.at_site(&route.from_amount());
        let interstitial = k.at_site(&route.to_amount());
        let pv = k.volume(plasma);
        let iv = k.volume(interstitial);
        let pe = k.site_parameter(&e);
        let ie = k.site_parameter(&e);
        let inside = k.multiply(pe, pv);
        let outside = k.multiply(ie, iv);
        let inside = k.as_dimension::<_, Pressure>(inside);
        let outside = k.as_dimension::<_, Pressure>(outside);
        let head = k.subtract(inside, outside);
        let conductance = k.site_parameter(&kf);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, laws.every_mode(), minutes)
    });
    laws.transfer_at(&route, filtration);
    Ok(laws.finish())
}

fn inflow(scope: BoundaryScope<'_>) -> Result<Fragment<'_, Boundary>, AdmissionError> {
    let (mut laws, ports) = scope.parts();
    let BoundaryWater { route: pair, drive } = ports.water("inflow")?;
    let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
    let rate = laws.kernel(|mut k| {
        let value = k.read(&drive);
        k.finish(value, laws.every_mode(), wave)
    });
    laws.transfer(Transfer::new(&pair, rate));
    Ok(laws.finish())
}

fn reflexes(
    scope: NervousScope<'_>,
    order: Order,
    change: Change,
) -> Result<Fragment<'_, Nervous>, AdmissionError> {
    let (mut laws, _ports) = scope.parts();
    let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
    let hours = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let speed_key = ParameterKey::new("ns.speed");
    let level_key = ParameterKey::new("ns.level");
    let (speed, level) = if order == Order::Forward {
        let speed = laws.parameter::<Rate<Dimensionless>>(speed_key, Admissible::nonnegative());
        (speed, laws.parameter::<Pressure>(level_key, Admissible::finite()))
    } else {
        let level = laws.parameter::<Pressure>(level_key, Admissible::finite());
        (
            laws.parameter::<Rate<Dimensionless>>(speed_key, Admissible::nonnegative()),
            level,
        )
    };
    let (filter, set_point, pressure) = if order == Order::Forward {
        let filter = laws.state::<Dimensionless>("filter", Admissible::unit_interval());
        let set_point = laws.set_point::<Dimensionless>("set-point", Admissible::unit_interval());
        (
            filter,
            set_point,
            laws.algebraic::<Pressure>("pressure", Admissible::finite()),
        )
    } else {
        let pressure = laws.algebraic::<Pressure>("pressure", Admissible::finite());
        let set_point = laws.set_point::<Dimensionless>("set-point", Admissible::unit_interval());
        (
            laws.state::<Dimensionless>("filter", Admissible::unit_interval()),
            set_point,
            pressure,
        )
    };
    let [on, off] = laws.modes("switch", ["on", "off"]);
    let gate = laws.state::<Dimensionless>("gate", Admissible::unit_interval());
    let relax = laws.kernel(|mut k| {
        let reference = k.state(&set_point);
        let now = k.state(&filter);
        let error = k.deviation(now, reference);
        let target = k.logistic(error);
        let now = k.state(&filter);
        let pace = k.parameter(&speed);
        let rate = k.relax(target, now, pace);
        k.finish(rate, laws.every_mode(), reflex)
    });
    let drift = laws.kernel(|mut k| {
        let zero = k.zero::<Rate<Reference<Dimensionless>>>();
        k.finish(zero, laws.every_mode(), hours)
    });
    let still = |modes| {
        laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            k.finish(zero, modes, reflex)
        })
    };
    let gap = |modes| {
        laws.kernel(|mut k| {
            let actual = k.algebraic(&pressure);
            let wanted = k.parameter(&level);
            let gap = k.subtract(actual, wanted);
            k.finish(gap, modes, reflex)
        })
    };
    let (when_on, when_off) = if order == Order::Forward {
        let when_on = still(on.only());
        (when_on, still(off.only()))
    } else {
        let when_off = still(off.only());
        (still(on.only()), when_off)
    };
    let closes = gap(laws.every_mode());
    let opens = gap(on.only());
    let shuts = gap(off.only());
    let upper = if change == Change::Validity { 2.0 } else { 1.0 };
    let trusted = ValidityRange::new(
        Measure::new(-1.0)?,
        Measure::new(upper)?,
        Basis::Fixture("synthetic"),
    )?;
    let opening = Guard::new(opens, Crossing::Rising, on.to(&off)?);
    let closing = Guard::new(shuts, Crossing::Falling, off.to(&on)?);
    if order == Order::Forward {
        laws.validity(&relax, trusted);
        laws.memory_rate(&filter, relax);
        laws.memory_rate(&set_point, drift);
        laws.memory_rate(&gate, when_on);
        laws.memory_rate(&gate, when_off);
        laws.residual(&pressure, closes);
        laws.event(opening);
        laws.event(closing);
    } else {
        laws.event(closing);
        laws.event(opening);
        laws.residual(&pressure, closes);
        laws.memory_rate(&gate, when_off);
        laws.memory_rate(&gate, when_on);
        laws.memory_rate(&set_point, drift);
        laws.validity(&relax, trusted);
        laws.memory_rate(&filter, relax);
    }
    Ok(laws.finish())
}

fn body(order: Order, change: Change) -> Result<CompiledBody, AdmissionError> {
    assemble(schema(order, change)?, |mut body| {
        if order == Order::Forward {
            let cv = circulation(body.cardiovascular()?, order, change)?;
            let fl = starling(body.fluids()?, order)?;
            let boundary = inflow(body.boundary()?)?;
            let ns = reflexes(body.nervous()?, order, change)?;
            body.add(cv);
            body.add(fl);
            body.add(boundary);
            body.add(ns);
        } else {
            let ns = reflexes(body.nervous()?, order, change)?;
            let boundary = inflow(body.boundary()?)?;
            let fl = starling(body.fluids()?, order)?;
            let cv = circulation(body.cardiovascular()?, order, change)?;
            body.add(fl);
            body.add(ns);
            body.add(cv);
            body.add(boundary);
        }
        body.compile()
    })
}

#[test]
fn neither_declaration_order_nor_fragment_order_reaches_the_body() -> Result<(), AdmissionError> {
    let forward = body(Order::Forward, Change::None)?;
    let backward = body(Order::Backward, Change::None)?;
    assert_eq!(forward.rows(), backward.rows());
    assert_eq!(forward.columns(), backward.columns());
    assert_eq!(forward.programs(), backward.programs());
    assert_eq!(forward.instances(), backward.instances());
    assert_eq!(forward.parameters(), backward.parameters());
    assert_eq!(forward.sparsity(), backward.sparsity());
    assert_eq!(forward.blocks(), backward.blocks());
    assert_eq!(forward.conserved_basis(), backward.conserved_basis());
    assert_eq!(forward.model_hash(), backward.model_hash());
    Ok(())
}

#[test]
fn compiling_one_body_twice_gives_one_hash() -> Result<(), AdmissionError> {
    assert_eq!(
        body(Order::Forward, Change::None)?.model_hash(),
        body(Order::Forward, Change::None)?.model_hash()
    );
    Ok(())
}

#[test]
fn a_changed_rate_class_law_left_out_space_or_reason_anatomy_or_validity_changes_the_hash()
-> Result<(), AdmissionError> {
    let base = body(Order::Forward, Change::None)?.model_hash();
    for change in [
        Change::RateClass,
        Change::Law,
        Change::LeftOut,
        Change::LeftOutReason,
        Change::Anatomy,
        Change::Validity,
    ] {
        assert_ne!(body(Order::Forward, change)?.model_hash(), base);
    }
    Ok(())
}

#[test]
fn the_refusal_named_does_not_depend_on_fragment_order() -> Result<(), AdmissionError> {
    let refused = |cardiovascular_first: bool| {
        assemble(schema(Order::Forward, Change::None)?, |mut body| {
            let (mut cv, _) = body.cardiovascular()?.parts();
            let (mut ns, _) = body.nervous()?.parts();
            for _ in 0..2 {
                cv.state::<Dimensionless>("twice", Admissible::finite());
                ns.state::<Dimensionless>("twice", Admissible::finite());
            }
            if cardiovascular_first {
                body.add(cv.finish());
                body.add(ns.finish());
            } else {
                body.add(ns.finish());
                body.add(cv.finish());
            }
            body.compile()
        })
        .map(|compiled| compiled.model_hash())
    };
    let named = Err(AdmissionError::Compile(CompileDefect::NamedTwice {
        law: LawName {
            owner: "CV",
            local: "twice",
        },
    }));
    assert_eq!(refused(true), named);
    assert_eq!(refused(false), named);
    Ok(())
}

/// Two memory slots whose rate laws are the same zero kernel, with a validity range on one of them.
fn twins(range_on_first: bool, first_built_first: bool) -> Result<ModelHash, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", position(0.0)?);
    let compiled = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let first = laws.state::<Dimensionless>("first", Admissible::finite());
        let second = laws.state::<Dimensionless>("second", Admissible::finite());
        let still = || {
            laws.kernel(|mut k| {
                let zero = k.zero::<Rate<Dimensionless>>();
                k.finish(zero, laws.every_mode(), reflex)
            })
        };
        let (one, two) = if first_built_first {
            let one = still();
            (one, still())
        } else {
            let two = still();
            (still(), two)
        };
        let range = ValidityRange::new(
            Measure::new(-1.0)?,
            Measure::new(1.0)?,
            Basis::Fixture("synthetic"),
        )?;
        laws.validity(if range_on_first { &one } else { &two }, range);
        laws.memory_rate(&first, one);
        laws.memory_rate(&second, two);
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(compiled.model_hash())
}

#[test]
fn a_validity_range_stays_with_its_kernel_however_alike_two_kernels_are() -> Result<(), AdmissionError> {
    assert_ne!(twins(true, true)?, twins(false, true)?);
    assert_eq!(twins(true, true)?, twins(true, false)?);
    Ok(())
}
