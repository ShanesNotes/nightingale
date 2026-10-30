//! The evaluator through the public seam: hand-derived residuals and Jacobians, and derivatives against
//! finite differences. Every number is synthetic.
use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

const SECOND: u64 = 1_000_000_000;

fn resolution(scheme: StageScheme) -> Result<DeclaredResolution, AdmissionError> {
    let scales = ErrorScales::uniform(Measure::new(1.0)?, Measure::new(0.0)?)?;
    Ok(DeclaredResolution::new(
        Digest::of_canonical(b"evaluator test"),
        1,
        4,
        ArithmeticProfile::contract_v1(),
        scheme,
        scales,
    ))
}

/// Everything a stage needs, built through admission.
struct Bench {
    body: CompiledBody,
    parts: InitialParts,
    identity: RunIdentity,
}
impl Bench {
    fn admitted(
        body: CompiledBody,
        packet: ParameterPacket<SyntheticFixture>,
        boundary: BoundaryPort,
    ) -> Result<Self, InitializationFailure> {
        let parameters = body.bind(packet)?;
        let request = InitializationRequest::consistent_instant(SimTime::from_ns(0));
        let system = body.initialization(parameters, boundary, request)?;
        let solved = system.workspace();
        let schema = Digest::of_canonical(b"evaluator test");
        let history = MethodHistory::new(schema, SimTime::from_ns(0), Vec::new());
        let revision = system.admit(solved, history)?;
        let identity = RunIdentity::new(
            body.model_hash(),
            revision.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution(StageScheme::backward_euler_doubled())?,
        );
        let parts = revision.into_parts();
        Ok(Self {
            body,
            parts,
            identity,
        })
    }
    fn declaring(self, scheme: StageScheme) -> Result<Self, AdmissionError> {
        let Self { body, parts, .. } = self;
        let identity = RunIdentity::new(
            body.model_hash(),
            parts.parameters.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution(scheme)?,
        );
        Ok(Self {
            body,
            parts,
            identity,
        })
    }
    fn problem(&self) -> CoupledProblem<'_> {
        let parts = &self.parts;
        let accepted = AcceptedStateView::new(
            parts.time,
            &parts.state,
            &parts.continuation,
            &parts.dense,
            &parts.pending,
        );
        let base = Digest::of_canonical(b"base");
        CoupledProblem::new(&self.body, &parts.parameters, &self.identity, accepted, base)
    }
}

fn dense(view: SparseView<'_>, rows: usize) -> Vec<Vec<f64>> {
    let mut out = vec![vec![0.0; rows]; rows];
    for (row, cells) in out.iter_mut().enumerate() {
        for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
            cells[view.columns()[entry].index()] = view.values()[entry];
        }
    }
    out
}

fn reservoirs() -> Result<(CompiledBody, SiteId, SiteId), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    let b = map.site::<VesselSegment>("b", at()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.water_boundary("inflow", BoundaryDirection::IntoBody, a.id(), Space::Lumen)?;
    schema.partial_volume(Measure::new(3.0)?)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g =
            laws.edge_parameter::<Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
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
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryWater { route: pair, drive } = ports.water("inflow")?;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&pair, rate));
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((body, a.id(), b.id()))
}

fn lumen(site: SiteId) -> RowKind {
    RowKind::Amount {
        site,
        space: Space::Lumen,
    }
}

fn delivery(
    name: &[u8],
    from: u64,
    to: u64,
    boundary: &'static str,
    amount: f64,
) -> Result<BoundaryEvent, AdmissionError> {
    Ok(BoundaryEvent::new(
        InputId::new(Digest::of_canonical(name)),
        SimTime::from_ns(from),
        BoundaryInput::Water(WaterDelivery {
            boundary,
            amount: Measure::new(amount)?,
            ends: SimTime::from_ns(to),
        }),
    ))
}

fn reservoir_bench(amounts: [f64; 2], events: Vec<BoundaryEvent>) -> Result<Bench, InitializationFailure> {
    let (body, a, b) = reservoirs()?;
    let network = Binding::Network(Network::Vascular);
    let mut packet = ParameterPacket::new();
    packet.synthetic(
        ParameterKey::new("cv.e"),
        network,
        Measure::<Elastance>::new(2.0)?,
        "synthetic",
    )?;
    let g = Measure::<Conductance<Water>>::new(0.5)?;
    packet.synthetic(ParameterKey::new("cv.g"), network, g, "synthetic")?;
    packet.start(lumen(a), Measure::<Water>::new(amounts[0])?, "synthetic")?;
    packet.start(lumen(b), Measure::<Water>::new(amounts[1])?, "synthetic")?;
    Bench::admitted(body, packet, BoundaryPort::new(events)?)
}

#[test]
fn the_two_reservoir_body_has_its_hand_derived_residual_and_jacobian() -> Result<(), InitializationFailure> {
    // q = g v (e a - e b) = 0.5 * 3 * (2 * 10 - 2 * 4) = 18, u = 14 / 7 = 2 per second.
    let bench = reservoir_bench([10.0, 4.0], vec![delivery(b"on", 0, 7 * SECOND, "inflow", 14.0)?])?;
    let problem = bench.problem();
    let ticket = problem.ticket(SECOND).expect("a ticket");
    let mut workspace = problem.workspace();
    workspace.derivative_mut().copy_from_slice(&[1.0, 2.0]);
    let point = workspace.point(ticket.at(SECOND).expect("an offset"));
    let stage = problem.evaluate(&point).expect("an evaluation");
    // F_a = a' + q - u, F_b = b' - q.
    assert_eq!(stage.residuals(), [1.0 + 18.0 - 2.0, 2.0 - 18.0]);
    let shift = ticket.shift(SECOND / 2).expect("a shift");
    let jacobian = problem.linearize(&point, shift).expect("a jacobian");
    // dq/da = g v e = 3, dq/db = -3, and the shift is 2 per second.
    assert_eq!(dense(jacobian.view(), 2), [[2.0 + 3.0, -3.0], [-3.0, 2.0 + 3.0]]);
    Ok(())
}

#[test]
fn a_stage_at_a_tickets_end_reads_the_left_limit_of_the_drive() -> Result<(), InitializationFailure> {
    // Equal amounts make q zero, so a's residual is minus the drive alone.
    let events = vec![
        delivery(b"first", 0, 2 * SECOND, "inflow", 2.0)?,
        delivery(b"second", 2 * SECOND, 4 * SECOND, "inflow", 20.0)?,
    ];
    let bench = reservoir_bench([5.0, 5.0], events)?;
    let problem = bench.problem();
    let ticket = problem.ticket(100 * SECOND).expect("a ticket");
    assert_eq!(
        ticket.length_ns(),
        2 * SECOND,
        "the ticket ends at the next breakpoint"
    );
    let workspace = problem.workspace();
    let end = workspace.point(ticket.at(ticket.length_ns()).expect("an offset"));
    let stage = problem.evaluate(&end).expect("an evaluation");
    assert_eq!(stage.residuals()[0], -1.0);
    Ok(())
}

#[test]
fn overlapping_deliveries_add_and_no_delivery_means_zero() -> Result<(), InitializationFailure> {
    let events = vec![
        delivery(b"long", 0, 10 * SECOND, "inflow", 10.0)?,
        delivery(b"short", 0, 5 * SECOND, "inflow", 15.0)?,
    ];
    let bench = reservoir_bench([5.0, 5.0], events)?;
    let problem = bench.problem();
    let ticket = problem.ticket(SECOND).expect("a ticket");
    let workspace = problem.workspace();
    let stage = problem
        .evaluate(&workspace.point(ticket.at(0).expect("an offset")))
        .expect("evaluated");
    assert_eq!(stage.residuals()[0], -4.0);
    let silent = reservoir_bench([5.0, 5.0], Vec::new())?;
    let problem = silent.problem();
    let ticket = problem.ticket(SECOND).expect("a ticket");
    let workspace = problem.workspace();
    let stage = problem
        .evaluate(&workspace.point(ticket.at(0).expect("an offset")))
        .expect("evaluated");
    assert_eq!(stage.residuals()[0], 0.0);
    Ok(())
}

/// Memory rows that read every operator the body language has, each at a smooth point.
fn every_operator() -> Result<(CompiledBody, LawName), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let schema = Schema::new(map.finish()?)?;
    let law = LawName {
        owner: "NS",
        local: "first",
    };
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let first = laws.state::<Dimensionless>("first", Admissible::finite());
        let second = laws.state::<Dimensionless>("second", Admissible::finite());
        let third = laws.algebraic::<Pressure>("third", Admissible::finite());
        let pace = laws.algebraic::<Rate<Dimensionless>>("pace", Admissible::finite());
        let set_point = laws.set_point::<Dimensionless>("set point", Admissible::finite());
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
        let reach = laws.parameter::<Pressure>(ParameterKey::new("ns.reach"), Admissible::positive());
        let divisor = laws.parameter::<Pressure>(ParameterKey::new("ns.divisor"), Admissible::positive());
        let width = laws.parameter::<Dimensionless>(ParameterKey::new("ns.width"), Admissible::positive());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let first_rate = laws.kernel(|mut k| {
            let x = k.state(&first);
            let y = k.state(&second);
            let w = k.parameter(&width);
            let high = k.smooth_max(x, y, w);
            let x = k.state(&first);
            let y = k.state(&second);
            let w = k.parameter(&width);
            let low = k.smooth_min(x, y, w);
            let sum = k.add(high, low);
            let exponential = k.exp(sum);
            let y = k.state(&second);
            let logistic = k.logistic(y);
            let target = k.scale(exponential, logistic);
            let x = k.state(&first);
            let rate = k.algebraic(&pace);
            let rate = k.relax(target, x, rate);
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.memory_rate(&first, first_rate);
        let second_rate = laws.kernel(|mut k| {
            let p = k.algebraic(&third);
            let r = k.parameter(&divisor);
            let ratio = k.divide(p, r);
            let ratio = k.as_dimension::<_, Dimensionless>(ratio);
            let log = k.ln(ratio);
            let measured = k.state(&first);
            let reference = k.state(&set_point);
            let error = k.deviation(measured, reference);
            let target = k.subtract(log, error);
            let y = k.state(&second);
            let rate = k.parameter(&speed);
            let rate = k.relax(target, y, rate);
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.memory_rate(&second, second_rate);
        let third_residual = laws.kernel(|mut k| {
            let p = k.algebraic(&third);
            let target = k.parameter(&reach);
            let residual = k.subtract(p, target);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&third, third_residual);
        let pace_residual = laws.kernel(|mut k| {
            let now = k.algebraic(&pace);
            let declared = k.parameter(&speed);
            let residual = k.subtract(now, declared);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&pace, pace_residual);
        let set_point_rate = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Reference<Dimensionless>>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.memory_rate(&set_point, set_point_rate);
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((body, law))
}

fn memory(local: &'static str) -> RowKind {
    RowKind::Memory(LawName { owner: "NS", local })
}

#[test]
fn derivatives_match_central_differences_on_every_operator() -> Result<(), InitializationFailure> {
    let (body, _) = every_operator()?;
    let mut packet = ParameterPacket::new();
    let bind = Binding::Body;
    packet.synthetic(
        ParameterKey::new("ns.speed"),
        bind,
        Measure::<Rate<Dimensionless>>::new(0.7)?,
        "synthetic",
    )?;
    packet.synthetic(
        ParameterKey::new("ns.reach"),
        bind,
        Measure::<Pressure>::new(1.5)?,
        "synthetic",
    )?;
    packet.synthetic(
        ParameterKey::new("ns.divisor"),
        bind,
        Measure::<Pressure>::new(2.0)?,
        "synthetic",
    )?;
    packet.synthetic(
        ParameterKey::new("ns.width"),
        bind,
        Measure::<Dimensionless>::new(0.3)?,
        "synthetic",
    )?;
    packet.start(memory("first"), Measure::<Dimensionless>::new(0.4)?, "synthetic")?;
    packet.start(
        memory("second"),
        Measure::<Dimensionless>::new(-0.2)?,
        "synthetic",
    )?;
    let third = RowKind::Algebraic(LawName {
        owner: "NS",
        local: "third",
    });
    packet.start(third, Measure::<Pressure>::new(1.5)?, "synthetic")?;
    let pace = RowKind::Algebraic(LawName {
        owner: "NS",
        local: "pace",
    });
    packet.start(pace, Measure::<Rate<Dimensionless>>::new(0.7)?, "synthetic")?;
    packet.start(
        memory("set point"),
        Measure::<Reference<Dimensionless>>::new(0.1)?,
        "synthetic",
    )?;
    let bench = Bench::admitted(body, packet, BoundaryPort::new(Vec::new())?)?;
    let problem = bench.problem();
    let ticket = problem.ticket(SECOND).expect("a ticket");
    let time = ticket.at(SECOND).expect("an offset");
    let shift = ticket.shift(SECOND).expect("a shift");
    let rows = bench.body.rows().len();
    let mut workspace = problem.workspace();
    let tangent = [0.3, -0.1, 0.2, 0.0, 0.0];
    workspace.derivative_mut().copy_from_slice(&tangent);
    let start = workspace.unknowns_mut().to_vec();
    let analytic = dense(
        problem
            .linearize(&workspace.point(time), shift)
            .expect("linearized")
            .view(),
        rows,
    );
    const STEP: f64 = 1e-6;
    for unknown in 0..rows {
        let mut residual_at = |offset: f64, tangent_offset: f64| -> Vec<f64> {
            workspace.unknowns_mut().copy_from_slice(&start);
            workspace.derivative_mut().copy_from_slice(&tangent);
            workspace.unknowns_mut()[unknown] += offset;
            workspace.derivative_mut()[unknown] += tangent_offset;
            let stage = problem.evaluate(&workspace.point(time)).expect("evaluated");
            stage.residuals().to_vec()
        };
        // dF/dy + shift * dF/dy' with the shift at one per second is the sum of the two derivatives.
        let (up, down) = (residual_at(STEP, STEP), residual_at(-STEP, -STEP));
        for (row, cells) in analytic.iter().enumerate() {
            let estimate = (up[row] - down[row]) / (2.0 * STEP);
            let exact = cells[unknown];
            assert!(
                (estimate - exact).abs() < 1e-7 * (1.0 + exact.abs()),
                "row {row} unknown {unknown}: {estimate} against {exact}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_smooth_limit_is_defined_where_its_root_is_zero() -> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let schema = Schema::new(map.finish()?)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let first = laws.state::<Dimensionless>("first", Admissible::finite());
        let width = laws.parameter::<Dimensionless>(ParameterKey::new("ns.width"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let rate = laws.kernel(|mut k| {
            let x = k.state(&first);
            let zero = k.zero::<Dimensionless>();
            let w = k.parameter(&width);
            let limit = k.smooth_max(x, zero, w);
            let x = k.state(&first);
            let speed = k.zero::<Rate<Dimensionless>>();
            let rate = k.relax(limit, x, speed);
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.memory_rate(&first, rate);
        body.add(laws.finish());
        body.compile()
    })?;
    let mut packet = ParameterPacket::new();
    packet.synthetic(
        ParameterKey::new("ns.width"),
        Binding::Body,
        Measure::<Dimensionless>::new(0.0)?,
        "synthetic",
    )?;
    packet.start(memory("first"), Measure::<Dimensionless>::new(0.0)?, "synthetic")?;
    let bench = Bench::admitted(body, packet, BoundaryPort::new(Vec::new())?)?;
    let problem = bench.problem();
    let ticket = problem.ticket(SECOND).expect("a ticket");
    let workspace = problem.workspace();
    let time = ticket.at(0).expect("an offset");
    let stage = problem.evaluate(&workspace.point(time)).expect("evaluated");
    assert_eq!(stage.residuals(), [0.0]);
    let jacobian = problem
        .linearize(&workspace.point(time), ticket.shift(SECOND).expect("a shift"))
        .expect("defined");
    assert_eq!(dense(jacobian.view(), 1), [[1.0]]);
    Ok(())
}

/// Forward Euler by hand: one stage at the start, its tangent the rates there.
fn forward_euler(bench: &Bench, span_ns: u64) -> Result<VerifiedStep, TrialFailure> {
    let problem = bench.problem();
    let ticket = problem.ticket(span_ns)?;
    let mut workspace = problem.workspace();
    let time = ticket.at(0)?;
    let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
        .map(|residual| -residual)
        .collect();
    workspace.derivative_mut().copy_from_slice(&rates);
    let stage = problem.evaluate(&workspace.point(time))?;
    let mut quadrature = problem.quadrature(&ticket);
    quadrature.add(&stage, Ratio::ONE)?;
    let history = bench.parts.continuation.clone();
    let proposal = problem.propose(&ticket, quadrature.finish()?, &workspace, history);
    proposal.verify(&problem)
}

#[test]
fn a_verified_step_publishes_the_start_plus_the_increment() -> Result<(), InitializationFailure> {
    let bench = reservoir_bench([10.0, 4.0], vec![delivery(b"on", 0, 7 * SECOND, "inflow", 14.0)?])?;
    let bench = bench.declaring(StageScheme::forward_euler())?;
    let step = forward_euler(&bench, SECOND / 4).expect("verified");
    let rows: Vec<RowId> = bench.body.rows().iter().map(Row::id).collect();
    // A quarter second of q = 18 and u = 2: a = 10 + (2 - 18) / 4, b = 4 + 18 / 4.
    assert_eq!(step.value(rows[0]), Compensated::of(&Measure::<Water>::new(6.0)?));
    assert_eq!(step.value(rows[1]), Compensated::of(&Measure::<Water>::new(8.5)?));
    let gross: Vec<(Compensated, Compensated)> = step.gross().collect();
    let water = |value| Measure::<Water>::new(value).map(|measure| Compensated::of(&measure));
    let circulation = bench
        .body
        .columns()
        .iter()
        .position(|column| column.terms().len() == 2);
    assert_eq!(gross[circulation.expect("a circulation column")].0, water(4.5)?);
    assert_eq!(
        gross[circulation.expect("a circulation column") ^ 1].0,
        water(0.5)?
    );
    assert_eq!(step.applied().len(), 1);
    Ok(())
}
