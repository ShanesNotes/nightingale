use crate::{
    AdmissionError, Basis, Chloride, EdgeKind, Energy, Potassium, SiteSet, Sodium, Substance, VascularEdge,
    Water,
};
use core::marker::PhantomData;

pub(crate) mod dimensions {
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
/// The sum of every ion's amount by its sign, in the amount unit.
pub enum Charge {}
pub enum Resistance {}
pub enum Inertance {}
pub enum Orifice {}
pub enum Temperature {}
pub enum Phase {}
pub enum Viscosity {}
/// An amount of one element's atoms, in the amount unit: what an element's conserved quantity totals.
pub enum Atoms {}
/// Dissolved particles that pull water: a space's osmolality is their concentration.
pub enum Osmoles {}
pub enum Ammonia {}
pub struct Rate<Q>(PhantomData<Q>);
pub struct Concentration<Q>(PhantomData<Q>);
/// Flow per pressure difference.
pub struct Conductance<Q>(PhantomData<Q>);
pub struct Product<A, B>(PhantomData<(A, B)>);
pub struct Quotient<A, B>(PhantomData<(A, B)>);
/// A set point in the unit of the variable it guards. It is its own dimension, so no law adds,
/// subtracts or relaxes across the two; `Kernel::deviation` is the one operator that takes their difference.
pub struct Reference<U: Sensed>(PhantomData<U>);
/// Volume per amount of a species, as the schema's fixed coefficient. It is no `Dimension`, so no parameter
/// takes it: albumin's partial volume, a parameter a wall's law reads, is a `Quotient<Volume, Albumin>`.
pub struct PartialVolume<Q>(PhantomData<Q>);
/// How far a measured value stands from its set point. It is its own dimension, so a deviation can never be
/// added to the variable it was read from. A law turns it into a dimensionless drive by dividing by a
/// declared width, or into a set point's rate by multiplying by a speed.
pub struct Deviation<U: Sensed>(PhantomData<U>);
/// A second-order binding constant: rate per amount of ligand per amount of free receptor.
pub struct Affinity<Q>(PhantomData<Q>);
pub struct PerSite<C: SiteSet, U>(PhantomData<(C, U)>);
pub struct PerEdge<U, E: EdgeKind = VascularEdge>(PhantomData<(U, E)>);

macro_rules! elements {
    ($($name:ident),*) => {
        /// An element whose atoms a reaction moves from one species to another, so where a reaction joins species its
        /// atoms, not each species, are what stays conserved.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Element {
            $($name),*
        }
        impl Element {
            pub(crate) const ALL: [Element; [$(Element::$name),*].len()] = [$(Element::$name),*];
        }
    };
}
elements!(Carbon, Hydrogen, Oxygen, Nitrogen);

/// What one unit of a species is made of, which a reaction's counts must balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Makeup {
    /// No reaction may use or make it: it only moves from pool to pool, holding the fixed charge it states.
    Carried { charge: i64 },
    /// Whole counts of each element's atoms, and no fixed charge. A fuel holds energy, whose content per unit the
    /// schema states; the rest hold none, measured against carbon dioxide, water and nitrogen gas.
    Atoms {
        elements: &'static [(Element, i64)],
        fuel: bool,
    },
    /// One unit of energy.
    Energy,
}

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
    Charge = "charge",
    Resistance = "resistance",
    Inertance = "inertance",
    Orifice = "orifice",
    Temperature = "temperature",
    Phase = "phase",
    Viscosity = "viscosity",
    Atoms = "atoms",
    Osmoles = "osmoles",
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
impl<U: Sensed> dimensions::Sealed for Deviation<U> {
    fn name() -> String {
        format!("deviation({})", U::name())
    }
}
impl<U: Sensed> Dimension for Deviation<U> {}
impl<Q: Dimension> dimensions::Sealed for Affinity<Q> {
    fn name() -> String {
        format!("affinity({})", Q::name())
    }
}
impl<Q: Dimension> Dimension for Affinity<Q> {}

impl material::Sealed for Dimensionless {}
impl NonMaterial for Dimensionless {}
impl material::Sealed for Phase {}
impl NonMaterial for Phase {}
impl<U: Sensed> material::Sealed for Reference<U> {}
impl<U: Sensed> NonMaterial for Reference<U> {}
/// A valve's flow is memory: a rate of an amount, never an amount, and no column reads it but its own edge's.
impl material::Sealed for Rate<Water> {}
impl NonMaterial for Rate<Water> {}

macro_rules! sensed {
    ($($dimension:ty),*) => {$(
        impl sensed::Sealed for $dimension {}
        impl Sensed for $dimension {}
    )*};
}
sensed!(Dimensionless, Pressure, Volume, Temperature, Length, Rate<Water>);

pub(crate) mod identities {
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
    Quotient<Pressure, Resistance> => Rate<Water>;
    Product<Resistance, Rate<Water>> => Pressure;
    Quotient<Pressure, Inertance> => Rate<Rate<Water>>;
    Product<Orifice, Product<Rate<Water>, Rate<Water>>> => Pressure;
    Product<Rate<Water>, Dimensionless> => Rate<Water>;
    Product<Pressure, Quotient<Dimensionless, Pressure>> => Dimensionless;
    Product<Pressure, Volume> => Energy;
    Product<Energy, Rate<Dimensionless>> => Rate<Energy>;
    Product<Pressure, Rate<Volume>> => Rate<Energy>;
    Rate<Sodium> => Rate<Chloride>;
    Rate<Potassium> => Rate<Chloride>;
    Quotient<Length, Length> => Dimensionless;
    Quotient<Temperature, Temperature> => Dimensionless;
    Quotient<Rate<Water>, Rate<Water>> => Dimensionless;
    Quotient<Charge, Charge> => Dimensionless;
    Quotient<Concentration<Charge>, Concentration<Charge>> => Dimensionless;
    Quotient<Concentration<Osmoles>, Concentration<Osmoles>> => Dimensionless;
    Quotient<Charge, Volume> => Concentration<Charge>;
    Quotient<Osmoles, Volume> => Concentration<Osmoles>;
}
// Generic, so a law that cancels a coefficient needs no row of its own; coherence refuses a concrete instance beside
// them.
impl<U: Dimension, V: Dimension> identities::Sealed<U> for Product<Quotient<U, V>, V> {}
impl<U: Dimension, V: Dimension> SameDimension<U> for Product<Quotient<U, V>, V> {}
impl<U: Dimension, V: Dimension> identities::Sealed<V> for Quotient<U, Quotient<U, V>> {}
impl<U: Dimension, V: Dimension> SameDimension<V> for Quotient<U, Quotient<U, V>> {}
impl<U: Dimension> identities::Sealed<U> for Quotient<U, Dimensionless> {}
impl<U: Dimension> SameDimension<U> for Quotient<U, Dimensionless> {}
impl<Q: Substance> identities::Sealed<Dimensionless> for Quotient<Concentration<Q>, Concentration<Q>> {}
impl<Q: Substance> SameDimension<Dimensionless> for Quotient<Concentration<Q>, Concentration<Q>> {}
impl<Q: Substance> identities::Sealed<Dimensionless> for Quotient<Q, Q> {}
impl<Q: Substance> SameDimension<Dimensionless> for Quotient<Q, Q> {}
impl<Q: Substance> identities::Sealed<Concentration<Q>> for Quotient<Q, Volume> {}
impl<Q: Substance> SameDimension<Concentration<Q>> for Quotient<Q, Volume> {}
impl<P: Substance, Q: Substance> identities::Sealed<Rate<Q>> for Product<Rate<P>, Quotient<Q, P>> {}
impl<P: Substance, Q: Substance> SameDimension<Rate<Q>> for Product<Rate<P>, Quotient<Q, P>> {}
impl<Q: Substance> identities::Sealed<Rate<Q>> for Product<Affinity<Q>, Product<Q, Q>> {}
impl<Q: Substance> SameDimension<Rate<Q>> for Product<Affinity<Q>, Product<Q, Q>> {}
impl<Q: Substance> identities::Sealed<Rate<Q>>
    for Product<Affinity<Concentration<Q>>, Product<Concentration<Q>, Q>>
{
}
impl<Q: Substance> SameDimension<Rate<Q>>
    for Product<Affinity<Concentration<Q>>, Product<Concentration<Q>, Q>>
{
}
impl<U: Sensed> identities::Sealed<Dimensionless> for Quotient<Deviation<U>, Deviation<U>> {}
impl<U: Sensed> SameDimension<Dimensionless> for Quotient<Deviation<U>, Deviation<U>> {}
impl<U: Sensed> identities::Sealed<Rate<Reference<U>>> for Product<Deviation<U>, Rate<Dimensionless>> {}
impl<U: Sensed> SameDimension<Rate<Reference<U>>> for Product<Deviation<U>, Rate<Dimensionless>> {}

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

/// Where a value is mathematically admissible. A trial that leaves it is refused, while a
/// `ValidityRange`, where a law is trusted, only reports.
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
