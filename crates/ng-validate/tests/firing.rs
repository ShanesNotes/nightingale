//! What a firing does and what admission holds a ring to. Each bench is the smallest body that shows one rule, and the
//! interior crossing and the two free levels are the design review's own counterexamples.

use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn one_site() -> Schema {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    Schema::new(map.finish().expect("a map")).expect("a schema")
}

fn wave() -> RateClass {
    RateClass::new(RateBand::Wave, 3_600_000_000_000).expect("a class")
}

fn lens(body: &CompiledBody, kind: RowKind) -> ObservationLens {
    let channel = ChannelSpec::new("a synthetic gauge", kind);
    let every = NonZeroU64::new(SECOND).expect("not zero");
    body.lens(vec![channel], every).expect("a lens")
}

fn cycle(points: u32) -> ParameterPacket<SyntheticFixture> {
    let period = NonZeroU64::new(SECOND).expect("not zero");
    let points = NonZeroU32::new(points).expect("not zero");
    ParameterPacket::periodic(Cycle::new(period, points).expect("a cycle"))
}

fn named(local: &'static str) -> LawName {
    LawName { owner: "NS", local }
}

/// `x` climbs from -1 at 2 a second, so `g = 1/4 - x x` crosses zero at 1/4 s and back at 3/4 s. A step of 1 s reads
/// `g < 0` at both its ends.
fn crossing_inside() -> CompiledBody {
    assemble(one_site(), |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let x = laws.state::<Dimensionless>("x", Admissible::finite());
        let slope = laws.parameter(ParameterKey::new("ns.slope"), Admissible::finite());
        let climb = laws.kernel(|mut k| {
            let value = k.parameter(&slope);
            k.finish(value, laws.every_mode(), wave())
        });
        laws.memory_rate(&x, climb);
        let [before, after] = laws.modes("state", ["before", "after"]);
        let quarter = laws.parameter::<Dimensionless>(ParameterKey::new("ns.quarter"), Admissible::finite());
        let zero = laws.kernel(|mut k| {
            let (one, other) = (k.state(&x), k.state(&x));
            let square = k.scale(one, other);
            let rest = k.parameter(&quarter);
            let gap = k.subtract(rest, square);
            k.finish(gap, before.only(), wave())
        });
        laws.event(Guard::new(zero, Crossing::Rising, before.to(&after)?));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

#[test]
fn a_guard_crossed_and_crossed_back_inside_a_step_refuses_it_and_lands() {
    let body = crossing_inside();
    let mut packet = ParameterPacket::new();
    for (key, value) in [("ns.slope", 2.0), ("ns.quarter", 0.25)] {
        if key == "ns.slope" {
            packet
                .synthetic(
                    ParameterKey::new(key),
                    Binding::Body,
                    measure::<Rate<Dimensionless>>(value),
                    "synthetic",
                )
                .expect("once");
        } else {
            packet
                .synthetic(
                    ParameterKey::new(key),
                    Binding::Body,
                    measure::<Dimensionless>(value),
                    "synthetic",
                )
                .expect("once");
        }
    }
    let x = RowKind::Memory(named("x").into());
    packet
        .start(x, measure::<Dimensionless>(-1.0), "synthetic")
        .expect("once");
    packet
        .start_mode(named("state"), "before", "synthetic")
        .expect("once");
    let core = ReferenceIntegrator::new(SECOND).with_floor(SECOND / 1024);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let session = admit_fixture(&body, packet, port, &core).expect("admitted");
    let Ok(session) = run_until(session, SimTime::from_ns(SECOND)) else {
        panic!("the bench faulted");
    };
    let view = session.record().accepted();
    assert_eq!(
        view.modes(),
        &[0],
        "values sort by name, so `after`, and the mode moved at the first root"
    );
    let segments: Vec<_> = session.record().conservation().segments().collect();
    let landed = segments[0].interval().end().nanoseconds();
    assert!(landed.abs_diff(SECOND / 4) <= SECOND / 512, "landed at {landed}");
}

/// Two guards on one variable, with `m` over both thresholds: the first in stable order wins, in the ring as in a run.
fn two_guards() -> CompiledBody {
    assemble(one_site(), |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let m = laws.state::<Dimensionless>("m", Admissible::finite());
        let y = laws.state::<Dimensionless>("y", Admissible::finite());
        let [low, first, second] = laws.modes("state", ["low", "first", "second"]);
        let fast = laws.parameter(ParameterKey::new("ns.fast"), Admissible::finite());
        let slow = laws.parameter(ParameterKey::new("ns.slow"), Admissible::finite());
        let rising = laws.kernel(|mut k| {
            let value = k.parameter(&fast);
            k.finish(value, laws.every_mode(), wave())
        });
        laws.memory_rate(&m, rising);
        for (mode, speed) in [(&low, &fast), (&first, &fast), (&second, &slow)] {
            let rate = laws.kernel(|mut k| {
                let value = k.parameter(speed);
                k.finish(value, mode.only(), wave())
            });
            laws.memory_rate(&y, rate);
        }
        for target in [&first, &second] {
            let zero = laws.kernel(|mut k| {
                let value = k.state(&m);
                k.finish(value, low.only(), wave())
            });
            laws.event(Guard::new(zero, Crossing::Rising, low.to(target)?));
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn guarded_packet(
    mut packet: ParameterPacket<SyntheticFixture>,
    start: f64,
) -> ParameterPacket<SyntheticFixture> {
    for (key, value) in [("ns.fast", 1.0), ("ns.slow", 3.0)] {
        packet
            .synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Rate<Dimensionless>>(value),
                "synthetic",
            )
            .expect("once");
    }
    for (local, value) in [("m", start), ("y", 0.0)] {
        let row = RowKind::Memory(named(local).into());
        packet
            .start(row, measure::<Dimensionless>(value), "synthetic")
            .expect("once");
    }
    packet
        .start_mode(named("state"), "low", "synthetic")
        .expect("once");
    packet
}

#[test]
fn of_two_guards_on_one_variable_the_same_one_wins_on_the_ring_and_in_a_run() {
    let body = two_guards();
    let core = ReferenceIntegrator::new(SECOND);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let session =
        admit_fixture(&body, guarded_packet(ParameterPacket::new(), -0.5), port, &core).expect("admitted");
    let Ok(session) = run_until(session, SimTime::from_ns(1)) else {
        panic!("the bench faulted");
    };
    // Values sort by name: `first`, `low`, `second`.
    let winner = session.record().accepted().modes()[0];
    assert_ne!(winner, 1, "a guard fired");

    let mut packet = guarded_packet(cycle(2), 1.0);
    packet.within(admission_tolerance());
    let parameters = body.bind(packet).expect("bound");
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let y = RowKind::Memory(named("y").into());
    let gauge = lens(&body, y);
    let request = InitializationRequest::at(SimTime::from_ns(0)).reading(&gauge);
    let mut system = body.initialization(parameters, port, request).expect("a system");
    let workspace = system.workspace();
    assert_eq!(system.sweep(&workspace, 0..1), Ok(1));
    let (residuals, _) = system.stage(&workspace, 1).expect("a stage");
    let row = (body.rows().iter())
        .position(|row| row.kind() == y)
        .expect("a row");
    let half_a_second_of = if winner == 0 { 1.0 } else { 3.0 };
    assert_eq!(residuals[row], -0.5 * half_a_second_of);
}

const HEAT: &str = "heat";

/// One store with one way out, whose law reads the store and has no slope in it: `speed (x - x) + loss`. Syntax calls
/// that regulated, and nothing regulates it.
fn seeming_regulation() -> CompiledBody {
    let mut schema = one_site();
    let site = AnatomyBuilder::new().site::<TissueSite>("body", origin()).id();
    schema.pool::<Energy>(site, Space::Thermal).expect("a pool");
    schema
        .boundary::<Energy>(HEAT, BoundaryDirection::OutOfBody, site, Space::Thermal)
        .expect("a route");
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let route = ports.route::<Energy>(HEAT)?.route;
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.speed"), Admissible::finite());
        let loss = laws.parameter::<Rate<Energy>>(ParameterKey::new("bd.loss"), Admissible::finite());
        let rate = laws.kernel(|mut k| {
            let (one, other) = (
                k.read(&route.amount::<Energy>()),
                k.read(&route.amount::<Energy>()),
            );
            let pace = k.parameter(&speed);
            let nothing = k.relax(one, other, pace);
            let leak = k.parameter(&loss);
            let total = k.add(nothing, leak);
            k.finish(total, laws.every_mode(), wave())
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn admitted_store(loss: f64) -> Option<InitializationFailure> {
    let body = seeming_regulation();
    let mut packet = cycle(100);
    let scale = |value| measure::<Dimensionless>(value);
    packet.within(ErrorScales::uniform(scale(1e-3), scale(1e-12)).expect("positive"));
    let bind = Binding::Body;
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet
        .synthetic(ParameterKey::new("bd.speed"), bind, speed, "synthetic")
        .expect("once");
    packet
        .synthetic(
            ParameterKey::new("bd.loss"),
            bind,
            measure::<Rate<Energy>>(loss),
            "synthetic",
        )
        .expect("once");
    let store = RowKind::Amount {
        site: AnatomyBuilder::new().site::<TissueSite>("body", origin()).id(),
        space: Space::Thermal,
        species: Species::Energy,
    };
    packet
        .start(store, measure::<Energy>(1.0), "synthetic")
        .expect("once");
    let parameters = body.bind(packet).expect("bound");
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let gauge = lens(&body, store);
    let request = InitializationRequest::at(SimTime::from_ns(0)).reading(&gauge);
    let system = body.initialization(parameters, port, request).expect("a system");
    let mut proposed = system.workspace();
    proposed.unknowns_mut().fill(7.0);
    let history = MethodHistory::new(Digest::of_canonical(b"a test"), SimTime::from_ns(0), Vec::new());
    system.admit(proposed, history).err()
}

#[test]
fn a_total_nothing_regulates_has_no_rank_on_the_ring() {
    assert!(matches!(
        admitted_store(0.0),
        Some(InitializationFailure::NoRank { .. })
    ));
}

#[test]
fn a_loss_each_stage_hides_is_a_net_flow_the_cycle_shows() {
    assert!(matches!(
        admitted_store(0.09),
        Some(InitializationFailure::NetBoundaryFlow { .. })
    ));
}

/// `p` sits past its guard at every point, so the mode the packet states cannot be the one that follows.
#[test]
fn a_mode_the_guards_would_have_moved_is_refused_at_its_point() {
    let body = assemble(one_site(), |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let p = laws.state::<Dimensionless>("p", Admissible::finite());
        let none = laws.kernel(|mut k| {
            let zero = k.zero();
            k.finish(zero, laws.every_mode(), wave())
        });
        laws.memory_rate(&p, none);
        let [before, after] = laws.modes("state", ["before", "after"]);
        let zero = laws.kernel(|mut k| {
            let value = k.state(&p);
            k.finish(value, before.only(), wave())
        });
        laws.event(Guard::new(zero, Crossing::Rising, before.to(&after)?));
        let back = laws.kernel(|mut k| {
            let zero = k.zero();
            let value = k.state(&p);
            let off = k.subtract(zero, value);
            k.finish(off, after.only(), wave())
        });
        laws.event(Guard::new(back, Crossing::Rising, after.to(&before)?));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = cycle(4);
    packet.within(admission_tolerance());
    let row = RowKind::Memory(named("p").into());
    packet
        .start(row, measure::<Dimensionless>(1.0), "synthetic")
        .expect("once");
    packet
        .start_mode(named("state"), "before", "synthetic")
        .expect("once");
    let parameters = body.bind(packet).expect("bound");
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let gauge = lens(&body, row);
    let request = InitializationRequest::at(SimTime::from_ns(0)).reading(&gauge);
    let system = body.initialization(parameters, port, request).expect("a system");
    let workspace = system.workspace();
    let history = MethodHistory::new(Digest::of_canonical(b"a test"), SimTime::from_ns(0), Vec::new());
    assert!(matches!(
        system.admit(workspace, history).err(),
        Some(InitializationFailure::GuardSweep { .. })
    ));
}

#[test]
fn a_trunk_past_straight_up_is_refused() {
    let past = Posture::new(measure(1.5), Legs::Standing).err();
    assert_eq!(past, Some(AdmissionError::PostureOutOfRange));
    assert!(Posture::new(measure(-1.0), Legs::Horizontal).is_ok());
}

#[test]
fn a_delivery_only_one_modes_law_reads_is_refused() {
    let tank = AnatomyBuilder::new().site::<VesselSegment>("tank", origin()).id();
    const INTAKE: &str = "intake";
    let body_of = |both: bool| {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", origin());
        let vessel = map.site::<VesselSegment>("tank", origin());
        map.contain(root.id(), vessel.id());
        map.connect(Network::Vascular, vessel.id(), vessel.id());
        let mut schema = Schema::new(map.finish().expect("a map")).expect("a schema");
        schema.partial_volume(measure(1.0)).expect("once");
        schema
            .boundary::<Water>(INTAKE, BoundaryDirection::IntoBody, vessel.id(), Space::Lumen)
            .expect("a route");
        assemble(schema, |mut body| {
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut cv);
            let still = cv.edge_kernel(|mut k| {
                let zero = k.zero::<Rate<Water>>();
                k.finish(zero, cv.every_mode(), wave())
            });
            cv.transfer_along(&route, still);
            body.add(cv.finish());
            let (mut laws, ports) = body.boundary()?.parts();
            let BoundaryRoute { route, drive } = ports.route::<Water>(INTAKE)?;
            let m = laws.state::<Dimensionless>("m", Admissible::finite());
            let none = laws.kernel(|mut k| {
                let zero = k.zero();
                k.finish(zero, laws.every_mode(), wave())
            });
            laws.memory_rate(&m, none);
            let [a, b] = laws.modes("state", ["a", "b"]);
            let reading = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, a.only(), wave())
            });
            laws.transfer(Transfer::new(&route, reading));
            let other = laws.kernel(|mut k| {
                let value = if both { k.read(&drive) } else { k.zero() };
                k.finish(value, b.only(), wave())
            });
            laws.transfer(Transfer::new(&route, other));
            for (from, to) in [(&a, &b), (&b, &a)] {
                let zero = laws.kernel(|mut k| {
                    let value = k.state(&m);
                    k.finish(value, from.only(), wave())
                });
                laws.event(Guard::new(zero, Crossing::Rising, from.to(to)?));
            }
            body.add(laws.finish());
            body.compile()
        })
        .expect("compiled")
    };
    let delivered = |body: &CompiledBody| {
        let mut packet = ParameterPacket::new();
        let store = RowKind::Amount {
            site: tank,
            space: Space::Lumen,
            species: Species::Water,
        };
        packet
            .start(store, measure::<Water>(1.0), "synthetic")
            .expect("once");
        let owner = "Boundary";
        let m = RowKind::Memory(LawName { owner, local: "m" }.into());
        packet
            .start(m, measure::<Dimensionless>(-1.0), "synthetic")
            .expect("once");
        let state = LawName {
            owner,
            local: "state",
        };
        packet.start_mode(state, "a", "synthetic").expect("once");
        let delivery = WaterDelivery {
            boundary: INTAKE,
            amount: measure(1.0),
            ends: SimTime::from_ns(SECOND),
        };
        let id = InputId::new(Digest::of_canonical(b"a delivery"));
        let event = BoundaryEvent::new(id, SimTime::from_ns(0), BoundaryInput::Water(delivery));
        let port = BoundaryPort::new(vec![event]).expect("parsed");
        let core = ReferenceIntegrator::new(SECOND);
        admit_fixture(body, packet, port, &core).err()
    };
    let unread = Some(InitializationFailure::Admission(AdmissionError::DriveNotRead {
        name: INTAKE,
    }));
    assert_eq!(delivered(&body_of(false)), unread);
    assert_eq!(delivered(&body_of(true)), None);
}

#[test]
fn no_flow_out_of_a_dry_pool_carries_nothing() {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let (a, b) = (
        map.site::<VesselSegment>("a", origin()),
        map.site::<VesselSegment>("b", origin()),
    );
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    map.connect(Network::Vascular, b.id(), a.id());
    let mut schema = Schema::new(map.finish().expect("a map")).expect("a schema");
    schema.partial_volume(measure(1.0)).expect("once");
    schema.carry::<Sodium, Blood>(Space::Lumen);
    let body = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), wave())
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());
        body.compile()
    })
    .expect("compiled");
    let dry = a.id();
    let mut packet = ParameterPacket::new();
    packet
        .start_rest(
            &body,
            |row| match row {
                RowKind::Amount { site, .. } if site == dry => 0.0,
                RowKind::Amount { .. } => 1.0,
                _ => 0.0,
            },
            "synthetic",
        )
        .expect("every row");
    let core = ReferenceIntegrator::new(SECOND);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let session = admit_fixture(&body, packet, port, &core).expect("admitted");
    assert!(run_until(session, SimTime::from_ns(SECOND)).is_ok());
}
