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

/// Two vessel segments joined by one conductance, with a water boundary into the first, as the arena's
/// accuracy case has them: the difference relaxes at twice the conductance and the sum takes the inflow.
struct Reservoirs {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
    start: [f64; 2],
    deliveries: Vec<(&'static [u8], u64, u64, f64)>,
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
        }
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
        let g = measure::<Conductance<Water>>(CONDUCTANCE);
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
    /// The closed form of `d' = -2 k d + u` and `s' = u` on each constant stretch of the inflow.
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
            let decay = exp(-2.0 * CONDUCTANCE * span);
            let settled = inflow / (2.0 * CONDUCTANCE);
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

/// A run driven by hand through the contract alone: each attempt's ticket comes from the timeline, each proposal
/// is verified on the problem built from the accepted image, and only a verified step moves the image.
struct Run<'m> {
    body: &'m CompiledBody,
    identity: RunIdentity,
    parts: ImageParts,
    revision: u64,
}
impl<'m> Run<'m> {
    fn admit(case: &'m Reservoirs, core: &dyn CoupledSolver) -> Self {
        let parameters = case.body.bind(case.packet()).expect("bound");
        let port = BoundaryPort::new(case.events()).expect("a timeline");
        let request = InitializationRequest::at(SimTime::from_ns(0));
        let system = case
            .body
            .initialization(parameters, port, request)
            .expect("a system");
        let solved = system.workspace();
        let resolution = core.resolution();
        let start = MethodHistory::new(resolution.policy(), SimTime::from_ns(0), Vec::new());
        let revision = system.admit(solved, start).expect("admitted");
        let seed = Seed::new([1, 2, 3, 4]);
        let identity = RunIdentity::new(case.body.model_hash(), revision.packet_hash(), seed, resolution);
        Self {
            body: &case.body,
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
    fn step(&mut self, core: &dyn CoupledSolver) -> Result<Taken, EngineFault> {
        let base = Digest::of_canonical(&self.revision.to_be_bytes());
        let mut refusals: Vec<Refused> = Vec::new();
        let verified = loop {
            let accepted = self.accepted();
            let problem =
                CoupledProblem::new(self.body, self.parts.parameters(), &self.identity, accepted, base);
            let attempt = match refusals.last() {
                None => Attempt::First {
                    span_ns: core.next_step(accepted),
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
fn every_step_passes_the_records_check_and_halving_the_cap_halves_the_error() {
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

/// A cap far past the relaxation time leaves the doubling estimate above the scales on the first step, so the
/// first attempts are refused for accuracy and the span the core carries on comes down.
#[test]
fn excess_error_is_refused_and_the_span_comes_down() {
    let case = Reservoirs::new([1_000_000.0, 1_000.0], Vec::new());
    let cap = 4_000 * SECOND;
    let core = Core::new(cap);
    let mut run = Run::admit(&case, &core);
    let first = run.step(&core).expect("a step");
    assert!(!first.refusals.is_empty());
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

/// Every call reads only what the seam hands it, so a run that changes core object at every step, as a resume
/// into a freshly built core does, takes the same steps to the same bits.
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
    let resolution = Core::new(10 * MILLISECOND).resolution();
    assert_eq!(resolution.minimum_interval_ns().get(), 100_000);
    let scales =
        |absolute, relative| ErrorScales::uniform(measure(absolute), measure(relative)).expect("scales");
    let expected = DeclaredResolution::new(
        resolution.policy(),
        resolution.minimum_interval_ns(),
        12,
        ArithmeticProfile::contract_v1(),
        StageScheme::backward_euler_doubled(),
        scales(0.1, 1e-3),
    );
    assert_eq!(expected.digest(), resolution.digest());
}
