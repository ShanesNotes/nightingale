use crate::{AdmissionError, Basis, EdgeKind, SiteSet, VascularEdge};
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
pub enum Sodium {}
pub enum Potassium {}
pub enum Chloride {}
pub enum Albumin {}
pub enum Vasopressin {}
pub enum Norepinephrine {}
pub enum Acetylcholine {}
/// Pressure per flow.
pub enum Resistance {}
/// Pressure per rate of change of flow.
pub enum Inertance {}
/// Pressure per flow squared: a valve's orifice loss.
pub enum Orifice {}
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
/// How far a measured value stands from its set point. It is its own dimension, so a deviation can never be
/// added to the variable it was read from (finding 14). A law turns it into a dimensionless drive by dividing by
/// a declared width, or into a set point's rate by multiplying by a speed.
pub struct Deviation<U: Sensed>(PhantomData<U>);
/// A second-order binding constant: rate per amount of ligand per amount of free receptor.
pub struct Affinity<Q>(PhantomData<Q>);
pub struct PerSite<C: SiteSet, U>(PhantomData<(C, U)>);
pub struct PerEdge<U, E: EdgeKind = VascularEdge>(PhantomData<(U, E)>);

/// What a pool holds. One closed list, so a row, a route's carried columns and a book all read one name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Species {
    Water,
    Sodium,
    Potassium,
    Chloride,
    Albumin,
    Vasopressin,
    Norepinephrine,
    Acetylcholine,
    Energy,
}
impl Species {
    /// Charge in units of the amount, zero for what the ledger keeps as no ion.
    pub(crate) const fn charge(self) -> i64 {
        match self {
            Species::Sodium | Species::Potassium => 1,
            Species::Chloride => -1,
            _ => 0,
        }
    }
    /// The local name of each per-species route and of its receptor effect: reclaim, release, reuptake, binding, effect.
    pub(crate) const fn names(self) -> [&'static str; 5] {
        match self {
            Species::Water => [
                "reclaim water",
                "release water",
                "reuptake water",
                "binding water",
                "effect water",
            ],
            Species::Sodium => [
                "reclaim sodium",
                "release sodium",
                "reuptake sodium",
                "binding sodium",
                "effect sodium",
            ],
            Species::Potassium => [
                "reclaim potassium",
                "release potassium",
                "reuptake potassium",
                "binding potassium",
                "effect potassium",
            ],
            Species::Chloride => [
                "reclaim chloride",
                "release chloride",
                "reuptake chloride",
                "binding chloride",
                "effect chloride",
            ],
            Species::Albumin => [
                "reclaim albumin",
                "release albumin",
                "reuptake albumin",
                "binding albumin",
                "effect albumin",
            ],
            Species::Vasopressin => [
                "reclaim vasopressin",
                "release vasopressin",
                "reuptake vasopressin",
                "binding vasopressin",
                "effect vasopressin",
            ],
            Species::Norepinephrine => [
                "reclaim norepinephrine",
                "release norepinephrine",
                "reuptake norepinephrine",
                "binding norepinephrine",
                "effect norepinephrine",
            ],
            Species::Acetylcholine => [
                "reclaim acetylcholine",
                "release acetylcholine",
                "reuptake acetylcholine",
                "binding acetylcholine",
                "effect acetylcholine",
            ],
            Species::Energy => [
                "reclaim energy",
                "release energy",
                "reuptake energy",
                "binding energy",
                "effect energy",
            ],
        }
    }
    pub(crate) fn dimension(self) -> String {
        match self {
            Species::Water => dimension::<Water>(),
            Species::Sodium => dimension::<Sodium>(),
            Species::Potassium => dimension::<Potassium>(),
            Species::Chloride => dimension::<Chloride>(),
            Species::Albumin => dimension::<Albumin>(),
            Species::Vasopressin => dimension::<Vasopressin>(),
            Species::Norepinephrine => dimension::<Norepinephrine>(),
            Species::Acetylcholine => dimension::<Acetylcholine>(),
            Species::Energy => dimension::<Energy>(),
        }
    }
}
mod substances {
    pub trait Sealed {}
}
/// A dimension a pool can hold: the type a law reads, tied to the name a row carries.
pub trait Substance: substances::Sealed + Dimension {
    const SPECIES: Species;
}
macro_rules! substances {
    ($($name:ident),*) => {$(
        impl substances::Sealed for $name {}
        impl Substance for $name {
            const SPECIES: Species = Species::$name;
        }
    )*};
}
substances!(
    Water,
    Sodium,
    Potassium,
    Chloride,
    Albumin,
    Vasopressin,
    Norepinephrine,
    Acetylcholine,
    Energy
);

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
    Sodium = "sodium",
    Potassium = "potassium",
    Chloride = "chloride",
    Albumin = "albumin",
    Vasopressin = "vasopressin",
    Norepinephrine = "norepinephrine",
    Acetylcholine = "acetylcholine",
    Resistance = "resistance",
    Inertance = "inertance",
    Orifice = "orifice",
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
    Quotient<Pressure, Resistance> => Rate<Water>;
    Product<Resistance, Rate<Water>> => Pressure;
    Quotient<Pressure, Inertance> => Rate<Rate<Water>>;
    Product<Orifice, Product<Rate<Water>, Rate<Water>>> => Pressure;
    Product<Quotient<Pressure, Length>, Length> => Pressure;
    Product<Rate<Water>, Dimensionless> => Rate<Water>;
    Product<Pressure, Quotient<Dimensionless, Pressure>> => Dimensionless;
    Product<Quotient<Pressure, Concentration<Albumin>>, Concentration<Albumin>> => Pressure;
    Product<Pressure, Volume> => Energy;
    Product<Energy, Rate<Dimensionless>> => Rate<Energy>;
    Rate<Sodium> => Rate<Chloride>;
    Rate<Potassium> => Rate<Chloride>;
}
/// Each holds for every amount, every sensed variable and every binding, so the table needs one line each.
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
