use crate::{AdmissionError, Basis, SiteClass};
use core::marker::PhantomData;

mod dimensions {
    pub trait Sealed {
        fn name() -> String;
    }
}
pub trait Dimension: dimensions::Sealed {}

mod material {
    pub trait Sealed {}
}
pub trait NonMaterial: material::Sealed + Dimension {}

mod sensed {
    pub trait Sealed {}
}
pub trait Sensed: sensed::Sealed + Dimension {}

pub enum Dimensionless {}
pub enum Pressure {}
pub enum Volume {}
pub enum Length {}
/// Simulated time, in seconds in a packet.
pub enum Time {}
/// Pressure per volume: a wall's stiffness, never a pressure.
pub enum Elastance {}
/// Water, whose amount is in the canonical amount unit; its volume derives from it.
pub enum Water {}
pub enum Energy {}
pub enum Temperature {}
pub enum Phase {}
pub enum Viscosity {}
pub enum Oxygen {}
pub enum Renin {}
pub enum Ammonia {}
pub struct Rate<Q>(PhantomData<Q>);
pub struct Concentration<Q>(PhantomData<Q>);
/// Flow per pressure difference.
pub struct Conductance<Q>(PhantomData<Q>);
pub struct Product<A, B>(PhantomData<(A, B)>);
pub struct Quotient<A, B>(PhantomData<(A, B)>);
/// A set point in the unit of the variable it guards. It is its own dimension, so no law adds,
/// subtracts or relaxes across the two; `Kernel::deviation` is the one operator that meets them.
pub struct Reference<U: Sensed>(PhantomData<U>);
/// Volume per amount of a species: a fixed structural coefficient of the schema, not a parameter.
pub struct PartialVolume<Q>(PhantomData<Q>);
pub struct PerSite<C: SiteClass, U>(PhantomData<(C, U)>);
pub struct PerEdge<U>(PhantomData<U>);

macro_rules! named {
    ($($dimension:ident = $name:literal),* $(,)?) => {$(
        impl dimensions::Sealed for $dimension {
            fn name() -> String {
                String::from($name)
            }
        }
        impl Dimension for $dimension {}
    )*};
}
named!(
    Dimensionless = "dimensionless",
    Pressure = "pressure",
    Volume = "volume",
    Length = "length",
    Time = "time",
    Elastance = "elastance",
    Water = "water",
    Energy = "energy",
    Temperature = "temperature",
    Phase = "phase",
    Viscosity = "viscosity",
    Oxygen = "oxygen",
    Renin = "renin",
    Ammonia = "ammonia",
);
impl<Q: Dimension> dimensions::Sealed for Rate<Q> {
    fn name() -> String {
        format!("rate({})", Q::name())
    }
}
impl<Q: Dimension> Dimension for Rate<Q> {}
impl<Q: Dimension> dimensions::Sealed for Concentration<Q> {
    fn name() -> String {
        format!("concentration({})", Q::name())
    }
}
impl<Q: Dimension> Dimension for Concentration<Q> {}
impl<Q: Dimension> dimensions::Sealed for Conductance<Q> {
    fn name() -> String {
        format!("conductance({})", Q::name())
    }
}
impl<Q: Dimension> Dimension for Conductance<Q> {}
impl<A: Dimension, B: Dimension> dimensions::Sealed for Product<A, B> {
    fn name() -> String {
        format!("product({},{})", A::name(), B::name())
    }
}
impl<A: Dimension, B: Dimension> Dimension for Product<A, B> {}
impl<A: Dimension, B: Dimension> dimensions::Sealed for Quotient<A, B> {
    fn name() -> String {
        format!("quotient({},{})", A::name(), B::name())
    }
}
impl<A: Dimension, B: Dimension> Dimension for Quotient<A, B> {}
impl<U: Sensed> dimensions::Sealed for Reference<U> {
    fn name() -> String {
        format!("reference({})", U::name())
    }
}
impl<U: Sensed> Dimension for Reference<U> {}

impl material::Sealed for Dimensionless {}
impl NonMaterial for Dimensionless {}
impl material::Sealed for Phase {}
impl NonMaterial for Phase {}
impl<U: Sensed> material::Sealed for Reference<U> {}
impl<U: Sensed> NonMaterial for Reference<U> {}

macro_rules! sensed {
    ($($dimension:ty),*) => {$(
        impl sensed::Sealed for $dimension {}
        impl Sensed for $dimension {}
    )*};
}
sensed!(Dimensionless, Pressure, Volume, Temperature, Length);

mod identities {
    pub trait Sealed<U> {}
}
/// The pinned table `Kernel::as_dimension` reads. Sealed: a new identity is a contract version, never
/// a cast.
pub trait SameDimension<U>: identities::Sealed<U> {}
macro_rules! same_dimension {
    ($($from:ty => $to:ty;)*) => {$(
        impl identities::Sealed<$to> for $from {}
        impl SameDimension<$to> for $from {}
    )*};
}
same_dimension! {
    Product<Elastance, Volume> => Pressure;
    Quotient<Pressure, Elastance> => Volume;
    Quotient<Pressure, Pressure> => Dimensionless;
    Quotient<Volume, Volume> => Dimensionless;
}

pub(crate) fn dimension<U: Dimension>() -> String {
    U::name()
}

/// A finite value of dimension `U` in its canonical unit.
pub struct Measure<U> {
    value: f64,
    unit: PhantomData<U>,
}
impl<U> Measure<U> {
    pub fn new(canonical: f64) -> Result<Self, AdmissionError> {
        if !canonical.is_finite() {
            return Err(AdmissionError::NonFinite);
        }
        // Adding +0.0 turns -0.0 into +0.0, so equal values have equal bits in a digest.
        Ok(Self {
            value: canonical + 0.0,
            unit: PhantomData,
        })
    }
    pub(crate) fn bits(&self) -> u64 {
        self.value.to_bits()
    }
    pub(crate) fn value(&self) -> f64 {
        self.value
    }
}

/// Where a value is mathematically admissible, the only set a trial is refused against. Where a law
/// is trusted is a `ValidityRange`, which only reports.
pub struct Admissible<U> {
    set: AdmissibleSet,
    unit: PhantomData<U>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissibleSet {
    Finite,
    Nonnegative,
    Positive,
    UnitInterval,
}
impl<U> Admissible<U> {
    fn of(set: AdmissibleSet) -> Self {
        Self {
            set,
            unit: PhantomData,
        }
    }
    pub fn finite() -> Self {
        Self::of(AdmissibleSet::Finite)
    }
    pub fn nonnegative() -> Self {
        Self::of(AdmissibleSet::Nonnegative)
    }
    pub fn positive() -> Self {
        Self::of(AdmissibleSet::Positive)
    }
    pub fn unit_interval() -> Self {
        Self::of(AdmissibleSet::UnitInterval)
    }
    pub(crate) fn set(&self) -> AdmissibleSet {
        self.set
    }
}

/// Where a law's form is trusted, with its basis. A state outside it still publishes, and the step
/// records the excursion.
pub struct ValidityRange<U> {
    lower: Measure<U>,
    upper: Measure<U>,
    basis: Basis,
}
impl<U> ValidityRange<U> {
    /// Refuses a lower bound above the upper bound.
    pub fn new(lower: Measure<U>, upper: Measure<U>, basis: Basis) -> Result<Self, AdmissionError> {
        if lower.value() > upper.value() {
            return Err(AdmissionError::ReversedValidityRange);
        }
        Ok(Self { lower, upper, basis })
    }
    pub(crate) fn parts(&self) -> (u64, u64, Basis) {
        (self.lower.bits(), self.upper.bits(), self.basis)
    }
}
