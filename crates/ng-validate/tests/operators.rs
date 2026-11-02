use core::fmt::Write;
use core::num::{NonZeroU32, NonZeroU64};
use expect_test::expect;
use ng_contract::*;
use ng_validate::*;
use std::collections::BTreeSet;

const SECOND: u64 = 1_000_000_000;
const ELASTANCE: f64 = 1.3;
const PARTIAL_VOLUME: f64 = 0.8;
const OPERATORS: [&str; 18] = [
    "Input",
    "Zero",
    "Add",
    "Subtract",
    "Multiply",
    "Divide",
    "AsDimension",
    "Scale",
    "SmoothMax",
    "SmoothMin",
    "Exp",
    "Logistic",
    "Ln",
    "Deviation",
    "Relax",
    "ConductanceFlow",
    "Volume",
    "Upwind",
];

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn memory(local: &'static str) -> RowKind {
    RowKind::Memory(LawName { owner: "NS", local }.into())
}

fn algebraic(local: &'static str) -> RowKind {
    RowKind::Algebraic(LawName { owner: "NS", local }.into())
}

fn operators(body: &CompiledBody) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for program in body.programs() {
        let listing = format!("{program:?}");
        let start = listing.find("ops: [").expect("a program lists its operations") + "ops: [".len();
        let (mut depth, mut name) = (0, String::new());
        for c in listing[start..].chars() {
            match c {
                '(' | '[' if depth == 0 => {
                    names.insert(core::mem::take(&mut name));
                    depth += 1;
                }
                '(' | '[' => depth += 1,
                ')' | ']' if depth == 0 => break,
                ')' | ']' => depth -= 1,
                c if depth == 0 && c.is_alphanumeric() => name.push(c),
                _ => {}
            }
        }
    }
    names
}

fn every_operator() -> Result<(CompiledBody, SiteId), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(measure(PARTIAL_VOLUME))?;
    schema.carry::<Albumin, Blood>(Space::Lumen);
    let body = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let e = cv.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g =
            cv.edge_parameter::<_, Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let flow = cv.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let upstream_elastance = k.upstream_parameter(&e);
            let downstream_elastance = k.downstream_parameter(&e);
            let upstream = k.multiply(upstream_elastance, upstream);
            let downstream = k.multiply(downstream_elastance, downstream);
            let upstream = k.as_dimension::<_, Pressure>(upstream);
            let downstream = k.as_dimension::<_, Pressure>(downstream);
            let head = k.subtract(upstream, downstream);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, cv.every_mode(), wave)
        });
        cv.transfer_along(&route, flow);
        body.add(cv.finish());

        let (mut ns, _) = body.nervous()?.parts();
        let first = ns.state::<Dimensionless>("first", Admissible::finite());
        let second = ns.state::<Dimensionless>("second", Admissible::finite());
        let third = ns.algebraic::<Pressure>("third", Admissible::finite());
        let pace = ns.algebraic::<Rate<Dimensionless>>("pace", Admissible::finite());
        let set_point = ns.set_point::<Dimensionless>("set point", Admissible::finite());
        let speed =
            ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
        let reach = ns.parameter::<Pressure>(ParameterKey::new("ns.reach"), Admissible::positive());
        let deviation_width = ns.parameter::<Deviation<Dimensionless>>(
            ParameterKey::new("ns.deviation-width"),
            Admissible::positive(),
        );
        let numerator = ns.parameter::<Pressure>(ParameterKey::new("ns.numerator"), Admissible::positive());
        let width = ns.parameter::<Dimensionless>(ParameterKey::new("ns.width"), Admissible::positive());
        let first_rate = ns.kernel(|mut k| {
            let (x, y, w) = (k.state(&first), k.state(&second), k.parameter(&width));
            let high = k.smooth_max(x, y, w);
            let (x, y, w) = (k.state(&first), k.state(&second), k.parameter(&width));
            let low = k.smooth_min(x, y, w);
            let sum = k.add(high, low);
            let tilted = k.add(sum, high);
            let exponential = k.exp(tilted);
            let y = k.state(&second);
            let logistic = k.logistic(y);
            let target = k.scale(exponential, logistic);
            let x = k.state(&first);
            let rate = k.algebraic(&pace);
            let rate = k.relax(target, x, rate);
            k.finish(rate, ns.every_mode(), wave)
        });
        ns.memory_rate(&first, first_rate);
        let second_rate = ns.kernel(|mut k| {
            let (p, r) = (k.algebraic(&third), k.parameter(&numerator));
            let ratio = k.divide(r, p);
            let ratio = k.as_dimension::<_, Dimensionless>(ratio);
            let log = k.ln(ratio);
            let (measured, reference) = (k.state(&first), k.state(&set_point));
            let error = k.deviation(measured, reference);
            let width = k.parameter(&deviation_width);
            let ratio = k.divide(error, width);
            let error: Expr<'_, '_, _, Dimensionless> = k.as_dimension(ratio);
            let target = k.subtract(log, error);
            let (y, rate) = (k.state(&second), k.parameter(&speed));
            let rate = k.relax(target, y, rate);
            k.finish(rate, ns.every_mode(), wave)
        });
        ns.memory_rate(&second, second_rate);
        let third_residual = ns.kernel(|mut k| {
            let (p, target) = (k.algebraic(&third), k.parameter(&reach));
            let residual = k.subtract(p, target);
            k.finish(residual, ns.every_mode(), wave)
        });
        ns.residual(&third, third_residual);
        let pace_residual = ns.kernel(|mut k| {
            let (now, declared) = (k.algebraic(&pace), k.parameter(&speed));
            let residual = k.subtract(now, declared);
            k.finish(residual, ns.every_mode(), wave)
        });
        ns.residual(&pace, pace_residual);
        let set_point_rate = ns.kernel(|mut k| {
            let zero = k.zero::<Rate<Reference<Dimensionless>>>();
            k.finish(zero, ns.every_mode(), wave)
        });
        ns.memory_rate(&set_point, set_point_rate);
        body.add(ns.finish());
        body.compile()
    })?;
    Ok((body, a.id()))
}

fn packet(
    body: &CompiledBody,
    a: SiteId,
    cycle: Option<Cycle>,
) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = cycle.map_or_else(ParameterPacket::new, ParameterPacket::periodic);
    let vascular = Binding::Network(Network::Vascular);
    packet.synthetic(
        ParameterKey::new("cv.e"),
        vascular,
        measure::<Elastance>(ELASTANCE),
        "synthetic",
    )?;
    let g = measure::<Conductance<Water>>(0.25);
    packet.synthetic(ParameterKey::new("cv.g"), vascular, g, "synthetic")?;
    for (key, value) in [("ns.reach", 1.5), ("ns.numerator", 2.0)] {
        packet.synthetic(
            ParameterKey::new(key),
            Binding::Body,
            measure::<Pressure>(value),
            "synthetic",
        )?;
    }
    let speed = measure::<Rate<Dimensionless>>(0.7);
    packet.synthetic(ParameterKey::new("ns.speed"), Binding::Body, speed, "synthetic")?;
    let width = measure::<Dimensionless>(0.3);
    packet.synthetic(ParameterKey::new("ns.width"), Binding::Body, width, "synthetic")?;
    let width = measure::<Deviation<Dimensionless>>(0.3);
    packet.synthetic(
        ParameterKey::new("ns.deviation-width"),
        Binding::Body,
        width,
        "synthetic",
    )?;
    let starts = [
        (memory("first"), 0.4),
        (memory("second"), -0.2),
        (memory("set point"), 0.1),
        (algebraic("third"), 1.5),
        (algebraic("pace"), 0.7),
    ];
    packet.start_rest(
        body,
        |kind| match kind {
            RowKind::Amount {
                site,
                species: Species::Water,
                ..
            } => [1.0, 4.0][usize::from(site == a)],
            RowKind::Amount { site, .. } => [0.5, 2.0][usize::from(site == a)],
            RowKind::Algebraic(Slot {
                law: LawName { owner: "CV", .. },
                ..
            }) => 0.25 * (ELASTANCE * (4.0 * PARTIAL_VOLUME) - ELASTANCE * (1.0 * PARTIAL_VOLUME)),
            kind => {
                let start = starts.iter().find(|(row, _)| *row == kind);
                start.unwrap_or_else(|| panic!("no start for {kind:?}")).1
            }
        },
        "synthetic",
    )?;
    Ok(packet)
}

fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn the_pinned_body_runs_every_operator_and_every_one_the_slice_runs() {
    let ran = operators(&every_operator().expect("compiled").0);
    assert_eq!(ran, OPERATORS.map(String::from).into());
    assert!(operators(&arena_body().expect("compiled")).is_subset(&ran));
}

#[test]
fn every_operators_value_stays_on_the_accepted_bits() {
    let (body, a) = every_operator().expect("compiled");
    let core = ReferenceIntegrator::new(SECOND / 2);
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let session =
        admit_fixture(&body, packet(&body, a, None).expect("a packet"), port, &core).expect("admitted");
    let Ok(session) = run_until(session, SimTime::from_ns(4 * SECOND)) else {
        panic!("the bench faulted");
    };
    let record = session.record();
    let view = record.accepted();
    let mut trace = String::new();
    writeln!(trace, "identity {}", hex(record.identity().digest())).expect("a string takes it");
    let (revision, time) = (record.revision(), view.time().nanoseconds());
    writeln!(
        trace,
        "revision {revision} at {time} ns, seal {}",
        hex(record.seal().digest())
    )
    .expect("a string takes it");
    for row in body.rows() {
        let [high, low] = view.value(row.id()).bits();
        writeln!(trace, "{high:016x} {low:016x}  {:?}", row.kind()).expect("a string takes it");
    }
    expect![[r#"
        identity 062cd83bbbcf5813381324753ed2b1d64686e6d8b92ae9f2f50288975f07b457
        revision 8 at 4000000000 ns, seal 857253110d24b3237775a8c2be58fc26a6436c5ae753b7ffe4cdec35172f088b
        4005b2ad2c18a5b3 bca0800000000000  Amount { site: SiteId { key: "a", class: VesselSegment }, space: Lumen, species: Water }
        3ff5b2ad2c18a5b3 bc90800000000000  Amount { site: SiteId { key: "a", class: VesselSegment }, space: Lumen, species: Albumin }
        40024d52d3e75a4d 3ca0800000000000  Amount { site: SiteId { key: "b", class: VesselSegment }, space: Lumen, species: Water }
        3ff24d52d3e75a4d 3c90800000000000  Amount { site: SiteId { key: "b", class: VesselSegment }, space: Lumen, species: Albumin }
        3fd7876fa4a4e37b 0000000000000000  Memory(Slot { law: LawName { owner: "NS", local: "first" }, at: Body })
        bfe6f58337f6b021 0000000000000000  Memory(Slot { law: LawName { owner: "NS", local: "second" }, at: Body })
        3fb999999999999a 0000000000000000  Memory(Slot { law: LawName { owner: "NS", local: "set point" }, at: Body })
        3fe6666666666666 0000000000000000  Algebraic(Slot { law: LawName { owner: "NS", local: "pace" }, at: Body })
        3ff8000000000000 0000000000000000  Algebraic(Slot { law: LawName { owner: "NS", local: "third" }, at: Body })
        3fbc410425712b9c 0000000000000000  Algebraic(Slot { law: LawName { owner: "CV", local: "circulation" }, at: Edge { from: SiteId { key: "a", class: VesselSegment }, to: SiteId { key: "b", class: VesselSegment } } })
    "#]].assert_eq(&trace);
}

#[test]
fn every_operators_slope_stays_on_the_bits_of_a_rings_matrix() {
    let (body, a) = every_operator().expect("compiled");
    let two = Cycle::new(
        NonZeroU64::new(SECOND).expect("not zero"),
        NonZeroU32::new(2).expect("not zero"),
    );
    let parameters = body.bind(packet(&body, a, Some(two.expect("a cycle"))).expect("a packet"));
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let ring = body
        .initialization(parameters.expect("bound"), port, request)
        .expect("a ring");
    let matrix = ring.linearize(&ring.workspace()).expect("a matrix");
    let slopes = matrix.view().values();
    let bits: Vec<u8> = slopes
        .iter()
        .flat_map(|slope| slope.to_bits().to_be_bytes())
        .collect();
    let digest = hex(Digest::of_canonical(&bits));
    let trace = format!("ring slopes {digest} over {} entries", slopes.len());
    expect!["ring slopes 82d43c3b9eeac9eef0a809ad877d29d52013ed44e609bb55a4927650c4d14ba3 over 63 entries"]
        .assert_eq(&trace);
}
