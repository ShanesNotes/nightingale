//! Synthetic guards whose crossings are visible to the declared step's stages.

use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite synthetic value")
}

#[test]
fn a_guard_at_the_end_of_a_step_hides_no_earlier_crossing_at_a_stage() {
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
    let core = ReferenceIntegrator::new(SECOND).with_floor(SECOND / 1000);
    let port = BoundaryPort::new(Vec::new()).expect("empty timeline");
    let session = admit_fixture(&body, packet, port, &core).expect("admitted");
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
