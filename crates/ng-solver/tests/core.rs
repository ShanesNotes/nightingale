use core::cell::Cell;
use ng_contract::*;
use ng_solver::Core;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const CONDUCTANCE: f64 = 0.001;
const EXTERIOR: &str = "exterior";

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

struct Reservoirs {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
    start: [f64; 2],
    deliveries: Vec<(&'static [u8], u64, u64, f64)>,
    conductance: f64,
}
impl Reservoirs {
    fn new(start: [f64; 2], deliveries: Vec<(&'static [u8], u64, u64, f64)>) -> Self {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", origin());
        let a = map.site::<VesselSegment>("a", origin());
        let b = map.site::<VesselSegment>("b", origin());
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.connect(Network::Vascular, a.id(), b.id());
        let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        schema
            .boundary::<Water>(EXTERIOR, BoundaryDirection::IntoBody, a.id(), Space::Lumen)
            .expect("a boundary");
        schema.partial_volume(measure(1.0)).expect("a partial volume");
        let body = assemble(schema, |mut body| {
            let (mut laws, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut laws);
            let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
            let g = laws.edge_parameter::<_, Conductance<Water>>(
                ParameterKey::new("cv.g"),
                Admissible::nonnegative(),
            );
            let wave = RateClass::new(RateBand::Wave, u64::MAX)?;
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
            let BoundaryRoute { route: pair, drive } = ports.route::<Water>(EXTERIOR)?;
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), wave)
            });
            laws.transfer(Transfer::new(&pair, rate));
            body.add(laws.finish());
            body.compile()
        })
        .expect("compiled");
        Self {
            body,
            a: a.id(),
            b: b.id(),
            start,
            deliveries,
            conductance: CONDUCTANCE,
        }
    }
    fn stiff(self, conductance: f64) -> Self {
        Self { conductance, ..self }
    }
    fn stepped() -> Self {
        Self::new(
            [500_500.0, 499_500.0],
            vec![(b"stepped inflow", 2_000, 6_000, 20_000.0)],
        )
    }
    fn packet(&self) -> ParameterPacket<SyntheticFixture> {
        let default = Binding::Network(Network::Vascular);
        let mut packet = ParameterPacket::new();
        packet
            .synthetic(
                ParameterKey::new("cv.e"),
                default,
                measure::<Elastance>(1.0),
                "synthetic",
            )
            .expect("once");
        let g = measure::<Conductance<Water>>(self.conductance);
        packet
            .synthetic(ParameterKey::new("cv.g"), default, g, "synthetic")
            .expect("once");
        for (site, amount) in [self.a, self.b].into_iter().zip(self.start) {
            let row = RowKind::Amount {
                site,
                space: Space::Lumen,
                species: Species::Water,
            };
            packet
                .start(row, measure::<Water>(amount), "synthetic")
                .expect("once");
        }
        packet
    }
    fn events(&self) -> Vec<BoundaryEvent> {
        (self.deliveries.iter())
            .map(|&(id, from, to, amount)| {
                let water = WaterDelivery {
                    boundary: EXTERIOR,
                    amount: measure(amount),
                    ends: at(to),
                };
                BoundaryEvent::new(
                    InputId::new(Digest::of_canonical(id)),
                    at(from),
                    BoundaryInput::Water(water),
                )
            })
            .collect()
    }
    fn rows(&self) -> [RowId; 2] {
        let row = |site| {
            let lumen = RowKind::Amount {
                site,
                space: Space::Lumen,
                species: Species::Water,
            };
            let found = self.body.rows().iter().find(|row| row.kind() == lumen);
            found.expect("the body declares both lumens").id()
        };
        [row(self.a), row(self.b)]
    }
    fn exact(&self, until: SimTime) -> [f64; 2] {
        let mut sum = self.start[0] + self.start[1];
        let mut difference = self.start[0] - self.start[1];
        let mut edges: Vec<u64> = (self.deliveries.iter())
            .flat_map(|&(_, from, to, _)| [from * SECOND, to * SECOND])
            .filter(|&edge| edge < until.nanoseconds())
            .collect();
        edges.push(until.nanoseconds());
        edges.sort_unstable();
        let mut reached = 0;
        for edge in edges {
            let inflow: f64 = (self.deliveries.iter())
                .filter(|&&(_, from, to, _)| from * SECOND <= reached && reached < to * SECOND)
                .map(|&(_, from, to, amount)| amount / (to - from) as f64)
                .sum();
            let span = (edge - reached) as f64 / 1e9;
            let decay = exp(-2.0 * self.conductance * span);
            let settled = inflow / (2.0 * self.conductance);
            difference = settled + (difference - settled) * decay;
            sum += inflow * span;
            reached = edge;
        }
        [(sum + difference) / 2.0, (sum - difference) / 2.0]
    }
}

struct Taken {
    refusals: Vec<Refused>,
    after_breakpoint: bool,
    interval: Interval,
}

struct Run<'m> {
    body: &'m CompiledBody,
    identity: RunIdentity,
    parts: ImageParts,
    revision: u64,
}
impl<'m> Run<'m> {
    fn admit(case: &'m Reservoirs, core: &dyn CoupledSolver) -> Self {
        Self::admit_body(&case.body, case.packet(), case.events(), core)
    }
    fn admit_body(
        body: &'m CompiledBody,
        packet: ParameterPacket<SyntheticFixture>,
        events: Vec<BoundaryEvent>,
        core: &dyn CoupledSolver,
    ) -> Self {
        let parameters = body.bind(packet).expect("bound");
        let port = BoundaryPort::new(events).expect("a timeline");
        let request = InitializationRequest::at(SimTime::from_ns(0));
        let system = body.initialization(parameters, port, request).expect("a system");
        let solved = system.workspace();
        let resolution = core.resolution();
        let start = MethodHistory::new(resolution.policy(), SimTime::from_ns(0), Vec::new());
        let revision = system.admit(solved, start).expect("admitted");
        let seed = Seed::new([1, 2, 3, 4]);
        let identity = RunIdentity::new(body.model_hash(), revision.packet_hash(), seed, resolution);
        Self {
            body,
            identity,
            parts: revision.into_parts(),
            revision: 0,
        }
    }
    fn accepted(&self) -> AcceptedStateView<'_> {
        let ImageParts {
            time,
            state,
            continuation,
            dense,
            ..
        } = &self.parts;
        AcceptedStateView::new(*time, state, continuation, dense)
    }
    fn problem(&self) -> CoupledProblem<'_> {
        let base = Digest::of_canonical(&self.revision.to_be_bytes());
        CoupledProblem::new(
            self.body,
            self.parts.parameters(),
            &self.identity,
            self.accepted(),
            base,
        )
    }
    fn step(&mut self, core: &dyn CoupledSolver) -> Result<Taken, EngineFault> {
        let mut refusals: Vec<Refused> = Vec::new();
        let verified = loop {
            let problem = self.problem();
            let attempt = match refusals.last() {
                None => Attempt::First {
                    span_ns: core.next_step(self.accepted()),
                },
                Some(refused) => Attempt::After(refused),
            };
            let outcome = self.parts.pending.attempt(&problem, attempt, |ticket| {
                let proposal = core.solve(&problem, ticket)?;
                Ok((proposal.verify(&problem)?, ()))
            });
            match outcome {
                Ok((verified, ())) => break verified,
                Err(TrialFailure::Refused(refused)) => refusals.push(refused),
                Err(TrialFailure::EngineFault(fault)) => return Err(fault),
            }
        };
        let ticket = verified.ticket();
        let (interval, after_breakpoint) = (ticket.interval(), ticket.after_breakpoint());
        let applied = self.parts.pending.after(ticket);
        let VerifiedParts {
            state,
            continuation,
            dense,
            ..
        } = verified.into_parts();
        self.parts.pending.advance(applied);
        (self.parts.time, self.parts.state) = (interval.end(), state);
        (self.parts.continuation, self.parts.dense) = (continuation, dense);
        self.revision += 1;
        Ok(Taken {
            refusals,
            after_breakpoint,
            interval,
        })
    }
    fn until(&mut self, core: &dyn CoupledSolver, end: SimTime, mut each: impl FnMut(&Self, &Taken)) {
        while self.parts.time < end {
            let taken = self.step(core).unwrap_or_else(|fault| panic!("{fault:?}"));
            each(self, &taken);
        }
    }
    fn amount(&self, row: RowId) -> f64 {
        hi(self.accepted().value(row))
    }
}

fn worst_error(case: &Reservoirs, cap_ns: u64, end: SimTime) -> f64 {
    let core = Core::new(cap_ns);
    let mut run = Run::admit(case, &core);
    let rows = case.rows();
    let mut worst = 0.0_f64;
    run.until(&core, end, |run, _| {
        let exact = case.exact(run.parts.time);
        for (row, exact) in rows.into_iter().zip(exact) {
            let error = (run.amount(row) - exact).abs();
            worst = if error > worst { error } else { worst };
        }
    });
    worst
}

#[test]
fn the_error_against_the_closed_form_halves_with_the_cap() {
    let case = Reservoirs::stepped();
    let end = at(12_000);
    let errors = [16, 8, 4].map(|seconds| worst_error(&case, seconds * SECOND, end));
    assert!(errors[0] < 5.0, "{errors:?}");
    for pair in errors.windows(2) {
        let ratio = pair[0] / pair[1];
        assert!((1.9..=2.1).contains(&ratio), "first order: {errors:?}");
    }
}

#[test]
fn no_span_passes_the_cap_and_a_quiet_body_runs_at_it() {
    let case = Reservoirs::stepped();
    let core = Core::new(16 * SECOND);
    let mut run = Run::admit(&case, &core);
    let mut lengths = Vec::new();
    run.until(&core, at(1_000), |run, taken| {
        assert!(core.next_step(run.accepted()) <= 16 * SECOND);
        assert!(taken.refusals.is_empty(), "{:?}", taken.refusals);
        lengths.push(taken.interval.length_ns());
    });
    assert!(lengths.iter().all(|&length| length == 16 * SECOND), "{lengths:?}");
}

#[test]
fn excess_error_is_refused_and_the_span_comes_down() {
    let case = Reservoirs::new([1_000_000.0, 1_000.0], Vec::new());
    let cap = 4_000 * SECOND;
    let core = Core::new(cap);
    let mut run = Run::admit(&case, &core);
    let first = run.step(&core).expect("a step");
    assert!(
        !first.refusals.is_empty(),
        "a cap far past the relaxation time is refused for accuracy first"
    );
    for refused in &first.refusals {
        assert!(
            matches!(refused.reason(), NumericalRefusal::Accuracy { .. }),
            "{refused:?}"
        );
    }
    assert!(first.interval.length_ns() < cap);
    let mut refused = 0;
    run.until(&core, at(8_000), |_, taken| refused += taken.refusals.len());
    assert!(refused <= 2, "{refused} refusals once the span is known");
    let [a, b] = case.rows().map(|row| run.amount(row));
    assert!((a - b).abs() < 100.0, "{a} and {b} have settled");
}

#[test]
fn the_history_starts_at_each_breakpoint() {
    let case = Reservoirs::new(
        [500_500.0, 499_500.0],
        vec![(b"first", 100, 300, 1_000.0), (b"second", 200, 250, 500.0)],
    );
    let core = Core::new(64 * SECOND);
    let mut run = Run::admit(&case, &core);
    let mut breakpoints = Vec::new();
    run.until(&core, at(1_000), |run, taken| {
        let started = run.parts.continuation.started_at();
        if taken.after_breakpoint {
            assert_eq!(started, taken.interval.start());
            breakpoints.push(started);
        }
        assert!(started <= taken.interval.start());
    });
    assert_eq!(breakpoints, [at(100), at(200), at(250), at(300)]);
}

#[test]
fn a_fresh_core_at_every_step_takes_the_same_steps_to_the_same_bits() {
    let case = Reservoirs::new(
        [500_500.0, 499_500.0],
        vec![(b"pulse", 30, 90, 5_000.0), (b"short", 45, 46, 100.0)],
    );
    let trace = |fresh: bool| {
        let kept = Core::new(16 * SECOND);
        let mut run = Run::admit(&case, &kept);
        let mut seen = Vec::new();
        while run.parts.time < at(400) {
            let core = Core::new(16 * SECOND);
            let core = if fresh { &core } else { &kept };
            let taken = run.step(core).expect("a step");
            let digest = run.parts.state.digest();
            seen.push((taken.interval, digest, run.parts.continuation.digest()));
        }
        seen
    };
    assert_eq!(trace(true), trace(false));
}

#[test]
fn two_caps_are_two_identities_and_two_grids() {
    let case = Reservoirs::stepped();
    let grid = |cap_ns| {
        let core = Core::new(cap_ns);
        let mut run = Run::admit(&case, &core);
        let mut grid = Vec::new();
        run.until(&core, at(100), |run, _| grid.push(run.parts.time));
        (core.resolution().digest(), grid)
    };
    let (coarse, fine) = (grid(SECOND), grid(SECOND / 4));
    assert_ne!(coarse.0, fine.0);
    assert_ne!(coarse.1, fine.1);
    assert_eq!(Core::new(SECOND).resolution().digest(), coarse.0);
    assert_eq!(fine.1.len(), 400);
}

#[test]
fn the_declared_resolution_is_the_arenas() {
    let (halved, doubled) = (Core::new(10 * MILLISECOND), Core::doubled(10 * MILLISECOND));
    for (core, scheme) in [
        (&halved, StageScheme::backward_euler_halved()),
        (&doubled, StageScheme::backward_euler_doubled()),
    ] {
        let resolution = core.resolution();
        assert_eq!(resolution.minimum_interval_ns().get(), 100_000);
        let scales =
            |absolute, relative| ErrorScales::uniform(measure(absolute), measure(relative)).expect("scales");
        let expected = DeclaredResolution::new(
            resolution.policy(),
            resolution.minimum_interval_ns(),
            12,
            ArithmeticProfile::contract_v1(),
            scheme,
            scales(0.1, 1e-3),
        );
        assert_eq!(expected.digest(), resolution.digest());
    }
    let (halved, doubled) = (halved.resolution(), doubled.resolution());
    assert_eq!(halved.policy(), doubled.policy(), "one continuation schema");
    assert_ne!(halved.digest(), doubled.digest(), "two runs");
}

fn coupled_memory(nonlinear: bool) -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("synthetic body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _) = body.nervous()?.parts();
        let memory = laws.state::<Dimensionless>("memory", Admissible::nonnegative());
        let x = laws.algebraic::<Dimensionless>("x", Admissible::finite());
        let y = laws.algebraic::<Dimensionless>("y", Admissible::finite());
        let speed = laws
            .parameter::<Rate<Dimensionless>>(ParameterKey::new("synthetic speed"), Admissible::positive());
        let coefficient = laws.parameter::<Dimensionless>(
            ParameterKey::new("synthetic coefficient"),
            Admissible::nonnegative(),
        );
        let class = RateClass::new(RateBand::Minutes, SECOND)?;
        let rate = laws.kernel(|mut k| {
            let speed = k.parameter(&speed);
            let rate = if nonlinear {
                let x = k.algebraic(&x);
                let squared = k.scale(x, x);
                let loss = k.scale(speed, squared);
                let zero = k.zero::<Rate<Dimensionless>>();
                k.subtract(zero, loss)
            } else {
                speed
            };
            k.finish(rate, laws.every_mode(), class)
        });
        laws.memory_rate(&memory, rate);
        for (own, other) in [(&x, &y), (&y, &x)] {
            let residual = laws.kernel(|mut k| {
                let own = k.algebraic(own);
                let other = k.algebraic(other);
                let coefficient = k.parameter(&coefficient);
                let scaled = k.scale(other, coefficient);
                let sum = k.add(own, scaled);
                let memory = k.state(&memory);
                let value = k.subtract(sum, memory);
                k.finish(value, laws.every_mode(), class)
            });
            laws.residual(own, residual);
        }
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    let speed = measure::<Rate<Dimensionless>>(4.0);
    (packet.synthetic(
        ParameterKey::new("synthetic speed"),
        Binding::Body,
        speed,
        "synthetic",
    ))
    .expect("once");
    let none = measure::<Dimensionless>(0.0);
    (packet.synthetic(
        ParameterKey::new("synthetic coefficient"),
        Binding::Body,
        none,
        "synthetic",
    ))
    .expect("once");
    let start = if nonlinear { 2.0 } else { 0.0 };
    packet.start_rest(&body, |_| start, "synthetic").expect("a start");
    (body, packet)
}

fn logarithm() -> (CompiledBody, ParameterPacket<SyntheticFixture>) {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("synthetic body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _) = body.nervous()?.parts();
        let x = laws.state::<Dimensionless>("x", Admissible::positive());
        let log = laws.algebraic::<Dimensionless>("log", Admissible::finite());
        let speed = laws
            .parameter::<Rate<Dimensionless>>(ParameterKey::new("synthetic speed"), Admissible::positive());
        let class = RateClass::new(RateBand::Minutes, SECOND)?;
        let rate = laws.kernel(|mut k| {
            let speed = k.parameter(&speed);
            let x = k.state(&x);
            let loss = k.scale(speed, x);
            let zero = k.zero::<Rate<Dimensionless>>();
            let rate = k.subtract(zero, loss);
            k.finish(rate, laws.every_mode(), class)
        });
        laws.memory_rate(&x, rate);
        let residual = laws.kernel(|mut k| {
            let x = k.state(&x);
            let logarithm = k.ln(x);
            let actual = k.algebraic(&log);
            let error = k.subtract(actual, logarithm);
            k.finish(error, laws.every_mode(), class)
        });
        laws.residual(&log, residual);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    let mut packet = ParameterPacket::new();
    let speed = measure::<Rate<Dimensionless>>(40.0);
    (packet.synthetic(
        ParameterKey::new("synthetic speed"),
        Binding::Body,
        speed,
        "synthetic",
    ))
    .expect("once");
    let start = |row| {
        if matches!(row, RowKind::Memory(_)) {
            1.0
        } else {
            0.0
        }
    };
    (packet.start_rest(&body, start, "synthetic: the logarithm of one")).expect("a start");
    (packet.row_scale(body.rows()[1].kind(), measure::<Dimensionless>(10.0))).expect("a row scale");
    (body, packet)
}

#[test]
fn no_cap_is_passed_at_the_edges_and_a_one_nanosecond_ticket_steps() {
    let case = Reservoirs::stepped();
    let core = Core::new(1);
    let mut run = Run::admit(&case, &core);
    for longest in [0, 1, 99_999, 100_000, SECOND, u64::MAX] {
        let capped = Core::new(longest);
        assert!(capped.next_step(run.accepted()) <= longest, "{longest}");
        assert_eq!(
            capped.resolution().digest(),
            Core::new(longest).resolution().digest()
        );
        assert_ne!(
            capped.resolution().digest(),
            Core::new(longest ^ 1).resolution().digest()
        );
    }
    let taken = run.step(&core).expect("a step");
    assert_eq!(taken.interval.length_ns(), 1);
}

#[test]
fn a_fresh_core_resumes_from_the_image_s_canonical_bytes() {
    let case = Reservoirs::new(
        [500_500.0, 499_500.0],
        vec![(b"pulse", 30, 90, 5_000.0), (b"short", 45, 46, 100.0)],
    );
    let constructors: [fn(u64) -> Core; 2] = [Core::new, Core::doubled];
    for build in constructors {
        let core = build(16 * SECOND);
        let mut uninterrupted = Run::admit(&case, &core);
        for _ in 0..4 {
            uninterrupted.step(&core).expect("a step");
        }
        let mut resumed = Run::admit(&case, &core);
        resumed.parts = ImageParts::parse(&case.body, &uninterrupted.parts.canonical()).expect("parsed");
        resumed.revision = uninterrupted.revision;
        for _ in 0..20 {
            let fresh = build(16 * SECOND);
            assert_eq!(
                core.next_step(uninterrupted.accepted()),
                fresh.next_step(resumed.accepted())
            );
            uninterrupted.step(&core).expect("a step");
            resumed.step(&fresh).expect("a step");
            assert_eq!(uninterrupted.parts.canonical(), resumed.parts.canonical());
        }
    }
}

#[test]
fn a_pivot_the_fixed_elimination_misses_is_taken_by_a_dense_solve() {
    let (body, packet) = coupled_memory(false);
    let core = Core::new(SECOND / 10);
    let mut run = Run::admit_body(&body, packet, Vec::new(), &core);
    let problem = run.problem();
    let attempt = Attempt::First { span_ns: SECOND / 10 };
    let (_, missed) = (run.parts.pending)
        .attempt(&problem, attempt, |ticket| {
            let workspace = problem.workspace();
            let point = workspace.point(ticket.at(SECOND / 20)?);
            let jacobian = problem.linearize(&point, ticket.shift(SECOND / 20)?)?;
            let missed = body.factor(&jacobian).is_err();
            Ok((core.solve(&problem, ticket)?.verify(&problem)?, missed))
        })
        .expect("verified");
    assert!(missed, "the fixed elimination must miss a numerical pivot");
    run.step(&core).expect("a step");
    for row in body.rows() {
        assert!((run.amount(row.id()) - 0.4).abs() < 1e-12, "{:?}", row.kind());
    }
}

#[test]
fn a_nonlinear_memory_meets_the_implicit_quadratic_root() {
    let (body, packet) = coupled_memory(true);
    let core = Core::new(SECOND / 50);
    let mut run = Run::admit_body(&body, packet, Vec::new(), &core);
    let taken = run.step(&core).expect("a step");
    assert_eq!(taken.interval.length_ns(), SECOND / 50);
    let root = |base: f64| 2.0 * base / (1.0 + (1.0 + 4.0 * 0.01 * 4.0 * base).sqrt());
    let expected = root(root(2.0));
    for row in body.rows() {
        let value = run.amount(row.id());
        assert!((value - expected).abs() < 1e-6, "{value} against {expected}");
    }
}

#[test]
fn a_guess_past_a_law_s_domain_falls_back_and_the_stage_still_solves() {
    let (body, packet) = logarithm();
    let core = Core::doubled(SECOND / 10);
    let mut run = Run::admit_body(&body, packet, Vec::new(), &core);
    let taken = run.step(&core).expect("a step");
    assert!(taken.refusals.is_empty(), "{:?}", taken.refusals);
    assert_eq!(taken.interval.length_ns(), SECOND / 10);
    let [x, log] = [0, 1].map(|at| run.amount(body.rows()[at].id()));
    assert!((x - 1.0 / 9.0).abs() < 1e-12, "{x}");
    assert!((exp(log) - 1.0 / 9.0).abs() < 1e-9, "{log}");
    let (stiff, packet) = logarithm();
    let halved = Core::new(SECOND / 10);
    let refused = Run::admit_body(&stiff, packet, Vec::new(), &halved)
        .step(&halved)
        .expect("a step")
        .refusals;
    assert!(
        (refused.iter()).any(|refused| matches!(refused.reason(), NumericalRefusal::Accuracy { .. })),
        "the second difference refuses this stiff first step: {refused:?}"
    );
}

#[test]
fn excess_error_at_the_floor_is_an_engine_fault() {
    let case = Reservoirs::new([1_000_000.0, 0.0], Vec::new()).stiff(10_000.0);
    let core = Core::new(100_000);
    let mut run = Run::admit(&case, &core);
    let Err(fault) = run.step(&core) else {
        panic!("a step at the floor was accepted");
    };
    assert_eq!(fault.kind(), EngineFaultKind::NoAdmissibleStep);
    assert!(
        matches!(fault.last_refusal(), Some(NumericalRefusal::Accuracy { .. })),
        "{fault:?}"
    );
}

struct Counting<'c> {
    core: &'c Core,
    refused: Cell<usize>,
}
impl CoupledSolver for Counting<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let solved = self.core.solve(problem, ticket);
        if let Err(TrialFailure::Refused(refused)) = &solved {
            assert!(
                matches!(refused.reason(), NumericalRefusal::Accuracy { .. }),
                "{refused:?}"
            );
            self.refused.set(self.refused.get() + 1);
        }
        solved
    }
}

#[test]
fn the_record_never_refuses_for_accuracy_a_step_the_core_proposed() {
    let case = Reservoirs::new([1_000_000.0, 1_000.0], Vec::new());
    let core = Core::new(4_000 * SECOND);
    let counting = Counting {
        core: &core,
        refused: Cell::new(0),
    };
    let mut run = Run::admit(&case, &counting);
    let mut refusals = 0;
    run.until(&counting, at(8_000), |_, taken| refusals += taken.refusals.len());
    assert!(refusals > 0, "the cap is far past the relaxation time");
    assert_eq!(counting.refused.get(), refusals, "every refusal is the core's");
}

#[test]
fn an_estimate_of_stages_off_its_ticket_s_set_is_a_method_mismatch() {
    let case = Reservoirs::stepped();
    let core = Core::new(16 * SECOND);
    let run = Run::admit(&case, &core);
    let problem = run.problem();
    let booked = |ticket: &StepTicket, stages: &[(u64, Ratio)]| -> Result<Increment, TrialFailure> {
        let workspace = problem.workspace();
        let mut quadrature = problem.quadrature(ticket);
        for &(offset, weight) in stages {
            quadrature.add(&problem.evaluate(&workspace.point(ticket.at(offset)?))?, weight)?;
        }
        Ok(quadrature.finish()?)
    };
    let kind = |read: Result<f64, TrialFailure>| match read {
        Err(TrialFailure::EngineFault(fault)) => Some(fault.kind()),
        _ => None,
    };
    let first = Attempt::First { span_ns: 16 * SECOND };
    let (_, (set, kept, halves, doubled)) = (run.parts.pending)
        .attempt(&problem, first, |ticket| {
            let (length, half) = (ticket.length_ns(), ticket.length_ns() / 2);
            let halves = [
                (half, ticket.fraction(half)),
                (length, ticket.fraction(length - half)),
            ];
            let doubled = [(length, Ratio::ZERO), halves[0], halves[1]];
            let kept = booked(ticket, &halves)?;
            let read = (
                problem.estimate(ticket, &kept),
                problem.estimate(ticket, &booked(ticket, &doubled)?),
            );
            let verified = core.solve(&problem, ticket)?.verify(&problem)?;
            Ok((verified, (problem.stage_set(ticket), kept, read.0, read.1)))
        })
        .expect("verified");
    assert_eq!(set, StageSet::Halved);
    assert_eq!(halves, Ok(0.0), "unmoved halves have no second difference");
    assert_eq!(kind(doubled), Some(EngineFaultKind::MethodMismatch));
    let shorter = Attempt::First { span_ns: 8 * SECOND };
    let (_, other) = (run.parts.pending)
        .attempt(&problem, shorter, |ticket| {
            let read = problem.estimate(ticket, &kept);
            Ok((core.solve(&problem, ticket)?.verify(&problem)?, read))
        })
        .expect("verified");
    assert_eq!(kind(other), Some(EngineFaultKind::BindingMismatch));
}
