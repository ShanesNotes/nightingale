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

mod anatomy;
mod failure;
mod identity;
mod provenance;
mod quantity;
mod scope;

pub use anatomy::*;
pub use failure::*;
pub use identity::*;
pub use provenance::*;
pub use quantity::*;
pub use scope::*;
