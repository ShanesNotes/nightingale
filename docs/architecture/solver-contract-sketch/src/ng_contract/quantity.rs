//! Dimensions, species and validated measures. Amount units are canonical (mol for species,
//! joule for energy); a derived volume never creates a second store.
use super::{AdmissionError, Basis, SiteClass};
use core::marker::PhantomData;

mod sealed {
    pub trait NonMaterial {}
}
/// Closed set of dimensions admitted for biological memory, excluding amount and energy.
pub trait NonMaterial: sealed::NonMaterial {}

pub enum Dimensionless {}
pub enum Phase {}
pub enum Pressure {}
pub enum Volume {}
pub enum Temperature {}
pub enum Length {}
pub enum Angle {}
pub enum Acceleration {}
pub enum Viscosity {}
impl sealed::NonMaterial for Dimensionless {}
impl NonMaterial for Dimensionless {}
impl sealed::NonMaterial for Phase {}
impl NonMaterial for Phase {}
/// A set point: slow memory in the physical unit of the variable it guards (a pressure, a
/// temperature, a stretch). It is its own dimension, so no law can read it as the guarded
/// variable; `Kernel::deviation` is the one operator that compares the two. Only a sensed
/// dimension has one, and a set point is never itself sensed, so no guard fires on one.
pub struct Reference<U: Sensed>(PhantomData<U>);
impl<U: Sensed> sealed::NonMaterial for Reference<U> {}
impl<U: Sensed> NonMaterial for Reference<U> {}
mod sensed {
    pub trait Sensed {}
}
/// A dimension a receptor or a guard can sense. Sealed; `Reference<_>` is not one.
pub trait Sensed: sensed::Sensed {}
macro_rules! sensed {
    ($($dimension:ty),*) => {$(
        impl sensed::Sensed for $dimension {}
        impl Sensed for $dimension {}
    )*};
}
sensed!(Dimensionless, Pressure, Volume, Temperature, Length);

/// Species marker with canonical amount unit mol.
pub enum Water {}
pub enum Sodium {}
pub enum Potassium {}
pub enum Chloride {}
pub enum Albumin {}
pub enum Oxygen {}
pub enum Ammonia {}
pub enum Renin {}
/// Sympathetic transmitter. Free, receptor-bound and metabolite forms are separate pools;
/// the compiler derives their conserved total (see `ConservedBasis`).
pub enum Norepinephrine {}
/// Vagal transmitter; its choline moiety is conserved across hydrolysis.
pub enum Acetylcholine {}
/// A held endocrine stand-in's surrogate ligand (AVP, angiotensin II, aldosterone, ANP); one
/// marker, one pool per ligand and site in the schema.
pub enum SurrogateLigand {}
/// Conserved energy quantity with canonical unit joule. Energy is a book, never a scalar cost.
pub enum Energy {}
pub enum Charge {}

pub struct Rate<Q>(PhantomData<Q>);
/// Flow per pressure difference; a dimensional primitive, not an organ-specific kernel.
pub struct Conductance<Q>(PhantomData<Q>);
/// Pressure per volume: a chamber's or vessel's stiffness, never a pressure.
pub enum Elastance {}
pub struct Concentration<Q>(PhantomData<Q>);
/// One value per site of class `C` in the anatomy: a vector-valued port, never a second store.
/// A site-class kernel reads its own site's entry (`Kernel::at_site`).
pub struct PerSite<C: SiteClass, U>(PhantomData<(C, U)>);

/// The dimension of `A * B` and of `A / B`, as the kernel operators produce them. A product or a
/// quotient becomes a named dimension only through a declared identity (`SameDimension`).
pub struct Product<A, B>(PhantomData<(A, B)>);
pub struct Quotient<A, B>(PhantomData<(A, B)>);
mod identities {
    pub trait SameDimension<U> {}
}
/// The pinned table of dimension identities `Kernel::as_dimension` accepts. Sealed: a new identity
/// is a contract version, never a cast.
pub trait SameDimension<U>: identities::SameDimension<U> {}
macro_rules! same_dimension {
    ($($from:ty => $to:ty;)*) => {$(
        impl identities::SameDimension<$to> for $from {}
        impl SameDimension<$to> for $from {}
    )*};
}
same_dimension! {
    Product<Elastance, Volume> => Pressure;
    Quotient<Pressure, Elastance> => Volume;
    Quotient<Pressure, Pressure> => Dimensionless;
    Quotient<Volume, Volume> => Dimensionless;
}

/// Validated finite value in a declared dimension; wire numbers are parsed outside laws.
pub struct Measure<U> {
    _value: f64,
    _unit: PhantomData<U>,
}
impl<U> Measure<U> {
    /// Refuses non-finite values at the boundary; laws never re-check.
    pub fn new(_canonical: f64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// The unit a source states, with its exact conversion to the canonical unit.
pub struct Unit<U> {
    _symbol: &'static str,
    _to_canonical: f64,
    _unit: PhantomData<U>,
}
impl<U> Unit<U> {
    pub fn canonical() -> Self {
        todo!()
    }
}
/// Where a value is mathematically admissible: the only set a trial is refused against
/// (`NumericalRefusal::TrialDomain`, then a smaller retry). The sets are closed on purpose. No
/// constructor takes bounds, so a physiological reference range cannot become a refusal rule and a
/// deteriorating patient keeps publishing; `ValidityRange` records where a law is trusted.
pub struct Admissible<U> {
    _set: AdmissibleSet,
    _unit: PhantomData<U>,
}
pub enum AdmissibleSet {
    Finite,
    Nonnegative,
    Positive,
    UnitInterval,
}
impl<U> Admissible<U> {
    pub fn finite() -> Self {
        todo!()
    }
    /// Amounts and other quantities that cannot go below zero.
    pub fn nonnegative() -> Self {
        todo!()
    }
    /// Quantities a law divides by or takes the logarithm of.
    pub fn positive() -> Self {
        todo!()
    }
    /// Fractions such as occupancy.
    pub fn unit_interval() -> Self {
        todo!()
    }
    pub fn set(&self) -> &AdmissibleSet {
        todo!()
    }
}
/// Where a law's form is trusted: a fitted, reviewed or fixture range with its basis. Leaving it
/// never refuses or faults. The state publishes and the receipt records a `ValidityExcursion`.
pub struct ValidityRange<U> {
    _bounds: (f64, f64),
    _basis: Basis,
    _unit: PhantomData<U>,
}
impl<U> ValidityRange<U> {
    pub fn new(_lower: Measure<U>, _upper: Measure<U>, _basis: Basis) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// An accepted amount held as an unevaluated sum of two f64 (two-sum). The record keeps amounts and
/// cumulative exchanges this way so closure is not limited by f64 rounding when an exterior flow is
/// small against a pool's turnover (`evidence/ledger-rounding.txt`); solver workspaces stay f64.
pub struct Compensated {
    _high: f64,
    _low: f64,
}
impl Compensated {
    /// The high and low words, for lossless serialization, sealing and restoration.
    pub fn parts(&self) -> (f64, f64) {
        todo!()
    }
    /// `self - other` as a compensated pair: window differences keep both words.
    pub fn minus(&self, _other: &Compensated) -> Compensated {
        todo!()
    }
    /// `self + increment` by two-sum: how an extent or a prefix is added.
    pub fn plus(&self, _increment: f64) -> Compensated {
        todo!()
    }
    /// Rounded once, for a solver coordinate or a display value; never before certification.
    pub fn value(&self) -> f64 {
        todo!()
    }
}
