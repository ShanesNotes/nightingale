use core::cell::Cell;
use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const ABOVE: usize = 0;
const BELOW: usize = 1;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite synthetic value")
}

#[test]
fn a_guard_at_the_end_of_a_step_hides_no_earlier_crossing_at_a_stage() {
    let store = MemoryStore::default();
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
    let schema = Schema::new(map.finish().expect("map")).expect("schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _) = body.nervous()?.parts();
        let x = laws.state::<Dimensionless>("x", Admissible::finite());
        let slope = laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("slope"), Admissible::finite());
        let radius =
            laws.parameter::<Dimensionless>(ParameterKey::new("radius squared"), Admissible::finite());
        let late = laws.parameter::<Dimensionless>(ParameterKey::new("late threshold"), Admissible::finite());
        let class = RateClass::new(RateBand::Wave, SECOND)?;
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&slope);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&x, rate);
        let [before, early, later] = laws.modes("state", ["before", "early", "later"]);
        let pulse = laws.kernel(|mut k| {
            let (a, b) = (k.state(&x), k.state(&x));
            let square = k.scale(a, b);
            let radius = k.parameter(&radius);
            let gap = k.subtract(radius, square);
            k.finish(gap, before.only(), class)
        });
        laws.event(Guard::new(pulse, Crossing::Rising, before.to(&early)?));
        let endpoint = laws.kernel(|mut k| {
            let (value, threshold) = (k.state(&x), k.parameter(&late));
            let gap = k.subtract(value, threshold);
            k.finish(gap, before.only(), class)
        });
        laws.event(Guard::new(endpoint, Crossing::Rising, before.to(&later)?));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            ParameterKey::new("slope"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(2.0),
            "synthetic",
        )
        .expect("slope");
    for (key, value) in [("radius squared", 0.01), ("late threshold", 0.6)] {
        packet
            .synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Dimensionless>(value),
                "synthetic",
            )
            .expect("parameter");
    }
    let name = |local| LawName { owner: "NS", local };
    packet
        .start(
            RowKind::Memory(name("x").into()),
            measure::<Dimensionless>(-1.0),
            "synthetic",
        )
        .expect("start");
    packet
        .start_mode(name("state"), "before", "synthetic")
        .expect("mode");
    let core = ReferenceIntegrator::new(SECOND).with_floor(NonZeroU64::new(SECOND / 1000).expect("a floor"));
    let port = BoundaryPort::new(Vec::new()).expect("empty timeline");
    let session = admit_fixture(&body, packet, port, &core, &store).expect("admitted");
    let Ok(session) = run_until(session, SimTime::from_ns(1)) else {
        panic!("synthetic run faulted");
    };
    let accepted = session.record().accepted();
    let landed = accepted.time().nanoseconds();
    assert!(
        landed.abs_diff(450_000_000) <= 1_000_000,
        "the crossing seen at the 500 ms stage was skipped"
    );
    assert_eq!(accepted.modes(), &[1], "the early guard must win");
}

struct Counted {
    core: ReferenceIntegrator,
    solves: Cell<u64>,
}
impl CoupledSolver for Counted {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.solves.set(self.solves.get() + 1);
        self.core.solve(problem, ticket)
    }
}

#[derive(Clone, Copy)]
enum Gap {
    Straight(f64),
    /// `1.1 (x + 1)^4 - 0.1`: a straight line from its start to its end at one second puts its root at 0.09 s, and
    /// it crosses at 0.55 s.
    Quartic,
}

struct Landed {
    at_ns: u64,
    modes: Vec<usize>,
    solves: u64,
}

fn first_step(first: Gap, second: Gap) -> Landed {
    let store = MemoryStore::default();
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
    let schema = Schema::new(map.finish().expect("map")).expect("schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _) = body.nervous()?.parts();
        let x = laws.state::<Dimensionless>("x", Admissible::finite());
        let slope = laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("slope"), Admissible::finite());
        let class = RateClass::new(RateBand::Wave, SECOND)?;
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&slope);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&x, rate);
        for (name, gap) in [("first", first), ("second", second)] {
            let [above, below] = laws.modes(name, ["above", "below"]);
            let zero = match gap {
                Gap::Straight(_) => {
                    let threshold = laws.parameter::<Dimensionless>(ParameterKey::new(name), Admissible::finite());
                    laws.kernel(|mut k| {
                        let (value, threshold) = (k.state(&x), k.parameter(&threshold));
                        let gap = k.subtract(value, threshold);
                        k.finish(gap, below.only(), class)
                    })
                }
                Gap::Quartic => {
                    let mut key = |key| laws.parameter::<Dimensionless>(ParameterKey::new(key), Admissible::finite());
                    let (one, coefficient, offset) = (key("one"), key("coefficient"), key("offset"));
                    laws.kernel(|mut k| {
                        let (value, one) = (k.state(&x), k.parameter(&one));
                        let shifted = k.add(value, one);
                        let square = k.scale(shifted, shifted);
                        let fourth = k.scale(square, square);
                        let coefficient = k.parameter(&coefficient);
                        let raised = k.scale(fourth, coefficient);
                        let offset = k.parameter(&offset);
                        let gap = k.subtract(raised, offset);
                        k.finish(gap, below.only(), class)
                    })
                }
            };
            laws.event(Guard::new(zero, Crossing::Rising, below.to(&above)?));
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            ParameterKey::new("slope"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(1.0),
            "synthetic",
        )
        .expect("slope");
    let mut values = Vec::new();
    for (name, gap) in [("first", first), ("second", second)] {
        match gap {
            Gap::Straight(threshold) => values.push((name, threshold)),
            Gap::Quartic => values.extend([("one", 1.0), ("coefficient", 1.1), ("offset", 0.1)]),
        }
    }
    for (key, value) in values {
        packet
            .synthetic(
                ParameterKey::new(key),
                Binding::Body,
                measure::<Dimensionless>(value),
                "synthetic",
            )
            .expect("parameter");
    }
    let name = |local| LawName { owner: "NS", local };
    packet
        .start(
            RowKind::Memory(name("x").into()),
            measure::<Dimensionless>(-1.0),
            "synthetic",
        )
        .expect("start");
    for mode in ["first", "second"] {
        packet.start_mode(name(mode), "below", "synthetic").expect("mode");
    }
    let core = Counted {
        core: ReferenceIntegrator::new(SECOND).with_floor(NonZeroU64::new(SECOND / 1000).expect("a floor")),
        solves: Cell::new(0),
    };
    let port = BoundaryPort::new(Vec::new()).expect("empty timeline");
    let session = admit_fixture(&body, packet, port, &core, &store).expect("admitted");
    core.solves.set(0);
    let Ok(session) = run_until(session, SimTime::from_ns(1)) else {
        panic!("synthetic run faulted");
    };
    let accepted = session.record().accepted();
    Landed {
        at_ns: accepted.time().nanoseconds(),
        modes: accepted.modes().to_vec(),
        solves: core.solves.get(),
    }
}

#[test]
fn a_landing_reads_the_guard_that_crossed_first_not_the_first_in_layout_order() {
    let landed = first_step(Gap::Straight(-0.05), Gap::Straight(-0.75));
    assert!(landed.at_ns.abs_diff(250_000_000) <= 1_000_000, "landed at {} ns", landed.at_ns);
    assert_eq!(landed.modes, [BELOW, ABOVE], "only the second guard fires");
    assert!(landed.solves <= 3, "{} solves: the step and two trials land a straight gap", landed.solves);
}

/// Read first in layout order, the later crossing lands just short of itself and fires only the earlier; read first,
/// the earlier lands past both.
#[test]
fn two_crossings_inside_the_floor_land_where_layout_order_puts_them() {
    let later_first = first_step(Gap::Straight(-0.4997), Gap::Straight(-0.5));
    assert_eq!(
        (later_first.at_ns, later_first.modes),
        (500_299_999, vec![BELOW, ABOVE])
    );
    let earlier_first = first_step(Gap::Straight(-0.5), Gap::Straight(-0.4997));
    assert_eq!(
        (earlier_first.at_ns, earlier_first.modes),
        (500_500_000, vec![ABOVE, ABOVE])
    );
}

#[test]
fn a_curved_gap_that_looked_first_gives_way_to_the_guard_that_crossed_first() {
    let landed = first_step(Gap::Quartic, Gap::Straight(-0.7));
    assert!(landed.at_ns.abs_diff(300_000_000) <= 1_000_000, "landed at {} ns", landed.at_ns);
    assert_eq!(landed.modes, [BELOW, ABOVE], "only the second guard fires");
    assert!(
        landed.solves <= 6,
        "{} solves: the step, three trials on the quartic's line and two on the straight gap",
        landed.solves
    );
}
