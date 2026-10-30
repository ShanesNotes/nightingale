//! Signature walk: one external consumer's path through every seam, compiled as a doctest from
//! outside the crate. Nothing here runs (the sketch's bodies are `todo!()`), but every value below
//! comes from a public producer and reaches a public consumer: the anatomy, a filter with a set
//! point, a guarded mode with seeded chance, one law for every capillary bed, compile, a boundary
//! timeline, initialize, one Newton-style stage, one block (beat) stage, quadrature, proposal,
//! validate (accept and refuse), the session step and its fault latch, checkpoint and resume,
//! split measurement, the certificate's history queries, the accuracy gate and observation
//! through a lens.
//!
//! ```
//! use solver_contract_sketch::ng_contract::catalog::{BaroreceptorAfferents, CapillaryAndVenousPressure, Reads};
//! use solver_contract_sketch::ng_contract::*;
//! use solver_contract_sketch::ng_record::*;
//! use solver_contract_sketch::ng_validate::*;
//! use solver_contract_sketch::{ng_observe, ng_receptor, ng_renal};
//!
//! // 0. Anatomy as data: sites with positions, containment, a vascular loop with beds in series,
//! // an enclosure and a served-by edge. The kidney is a region; no law names a site.
//! fn anatomy() -> Result<Schema, AdmissionError> {
//!     let at = |axial: f64| -> Result<Position, AdmissionError> { // synthetic heights
//!         Ok(Position::new(Measure::new(axial)?, Measure::new(0.0)?, Measure::new(0.0)?))
//!     };
//!     let mut map = AnatomyBuilder::new();
//!     let body = map.site::<TissueSite>("body", at(0.0)?);
//!     let kidney = map.site::<TissueSite>("kidney", at(0.5)?);
//!     let heart = map.site::<VesselSegment>("heart", at(1.0)?);
//!     let glomerular = map.site::<CapillaryBed>("glomerular bed", at(0.5)?);
//!     let peritubular = map.site::<CapillaryBed>("peritubular bed", at(0.5)?);
//!     let lung = map.site::<CapillaryBed>("pulmonary bed", at(1.2)?);
//!     let pleura = map.site::<EnclosedSpace>("pleural space", at(1.2)?);
//!     let thoracic = map.site::<NerveSegment>("thoracic segment", at(1.1)?);
//!     for site in [kidney.id(), heart.id(), lung.id(), pleura.id(), thoracic.id()] {
//!         map.contain(body.id(), site);
//!     }
//!     map.contain(kidney.id(), glomerular.id());
//!     map.contain(kidney.id(), peritubular.id());
//!     for (from, to) in [(heart.id(), glomerular.id()), (glomerular.id(), peritubular.id()), (peritubular.id(), heart.id())] {
//!         map.connect(Network::Vascular, from, to); // two beds in series, closing a loop
//!     }
//!     map.connect(Network::Vascular, heart.id(), lung.id());
//!     map.connect(Network::Vascular, lung.id(), heart.id());
//!     let pleural = Compliance::Elastic { elastance: ParameterKey::new("pleura.elastance"),
//!         unstressed: ParameterKey::new("pleura.unstressed") };
//!     map.enclose(&pleura, &[lung.id()], pleural);
//!     map.served_by(lung.id(), &thoracic, Afferent::Nociceptive);
//!     let _kidney: RegionId = map.region("kidney", kidney.id());
//!     Schema::new(map.finish()?)
//! }
//!
//! // 1. Graph construction: NS reads the map's CV-to-NS edge, compares it with a set point that
//! // drifts slowly, and relaxes a filter toward the saturating response.
//! fn baroreflex<'b>(scope: NervousScope<'b>) -> Result<Fragment<'b, Nervous>, AdmissionError> {
//!     let (mut laws, _transmitters) = scope.parts();
//!     let afferents: BaroreceptorAfferents<Reads<'b, Nervous>> = laws.receive();
//!     let filter = laws.state::<Dimensionless>("afferent-filter", Admissible::unit_interval());
//!     let set_point = laws.set_point::<Dimensionless>("stretch-set-point", Admissible::unit_interval());
//!     let chronic = laws.parameter(ParameterKey::new("ns.chronic-set-point"), Admissible::unit_interval());
//!     let inverse_tau = laws.parameter(ParameterKey::new("ns.afferent-rate"), Admissible::nonnegative());
//!     let resetting = laws.parameter(ParameterKey::new("ns.resetting-rate"), Admissible::nonnegative());
//!     let (reflex, slow) = (RateClass::new(RateBand::Reflex, 1_000_000_000)?, RateClass::new(RateBand::Hours, 3_600_000_000_000)?);
//!     let relax = laws.kernel(|mut k| {
//!         let stretch = k.read(&afferents.carotid_sinus_stretch);
//!         let reference = k.state(&set_point);
//!         let error = k.deviation(stretch, reference);
//!         let target = k.logistic(error);
//!         let current = k.state(&filter);
//!         let speed = k.parameter(&inverse_tau);
//!         let rate = k.relax(target, current, speed);
//!         k.finish(rate, laws.every_mode(), reflex)
//!     });
//!     let drift = laws.kernel(|mut k| {
//!         let target = k.parameter(&chronic);
//!         let current = k.state(&set_point);
//!         let speed = k.parameter(&resetting);
//!         let rate = k.relax(target, current, speed);
//!         k.finish(rate, laws.every_mode(), slow)
//!     });
//!     // Where the filter law is trusted: leaving it is reported on the receipt, never refused.
//!     let trusted = ValidityRange::new(Measure::new(-1.0)?, Measure::new(1.0)?, Basis::Fixture("fixture design range"))?;
//!     laws.validity(&relax, trusted);
//!     laws.memory_rate(&filter, relax);
//!     laws.memory_rate(&set_point, drift);
//!     Ok(laws.finish())
//! }
//!
//! // 1b. A mode and its guards: a valve opens when chamber pressure rises past its outflow and
//! // shuts when it falls back. Each guard is a kernel that holds in the mode it leaves.
//! fn valve<'b>(scope: CardiovascularScope<'b>) -> Result<Fragment<'b, Cardiovascular>, AdmissionError> {
//!     let (mut laws, ports) = scope.parts();
//!     let wave = || RateClass::new(RateBand::Wave, 1_000_000);
//!     let outflow = laws.algebraic::<Pressure>("valve-outflow", Admissible::finite());
//!     let [shut, open] = laws.modes("valve", ["shut", "open"]);
//!     let head = |modes, rate_class| laws.kernel(|mut k| {
//!         let upstream = k.algebraic(&ports.chamber_pressure);
//!         let downstream = k.algebraic(&outflow);
//!         let head = k.subtract(upstream, downstream);
//!         k.finish(head, modes, rate_class)
//!     });
//!     let opening = Guard::new(head(shut.only(), wave()?), Crossing::Rising, shut.to(&open)?);
//!     let closing = Guard::new(head(open.only(), wave()?), Crossing::Falling, open.to(&shut)?);
//!     laws.event(opening);
//!     laws.event(closing);
//!     ectopy(&mut laws)?;
//!     Ok(laws.finish())
//! }
//!
//! // 1c. Seeded chance in the body: a hazard accrues in memory at a rate the clock-tied forcing
//! // varies, and fires a mode change when it crosses its stream's draw. The seed fixes the dice;
//! // physiology and treatment set the rate.
//! fn ectopy<'b>(laws: &mut FragmentBuilder<'b, Cardiovascular>) -> Result<(), AdmissionError> {
//!     let dice = laws.stream("ectopic-beat");
//!     let wobble = laws.forcing("hazard-variability", 1_000_000_000)?;
//!     let hazard = laws.state::<Dimensionless>("ectopy-hazard", Admissible::nonnegative());
//!     let ceiling = laws.parameter(ParameterKey::new("cv.ectopy-ceiling"), Admissible::nonnegative());
//!     let pace = laws.parameter(ParameterKey::new("cv.ectopy-rate"), Admissible::nonnegative());
//!     let [sinus, ectopic] = laws.modes("rhythm", ["sinus", "ectopic"]);
//!     let (beat, every) = (RateClass::new(RateBand::Beat, 100_000_000)?, laws.every_mode());
//!     let accrue = laws.kernel(|mut k| {
//!         let limit = k.parameter(&ceiling);
//!         let varied = k.forcing(&wobble);
//!         let target = k.scale(limit, varied);
//!         let current = k.state(&hazard);
//!         let speed = k.parameter(&pace);
//!         let rate = k.relax(target, current, speed);
//!         k.finish(rate, every, beat)
//!     });
//!     // Fires when the integral reaches the stream's exponential draw; the contract resets it.
//!     laws.hazard(HazardGuard::new(&hazard, dice, sinus.to(&ectopic)?));
//!     laws.memory_rate(&hazard, accrue);
//!     Ok(())
//! }
//!
//! // 1d. One law for every capillary bed: Starling filtration declared once for the class and
//! // read at each bed's own pressures; a bed's enclosure sets its extravascular pressure.
//! fn starling<'b>(scope: FluidsScope<'b>) -> Result<Fragment<'b, Fluids>, AdmissionError> {
//!     let (mut laws, ports) = scope.parts();
//!     let route = ports.starling(&mut laws);
//!     let pressures: CapillaryAndVenousPressure<Reads<'b, Fluids>> = laws.receive();
//!     let kf = laws.site_parameter::<CapillaryBed, Conductance<Water>>(ParameterKey::new("fl.kf"),
//!         Admissible::nonnegative());
//!     let minutes = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
//!     let filtration = laws.site_kernel(|mut k| {
//!         let inside = k.at_site(&pressures.capillary_pressure);
//!         let outside = k.at_site(&ports.extravascular_pressure);
//!         let head = k.subtract(inside, outside);
//!         let conductance = k.site_parameter(&kf);
//!         let flow = k.conductance_flow(conductance, head);
//!         k.finish(flow, laws.every_mode(), minutes)
//!     });
//!     laws.transfer_at(&route, filtration);
//!     Ok(laws.finish())
//! }
//!
//! // 2. Compile: organ crates return fragments; held cards are generated from the catalog.
//! fn compile() -> Result<CompiledBody, AdmissionError> {
//!     assemble(anatomy()?, |mut body| {
//!         let kidney = ng_renal::declare(body.renal()?);
//!         body.add(kidney);
//!         let heart = valve(body.cardiovascular()?)?;
//!         body.add(heart);
//!         let fluids = starling(body.fluids()?)?;
//!         body.add(fluids);
//!         let nerves = baroreflex(body.nervous()?)?;
//!         body.add(nerves);
//!         let receptors = ng_receptor::declare(body.receptor()?);
//!         body.add(receptors);
//!         let boundary = boundary_routes(body.boundary()?);
//!         body.add(boundary);
//!         let surrogates = HeldDeclaration::new(vec![HeldField::signal("surrogate AVP, angiotensin II, aldosterone, ANP release")],
//!             vec![HeldSupply::new("surrogate ligand secretion", Book::Moiety(Moiety::HeldLigand))],
//!             vec![79], "arena fixture: fixed gains, no adaptation")?;
//!         let endocrine = endocrine_stand_in(body.hold::<Endocrine>(surrogates)?);
//!         body.add(endocrine);
//!         held::<Respiratory>(&mut body)?;
//!         held::<Metabolism>(&mut body)?;
//!         held::<Musculoskeletal>(&mut body)?;
//!         held::<Hepatic>(&mut body)?;
//!         held::<Gastrointestinal>(&mut body)?;
//!         held::<Immune>(&mut body)?;
//!         held::<Hematology>(&mut body)?;
//!         held::<BloodChemistry>(&mut body)?;
//!         body.compile()
//!     })
//! }
//! fn held<'b, S: OutsideSlice>(body: &mut Assembly<'b>) -> Result<(), AdmissionError> {
//!     let declaration = HeldDeclaration::<S>::new(vec![HeldField::signal(S::CARD)], vec![], vec![], "arena fixture")?;
//!     let fragment = stand_in(body.hold::<S>(declaration)?);
//!     body.add(fragment);
//!     Ok(())
//! }
//!
//! // 2b. A boundary timeline from the payload producers: posture at 07:00 and a meal at 07:30.
//! fn morning(meal_schema: Digest) -> Result<BoundaryPort, AdmissionError> {
//!     let at = |minutes: u64| SimTime::from_ns(minutes * 60_000_000_000);
//!     let id = |name: &[u8]| InputId::new(Digest::of_canonical(name));
//!     let day = ClockPhase::new(0, 86_400_000_000_000)?;
//!     let room = Environment::new(1.0, 1.0, 0.5, 0.0, 1.0, 1.0, 1.0, vec![1.0])?; // synthetic
//!     let seated = Posture::new(90.0, Legs::SeatedDependent)?;
//!     let meal = Composition::new(meal_schema, vec![1.0, 1.0])?; // synthetic, not physiological
//!     let breakfast = Intake::new(IntakeRoute::Oral, meal, Delivery::Continuous { ends: at(65) });
//!     BoundaryPort::new(vec![
//!         BoundaryEvent::new(id(b"clock"), at(0), BoundaryInput::Clock(day)),
//!         BoundaryEvent::new(id(b"room"), at(0), BoundaryInput::Environment(room)),
//!         BoundaryEvent::new(id(b"awake"), at(0), BoundaryInput::Sleep(SleepState::Awake)),
//!         BoundaryEvent::new(id(b"sit"), at(5), BoundaryInput::Posture(seated)),
//!         BoundaryEvent::new(id(b"breakfast"), at(35), BoundaryInput::Intake(breakfast)),
//!     ])
//! }
//!
//! // A solver core, written outside the contract.
//! struct Core {
//!     policy: Digest,
//! }
//! /// Where a deterministic sparse factorization would go. View row `r` is residual `r`, and each
//! /// column's index is its slot in the workspace.
//! fn newton_update(jacobian: SparseView<'_>, residual: &[f64], unknowns: &mut [f64]) {
//!     let offsets = jacobian.row_offsets();
//!     for (r, row) in jacobian.rows().iter().enumerate() {
//!         let _equation = row.index();
//!         let entries = offsets[r] as usize..offsets[r + 1] as usize;
//!         let _pivot = (residual[r], &jacobian.values()[entries.clone()]);
//!         for column in &jacobian.columns()[entries] {
//!             let _slot: &mut f64 = &mut unknowns[column.index()];
//!         }
//!     }
//! }
//!
//! // 3. Initialize once: request, guess and packet go in; provenance comes from the contract.
//! impl Initializer for Core {
//!     fn initialize(&self, system: InitializationSystem<'_>) -> Result<InitialRevision, InitializationFailure> {
//!         let mut workspace = system.workspace();
//!         let residual = system.evaluate(&workspace)?;
//!         let jacobian = system.linearize(&workspace)?;
//!         newton_update(jacobian.view(), residual.residuals(), workspace.unknowns_mut());
//!         system.admit(workspace, MethodHistory::new(self.policy, SimTime::from_ns(0), vec![], vec![]))
//!     }
//! }
//! fn admit<'m>(body: &'m CompiledBody, core: &'m Core) -> Option<Session<'m, Running>> {
//!     let mut packet = ParameterPacket::<SyntheticFixture>::new();
//!     let anchor = Measure::<Pressure>::new(1.0).ok()?; // synthetic, deliberately not physiological
//!     let mean = Synthetic::new(AnchorId::P4, anchor, Unit::canonical(), "arena stand-in").over(AnchorStatistic::CycleMean);
//!     packet.anchor(mean).ok()?;
//!     let kf = Measure::<Conductance<Water>>::new(1.0).ok()?;
//!     packet.synthetic(ParameterKey::new("rn.kf"), kf, Unit::canonical(), "arena design value").ok()?;
//!     let guess = Measure::<Elastance>::new(1.0).ok()?;
//!     let fitted = Calibrated::new(guess, vec![AnchorId::P4]).ok()?;
//!     packet.calibrated::<Elastance>(ParameterKey::new("cv.elastance"), fitted).ok()?;
//!     let parameters = body.bind(packet).ok()?;
//!     let boundary = morning(Digest::of_canonical(b"fixture meal schema")).ok()?;
//!     let request = InitializationRequest::new(InitializationGoal::ConsistentInstant,
//!         InitializationBudget::new(50, 0), InitialStateGuess::PacketValues, SimTime::from_ns(0));
//!     let system = body.initialization(parameters, &boundary, request).ok()?;
//!     let origin = core.initialize(system).ok()?;
//!     let _how = origin.provenance();
//!     let identity = RunIdentity::new(body.model_hash(), origin.packet_hash(),
//!         Seed::new([1, 2, 3, 4]), core.resolution());
//!     Session::admit(body, core, origin, identity, boundary).ok()
//! }
//!
//! impl CoupledSolver for Core {
//!     fn resolution(&self) -> DeclaredResolution {
//!         let backward_euler = StageScheme::tableau(vec![1.0], vec![1.0], vec![1.0], vec![]).unwrap_or_else(|_| panic!("tableau"));
//!         let scales = ErrorScales::uniform(1e-9, 1e-6).unwrap_or_else(|_| panic!("scales"));
//!         DeclaredResolution::new(self.policy, 1_000, ArithmeticProfile::contract_v1(),
//!             StageScheme::step_doubled(backward_euler), scales)
//!     }
//!     fn next_step(&self, problem: &CoupledProblem<'_>, start: AcceptedStateView<'_>)
//!         -> Result<StepTicket, EngineFault> {
//!         let preferred = start.continuation().method_history().integer_counters().first().copied();
//!         let wanted = start.time().plus_ns(preferred.unwrap_or(1_000_000)).ok_or_else(|| {
//!             EngineFault::new(EngineFaultKind::NoAdmissibleStep, start.time(), 0, vec![])
//!         })?;
//!         // The contract clips this at the next breakpoint, which the core never sees.
//!         problem.ticket(start, wanted)
//!     }
//!     fn solve<'p>(&self, problem: &'p CoupledProblem<'_>, start: AcceptedStateView<'_>,
//!         ticket: &StepTicket) -> Result<StepProposal<'p>, TrialFailure> {
//!         // A one-step core keeps no method history across a breakpoint; it restarts step control.
//!         let _restart = ticket.after_breakpoint();
//!         newton_stage(self.policy, problem, start, ticket)
//!     }
//! }
//!
//! // 4. One Newton-style stage: backward Euler over the whole ticket (evaluate + linearize).
//! fn newton_stage<'p>(policy: Digest, problem: &'p CoupledProblem<'_>, start: AcceptedStateView<'_>,
//!     ticket: &StepTicket) -> Result<StepProposal<'p>, TrialFailure> {
//!     let shift = Shift::new(1e9 / ticket.length_ns() as f64)?;
//!     let mut workspace = problem.workspace(start);
//!     let point = workspace.point(ticket.at(ticket.length_ns())?)?;
//!     let residual = problem.evaluate(Scope::Whole, &point)?;
//!     let jacobian = problem.linearize(Scope::Whole, &point, shift)?;
//!     newton_update(jacobian.view(), residual.residuals(), workspace.unknowns_mut());
//!     let _tangent: &mut [f64] = workspace.derivative_mut(); // y' = (y - y0) / h
//!     let point = workspace.point(ticket.at(ticket.length_ns())?)?;
//!     let converged = problem.evaluate(Scope::Whole, &point)?;
//!     // 5. Quadrature: one evaluation at weight 1 is backward Euler's extents and memory increment.
//!     let mut quadrature = problem.quadrature(ticket, EvaluationPlan::Coupled)?;
//!     quadrature.add(&converged, StageWeight::new(1.0)?)?;
//!     let mut dense = problem.dense_output(start, ticket);
//!     dense.node(&point)?;
//!     let end = workspace.candidate()?;
//!     let modes = problem.transitions(start.state(), &end, ticket)?;
//!     // 6. Proposal: amounts are rebuilt from the paired extents, not taken from `end`.
//!     problem.proposal(ticket, end, quadrature.finish()?, modes,
//!         MethodHistory::new(policy, ticket.interval().start(), vec![], vec![]), dense.finish()?)
//! }
//!
//! // 4b. One block stage: dependency blocks evaluated in two bins each (a beat core's shape).
//! fn block_stage<'p>(policy: Digest, problem: &'p CoupledProblem<'_>, start: AcceptedStateView<'_>,
//!     ticket: &StepTicket, plan: EvaluationPlan<'_>) -> Result<StepProposal<'p>, TrialFailure> {
//!     let mut workspace = problem.workspace(start);
//!     let mut quadrature = problem.quadrature(ticket, plan)?;
//!     for block in problem.blocks(plan)? {
//!         let _structure = (block.rows(), block.rate_class().band());
//!         let _why: Option<RowDiagnosis> = block.rows().first().map(|row| problem.diagnose(*row, 0.0));
//!         // A block's local solution goes back to the workspace by each unknown's index.
//!         let local = vec![0.0; block.unknowns().len()];
//!         for (k, id) in block.unknowns().iter().enumerate() {
//!             workspace.unknowns_mut()[id.index()] += local[k];
//!         }
//!         for bin_end in [ticket.length_ns() / 2, ticket.length_ns()] {
//!             let point = workspace.point(ticket.at(bin_end)?)?;
//!             let rates = problem.evaluate(Scope::Block(block), &point)?;
//!             quadrature.add(&rates, StageWeight::new(0.5)?)?;
//!         }
//!     }
//!     let mut dense = problem.dense_output(start, ticket);
//!     dense.node(&workspace.point(ticket.at(ticket.length_ns())?)?)?;
//!     let end = workspace.candidate()?;
//!     let modes = problem.transitions(start.state(), &end, ticket)?;
//!     problem.proposal(ticket, end, quadrature.finish()?, modes,
//!         MethodHistory::new(policy, ticket.interval().start(), vec![], vec![]), dense.finish()?)
//! }
//!
//! // 7. Validate: immutable, returns the whole next image or a refusal. Only `step` publishes.
//! fn dry_validate(session: &Session<'_, Running>, core: &Core) -> Option<NextImage> {
//!     let problem = session.problem();
//!     let start = session.record().accepted();
//!     let ticket = core.next_step(&problem, start).ok()?;
//!     let proposal = core.solve(&problem, start, &ticket).ok()?;
//!     match session.record().validate(proposal) {
//!         Ok(next) => Some(next),
//!         Err(TrialFailure::Refused(refused)) => {
//!             let _smaller: &StepTicket = refused.retry().ticket();
//!             None
//!         }
//!         Err(TrialFailure::EngineFault(_latches)) => None,
//!     }
//! }
//!
//! // 8. Step: accept, refuse and retry smaller, or latch on a fault and recover on a new branch.
//! fn advance<'m>(session: Session<'m, Running>, earlier: &Checkpoint)
//!     -> Result<Session<'m, Running>, CheckpointError> {
//!     match session.step() {
//!         Step::Accepted(running, _receipt) => Ok(running),
//!         Step::Refused(running, refused) => match running.retry(refused) {
//!             Step::Accepted(running, _) | Step::Refused(running, _) => Ok(running),
//!             Step::Faulted(stopped) => stopped.recover(earlier, BranchId::new(Digest::of_canonical(b"b2"))),
//!         },
//!         Step::Faulted(stopped) => {
//!             let _report: FaultReport = stopped.export();
//!             stopped.recover(earlier, BranchId::new(Digest::of_canonical(b"b1")))
//!         }
//!     }
//! }
//!
//! // 9. Checkpoint and resume: state, continuation and the last dense segment together.
//! fn resume<'m>(session: &Session<'m, Running>, body: &'m CompiledBody, core: &'m Core,
//!     history: &dyn HistoryStore) -> Result<Session<'m, Running>, CheckpointError> {
//!     let codec = PostcardCheckpointCodec;
//!     let sealed = codec.encode(&session.checkpoint())?;
//!     let stored: Vec<u8> = sealed.as_bytes().to_vec(); // to durable storage and back
//!     let bytes = CanonicalCheckpoint::from_bytes(stored);
//!     let resumed = codec.decode(&bytes, session.record().identity(), body, core, history)?;
//!     let _segment = resumed.record().accepted().last_segment().basis().degree();
//!     Ok(resumed)
//! }
//!
//! // 10. Split measurement: a qualification only from four-family traces and a measured reference error.
//! fn qualify(reference: &PatientRecord, refined: &PatientRecord, split: &PatientRecord)
//!     -> Option<SplitQualification> {
//!     let grid = || TraceGrid::new(SimTime::from_ns(0), 1_000_000, 7_200).ok();
//!     let reference = reference.split_trace(grid()?).ok()?;
//!     let error = ReferenceError::self_convergence(&reference, &refined.split_trace(grid()?).ok()?).ok()?;
//!     let budget = SplitBudget::new([1.0; 4], 0.25).ok()?;
//!     SplitMeasurement::new(reference, split.split_trace(grid()?).ok()?, error, budget).ok()?.qualify().ok()
//! }
//!
//! // 11. Certificate: one sealed view of one record, with the segment queries it needs.
//! fn certify_run(session: &Session<'_, Running>, body: &CompiledBody) -> Option<Certificate> {
//!     let view = session.record().conservation();
//!     let _sealed: &HistorySeal = view.seal();
//!     let tolerances = view.basis().combinations().iter()
//!         .map(|combination| QuantityTolerance::new(combination.id(), 1e-9, 1e-9))
//!         .collect::<Result<Vec<_>, _>>().ok()?;
//!     let windows = Sliding24Hours::covering(48 * 3_600_000_000_000).ok()?;
//!     let policy = CertificatePolicy::new(body.conserved_basis(), tolerances, ExpectedDrift::Zero, windows).ok()?;
//!     let first = view.basis().combinations().first()?.id();
//!     let held = view.basis().coverage().held_routes();
//!     for segment in view.segments(&view.span()) {
//!         let _shape = (segment.interval().start(), segment.basis().family(), segment.basis().degree());
//!         let _inventory = segment.inventory(first).coefficients();
//!         let _exchange = (segment.inflow(first).start(), segment.outflow(first).coefficients());
//!         for jump in segment.impulses() {
//!             let _bolus = (jump.at(), jump.quantity(), jump.amount());
//!         }
//!         for route in held {
//!             let _support = (route.name(), route.quantity(), segment.held_inflow(route.id()).start(),
//!                 segment.held_outflow(route.id()).coefficients());
//!         }
//!     }
//!     certify(&view, &policy).ok()
//! }
//!
//! // 12. Accuracy: the core on manufactured cases and against the contract's reference integrator.
//! fn accuracy_gate(body: &CompiledBody, core: &Core) -> Option<AccuracyEvidence> {
//!     let cases = vec![AccuracyCase::TwoReservoir, AccuracyCase::DrainAndRefill,
//!         AccuracyCase::SliceReference { factor: 100 }];
//!     let _sources: &[RefusalSource] = body.refusal_sources();
//!     let policy = AccuracyPolicy::new(cases, ReferenceIntegrator::new(100_000).ok()?, 1e-9, 1e-6).ok()?;
//!     accuracy(body, core, core, &policy).ok()
//! }
//!
//! // 13. Observe through a lens compiled apart from the body: read-only, one revision at a time,
//! // with the observer's own state checkpointed beside the record's.
//! fn chart(session: &Session<'_, Running>, body: &CompiledBody) -> Option<Vec<(SimTime, Vec<f64>)>> {
//!     let pressure = ChannelSpec::new("arterial pressure", "cv.arterial-pressure", None, Access::Device)
//!         .transformed(Transform::new(0, 0));
//!     let lens = body.lens(ObservationCatalog::new(vec![pressure], OutputGrid::new(4_000_000).ok()?).ok()?).ok()?;
//!     let view = session.observation(&lens).ok()?;
//!     let mut monitor = ng_observe::Observer::new(&lens, ObservationSeed::new([5, 6, 7, 8]));
//!     let _findings = monitor.advance(&view).ok()?;
//!     let _saved = monitor.checkpoint();
//!     let channels: Vec<ChannelId> = view.channels().iter().map(|channel| channel.id()).collect();
//!     let _now = channels.iter().map(|&channel| view.value(channel)).collect::<Result<Vec<_>, _>>().ok()?;
//!     let mut samples = Vec::new();
//!     for at in view.grid() {
//!         let row = channels.iter().map(|&channel| view.sample(channel, at)).collect::<Result<_, _>>().ok()?;
//!         samples.push((at, row));
//!     }
//!     Some(samples)
//! }
//! ```
