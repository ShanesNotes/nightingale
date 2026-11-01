use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite synthetic value")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn schema() -> Schema {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    Schema::new(map.finish().expect("an anatomy")).expect("a schema")
}

fn rate_class() -> RateClass {
    RateClass::new(RateBand::Wave, 1_000_000_000).expect("a class")
}

fn history() -> MethodHistory {
    MethodHistory::new(
        Digest::of_canonical(b"review synthetic start"),
        SimTime::from_ns(0),
        Vec::new(),
    )
}

#[test]
fn a_one_point_cycle_with_a_total_nothing_regulates_has_no_rank() {
    let site = AnatomyBuilder::new().site::<TissueSite>("body", origin()).id();
    let mut schema = schema();
    schema.pool::<Energy>(site, Space::Thermal).expect("a pool");
    schema
        .boundary::<Energy>("heat", BoundaryDirection::OutOfBody, site, Space::Thermal)
        .expect("a boundary");
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let route = ports.route::<Energy>("heat")?.route;
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.speed"), Admissible::finite());
        let rate = laws.kernel(|mut k| {
            let one = k.read(&route.amount::<Energy>());
            let other = k.read(&route.amount::<Energy>());
            let speed = k.parameter(&speed);
            let zero = k.relax(one, other, speed);
            k.finish(zero, laws.every_mode(), rate_class())
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let cycle = Cycle::new(
        NonZeroU64::new(1_000_000_000).unwrap(),
        NonZeroU32::new(1).unwrap(),
    )
    .expect("a one-point cycle");
    let mut packet = ParameterPacket::periodic(cycle);
    packet
        .synthetic(
            ParameterKey::new("bd.speed"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(1.0),
            "synthetic",
        )
        .expect("a parameter");
    let row = RowKind::Amount {
        site,
        space: Space::Thermal,
        species: Species::Energy,
    };
    packet
        .start(row, measure::<Energy>(1.0), "synthetic")
        .expect("a start");
    let parameters = body.bind(packet).expect("bound");
    let system = body
        .initialization(
            parameters,
            BoundaryPort::new(Vec::new()).unwrap(),
            InitializationRequest::at(SimTime::from_ns(0)),
        )
        .expect("a system");
    let mut proposed = system.workspace();
    proposed.unknowns_mut().fill(7.0);
    let error = system.admit(proposed, history()).err();
    assert!(
        matches!(error, Some(InitializationFailure::NoRank { .. })),
        "an arbitrary free total must not be admitted"
    );
}

#[test]
fn two_proportional_rows_gain_no_rank_from_rounding() {
    let body = assemble(schema(), |mut body| {
        let (mut laws, _) = body.nervous()?.parts();
        let x = laws.algebraic::<Dimensionless>("x", Admissible::finite());
        let y = laws.algebraic::<Dimensionless>("y", Admissible::finite());
        let a = laws.parameter::<Dimensionless>(ParameterKey::new("ns.a"), Admissible::finite());
        let b = laws.parameter::<Dimensionless>(ParameterKey::new("ns.b"), Admissible::finite());
        let c = laws.parameter::<Dimensionless>(ParameterKey::new("ns.c"), Admissible::finite());
        for (row, scaled) in [(&x, false), (&y, true)] {
            let residual = laws.kernel(|mut k| {
                let xv = k.algebraic(&x);
                let yv = k.algebraic(&y);
                let av = k.parameter(&a);
                let bv = k.parameter(&b);
                let ax = k.scale(xv, av);
                let by = k.scale(yv, bv);
                let sum = k.add(ax, by);
                let value = if scaled {
                    let cv = k.parameter(&c);
                    k.scale(sum, cv)
                } else {
                    sum
                };
                k.finish(value, laws.every_mode(), rate_class())
            });
            laws.residual(row, residual);
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    for (key, value) in [("ns.a", 3.0), ("ns.b", 7.0), ("ns.c", 0.1)] {
        packet
            .synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Dimensionless>(value),
                "synthetic",
            )
            .expect("a parameter");
    }
    for (local, value) in [("x", 7.0), ("y", -3.0)] {
        packet
            .start(
                RowKind::Algebraic(LawName { owner: "NS", local }.into()),
                measure::<Dimensionless>(value),
                "synthetic",
            )
            .expect("a start");
    }
    let parameters = body.bind(packet).expect("bound");
    let system = body
        .initialization(
            parameters,
            BoundaryPort::new(Vec::new()).unwrap(),
            InitializationRequest::at(SimTime::from_ns(0)),
        )
        .expect("a system");
    let proposed = system.workspace();
    let error = system.admit(proposed, history()).err();
    assert!(
        matches!(error, Some(InitializationFailure::NoRank { .. })),
        "0.1 times the same equation leaves one free level"
    );
}

#[test]
fn two_fits_of_one_anchors_worth_of_information_have_no_rank() {
    let site = AnatomyBuilder::new().site::<TissueSite>("body", origin()).id();
    let mut schema = schema();
    schema.pool::<Energy>(site, Space::Thermal).expect("a pool");
    schema
        .boundary::<Energy>("heat", BoundaryDirection::OutOfBody, site, Space::Thermal)
        .expect("a boundary");
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let route = ports.route::<Energy>("heat")?.route;
        let reading = laws.algebraic::<Energy>("reading", Admissible::finite());
        let a = laws.parameter::<Energy>(ParameterKey::new("bd.a"), Admissible::finite());
        let b = laws.parameter::<Energy>(ParameterKey::new("bd.b"), Admissible::finite());
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        let residual = laws.kernel(|mut k| {
            let amount = k.read(&route.amount::<Energy>());
            let av = k.parameter(&a);
            let bv = k.parameter(&b);
            let sum = k.add(av, bv);
            let target = k.add(amount, sum);
            let read = k.algebraic(&reading);
            let value = k.subtract(read, target);
            k.finish(value, laws.every_mode(), class)
        });
        laws.residual(&reading, residual);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    let amount = RowKind::Amount {
        site,
        space: Space::Thermal,
        species: Species::Energy,
    };
    let reading = RowKind::Algebraic(
        LawName {
            owner: "Boundary",
            local: "reading",
        }
        .into(),
    );
    packet
        .start(amount, measure::<Energy>(1.0), "synthetic")
        .expect("a start");
    packet
        .start(reading, measure::<Energy>(6.0), "synthetic")
        .expect("a start");
    for (key, anchor, guess) in [("bd.a", "anchor-a", 2.0), ("bd.b", "anchor-b", 3.0)] {
        let anchor = AnchorId::new(anchor);
        packet
            .anchor(Synthetic::new(
                anchor,
                Snapshot::new(reading, "gauge", AnchorStatistic::Instant, measure::<Energy>(6.0)),
                "synthetic",
            ))
            .expect("an anchor");
        packet
            .calibrated(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Energy>(guess),
                anchor,
            )
            .expect("a fit");
    }
    let lens = body
        .lens(
            vec![ChannelSpec::new("gauge", reading)],
            NonZeroU64::new(1_000_000_000).unwrap(),
        )
        .expect("a lens");
    let parameters = body.bind(packet).expect("bound");
    let system = body
        .initialization(
            parameters,
            BoundaryPort::new(Vec::new()).unwrap(),
            InitializationRequest::at(SimTime::from_ns(0)).reading(&lens),
        )
        .expect("a system");
    let proposed = system.workspace();
    let error = system.admit(proposed, history()).err();
    assert!(
        matches!(error, Some(InitializationFailure::NoRank { .. })),
        "two copies of one reading cannot determine two parameters"
    );
}

/// At an instant a memory's row is its value less its start, so its slope on itself is one, whatever its rate. A slope of
/// the rate would read as no rank on a row that has one.
#[test]
fn an_instants_stage_reads_the_row_it_returns() -> Result<(), InitializationFailure> {
    let body = assemble(schema(), |mut body| {
        let (mut ns, _) = body.nervous()?.parts();
        let held = ns.state::<Dimensionless>("held", Admissible::finite());
        let class = rate_class();
        let still = ns.kernel(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            k.finish(zero, ns.every_mode(), class)
        });
        ns.memory_rate(&held, still);
        body.add(ns.finish());
        body.compile()
    })?;
    let mut packet = ParameterPacket::new();
    packet.start_rest(&body, |_| 1.0, "synthetic")?;
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(parameters, BoundaryPort::new(Vec::new())?, request)?;
    let workspace = system.workspace();
    let (_, stage) = system.stage(&workspace, 0)?;
    assert_eq!(stage.view().values()[0], 1.0);
    Ok(())
}
