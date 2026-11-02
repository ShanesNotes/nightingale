use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

/// `x` decays to zero in either mode, twice as fast in `b`, so a body at rest is a start in both.
fn body() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let origin = Position::new(measure(0.0), measure(0.0), measure(0.0));
    map.site::<TissueSite>("body", origin);
    assemble(Schema::new(map.finish()?)?, |mut assembly| {
        let (mut ns, _) = assembly.nervous()?.parts();
        let x = ns.state::<Dimensionless>("x", Admissible::finite());
        let threshold = ns.parameter::<Dimensionless>(ParameterKey::new("threshold"), Admissible::positive());
        let speed = ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("speed"), Admissible::positive());
        let [a, b] = ns.modes("switch", ["a", "b"]);
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
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
        assembly.compile()
    })
}

/// The packet states `a`. A ring that sweeps through `b` and comes back to rest is admitted in `b`.
fn admitted(body: &CompiledBody, through_b: bool) -> Result<InitialRevision, InitializationFailure> {
    let period = NonZeroU64::new(2_000_000_000).expect("not zero");
    let cycle = Cycle::new(period, NonZeroU32::new(2).expect("not zero"))?;
    let mut packet = ParameterPacket::periodic(cycle);
    let threshold = measure::<Dimensionless>(1.0);
    packet.synthetic(
        ParameterKey::new("threshold"),
        Binding::Body,
        threshold,
        "synthetic",
    )?;
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(ParameterKey::new("speed"), Binding::Body, speed, "synthetic")?;
    packet.start_rest(body, |_| 0.0, "synthetic")?;
    let law = LawName {
        owner: "NS",
        local: "switch",
    };
    packet.start_mode(law, "a", "synthetic")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let mut system = body.initialization(body.bind(packet)?, port, request)?;
    let mut workspace = system.workspace();
    if through_b {
        workspace.unknowns_mut().fill(2.0);
        assert!(system.sweep(&workspace, 0..2)? > 0);
        workspace.unknowns_mut().fill(0.0);
    }
    assert_eq!(system.sweep(&workspace, 0..2)?, 0);
    let history = Digest::of_canonical(b"one history");
    let start = MethodHistory::new(history, SimTime::from_ns(0), Vec::new());
    system.admit(workspace, start)
}

/// Two starts in different modes are two starts, whatever else they share, so they cannot name one identity.
#[test]
fn the_modes_a_ring_reached_are_in_the_hash_a_run_names() -> Result<(), InitializationFailure> {
    let body = body()?;
    let stayed = admitted(&body, false)?.packet_hash();
    assert_eq!(stayed, admitted(&body, false)?.packet_hash());
    assert_ne!(stayed, admitted(&body, true)?.packet_hash());
    Ok(())
}

/// The packet supplied `a`, so a `b` the ring reached was solved, and an `a` it kept came from the fixture.
#[test]
fn a_mode_the_ring_moved_is_reported_as_solved() -> Result<(), InitializationFailure> {
    let body = body()?;
    let provenance_of = |through_b| -> Result<ValueProvenance, InitializationFailure> {
        let revision = admitted(&body, through_b)?;
        let record = (revision.provenance().values().iter())
            .find(|record| matches!(record.name, ValueName::Mode { .. }))
            .expect("a mode's record");
        Ok(record.provenance)
    };
    assert_eq!(provenance_of(false)?, ValueProvenance::Fixture);
    assert_eq!(provenance_of(true)?, ValueProvenance::Solved);
    Ok(())
}
