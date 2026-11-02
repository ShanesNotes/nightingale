use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn reading() -> RowKind {
    RowKind::Algebraic(
        LawName {
            owner: "Boundary",
            local: "reading",
        }
        .into(),
    )
}

fn body() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let origin = Position::new(measure(0.0), measure(0.0), measure(0.0));
    let site = map.site::<TissueSite>("body", origin).id();
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(site, Space::Thermal)?;
    schema.boundary::<Energy>("heat", BoundaryDirection::OutOfBody, site, Space::Thermal)?;
    assemble(schema, |mut assembly| {
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let (mut ns, _) = assembly.nervous()?.parts();
        let x = ns.state::<Dimensionless>("x", Admissible::finite());
        let threshold = ns.parameter::<Dimensionless>(ParameterKey::new("threshold"), Admissible::positive());
        let speed = ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("speed"), Admissible::positive());
        let [a, b] = ns.modes("switch", ["a", "b"]);
        for (mode, doubled) in [(&a, false), (&b, true)] {
            let rate = ns.kernel(|mut k| {
                let (zero, current, speed) = (k.zero(), k.state(&x), k.parameter(&speed));
                let rate = k.relax(zero, current, speed);
                let rate = if doubled { k.add(rate, rate) } else { rate };
                k.finish(rate, mode.only(), class)
            });
            ns.memory_rate(&x, rate);
        }
        let entering = ns.kernel(|mut k| {
            let (value, threshold) = (k.state(&x), k.parameter(&threshold));
            let gap = k.subtract(value, threshold);
            k.finish(gap, a.only(), class)
        });
        ns.event(Guard::new(entering, Crossing::Rising, a.to(&b)?));
        let leaving = ns.kernel(|mut k| {
            let (zero, value, threshold) = (k.zero(), k.state(&x), k.parameter(&threshold));
            let negative = k.subtract(zero, value);
            let gap = k.subtract(negative, threshold);
            k.finish(gap, b.only(), class)
        });
        ns.event(Guard::new(leaving, Crossing::Rising, b.to(&a)?));
        assembly.add(ns.finish());
        let (mut bd, ports) = assembly.boundary()?.parts();
        let route = ports.route::<Energy>("heat")?.route;
        let reading = bd.algebraic::<Energy>("reading", Admissible::finite());
        let level = bd.parameter::<Energy>(ParameterKey::new("level"), Admissible::finite());
        let speed =
            bd.parameter::<Rate<Dimensionless>>(ParameterKey::new("heat speed"), Admissible::positive());
        let rate = bd.kernel(|mut k| {
            let (amount, level, speed) = (
                k.read(&route.amount::<Energy>()),
                k.parameter(&level),
                k.parameter(&speed),
            );
            let rate = k.relax(amount, level, speed);
            k.finish(rate, bd.every_mode(), class)
        });
        bd.transfer(Transfer::new(&route, rate));
        let residual = bd.kernel(|mut k| {
            let (read, amount) = (k.algebraic(&reading), k.read(&route.amount::<Energy>()));
            let residual = k.subtract(read, amount);
            k.finish(residual, bd.every_mode(), class)
        });
        bd.residual(&reading, residual);
        assembly.add(bd.finish());
        assembly.compile()
    })
}

#[derive(Clone, Copy)]
struct Start {
    moment_ns: u64,
    through_b: bool,
    start_up: bool,
    fit_up: bool,
    history: &'static [u8],
}
const BASE: Start = Start {
    moment_ns: 0,
    through_b: false,
    start_up: false,
    fit_up: false,
    history: b"one history",
};

fn admitted(body: &CompiledBody, start: Start) -> Result<InitialRevision, InitializationFailure> {
    let period = NonZeroU64::new(2_000_000_000).expect("not zero");
    let cycle = Cycle::new(period, NonZeroU32::new(2).expect("not zero"))?;
    let mut packet = ParameterPacket::periodic(cycle);
    packet.within(ErrorScales::uniform(measure(1e-12), measure(1e-9))?);
    let threshold = measure::<Dimensionless>(1.0);
    packet.synthetic(
        ParameterKey::new("threshold"),
        Binding::Body,
        threshold,
        "synthetic",
    )?;
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(ParameterKey::new("speed"), Binding::Body, speed, "synthetic")?;
    let heat_speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(
        ParameterKey::new("heat speed"),
        Binding::Body,
        heat_speed,
        "synthetic",
    )?;
    let anchor = AnchorId::new("fixture.heat");
    let read = Snapshot::new(
        reading(),
        "gauge",
        AnchorStatistic::CycleMean,
        measure::<Energy>(0.5),
    );
    packet.anchor(Synthetic::new(anchor, read, "synthetic"))?;
    let guess = measure::<Energy>(0.5);
    packet.calibrated(ParameterKey::new("level"), Binding::Body, guess, anchor)?;
    let rest = |kind| {
        if matches!(kind, RowKind::Memory(_)) {
            0.0
        } else {
            0.5
        }
    };
    packet.start_rest(body, rest, "synthetic")?;
    let law = LawName {
        owner: "NS",
        local: "switch",
    };
    packet.start_mode(law, "a", "synthetic")?;
    let lens = body.lens(
        vec![ChannelSpec::new("gauge", reading()).averaged(NonZeroU64::new(2).expect("not zero"))],
        NonZeroU64::new(1_000_000_000).expect("not zero"),
    )?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(start.moment_ns)).reading(&lens);
    let mut system = body.initialization(body.bind(packet)?, port, request)?;
    let mut workspace = system.workspace();
    if start.through_b {
        let rest = workspace.unknowns_mut().to_vec();
        workspace.unknowns_mut().fill(2.0);
        assert!(system.sweep(&workspace, 0..2)? > 0);
        workspace.unknowns_mut().copy_from_slice(&rest);
    }
    assert_eq!(system.sweep(&workspace, 0..2)?, 0);
    let (points, fits) = workspace.unknowns_mut().split_at_mut(2 * body.rows().len());
    let up = |value: &mut f64| *value = f64::from_bits(value.to_bits() + 1);
    if start.start_up {
        points.iter_mut().for_each(up);
    }
    if start.fit_up {
        fits.iter_mut().for_each(up);
    }
    let history = Digest::of_canonical(start.history);
    let history = MethodHistory::new(history, SimTime::from_ns(0), Vec::new());
    system.admit(workspace, history)
}

fn moved(body: &CompiledBody, start: Start) -> Result<Vec<&'static str>, InitializationFailure> {
    let records = |start| -> Result<Vec<_>, InitializationFailure> {
        let revision = admitted(body, start)?;
        let values = revision.provenance().values().iter();
        Ok(values
            .map(|record| (record.name, record.provenance, record.value.bits()))
            .collect())
    };
    let (base, changed) = (records(BASE)?, records(start)?);
    assert_eq!(base.len(), changed.len());
    let mut kinds: Vec<_> = (base.iter().zip(&changed))
        .filter(|(a, b)| a != b)
        .map(|((name, _, _), _)| match name {
            ValueName::Parameter { .. } => "fit",
            ValueName::Start(_) => "start",
            ValueName::Mode { .. } => "mode",
        })
        .collect();
    kinds.dedup();
    Ok(kinds)
}

#[test]
fn each_part_of_an_admitted_start_moves_the_hash_a_run_names() -> Result<(), InitializationFailure> {
    let body = body()?;
    let hash = |start| admitted(&body, start).map(|revision| revision.packet_hash());
    assert_eq!(hash(BASE)?, hash(BASE)?);
    let parts = [
        (
            "moment",
            vec![],
            Start {
                moment_ns: 2_000_000_000,
                ..BASE
            },
        ),
        (
            "start",
            vec!["start"],
            Start {
                start_up: true,
                ..BASE
            },
        ),
        (
            "modes",
            vec!["mode"],
            Start {
                through_b: true,
                ..BASE
            },
        ),
        ("fits", vec!["fit"], Start { fit_up: true, ..BASE }),
        (
            "history",
            vec![],
            Start {
                history: b"another history",
                ..BASE
            },
        ),
    ];
    for (part, kinds, start) in parts {
        assert_eq!(moved(&body, start)?, kinds, "{part} moves only itself");
        assert_ne!(
            hash(BASE)?,
            hash(start)?,
            "{part} is outside the hash a run names"
        );
    }
    Ok(())
}

/// The packet supplied `a`, so a `b` the ring reached was solved, and an `a` it kept came from the fixture.
#[test]
fn a_mode_the_ring_moved_is_reported_as_solved() -> Result<(), InitializationFailure> {
    let body = body()?;
    let provenance_of = |through_b| -> Result<ValueProvenance, InitializationFailure> {
        let revision = admitted(&body, Start { through_b, ..BASE })?;
        let record = (revision.provenance().values().iter())
            .find(|record| matches!(record.name, ValueName::Mode { .. }))
            .expect("a mode's record");
        Ok(record.provenance)
    };
    assert_eq!(provenance_of(false)?, ValueProvenance::Fixture);
    assert_eq!(provenance_of(true)?, ValueProvenance::Solved);
    Ok(())
}
