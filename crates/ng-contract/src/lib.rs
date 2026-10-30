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
//! An admissible set takes no bounds, so a reference range cannot become a refusal rule.
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

mod anatomy;
mod body;
mod compile;
mod failure;
mod graph;
mod identity;
mod law;
mod provenance;
mod quantity;
mod scope;
mod system;

pub use anatomy::*;
pub use body::*;
pub use failure::*;
pub use identity::*;
pub use law::*;
pub use provenance::*;
pub use quantity::*;
pub use scope::*;
pub use system::*;
