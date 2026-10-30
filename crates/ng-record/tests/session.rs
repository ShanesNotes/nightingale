//! The transaction at the record's own seam, with the smallest honest core. The full table of refusals
//! and faults is in `ng-validate`.
use ng_contract::*;
use ng_record::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Result<Measure<U>, AdmissionError> {
    Measure::new(value)
}

fn site_at() -> Result<Position, AdmissionError> {
    Ok(Position::new(measure(0.0)?, measure(0.0)?, measure(0.0)?))
}

struct Reservoirs {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
}

fn reservoirs() -> Result<Reservoirs, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", site_at()?);
    let a = map.site::<VesselSegment>("a", site_at()?);
    let b = map.site::<VesselSegment>("b", site_at()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.water_boundary("inflow", BoundaryDirection::IntoBody, a.id(), Space::Lumen)?;
    schema.partial_volume(measure(1.0)?)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.edge_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g =
            laws.edge_parameter::<Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let difference = k.subtract(upstream, downstream);
            let elastance = k.edge_parameter(&e);
            let pressure = k.multiply(elastance, difference);
            let head = k.as_dimension::<_, Pressure>(pressure);
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
    Ok(Reservoirs {
        body,
        a: a.id(),
        b: b.id(),
    })
}

struct Explicit {
    interval_ns: u64,
    refuse_at: Option<SimTime>,
}
impl CoupledSolver for Explicit {
    fn resolution(&self) -> DeclaredResolution {
        let scales = (measure(1.0).and_then(|absolute| ErrorScales::uniform(absolute, measure(0.0)?)))
            .expect("positive scales");
        DeclaredResolution::new(
            Digest::of_canonical(b"explicit test core"),
            SECOND / 4,
            4,
            ArithmeticProfile::contract_v1(),
            StageScheme::forward_euler(),
            scales,
        )
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if self.refuse_at == Some(ticket.interval().start()) && ticket.attempt() == 0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        let mut workspace = problem.workspace();
        let time = ticket.at(0)?;
        let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
            .map(|residual| -residual)
            .collect();
        workspace.derivative_mut().copy_from_slice(&rates);
        let stage = problem.evaluate(&workspace.point(time))?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let schema = problem.accepted().method_history().schema();
        let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

fn admitted<'m>(
    fixture: &'m Reservoirs,
    core: &'m Explicit,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let Reservoirs { body, a, b } = fixture;
    let network = Binding::Network(Network::Vascular);
    let lumen = |site: SiteId| RowKind::Amount {
        site,
        space: Space::Lumen,
    };
    let mut packet = ParameterPacket::new();
    packet.synthetic(
        ParameterKey::new("cv.e"),
        network,
        measure::<Elastance>(1.0)?,
        "synthetic",
    )?;
    packet.synthetic(
        ParameterKey::new("cv.g"),
        network,
        measure::<Conductance<Water>>(0.25)?,
        "synthetic",
    )?;
    packet.start(lumen(*a), measure::<Water>(9.0)?, "synthetic")?;
    packet.start(lumen(*b), measure::<Water>(1.0)?, "synthetic")?;
    let parameters = body.bind(packet)?;
    let delivery = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"delivery")),
        SimTime::from_ns(SECOND),
        BoundaryInput::Water(WaterDelivery {
            boundary: "inflow",
            amount: measure(2.0)?,
            ends: SimTime::from_ns(3 * SECOND),
        }),
    );
    let origin = SimTime::from_ns(0);
    let system = body.initialization(
        parameters,
        BoundaryPort::new(vec![delivery])?,
        InitializationRequest::consistent_instant(origin),
    )?;
    let solved = system.workspace();
    let history = MethodHistory::new(core.resolution().policy(), origin, Vec::new());
    let revision = system.admit(solved, history)?;
    let identity = RunIdentity::new(
        body.model_hash(),
        revision.packet_hash(),
        Seed::new([1, 2, 3, 4]),
        core.resolution(),
    );
    Ok(Session::admit(body, core, revision, identity)?)
}

fn accepted(step: Step<'_>) -> (Session<'_, Running>, StepReceipt) {
    match step {
        Step::Accepted(session, receipt) => (session, receipt),
        Step::Refused(session) => panic!("refused: {:?}", session.refused()),
        Step::Faulted(session) => panic!("faulted: {:?}", session.fault()),
    }
}

#[test]
fn an_accepted_step_moves_every_part_and_chains_its_seal() -> Result<(), InitializationFailure> {
    let fixture = reservoirs()?;
    let core = Explicit {
        interval_ns: SECOND,
        refuse_at: None,
    };
    let session = admitted(&fixture, &core)?;
    let before = session.record().digests();
    let seal = session.record().seal();
    let (session, receipt) = accepted(session.step());
    let after = session.record().digests();
    assert_eq!(receipt.revision(), 1);
    assert_ne!(receipt.seal(), seal);
    assert_eq!(receipt.seal(), session.record().seal());
    assert_eq!(after.revision, 1);
    assert_eq!(after.time, SimTime::from_ns(SECOND));
    assert_ne!(after.state, before.state);
    assert_ne!(after.segment, before.segment);
    assert_ne!(after.ledger, before.ledger);
    assert_eq!(after.streams, before.streams);
    assert_eq!(
        after.pending, before.pending,
        "no input was due on the first step"
    );
    let (session, receipt) = accepted(session.step());
    assert_eq!(receipt.applied().len(), 1);
    assert_ne!(session.record().digests().pending, after.pending);
    Ok(())
}

#[test]
fn a_refused_session_keeps_the_record_and_retries_smaller() -> Result<(), InitializationFailure> {
    let fixture = reservoirs()?;
    let core = Explicit {
        interval_ns: SECOND,
        refuse_at: Some(SimTime::from_ns(SECOND)),
    };
    let (session, _) = accepted(admitted(&fixture, &core)?.step());
    let before = session.record().digests();
    let Step::Refused(session) = session.step() else {
        panic!("not refused");
    };
    assert_eq!(session.refused().retry_ns(), SECOND / 2);
    assert_eq!(session.record().digests(), before);
    let (session, receipt) = accepted(session.retry());
    assert_eq!(receipt.applied().len(), 1);
    assert_eq!(
        session.record().accepted().time(),
        SimTime::from_ns(3 * SECOND / 2)
    );
    Ok(())
}

#[test]
fn a_refusal_whose_retry_passes_the_floor_latches_a_fault() -> Result<(), InitializationFailure> {
    let fixture = reservoirs()?;
    let core = Explicit {
        interval_ns: SECOND / 4,
        refuse_at: Some(SimTime::from_ns(0)),
    };
    let Step::Faulted(stopped) = admitted(&fixture, &core)?.step() else {
        panic!("not faulted");
    };
    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
    assert_eq!(
        stopped.fault().last_refusal(),
        Some(NumericalRefusal::IterationLimit)
    );
    assert_eq!(stopped.record().revision(), 0);
    Ok(())
}
