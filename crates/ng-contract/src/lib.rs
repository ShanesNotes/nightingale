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
//!     Ok(ports.water("inflow")?.route)
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
//!     set_point: &StateSlot<'b, Nervous, Reference<Pressure>>) -> Expr<'b, 'k, Nervous, Pressure> {
//!     let reference = k.state(set_point);
//!     k.deviation(pressure, reference)
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
//! ```compile_fail,E0599
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

mod admission;
mod anatomy;
mod body;
mod boundary;
mod compensated;
mod compile;
mod conservation;
#[allow(dead_code)]
mod evaluate;
mod failure;
mod graph;
mod identity;
mod law;
mod packet;
mod pinned;
#[allow(dead_code)]
mod problem;
mod provenance;
#[allow(dead_code)]
mod quadrature;
mod quantity;
#[allow(dead_code)]
mod scheme;
mod scope;
mod sources;
mod state;
mod system;
mod time;

pub use admission::*;
pub use anatomy::*;
pub use body::*;
pub use boundary::{BoundaryEvent, BoundaryInput, BoundaryPort, InputId, PendingInputs, WaterDelivery};
pub use compensated::*;
pub use conservation::*;
pub use failure::*;
pub use identity::*;
pub use law::*;
pub use packet::*;
pub use pinned::exp;
pub use problem::*;
pub use provenance::*;
pub use quadrature::*;
pub use quantity::*;
pub use scheme::*;
pub use scope::*;
pub use sources::{ExcursionSide, RefusalSource, SourceId, ValidityExcursion};
pub use state::*;
pub use system::*;
pub use time::*;
