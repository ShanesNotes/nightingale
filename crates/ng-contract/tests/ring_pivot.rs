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
