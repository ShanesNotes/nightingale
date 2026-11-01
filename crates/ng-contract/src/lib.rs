//! Anatomy, declaration language, compiler, catalog, units, PK claims, problem and seam, admission system,
//! split measurement and observation lenses.
//!
//! # Type rules
//!
//! A site has a position.
//! ```compile_fail,E0061
//! use ng_contract::*;
//! let _ = AnatomyBuilder::new().site::<TissueSite>("body");
//! ```
//! ```
//! use ng_contract::*;
//! fn place(height: Position) {
//!     let _ = AnatomyBuilder::new().site::<TissueSite>("body", height);
//! }
//! ```
//! A measure is finite because only `Measure::new` makes one.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! let _ = Measure::<Length> { value: f64::NAN, unit: core::marker::PhantomData };
//! ```
//! ```
//! use ng_contract::*;
//! let _ = Measure::<Length>::new(1.0);
//! ```
//! A position is three lengths, never bare numbers.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! let _ = Position::new(0.0, 0.0, 0.0);
//! ```
//! ```
//! use ng_contract::*;
//! fn height() -> Result<Position, AdmissionError> {
//!     Ok(Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?))
//! }
//! ```
//! Only an enclosed space encloses.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn enclose(map: &mut AnatomyBuilder, bed: &Site<CapillaryBed>) {
//!     map.enclose(bed, &[], Compliance::Rigid);
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn enclose(map: &mut AnatomyBuilder, space: &Site<EnclosedSpace>) {
//!     map.enclose(space, &[], Compliance::Rigid);
//! }
//! ```
//! A served-by edge ends at a nerve segment.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn report(map: &mut AnatomyBuilder, site: SiteId, tissue: &Site<TissueSite>) {
//!     map.served_by(site, tissue, Afferent::Stretch);
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn report(map: &mut AnatomyBuilder, site: SiteId, segment: &Site<NerveSegment>) {
//!     map.served_by(site, segment, Afferent::Stretch);
//! }
//! ```
//! The six site classes are closed; a new class is a contract version, so there is no legal twin.
//! ```compile_fail,E0277
//! enum Spleen {}
//! impl ng_contract::SiteClass for Spleen {}
//! ```
//! A schema comes only from a checked anatomy.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn schema(map: AnatomyBuilder) -> Result<Schema, AdmissionError> {
//!     Schema::new(map)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn schema(map: AnatomyBuilder) -> Result<Schema, AdmissionError> {
//!     Schema::new(map.finish()?)
//! }
//! ```
//! Nothing is declared after the checks ran.
//! ```compile_fail,E0382
//! use ng_contract::*;
//! fn late(mut map: AnatomyBuilder, height: Position) {
//!     let _ = map.finish();
//!     map.site::<TissueSite>("late", height);
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn early(mut map: AnatomyBuilder, height: Position) {
//!     map.site::<TissueSite>("early", height);
//!     let _ = map.finish();
//! }
//! ```
//! No law can name or hold a pool: the registry's handle on one is private.
//! ```compile_fail,E0603
//! use ng_contract::PoolRef;
//! fn steal(_: PoolRef) {}
//! ```
//! ```
//! use ng_contract::*;
//! fn granted<'b>(ports: &BoundaryPorts<'b>) -> Result<Pair<'b, Boundary, Water>, AdmissionError> {
//!     Ok(ports.route::<Water>("inflow")?.route)
//! }
//! ```
//! A transfer takes a pair, never its donor alone.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn unpaired<'b>(donor: Endpoint<'b, Renal, Water>, rate: PureKernel<'b, Renal, Rate<Water>>)
//!     -> Transfer<'b, Renal, Water> {
//!     Transfer::new(&donor, rate)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn paired<'b>(pair: &Pair<'b, Renal, Water>, rate: PureKernel<'b, Renal, Rate<Water>>) -> Transfer<'b, Renal, Water> {
//!     Transfer::new(pair, rate)
//! }
//! ```
//! Material cannot hide in a law's own memory: an amount is not a memory dimension.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn shadow<'b>(scope: &mut FragmentBuilder<'b, Renal>, domain: Admissible<Water>) {
//!     scope.state::<Water>("shadow-water", domain);
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn memory<'b>(laws: &mut FragmentBuilder<'b, Renal>) {
//!     let _ = laws.state::<Dimensionless>("filter", Admissible::unit_interval());
//! }
//! ```
//! A renal law cannot take a cardiovascular route: the route keeps its owner's brand.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn foreign<'b>(scope: &mut FragmentBuilder<'b, Renal>, route: Pair<'b, Cardiovascular, Water>,
//!     rate: PureKernel<'b, Renal, Rate<Water>>) {
//!     scope.transfer(Transfer::new(&route, rate));
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own<'b>(laws: &mut FragmentBuilder<'b, Renal>, route: &Pair<'b, Renal, Water>,
//!     rate: PureKernel<'b, Renal, Rate<Water>>) {
//!     laws.transfer(Transfer::new(route, rate));
//! }
//! ```
//! An expression cannot leave its kernel: one read in a finished kernel does not type-check in the next.
//! ```compile_fail,E0521
//! use ng_contract::*;
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
//! ```
//! use ng_contract::*;
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
//! Two kernels open from one fragment builder cannot share an expression.
//! ```compile_fail,E0521
//! use ng_contract::*;
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
//! ```
//! use ng_contract::*;
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
//! An admissible set takes no bounds, so a healthy range cannot become a refusal rule.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn refuse_low_pressure(low: Measure<Pressure>, high: Measure<Pressure>) -> Admissible<Pressure> {
//!     Admissible::new(low, high)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn validity(low: Measure<Pressure>, high: Measure<Pressure>) -> Result<ValidityRange<Pressure>, AdmissionError> {
//!     ValidityRange::new(low, high, Basis::Fixture("synthetic range"))
//! }
//! ```
//! A kernel's result takes no admissible set, so a threshold cannot become a sign constraint on a
//! computed expression.
//! ```compile_fail,E0061
//! use ng_contract::*;
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
//! ```
//! use ng_contract::*;
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
//! An expression carries no number, so no host math runs on it.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn host<'b, 'k>(x: Expr<'b, 'k, Renal, Dimensionless>) -> f64 {
//!     x.value().exp()
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn pinned<'b, 'k>(k: &mut Kernel<'b, 'k, Renal>, x: Expr<'b, 'k, Renal, Dimensionless>)
//!     -> Expr<'b, 'k, Renal, Dimensionless> {
//!     k.exp(x)
//! }
//! ```
//! A guard fires only its own owner's transition: a renal guard cannot switch a valve.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn foreign<'b>(zero: PureKernel<'b, Renal, Pressure>, valve: ModeTransition<'b, Cardiovascular>)
//!     -> Guard<'b, Renal, Pressure> {
//!     Guard::new(zero, Crossing::Rising, valve)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own<'b>(zero: PureKernel<'b, Renal, Pressure>, transition: ModeTransition<'b, Renal>)
//!     -> Guard<'b, Renal, Pressure> {
//!     Guard::new(zero, Crossing::Rising, transition)
//! }
//! ```
//! A law declared for one site class reads only that class's entries: a capillary-bed law cannot read
//! a junction's value.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn crossed<'b>(laws: &FragmentBuilder<'b, Fluids>, released: &ReadPort<'b, Fluids, PerSite<Junction, Dimensionless>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Dimensionless, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let value = k.at_site(released);
//!         k.finish(value, laws.every_mode(), class)
//!     })
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own<'b>(laws: &FragmentBuilder<'b, Fluids>, pressure: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Pressure, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let value = k.at_site(pressure);
//!         k.finish(value, laws.every_mode(), class)
//!     })
//! }
//! ```
//! A set point is not the variable it guards: no law subtracts across the two.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn as_pressure<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, pressure: Expr<'b, 'k, Nervous, Pressure>,
//!     set_point: &StateSlot<'b, Nervous, Reference<Pressure>>) -> Expr<'b, 'k, Nervous, Pressure> {
//!     let reference = k.state(set_point);
//!     k.subtract(pressure, reference)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn error<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, pressure: Expr<'b, 'k, Nervous, Pressure>,
//!     set_point: &StateSlot<'b, Nervous, Reference<Pressure>>) -> Expr<'b, 'k, Nervous, Deviation<Pressure>> {
//!     let reference = k.state(set_point);
//!     k.deviation(pressure, reference)
//! }
//! ```
//! A deviation is its own dimension, so a set point can never become the variable it guards (finding 14).
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn as_variable<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, pressure: Expr<'b, 'k, Nervous, Pressure>,
//!     off: Expr<'b, 'k, Nervous, Deviation<Pressure>>) -> Expr<'b, 'k, Nervous, Pressure> {
//!     k.add(pressure, off)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn drive<'b, 'k>(k: &mut Kernel<'b, 'k, Nervous>, off: Expr<'b, 'k, Nervous, Deviation<Pressure>>,
//!     width: Expr<'b, 'k, Nervous, Deviation<Pressure>>) -> Expr<'b, 'k, Nervous, Dimensionless> {
//!     let ratio = k.divide(off, width);
//!     k.as_dimension(ratio)
//! }
//! ```
//! A route comes from a scope, whole. A binding is a transfer of one ligand between two pools, so nothing can write a
//! reaction from raw stoichiometry: a hand-made route does not compile.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn forged<'b>() -> SiteRoute<'b, Receptor, Junction, Norepinephrine> {
//!     SiteRoute { family: todo!(), brand: todo!(), kind: todo!() }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn granted<'b>(ports: &ReceptorPorts<'b>, laws: &mut FragmentBuilder<'b, Receptor>)
//!     -> SiteRoute<'b, Receptor, Junction, Norepinephrine> {
//!     ports.binding(laws)
//! }
//! ```
//! A flow row belongs to its own kind of edge: a plain edge's law cannot read a valve's.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn crossed<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular, VascularEdge>,
//!     route: &EdgeRoute<'b, Cardiovascular, Water, ValveEdge>) -> Expr<'b, 'k, Cardiovascular, Rate<Water>> {
//!     k.at_edge(&route.flow())
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular, ValveEdge>, route: &EdgeRoute<'b, Cardiovascular, Water, ValveEdge>)
//!     -> Expr<'b, 'k, Cardiovascular, Rate<Water>> {
//!     k.at_edge(&route.flow())
//! }
//! ```
//! A product gets a name only through the identity table: volume times volume is not a pressure.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn invented<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, a: Expr<'b, 'k, Cardiovascular, Volume>,
//!     b: Expr<'b, 'k, Cardiovascular, Volume>) -> Expr<'b, 'k, Cardiovascular, Pressure> {
//!     let product = k.multiply(a, b);
//!     k.as_dimension(product)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn pressure<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, e: Expr<'b, 'k, Cardiovascular, Elastance>,
//!     v: Expr<'b, 'k, Cardiovascular, Volume>) -> Expr<'b, 'k, Cardiovascular, Pressure> {
//!     let product = k.multiply(e, v);
//!     k.as_dimension(product)
//! }
//! ```
//! A guard never fires on a set point: only a sensed dimension is guarded.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn on_reference<'b>(zero: PureKernel<'b, Nervous, Reference<Pressure>>, change: ModeTransition<'b, Nervous>)
//!     -> Guard<'b, Nervous, Reference<Pressure>> {
//!     Guard::new(zero, Crossing::Rising, change)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn on_pressure<'b>(zero: PureKernel<'b, Nervous, Pressure>, change: ModeTransition<'b, Nervous>)
//!     -> Guard<'b, Nervous, Pressure> {
//!     Guard::new(zero, Crossing::Rising, change)
//! }
//! ```
//! A parameter's dimension is one of the contract's, never an invented unit.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! enum Invented {}
//! fn invent<'b>(laws: &mut FragmentBuilder<'b, Nervous>) {
//!     let _ = laws.parameter::<Invented>(ParameterKey::new("fixture.invented"), Admissible::finite());
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn declare<'b>(laws: &mut FragmentBuilder<'b, Nervous>) {
//!     let _ = laws.parameter::<Pressure>(ParameterKey::new("fixture.pressure"), Admissible::finite());
//! }
//! ```
//! A per-site vector is not a scalar, so the whole-body read refuses it.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn vector<'b>(laws: &FragmentBuilder<'b, Fluids>, volumes: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Volume>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, PerSite<CapillaryBed, Volume>> {
//!     laws.kernel(|mut k| {
//!         let volume = k.read(volumes);
//!         k.finish(volume, laws.every_mode(), class)
//!     })
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn entry<'b>(laws: &FragmentBuilder<'b, Fluids>, volumes: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Volume>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Volume, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let volume = k.at_site(volumes);
//!         k.finish(volume, laws.every_mode(), class)
//!     })
//! }
//! ```
//! A whole-body kernel has no site, so it reads no site's entry.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn whole<'b>(laws: &FragmentBuilder<'b, Fluids>, volumes: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Volume>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Volume> {
//!     laws.kernel(|mut k| {
//!         let volume = k.at_site(volumes);
//!         k.finish(volume, laws.every_mode(), class)
//!     })
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn bed<'b>(laws: &FragmentBuilder<'b, Fluids>, volumes: &ReadPort<'b, Fluids, PerSite<CapillaryBed, Volume>>,
//!     class: RateClass) -> PureKernel<'b, Fluids, Volume, CapillaryBed> {
//!     laws.site_kernel(|mut k| {
//!         let volume = k.at_site(volumes);
//!         k.finish(volume, laws.every_mode(), class)
//!     })
//! }
//! ```
//! An edge kernel is not declared for a site class, so it reads no site's entry; it reads its edge's.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn edge<'b>(laws: &FragmentBuilder<'b, Cardiovascular>, lumens: &ReadPort<'b, Cardiovascular, PerSite<VesselSegment, Water>>,
//!     class: RateClass) -> PureKernel<'b, Cardiovascular, Water, VascularEdge> {
//!     laws.edge_kernel(|mut k| {
//!         let amount = k.at_site(lumens);
//!         k.finish(amount, laws.every_mode(), class)
//!     })
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn edge<'b>(laws: &FragmentBuilder<'b, Cardiovascular>, route: &EdgeRoute<'b, Cardiovascular, Water>,
//!     class: RateClass) -> PureKernel<'b, Cardiovascular, Water, VascularEdge> {
//!     laws.edge_kernel(|mut k| {
//!         let amount = k.at_edge(&route.from_amount());
//!         k.finish(amount, laws.every_mode(), class)
//!     })
//! }
//! ```
//! Two assemblies never mix: a fragment is added only to the assembly that issued its scope.
//! ```compile_fail,E0521
//! use ng_contract::*;
//! fn cross(first: Schema, second: Schema) -> Result<CompiledBody, AdmissionError> {
//!     assemble(first, |mut outer| {
//!         let (laws, _ports) = outer.nervous()?.parts();
//!         let stray = laws.finish();
//!         assemble(second, |mut inner| {
//!             inner.add(stray);
//!             inner.compile()
//!         })
//!     })
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own(schema: Schema) -> Result<CompiledBody, AdmissionError> {
//!     assemble(schema, |mut body| {
//!         let (laws, _ports) = body.nervous()?.parts();
//!         let fragment = laws.finish();
//!         body.add(fragment);
//!         body.compile()
//!     })
//! }
//! ```
//! An engine fault cannot be converted to physiological demise.
//! ```compile_fail,E0277
//! use ng_contract::{Demise, EngineFault};
//! fn hide_bug(fault: EngineFault) -> Demise { fault.into() }
//! ```
//! ```
//! use ng_contract::{EngineFault, EngineFaultKind};
//! fn diagnose(fault: EngineFault) -> EngineFaultKind { fault.kind() }
//! ```
//! Extents cannot be booked as one sampled rate times the step: only the quadrature makes them.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn sampled(layout: Digest, forward: Vec<Compensated>, reverse: Vec<Compensated>) -> Extents {
//!     Extents { layout, forward, reverse }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn integrated(mut quadrature: Quadrature, stage: &StageEvaluation) -> Result<Increment, EngineFault> {
//!     quadrature.add(stage, Ratio::ONE)?;
//!     quadrature.finish()
//! }
//! ```
//! A compensated value has no order, because `>` reads false on a NaN and passes what overflowed.
//! ```compile_fail,E0369
//! use ng_contract::Compensated;
//! fn exceeds(value: Compensated, limit: Compensated) -> bool { value > limit }
//! ```
//! ```
//! use ng_contract::Compensated;
//! fn exceeds(value: Compensated, limit: Compensated) -> bool { !value.at_most(limit) }
//! ```
//! A core commits to a span and gets a ticket only to solve. No problem issues one, so a span of any size
//! asks nothing.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn probe(problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
//!     problem.ticket(1 << 60)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn commit(accepted: AcceptedStateView<'_>, span_ns: u64) -> u64 {
//!     accepted.method_history().counters().first().copied().unwrap_or(span_ns)
//! }
//! ```
//! `next_step` returns a span, so a core cannot hand the session a ticket of its choosing.
//! ```compile_fail,E0053
//! use ng_contract::*;
//! struct Asks;
//! impl CoupledSolver for Asks {
//!     fn resolution(&self) -> DeclaredResolution { todo!() }
//!     fn next_step(&self, accepted: AcceptedStateView<'_>) -> StepTicket { todo!() }
//!     fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket)
//!         -> Result<StepProposal, TrialFailure> { todo!() }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! struct Commits;
//! impl CoupledSolver for Commits {
//!     fn resolution(&self) -> DeclaredResolution { todo!() }
//!     fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 { 1_000_000 }
//!     fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket)
//!         -> Result<StepProposal, TrialFailure> { todo!() }
//! }
//! ```
//! An accepted view lends no timeline, so a problem has none to reach.
//! ```compile_fail,E0609
//! use ng_contract::*;
//! fn reach(accepted: AcceptedStateView<'_>) { let _ = accepted.pending; }
//! ```
//! ```
//! use ng_contract::*;
//! fn head(accepted: AcceptedStateView<'_>) -> SimTime { accepted.time() }
//! ```
//! A core reads a ticket and cannot rewrite one.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn stretched(ticket: &StepTicket, interval: Interval) -> StepTicket {
//!     StepTicket { interval, ..ticket.clone() }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn read(ticket: &StepTicket) -> Interval { ticket.interval() }
//! ```
//! Bound values in a parsed image are lent to a record and never move into a new admission.
//! ```compile_fail,E0616
//! use ng_contract::*;
//! fn reuse(parts: ImageParts) -> BoundParameters { parts.parameters }
//! ```
//! ```
//! use ng_contract::*;
//! fn read(parts: &ImageParts) -> PacketHash { parts.parameters().packet_hash() }
//! ```
//! The solver holds no dice: chance is drawn inside a law from a keyed stream.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn solver_dice(problem: &CoupledProblem<'_>) -> f64 { problem.threshold(0, 1) }
//! ```
//! ```
//! use ng_contract::*;
//! fn law_dice<'b, 'k>(k: &mut Kernel<'b, 'k, Cardiovascular>, stream: &Stream<'b, Cardiovascular>)
//!     -> Expr<'b, 'k, Cardiovascular, Dimensionless> {
//!     k.draw(stream)
//! }
//! ```
//! A problem keeps the run's seed and every threshold to itself, so a core computes no draw of its own.
//! ```compile_fail,E0616
//! use ng_contract::*;
//! fn read_the_dice(problem: &CoupledProblem<'_>) -> &RunIdentity { problem.identity }
//! ```
//! ```
//! use ng_contract::*;
//! fn head(problem: &CoupledProblem<'_>) -> SimTime { problem.accepted().time() }
//! ```
//! A core locates no event and claims no mode change: a proposal has nowhere to put one, and a problem has no
//! `transitions`, because the attempt lands a guard from values `verify` published.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn locate(problem: &CoupledProblem<'_>, a: &StageEvaluation, b: &StageEvaluation, ticket: &StepTicket) {
//!     let _ = problem.transitions(a, b, ticket);
//! }
//! ```
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn claim(problem: &CoupledProblem<'_>, ticket: &StepTicket, increment: Increment, end: &SolveWorkspace,
//!     fired: Vec<ModeValue>, history: MethodHistory) -> StepProposal {
//!     problem.propose(ticket, increment, end, fired, history)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn honest(problem: &CoupledProblem<'_>, ticket: &StepTicket, increment: Increment, end: &SolveWorkspace,
//!     history: MethodHistory) -> StepProposal {
//!     problem.propose(ticket, increment, end, history)
//! }
//! ```
//! A firing comes only from an attempt, so a receipt cannot be written by hand.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn forged(stream: LawName, at: SimTime, entered: ModeValue) -> Fired {
//!     Fired { stream, at, ordinal: 0, threshold_bits: 0, entered }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn read(fired: &Fired) -> (LawName, SimTime, u64) { (fired.stream(), fired.at(), fired.ordinal()) }
//! ```
//! A stream comes only from its owner's builder, which records the guard it owes.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! fn minted<'b>() -> Stream<'b, Renal> {
//!     Stream { name: "dice", brand: core::marker::PhantomData, owner: core::marker::PhantomData }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn declared<'b>(laws: &mut FragmentBuilder<'b, Renal>) -> Stream<'b, Renal> { laws.stream("dice") }
//! ```
//! A guard draws only on its own owner's stream.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn foreign<'b>(integral: &StateSlot<'b, Renal, Dimensionless>, dice: &Stream<'b, Cardiovascular>,
//!     change: ModeTransition<'b, Renal>) -> HazardGuard<'b, Renal> {
//!     HazardGuard::new(integral, dice, change)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn own<'b>(integral: &StateSlot<'b, Renal, Dimensionless>, dice: &Stream<'b, Renal>,
//!     change: ModeTransition<'b, Renal>) -> HazardGuard<'b, Renal> {
//!     HazardGuard::new(integral, dice, change)
//! }
//! ```
//! A hazard integral is dimensionless: a threshold is `-ln u`, so a pressure or a set point cannot reach one.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn on_reference<'b>(integral: &StateSlot<'b, Nervous, Reference<Pressure>>, dice: &Stream<'b, Nervous>,
//!     change: ModeTransition<'b, Nervous>) -> HazardGuard<'b, Nervous> {
//!     HazardGuard::new(integral, dice, change)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn on_memory<'b>(integral: &StateSlot<'b, Nervous, Dimensionless>, dice: &Stream<'b, Nervous>,
//!     change: ModeTransition<'b, Nervous>) -> HazardGuard<'b, Nervous> {
//!     HazardGuard::new(integral, dice, change)
//! }
//! ```
//! A forcing is a factor with no unit of amount, so it varies a rate and never adds to one (I10).
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn injected<'b, 'k>(k: &mut Kernel<'b, 'k, Renal>, rate: Expr<'b, 'k, Renal, Rate<Water>>,
//!     wobble: &Forcing<'b, Renal>) -> Expr<'b, 'k, Renal, Rate<Water>> {
//!     let varied = k.forcing(wobble);
//!     k.add(rate, varied)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn varied<'b, 'k>(k: &mut Kernel<'b, 'k, Renal>, rate: Expr<'b, 'k, Renal, Rate<Water>>,
//!     wobble: &Forcing<'b, Renal>) -> Expr<'b, 'k, Renal, Rate<Water>> {
//!     let factor = k.forcing(wobble);
//!     k.scale(rate, factor)
//! }
//! ```
//! A forcing's period and amplitude are packet values, never numbers in a fragment (finding 15).
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn literal<'b>(laws: &mut FragmentBuilder<'b, Renal>) -> Forcing<'b, Renal> {
//!     laws.forcing("wobble", 1_000_000_000, 0.1)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn declared<'b>(laws: &mut FragmentBuilder<'b, Renal>) -> Forcing<'b, Renal> {
//!     laws.forcing("wobble", ParameterKey::new("rn.period"), ParameterKey::new("rn.amplitude"))
//! }
//! ```
//! A reviewed packet's anchor needs a citation.
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn uncited(id: AnchorId, read: Snapshot<Pressure>) -> Specified<Pressure> {
//!     Specified::new(id, read)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn cited(id: AnchorId, read: Snapshot<Pressure>, citation: Citation) -> Specified<Pressure> {
//!     Specified::new(id, read, citation)
//! }
//! ```
//! A synthetic value cannot be an anchor in a reviewed packet.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn launder(packet: &mut ParameterPacket<Reviewed>, fake: Synthetic<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.anchor(fake)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn fixture(packet: &mut ParameterPacket<SyntheticFixture>, value: Synthetic<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.anchor(value)
//! }
//! ```
//! A fixture value set by hand has no way into a reviewed packet.
//! ```compile_fail,E0599
//! use ng_contract::*;
//! fn launder(packet: &mut ParameterPacket<Reviewed>, key: ParameterKey, value: Measure<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.synthetic(key, Binding::Body, value, "synthetic")
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn fixture(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, value: Measure<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.synthetic(key, Binding::Body, value, "synthetic")
//! }
//! ```
//! An anchor states its method and window (rule 17).
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn silent(row: RowKind, value: Measure<Pressure>) -> Snapshot<Pressure> {
//!     Snapshot::new(row, value)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn stated(row: RowKind, value: Measure<Pressure>) -> Snapshot<Pressure> {
//!     Snapshot::new(row, "arterial line", AnchorStatistic::CycleMean, value)
//! }
//! ```
//! An anchor's value does not leave it, so no packet entry can copy one (I14).
//! ```compile_fail,E0616
//! use ng_contract::*;
//! fn pinned(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, read: Snapshot<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.synthetic(key, Binding::Body, read.value, "the anchor")
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn fitted(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, guess: Measure<Pressure>,
//!     anchor: AnchorId) -> Result<(), AdmissionError> {
//!     packet.calibrated(key, Binding::Body, guess, anchor)
//! }
//! ```
//! A fit names its anchor, so it is one unknown with one row.
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn aimless(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, guess: Measure<Pressure>)
//!     -> Result<(), AdmissionError> {
//!     packet.calibrated(key, Binding::Body, guess)
//! }
//! ```
//! A patient's draw takes the packet seed, never the run's (I9).
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn crossed(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, seed: Seed)
//!     -> Result<(), AdmissionError> {
//!     packet.sampled(key, Binding::Body, seed)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn drawn(packet: &mut ParameterPacket<SyntheticFixture>, key: ParameterKey, seed: PacketSeed)
//!     -> Result<(), AdmissionError> {
//!     packet.sampled(key, Binding::Body, seed)
//! }
//! ```
//! A lens is no part of a run's identity, so a new lens reads any replay.
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn watched(model: ModelHash, packet: PacketHash, seed: Seed, resolution: DeclaredResolution,
//!     lens: &ObservationLens) -> RunIdentity {
//!     RunIdentity::new(model, packet, seed, resolution, lens.digest())
//! }
//! ```
//! Seed domains never mix: only the run's seed goes in a run's identity.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn crossed(model: ModelHash, packet: PacketHash, seed: ObservationSeed, resolution: DeclaredResolution)
//!     -> RunIdentity {
//!     RunIdentity::new(model, packet, seed, resolution)
//! }
//! ```
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn crossed(model: ModelHash, packet: PacketHash, seed: PacketSeed, resolution: DeclaredResolution)
//!     -> RunIdentity {
//!     RunIdentity::new(model, packet, seed, resolution)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn run(model: ModelHash, packet: PacketHash, seed: Seed, resolution: DeclaredResolution) -> RunIdentity {
//!     RunIdentity::new(model, packet, seed, resolution)
//! }
//! ```
//! Only the receiver the coupling map draws reads an arrow: RN cannot read what CV sends NS.
//! ```compile_fail,E0271
//! use ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Renal>) {
//!     let _stretch = laws.receive::<BaroreceptorAfferents<_>>();
//! }
//! ```
//! ```
//! use ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Nervous>) -> BaroreceptorAfferents<Reads<'b, Nervous>> {
//!     laws.receive::<BaroreceptorAfferents<_>>()
//! }
//! ```
//! Only the sender the coupling map draws emits an arrow: NS cannot define what CV sends.
//! ```compile_fail,E0271
//! use ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Nervous>) {
//!     let _stretch = laws.emit::<BaroreceptorAfferents<_>>();
//! }
//! ```
//! ```
//! use ng_contract::{catalog::*, *};
//! fn wire<'b>(laws: &mut FragmentBuilder<'b, Cardiovascular>) -> BaroreceptorAfferents<Defines<'b, Cardiovascular>> {
//!     laws.emit::<BaroreceptorAfferents<_>>()
//! }
//! ```
//! A coupling's ports come only from `emit` and `receive`: no one outside the contract can make the token.
//! ```compile_fail,E0603
//! use ng_contract::{catalog::*, *};
//! fn forged<'b>() -> BaroreceptorAfferents<Reads<'b, Renal>> {
//!     BaroreceptorAfferents::issue(ng_contract::catalog::private::Token(())).0
//! }
//! ```
//! ```
//! use ng_contract::{catalog::*, *};
//! fn granted<'b>(laws: &FragmentBuilder<'b, Nervous>) -> BaroreceptorAfferents<Reads<'b, Nervous>> {
//!     laws.receive()
//! }
//! ```
//! A per-site field takes a kernel of its site class.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! fn flat<'b>(laws: &mut FragmentBuilder<'b, Cardiovascular>,
//!     field: &Output<'b, Cardiovascular, PerSite<CapillaryBed, Pressure>>,
//!     value: PureKernel<'b, Cardiovascular, Pressure>) {
//!     laws.define_at(field, value);
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn per_bed<'b>(laws: &mut FragmentBuilder<'b, Cardiovascular>,
//!     field: &Output<'b, Cardiovascular, PerSite<CapillaryBed, Pressure>>,
//!     value: PureKernel<'b, Cardiovascular, Pressure, CapillaryBed>) {
//!     laws.define_at(field, value);
//! }
//! ```
//! A held stand-in cannot enter the body without declaring what it supplies.
//! ```compile_fail,E0061
//! use ng_contract::*;
//! fn undeclared<'b>(body: &mut Assembly<'b>) -> Result<HeldScope<'b, Endocrine>, AdmissionError> {
//!     body.hold::<Endocrine>()
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn declared<'b>(body: &mut Assembly<'b>) -> Result<HeldScope<'b, Endocrine>, AdmissionError> {
//!     body.hold(HeldDeclaration::new(vec![], vec![], "no surrogate route yet"))
//! }
//! ```
//! A card the slice runs is never held.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! fn stood_in<'b>(body: &mut Assembly<'b>, declaration: HeldDeclaration<Cardiovascular>)
//!     -> Result<HeldScope<'b, Cardiovascular>, AdmissionError> {
//!     body.hold(declaration)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! fn run<'b>(body: &mut Assembly<'b>) -> Result<CardiovascularScope<'b>, AdmissionError> {
//!     body.cardiovascular()
//! }
//! ```

extern crate alloc;

mod admission;
mod anatomy;
mod body;
mod boundary;
pub mod catalog;
mod chance;
mod compensated;
mod compile;
mod conservation;
mod elimination;
mod evaluate;
mod failure;
mod graph;
mod identity;
mod image;
mod landing;
mod law;
mod lens;
mod packet;
mod pinned;
mod problem;
mod provenance;
mod quadrature;
mod quantity;
mod ring;
mod scheme;
mod scope;
mod seam;
mod sources;
mod state;
mod system;
mod time;

pub use admission::*;
pub use anatomy::*;
pub use body::*;
pub use boundary::{
    AppliedInputs, Attempt, BoundaryEvent, BoundaryInput, BoundaryPort, InputId, Legs, PendingInputs,
    Posture, WaterDelivery,
};
pub use compensated::*;
pub use conservation::*;
pub use elimination::Factors;
pub use failure::*;
pub use identity::*;
pub use image::*;
pub use law::*;
pub use lens::{Channel, ChannelSpec, ObservationLens};
pub use packet::*;
pub use pinned::exp;
pub use problem::*;
pub use provenance::*;
pub use quadrature::*;
pub use quantity::*;
pub use ring::eliminate;
pub use scheme::*;
pub use scope::*;
pub use seam::*;
pub use sources::{ExcursionSide, RefusalSource, SourceId, ValidityExcursion};
pub use state::*;
pub use system::*;
pub use time::*;
