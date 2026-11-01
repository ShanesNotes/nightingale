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
//!     let wobble = laws.forcing("hazard-variability", ParameterKey::new("cv.hazard-period"),
//!         ParameterKey::new("cv.hazard-amplitude"));
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
//!     // Fires when the integral reaches the stream's exponential draw; the contract integrates the rate's
//!     // positive part, so a target the forcing lowers cannot lower the integral, and resets it on firing.
//!     laws.hazard(HazardGuard::new(&hazard, &dice, sinus.to(&ectopic)?));
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
//!     for r in 0..residual.len() {
//!         let entries = offsets[r]..offsets[r + 1];
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
//!         system.admit(workspace, MethodHistory::new(self.policy, SimTime::from_ns(0), vec![]))
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
//!     let system = body.initialization(parameters, boundary, request).ok()?;
//!     let origin = core.initialize(system).ok()?;
//!     let _how = origin.provenance();
//!     let identity = RunIdentity::new(body.model_hash(), origin.packet_hash(),
//!         Seed::new([1, 2, 3, 4]), core.resolution());
//!     Session::admit(body, core, origin, identity).ok()
//! }
//!
//! impl CoupledSolver for Core {
//!     fn resolution(&self) -> DeclaredResolution {
//!         let scale = |value| Measure::<Dimensionless>::new(value).unwrap_or_else(|_| panic!("a scale"));
//!         let scales = ErrorScales::uniform(scale(1e-9), scale(1e-6)).unwrap_or_else(|_| panic!("scales"));
//!         DeclaredResolution::new(self.policy, 1_000, 12, ArithmeticProfile::contract_v1(),
//!             StageScheme::backward_euler_doubled(), scales)
//!     }
//!     fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
//!         let preferred = accepted.method_history().counters().first().copied();
//!         // A commitment: the session issues the ticket, and no core asks where a breakpoint is.
//!         preferred.unwrap_or(1_000_000)
//!     }
//!     fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket)
//!         -> Result<StepProposal, TrialFailure> {
//!         // A one-step core keeps no method history across a breakpoint; it restarts step control.
//!         let _restart = ticket.after_breakpoint();
//!         newton_stage(self.policy, problem, ticket)
//!     }
//! }
//!
//! // 4. Newton-style stages: backward Euler over the whole ticket and over its two parts
//! // (evaluate + linearize).
//! fn newton_stage(policy: Digest, problem: &CoupledProblem<'_>, ticket: &StepTicket)
//!     -> Result<StepProposal, TrialFailure> {
//!     let (length, part) = (ticket.length_ns(), ticket.length_ns() / 2);
//!     let stage = |workspace: &mut SolveWorkspace, offset: u64, span: u64|
//!         -> Result<StageEvaluation, TrialFailure> {
//!         let shift = ticket.shift(span)?;
//!         let point = workspace.point(ticket.at(offset)?);
//!         let residual = problem.evaluate(&point)?;
//!         let jacobian = problem.linearize(&point, shift)?;
//!         newton_update(jacobian.view(), residual.residuals(), workspace.unknowns_mut());
//!         let _tangent: &mut [f64] = workspace.derivative_mut(); // y' = (y - y0) / h
//!         problem.evaluate(&workspace.point(ticket.at(offset)?))
//!     };
//!     let whole = stage(&mut problem.workspace(), length, length)?;
//!     let mut workspace = problem.workspace();
//!     let first = stage(&mut workspace, part, part)?;
//!     let second = stage(&mut workspace, length, length - part)?;
//!     // 5. Quadrature: the whole stage reads the error at weight zero; its parts carry the increment.
//!     let mut quadrature = problem.quadrature(ticket);
//!     quadrature.add(&whole, Ratio::ZERO)?;
//!     quadrature.add(&first, ticket.fraction(part))?;
//!     quadrature.add(&second, ticket.fraction(length - part))?;
//!     // 6. Proposal: amounts are rebuilt from the paired extents, not taken from the workspace. A core
//!     // says nothing of a guard: the attempt reads a crossing from the verified state and re-solves shorter.
//!     Ok(problem.propose(ticket, quadrature.finish()?, &workspace,
//!         MethodHistory::new(policy, ticket.interval().start(), vec![])))
//! }
//!
//! // 4b. One block stage: dependency blocks evaluated in two bins each (a beat core's shape).
//! fn block_stage(policy: Digest, problem: &CoupledProblem<'_>, ticket: &StepTicket,
//!     plan: EvaluationPlan<'_>) -> Result<StepProposal, TrialFailure> {
//!     let mut workspace = problem.workspace();
//!     let mut quadrature = problem.quadrature_for(ticket, plan)?;
//!     let half = ticket.fraction(ticket.length_ns() / 2);
//!     for block in problem.blocks(plan)? {
//!         let _structure = (block.rows(), block.rate_class().band());
//!         // A block's local solution goes back to the workspace by each unknown's index.
//!         let local = vec![0.0; block.unknowns().len()];
//!         for (k, id) in block.unknowns().iter().enumerate() {
//!             workspace.unknowns_mut()[id.index()] += local[k];
//!         }
//!         for bin_end in [ticket.length_ns() / 2, ticket.length_ns()] {
//!             let point = workspace.point(ticket.at(bin_end)?);
//!             let rates = problem.evaluate_block(block, &point)?;
//!             quadrature.add(&rates, half)?;
//!         }
//!     }
//!     Ok(problem.propose(ticket, quadrature.finish()?, &workspace,
//!         MethodHistory::new(policy, ticket.interval().start(), vec![])))
//! }
//!
//! // 7. Validate: immutable, returns the whole next image or a refusal. Only `step` publishes.
//! fn dry_validate(session: &Session<'_, Running>, core: &Core) -> Option<NextImage> {
//!     let record = session.record();
//!     let span_ns = core.next_step(record.accepted());
//!     match record.attempt(core, Attempt::First { span_ns }) {
//!         Ok(next) => Some(next),
//!         Err(TrialFailure::Refused(refused)) => record.attempt(core, Attempt::After(&refused)).ok(),
//!         Err(TrialFailure::EngineFault(_latches)) => None,
//!     }
//! }
//!
//! // 8. Step: accept, refuse and retry smaller, or latch on a fault and recover on a new branch.
//! fn advance<'m>(session: Session<'m, Running>, earlier: &Checkpoint, history: &mut dyn HistoryStore)
//!     -> Result<Session<'m, Running>, CheckpointError> {
//!     let mut attempt = session.step();
//!     loop {
//!         attempt = match attempt {
//!             Step::Accepted(running, _receipt) => return Ok(running),
//!             Step::Refused(retrying) => {
//!                 let _why = retrying.refused().reason();
//!                 retrying.retry()
//!             }
//!             Step::Faulted(stopped) => {
//!                 let _report: FaultReport = stopped.export(history)?;
//!                 return stopped.recover(earlier, BranchId::new(Digest::of_canonical(b"b1")));
//!             }
//!         };
//!     }
//! }
//!
//! // 9. Checkpoint and resume: state, continuation and the last dense segment together.
//! fn resume<'m>(session: &Session<'m, Running>, body: &'m CompiledBody, core: &'m Core,
//!     history: &mut dyn HistoryStore) -> Result<Session<'m, Running>, CheckpointError> {
//!     let saved = session.checkpoint(history)?;
//!     let stored: Vec<u8> = saved.as_bytes().to_vec(); // to durable storage and back
//!     let bytes = Checkpoint::from_bytes(stored);
//!     let resumed = Session::resume(&bytes, session.record().identity(), body, core, history)?;
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
//!     let _sealed: HistorySeal = view.seal();
//!     let limit = |value| Measure::<Water>::new(value).ok();
//!     let mut quantities = Vec::new();
//!     for combination in view.basis().combinations() {
//!         let relative = Measure::new(1e-9).ok()?;
//!         quantities.push(QuantityPolicy::new(combination.id(), limit(1e-9)?, relative, limit(1.0)?).ok()?);
//!     }
//!     let windows = Sliding24Hours::covering(48 * 3_600_000_000_000).ok()?;
//!     let policy = CertificatePolicy::new(body.conserved_basis(), quantities, ExpectedDrift::Zero, windows).ok()?;
//!     let first = view.basis().combinations().first()?.id();
//!     let held = view.basis().coverage().held_routes();
//!     for segment in view.segments() {
//!         let piece = segment.interval();
//!         let _shape = (piece.start(), segment.basis().family(), segment.basis().degree());
//!         let _inventory = segment.inventory(first).on(piece);
//!         let _exchange = (segment.inflow(first).on(piece), segment.outflow(first).on(piece));
//!         for jump in segment.impulses() {
//!             let _bolus = (jump.at(), jump.quantity(), jump.amount());
//!         }
//!         for route in held {
//!             let _support = (route.name(), route.quantity(), segment.held_inflow(route.id()).on(piece),
//!                 segment.held_outflow(route.id()).on(piece));
//!         }
//!     }
//!     certify(&view, &policy).ok()
//! }
//!
//! // 12. Accuracy: the core on manufactured cases and against the contract's reference integrator.
//! fn accuracy_gate(body: &CompiledBody, core: &Core) -> Option<AccuracyEvidence> {
//!     let reference = ReferenceIntegrator::new(100_000);
//!     let cases = vec![AccuracyCase::TwoReservoir(TwoReservoirs::stepped_inflow().ok()?),
//!         AccuracyCase::DrainAndRefill, AccuracyCase::SliceReference { reference, factor: 100 }];
//!     let _sources: &[RefusalSource] = body.refusal_sources();
//!     let policy = AccuracyPolicy::new(cases, Measure::new(1e-9).ok()?, Measure::new(1e-6).ok()?, 1_000_000).ok()?;
//!     accuracy(core, &policy).ok()
//! }
//!
//! // 13. Observe through a lens compiled apart from the body: read-only, one revision at a time,
//! // with the observer's own state checkpointed beside the record's.
//! fn chart(session: &Session<'_, Running>, body: &CompiledBody) -> Option<Vec<ng_observe::Observation>> {
//!     let pressure = RowKind::Algebraic { owner: "CV", local: "arterial-pressure" };
//!     let channel = ChannelSpec::new("arterial line", pressure);
//!     let lens = body.lens(vec![channel], core::num::NonZeroU64::new(4_000_000)?).ok()?;
//!     let view = session.observation(&lens).ok()?;
//!     let _samples: Vec<(SimTime, Vec<Compensated>)> = view.samples();
//!     let mut monitor = ng_observe::Observer::new(&lens, ObservationSeed::new([5, 6, 7, 8]));
//!     let observations = monitor.advance(&view).ok()?;
//!     let _saved = monitor.checkpoint();
//!     Some(observations)
//! }
//! ```
