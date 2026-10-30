//! Solver and core contract sketch: organ laws are immutable data compiled by the proposed
//! ng-contract. Module names mirror proposed crates in the crate table (solver-contract.md
//! section 1), not installed crates. Bodies are unimplemented; doctests prove API shape only.
//! The crate documentation below is `docs/architecture/solver-contract.md`; its three caller
//! examples compile as doctests, and [`signature_walk`] walks every seam from outside.
//!
//! The passing example type-checks an RN declaration without invoking a stub.
//! ```
//! use solver_contract_sketch::{ng_contract::*, ng_renal};
//! fn kidney<'b>(scope: RenalScope<'b>) -> Fragment<'b, Renal> {
//!     ng_renal::declare(scope)
//! }
//! let _declaration = kidney;
//! ```
//!
//! # Forbidden moves
//!
//! No organ can name or hold an authoritative pool; even the pool handle is private.
//! ```compile_fail,E0603
//! use solver_contract_sketch::ng_contract::PoolRef;
//! fn steal(_: PoolRef) {}
//! ```
//!
//! A vital is an observation; the accepted record has no setter for it.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_record::PatientRecord;
//! fn overwrite(record: &mut PatientRecord) { record.set_mean_arterial_pressure(1.0); }
//! ```
//!
//! A transfer cannot be made with just its donor. The only constructor takes a pair.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn unpaired<'b>(donor: Endpoint<'b, Renal, Water>, rate: PureKernel<'b, Renal, Rate<Water>>)
//!     -> Transfer<'b, Renal, Water> {
//!     Transfer::new(&donor, rate)
//! }
//! ```
//!
//! The neighbouring organ's internal law state is not a public input.
//! ```compile_fail,E0603
//! use solver_contract_sketch::ng_cardiovascular::internals::ChamberMemory;
//! fn steal(_: ChamberMemory) {}
//! ```
//!
//! A physiological anchor needs a Citation; provenance cannot be omitted.
//! ```compile_fail,E0061
//! use solver_contract_sketch::ng_contract::*;
//! fn uncited(value: Measure<Pressure>, unit: Unit<Pressure>) -> Specified<Pressure> {
//!     Specified::new(AnchorId::P4, value, unit)
//! }
//! ```
//!
//! An engine fault cannot be converted to physiological demise.
//! ```compile_fail,E0277
//! use solver_contract_sketch::ng_contract::{EngineFault, Demise};
//! fn hide_bug(fault: EngineFault) -> Demise { fault.into() }
//! ```
//!
//! Material cannot be hidden in an organ's private differential state.
//! ```compile_fail,E0277
//! use solver_contract_sketch::ng_contract::*;
//! fn shadow<'b>(scope: &mut FragmentBuilder<'b, Renal>, domain: Admissible<Water>) {
//!     scope.state::<Water>("shadow-water", domain);
//! }
//! ```
//!
//! A renal law cannot take a cardiovascular route capability.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn foreign<'b>(scope: &mut FragmentBuilder<'b, Renal>, route: Pair<'b, Cardiovascular, Water>,
//!     rate: PureKernel<'b, Renal, Rate<Water>>) {
//!     scope.transfer(Transfer::new(&route, rate));
//! }
//! ```
//!
//! An expression cannot leave its kernel: one read in a finished kernel does not type-check in
//! the next (the judge's cross-kernel probe, in the scoped API).
//! ```compile_fail,E0521
//! use solver_contract_sketch::ng_contract::*;
//! type Close<'b> = (ModeSet<'b, Renal>, RateClass);
//! fn cross<'b>(laws: &FragmentBuilder<'b, Renal>, port: &ReadPort<'b, Renal, Pressure>,
//!     first: Close<'b>, second: Close<'b>) -> PureKernel<'b, Renal, Pressure> {
//!     let mut stolen = None;
//!     let _first = laws.kernel(|mut k| {
//!         stolen = Some(k.read(port));
//!         let own = k.read(port);
//!         k.finish(own, first.0, first.1)
//!     });
//!     laws.kernel(|k| k.finish(stolen.unwrap(), second.0, second.1))
//! }
//! ```
//! Legal twin: each kernel reads the port itself.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! type Close<'b> = (ModeSet<'b, Renal>, RateClass);
//! fn own<'b>(laws: &FragmentBuilder<'b, Renal>, port: &ReadPort<'b, Renal, Pressure>,
//!     first: Close<'b>, second: Close<'b>) -> PureKernel<'b, Renal, Pressure> {
//!     let _first = laws.kernel(|mut k| {
//!         let own = k.read(port);
//!         k.finish(own, first.0, first.1)
//!     });
//!     laws.kernel(|mut k| {
//!         let own = k.read(port);
//!         k.finish(own, second.0, second.1)
//!     })
//! }
//! ```
//!
//! Two kernels open from one fragment builder cannot share an expression (the judge's
//! double-builder probe, in the scoped API).
//! ```compile_fail,E0521
//! use solver_contract_sketch::ng_contract::*;
//! type Close<'b> = (ModeSet<'b, Renal>, RateClass);
//! fn cross<'b>(laws: &FragmentBuilder<'b, Renal>, port: &ReadPort<'b, Renal, Pressure>,
//!     outer: Close<'b>, inner: Close<'b>) -> PureKernel<'b, Renal, Pressure> {
//!     laws.kernel(|mut first| {
//!         let foreign = first.read(port);
//!         let _second = laws.kernel(|mut second| {
//!             let own = second.read(port);
//!             let mixed = second.add(own, foreign);
//!             second.finish(mixed, inner.0, inner.1)
//!         });
//!         let own = first.read(port);
//!         first.finish(own, outer.0, outer.1)
//!     })
//! }
//! ```
//! Legal twin: the inner kernel builds from its own reads.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! type Close<'b> = (ModeSet<'b, Renal>, RateClass);
//! fn nested<'b>(laws: &FragmentBuilder<'b, Renal>, port: &ReadPort<'b, Renal, Pressure>,
//!     outer: Close<'b>, inner: Close<'b>) -> PureKernel<'b, Renal, Pressure> {
//!     laws.kernel(|mut first| {
//!         let _second = laws.kernel(|mut second| {
//!             let a = second.read(port);
//!             let b = second.read(port);
//!             let sum = second.add(a, b);
//!             second.finish(sum, inner.0, inner.1)
//!         });
//!         let own = first.read(port);
//!         first.finish(own, outer.0, outer.1)
//!     })
//! }
//! ```
//!
//! The certificate cannot pair one record's inventories with another's ledger: records lend no
//! separate histories (the judge's mixed-record probe).
//! ```compile_fail,E0599
//! use solver_contract_sketch::{ng_record::PatientRecord, ng_validate::*};
//! fn cross(first: &PatientRecord, second: &PatientRecord, policy: &CertificatePolicy)
//!     -> Result<Certificate, CertificateFailure> {
//!     certify(first.inventory_history(), second.ledger(), policy)
//! }
//! ```
//! Legal twin: one sealed view of one record.
//! ```
//! use solver_contract_sketch::{ng_record::PatientRecord, ng_validate::*};
//! fn one(record: &PatientRecord, policy: &CertificatePolicy)
//!     -> Result<Certificate, CertificateFailure> {
//!     certify(&record.conservation(), policy)
//! }
//! ```
//!
//! A faulted session is latched: it can be inspected, exported or branched from, never stepped.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_record::*;
//! fn again(stopped: Session<'_, Faulted>) -> Step<'_> { stopped.step() }
//! ```
//! Legal twin: a running session steps.
//! ```
//! use solver_contract_sketch::ng_record::*;
//! fn once(running: Session<'_, Running>) -> Step<'_> { running.step() }
//! ```
//!
//! Publication happens only inside the session; a lent record is read-only.
//! ```compile_fail,E0596
//! use solver_contract_sketch::ng_record::*;
//! fn sneak(session: &Session<'_, Running>, next: NextImage) -> StepReceipt {
//!     session.record().publish(next)
//! }
//! ```
//!
//! Only the receiver the coupling map draws can read an edge: RN cannot read the baroreceptor
//! afferents drawn CV to NS.
//! ```compile_fail,E0271
//! use solver_contract_sketch::ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Renal>) {
//!     let _stretch = laws.receive::<BaroreceptorAfferents<_>>();
//! }
//! ```
//! Legal twin: NS reads them.
//! ```
//! use solver_contract_sketch::ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Nervous>) -> BaroreceptorAfferents<Reads<'b, Nervous>> {
//!     laws.receive::<BaroreceptorAfferents<_>>()
//! }
//! ```
//!
//! A synthetic value cannot enter a reviewed packet as an anchor.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn launder(packet: &mut ParameterPacket<Reviewed>, fake: Synthetic<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.anchor(fake)
//! }
//! ```
//! Legal twin: the arena's fixture packet holds synthetic anchors.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn fixture(packet: &mut ParameterPacket<SyntheticFixture>, value: Synthetic<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.anchor(value)
//! }
//! ```
//!
//! Split qualification is not a byte array.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn unsupported_split() -> EvaluationPlan<'static> { EvaluationPlan::QualifiedSplit(&[0; 32]) }
//! ```
//! Legal twin: it comes only from a four-family measurement.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn measured(measurement: SplitMeasurement) -> Result<SplitQualification, SplitRejected> {
//!     measurement.qualify()
//! }
//! ```
//!
//! Nor from invented samples: a split trace comes only from a record's accepted history (the
//! judge's fabricated-split probe).
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn invented(id: &RunIdentity, grid: TraceGrid, state: FamilySamples<StateFamily>,
//!     exchange: FamilySamples<ExchangeFamily>, observable: FamilySamples<ObservableFamily>,
//!     events: FamilySamples<EventTimeFamily>) -> SplitTrace {
//!     SplitTrace::new(id, grid, state, exchange, observable, events)
//! }
//! ```
//! Legal twin: the record samples its own sealed history.
//! ```
//! use solver_contract_sketch::{ng_contract::*, ng_record::PatientRecord};
//! fn sampled(record: &PatientRecord, grid: TraceGrid) -> Result<SplitTrace, AdmissionError> {
//!     record.split_trace(grid)
//! }
//! ```
//!
//! An admissible set has no bounds constructor, so a reference range cannot become a refusal rule
//! and a deteriorating patient keeps publishing.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn refuse_low_pressure(low: Measure<Pressure>, high: Measure<Pressure>) -> Admissible<Pressure> {
//!     Admissible::new(low, high)
//! }
//! ```
//!
//! Nor can a threshold be smuggled in as a sign constraint on a computed expression: a refusal set
//! attaches to a slot, and a kernel's output takes none (the judge's shifted-reference probe).
//! ```compile_fail,E0061
//! use solver_contract_sketch::ng_contract::*;
//! fn threshold<'b>(laws: &FragmentBuilder<'b, Renal>, pressure: &ReadPort<'b, Renal, Pressure>,
//!     reference: &Parameter<'b, Renal, Pressure>, modes: ModeSet<'b, Renal>, class: RateClass)
//!     -> PureKernel<'b, Renal, Pressure> {
//!     laws.kernel(|mut k| {
//!         let p = k.read(pressure);
//!         let a = k.parameter(reference);
//!         let shifted = k.subtract(p, a);
//!         k.finish(shifted, Admissible::nonnegative(), modes, class)
//!     })
//! }
//! ```
//! Legal twin: the kernel closes without a refusal set; where it is trusted is a range that only
//! reports, and a slot takes one of the closed sets.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn declare<'b>(laws: &mut FragmentBuilder<'b, Renal>, pressure: &ReadPort<'b, Renal, Pressure>,
//!     reference: &Parameter<'b, Renal, Pressure>, modes: ModeSet<'b, Renal>, class: RateClass,
//!     low: Measure<Pressure>, high: Measure<Pressure>) -> Result<StateSlot<'b, Renal, Dimensionless>, AdmissionError> {
//!     let shifted = laws.kernel(|mut k| {
//!         let p = k.read(pressure);
//!         let a = k.parameter(reference);
//!         let difference = k.subtract(p, a);
//!         k.finish(difference, modes, class)
//!     });
//!     laws.validity(&shifted, ValidityRange::new(low, high, Basis::Fixture("fixture design range"))?);
//!     Ok(laws.state::<Dimensionless>("occupancy-like memory", Admissible::unit_interval()))
//! }
//! ```
//!
//! A held stand-in cannot enter the body without declaring what it supplies.
//! ```compile_fail,E0061
//! use solver_contract_sketch::ng_contract::*;
//! fn undeclared<'b>(body: &mut Assembly<'b>) -> Result<HeldScope<'b, Endocrine>, AdmissionError> {
//!     body.hold::<Endocrine>()
//! }
//! ```
//! Legal twin: the declaration names its supply routes and the feedback it does not answer.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn declared<'b>(body: &mut Assembly<'b>) -> Result<HeldScope<'b, Endocrine>, AdmissionError> {
//!     let declaration = HeldDeclaration::new(vec![HeldField::signal("surrogate AVP, angiotensin II, aldosterone, ANP release")],
//!         vec![HeldSupply::new("surrogate ligand secretion", Book::Moiety(Moiety::HeldLigand))],
//!         vec![79], "arena fixture: fixed gains, no adaptation")?;
//!     body.hold::<Endocrine>(declaration)
//! }
//! ```
//!
//! Extents cannot be booked as one sampled rate times the step: only the quadrature makes them.
//! ```compile_fail,E0451
//! use solver_contract_sketch::ng_contract::*;
//! fn sampled(layout: Digest, forward: Vec<Compensated>, reverse: Vec<Compensated>) -> Extents {
//!     // One sampled rate times the step, booked by hand.
//!     Extents { _layout: layout, _forward: forward, _reverse: reverse }
//! }
//! ```
//! Legal twin: weighted stage evaluations, finished by the quadrature.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn integrated(mut quadrature: Quadrature, stage: &StageEvaluation) -> Result<Increment, EngineFault> {
//!     quadrature.add(stage, Ratio::ONE)?;
//!     quadrature.finish()
//! }
//! ```
//!
//! A reaction cannot be written from raw stoichiometry; a scope issues it after element and charge
//! accounting.
//! ```compile_fail,E0451
//! use solver_contract_sketch::ng_contract::*;
//! use core::marker::PhantomData;
//! fn make_sodium<'b>(brand: Brand<'b>) -> BalancedReaction<'b, Receptor> {
//!     BalancedReaction { _stoichiometry: vec![(0, 1)], _brand: brand, _owner: PhantomData }
//! }
//! ```
//! Legal twin: the receptor scope issues its balanced bindings.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn bindings<'b>(scope: ReceptorScope<'b>) -> Vec<BalancedReaction<'b, Receptor>> {
//!     scope.parts().1.bindings
//! }
//! ```
//!
//! A kernel cannot reach host math, whose precision Rust leaves unspecified: an expression carries
//! no value to call `f64::exp` on.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn host<'b, 'k>(x: Expr<'b, 'k, Renal, Dimensionless>) -> f64 {
//!     x.value().exp()
//! }
//! ```
//! Legal twin: the contract's pinned operator.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn pinned<'b, 'k>(k: &mut Kernel<'b, 'k, Renal>, x: Expr<'b, 'k, Renal, Dimensionless>)
//!     -> Expr<'b, 'k, Renal, Dimensionless> {
//!     k.exp(x)
//! }
//! ```
//!
//! A guard fires only its own owner's mode transition: a renal guard cannot switch a CV valve.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn foreign<'b>(zero: PureKernel<'b, Renal, Pressure>, valve: ModeTransition<'b, Cardiovascular>)
//!     -> Guard<'b, Renal, Pressure> {
//!     Guard::new(zero, Crossing::Rising, valve)
//! }
//! ```
//! Legal twin: the owner's own transition.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn own<'b>(zero: PureKernel<'b, Renal, Pressure>, transition: ModeTransition<'b, Renal>)
//!     -> Guard<'b, Renal, Pressure> {
//!     Guard::new(zero, Crossing::Rising, transition)
//! }
//! ```
//!
//! A law declared for one site class reads only that class's entries: a capillary-bed law cannot
//! read a junction's value.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn crossed<'b>(laws: &FragmentBuilder<'b, Fluids>, released: &ReadPort<'b, Fluids, PerSite<Junction, Dimensionless>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Dimensionless, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let value = k.at_site(released);
//!         k.finish(value, laws.every_mode(), class)
//!     })
//! }
//! ```
//! Legal twin: the bed law reads the bed's own entry.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn own<'b>(laws: &FragmentBuilder<'b, Fluids>, pressure: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Pressure, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let value = k.at_site(pressure);
//!         k.finish(value, laws.every_mode(), class)
//!     })
//! }
//! ```
//!
//! A set point is not the variable it guards: no law can add, subtract or relax across the two.
//! ```compile_fail,E0308
//! use solver_contract_sketch::ng_contract::*;
//! fn as_pressure<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, pressure: Expr<'b, 'k, Nervous, Pressure>,
//!     set_point: &StateSlot<'b, Nervous, Reference<Pressure>>) -> Expr<'b, 'k, Nervous, Pressure> {
//!     let reference = k.state(set_point);
//!     k.subtract(pressure, reference)
//! }
//! ```
//! Legal twin: the deviation operator compares them.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn error<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, pressure: Expr<'b, 'k, Nervous, Pressure>,
//!     set_point: &StateSlot<'b, Nervous, Reference<Pressure>>) -> Expr<'b, 'k, Nervous, Pressure> {
//!     let reference = k.state(set_point);
//!     k.deviation(pressure, reference)
//! }
//! ```
//!
//! A product gets a name only through the pinned identity table, never by cast: volume times
//! volume is not a pressure.
//! ```compile_fail,E0277
//! use solver_contract_sketch::ng_contract::*;
//! fn invented<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, a: Expr<'b, 'k, Cardiovascular, Volume>,
//!     b: Expr<'b, 'k, Cardiovascular, Volume>) -> Expr<'b, 'k, Cardiovascular, Pressure> {
//!     let product = k.multiply(a, b);
//!     k.as_dimension(product)
//! }
//! ```
//! Legal twin: elastance times volume is a pressure.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn pressure<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, e: Expr<'b, 'k, Cardiovascular, Elastance>,
//!     v: Expr<'b, 'k, Cardiovascular, Volume>) -> Expr<'b, 'k, Cardiovascular, Pressure> {
//!     let product = k.multiply(e, v);
//!     k.as_dimension(product)
//! }
//! ```
//!
//! A core cannot see the boundary timeline, so no step can depend on a later input (the causal
//! step rule): the record cuts a passed span at the next breakpoint.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn peek(problem: &CoupledProblem<'_>) -> Option<SimTime> {
//!     problem.boundary().next_breakpoint(problem.accepted().time())
//! }
//! ```
//! Legal twin: the core commits to a span from the accepted view, and the session issues the ticket.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn step(accepted: AcceptedStateView<'_>, intrinsic_span_ns: u64)
//!     -> u64 {
//!     accepted.method_history().counters().first().copied().unwrap_or(intrinsic_span_ns)
//! }
//! ```
//!
//! The solver holds no dice: chance is drawn inside a law from a keyed stream.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn solver_dice(problem: &CoupledProblem<'_>) -> f64 { problem.threshold(0, 1) }
//! ```
//! Legal twin: a kernel draws on its owner's stream.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn law_dice<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, stream: &Stream<'b, Cardiovascular>)
//!     -> Expr<'b, 'k, Cardiovascular, Dimensionless> {
//!     k.draw(stream)
//! }
//! ```
//!
//! Seed domains never mix: an observer cannot draw measurement error from the run seed.
//! ```compile_fail,E0308
//! use solver_contract_sketch::{ng_contract::*, ng_observe::Observer};
//! fn noisy<'l>(lens: &'l ObservationLens, run: Seed) -> Observer<'l> { Observer::new(lens, run) }
//! ```
//! Legal twin: the observation seed.
//! ```
//! use solver_contract_sketch::{ng_contract::*, ng_observe::Observer};
//! fn noisy<'l>(lens: &'l ObservationLens, seed: ObservationSeed) -> Observer<'l> { Observer::new(lens, seed) }
//! ```
//!
//! A guard never fires on a set point itself: only a sensed dimension can be guarded.
//! ```compile_fail,E0277
//! use solver_contract_sketch::ng_contract::*;
//! fn on_reference<'b>(zero: PureKernel<'b, Nervous, Reference<Pressure>>, change: ModeTransition<'b, Nervous>)
//!     -> Guard<'b, Nervous, Reference<Pressure>> {
//!     Guard::new(zero, Crossing::Rising, change)
//! }
//! ```
//! Legal twin: a guard on the sensed pressure.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn on_pressure<'b>(zero: PureKernel<'b, Nervous, Pressure>, change: ModeTransition<'b, Nervous>)
//!     -> Guard<'b, Nervous, Pressure> {
//!     Guard::new(zero, Crossing::Rising, change)
//! }
//! ```
//!
//! A refused session can only retry, so a host cannot drop a refusal and restart the budget.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_record::*;
//! fn restart(refused: Session<'_, Retrying>) -> Step<'_> { refused.step() }
//! ```
//! Legal twin: the smaller retry of the same base.
//! ```
//! use solver_contract_sketch::ng_record::*;
//! fn again(refused: Session<'_, Retrying>) -> Step<'_> { refused.retry() }
//! ```
//!
//! No problem issues a ticket, so a span of any size asks nothing.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! fn probe(problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
//!     problem.ticket(1 << 60)
//! }
//! ```
//! Legal twin: a span is a number the core commits to.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn commit(accepted: AcceptedStateView<'_>, span_ns: u64) -> u64 {
//!     accepted.method_history().counters().first().copied().unwrap_or(span_ns)
//! }
//! ```
//!
//! An accepted view lends no timeline, so a problem has none to reach.
//! ```compile_fail,E0609
//! use solver_contract_sketch::ng_contract::*;
//! fn reach(accepted: AcceptedStateView<'_>) { let _ = accepted.pending; }
//! ```
//! Legal twin: the head's own time.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn head(accepted: AcceptedStateView<'_>) -> SimTime { accepted.time() }
//! ```
//!
//! A refused session is mid-step, so it takes no input.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! use solver_contract_sketch::ng_record::*;
//! fn late(refused: &mut Session<'_, Retrying>, input: AuthorizedInput)
//!     -> Result<InputReceipt, AdmissionError> {
//!     refused.enqueue(input)
//! }
//! ```
//! Legal twin: a running session takes one.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! use solver_contract_sketch::ng_record::*;
//! fn late(running: &mut Session<'_, Running>, input: AuthorizedInput)
//!     -> Result<InputReceipt, AdmissionError> {
//!     running.enqueue(input)
//! }
//! ```
//!
//! A host cannot hand the record a ticket: an attempt issues its own.
//! ```compile_fail,E0599
//! use solver_contract_sketch::ng_contract::*;
//! use solver_contract_sketch::ng_record::*;
//! fn swap(record: &PatientRecord<'_>, ticket: &StepTicket, proposal: StepProposal)
//!     -> Result<NextImage, TrialFailure> {
//!     record.validate(ticket, proposal)
//! }
//! ```
//! Legal twin: a host drives a core by hand through an attempt.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! use solver_contract_sketch::ng_record::*;
//! fn by_hand(record: &PatientRecord<'_>, core: &dyn CoupledSolver) -> Result<NextImage, TrialFailure> {
//!     let span_ns = core.next_step(record.accepted());
//!     record.attempt(core, Attempt::First { span_ns })
//! }
//! ```
//!
//! Bound values in a parsed image are lent to a record and never move into a new admission.
//! ```compile_fail,E0616
//! use solver_contract_sketch::ng_contract::*;
//! fn reuse(parts: ImageParts) -> BoundParameters { parts.parameters }
//! ```
//! Legal twin: a record reads them.
//! ```
//! use solver_contract_sketch::ng_contract::*;
//! fn read(parts: &ImageParts) -> PacketHash { parts.parameters().packet_hash() }
//! ```

#![forbid(unsafe_code)]
#![doc = include_str!("../../solver-contract.md")]

pub mod ng_cardiovascular;
pub mod ng_contract;
pub mod ng_fluids;
pub mod ng_nervous;
pub mod ng_observe;
pub mod ng_receptor;
pub mod ng_record;
pub mod ng_renal;
pub mod ng_solver;
pub mod ng_validate;
pub mod signature_walk;
