use crate::{
    Albumin, Aldosterone, AminoAcids, AngiotensinII, BoundaryDirection, Calcium, CarbonDioxide, Chloride,
    Cortisol, Endocrine, Energy, FixedAcid, Gastrointestinal, Glucagon, Glucose, Hematology, Hemoglobin,
    Hepatic, Insulin, Iron, Magnesium, Metabolism, NatriureticPeptide, NitrogenGas, Owner, Oxygen,
    ParathyroidHormone, Phosphate, Potassium, Renal, Renin, Respiratory, Sodium, Substance, Triglyceride,
    Vasopressin, Water,
};
use core::marker::PhantomData;

mod sealed {
    pub trait Process {}
}
/// A route across the body's boundary that one card's law serves. The contract states its card, species, direction
/// and name, so composition data places it at a site and nothing else, and only that card's ports serve it.
/// ```compile_fail,E0271
/// use ng_contract::*;
/// fn served<'b>(ports: &RespiratoryPorts<'b>) -> Result<BoundaryRoute<'b, Water, Respiratory>, AdmissionError> {
///     ports.boundary::<Urine>()
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn served<'b>(ports: &RenalPorts<'b>) -> Result<BoundaryRoute<'b, Water, Renal>, AdmissionError> {
///     ports.boundary::<Urine>()
/// }
/// ```
/// Only the contract makes a process.
/// ```compile_fail,E0277
/// use ng_contract::*;
/// enum Mine {}
/// impl BoundaryProcess for Mine {
///     type Card = Respiratory;
///     type Quantity = Water;
///     const NAME: &'static str = "mine";
///     const DIRECTION: BoundaryDirection = BoundaryDirection::OutOfBody;
///     const DRAINS: bool = false;
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn named() -> &'static str {
///     <Expired<Water> as BoundaryProcess>::NAME
/// }
/// ```
pub trait BoundaryProcess: sealed::Process {
    type Card: Owner;
    type Quantity: Substance;
    const NAME: &'static str;
    const DIRECTION: BoundaryDirection;
    /// A drain carries every species its pool holds at its water's rate, as the urine does.
    const DRAINS: bool;
}
/// A hormone entering the body by its secretion.
pub struct Secretion<H>(PhantomData<H>);
/// A hormone leaving by its first-order clearance.
pub struct Clearance<H>(PhantomData<H>);
/// What leaves the small intestine's lumen in the balanced fecal exit.
pub struct Feces<Q>(PhantomData<Q>);
/// What sweat carries out of the skin bed.
pub struct Sweat<Q>(PhantomData<Q>);
/// A room gas entering the conducting airway with a breath.
pub struct Inspired<Q>(PhantomData<Q>);
/// What leaves the conducting airway in expired gas.
pub struct Expired<Q>(PhantomData<Q>);
pub enum AngiotensinIIMade {}
pub enum InsulinFirstPass {}
pub enum AlbuminSynthesis {}
pub enum AlbuminLoss {}
pub enum FixedAcidMade {}
pub enum HemoglobinProduction {}
pub enum HemoglobinDestruction {}
pub enum SkinEvaporation {}
pub enum DryHeatLoss {}
pub enum SkinEvaporativeHeat {}
/// Heat leaving the conducting airway's own store with expired gas: warming the breath and its water's latent heat.
pub enum AirwayHeat {}
pub enum Urine {}

macro_rules! processes {
    (@drains) => { false };
    (@drains drains) => { true };
    ($($process:ty = $name:literal: $card:ident, $quantity:ident, $direction:ident $(, $drains:ident)?;)*) => {
        $(
            impl sealed::Process for $process {}
            impl BoundaryProcess for $process {
                type Card = $card;
                type Quantity = $quantity;
                const NAME: &'static str = $name;
                const DIRECTION: BoundaryDirection = BoundaryDirection::$direction;
                const DRAINS: bool = processes!(@drains $($drains)?);
            }
        )*
        #[cfg(test)]
        pub(super) const NAMES: &[&str] = &[$($name),*];
    };
}
processes! {
    Secretion<Insulin> = "insulin secretion": Endocrine, Insulin, IntoBody;
    Clearance<Insulin> = "insulin clearance": Endocrine, Insulin, OutOfBody;
    Secretion<Glucagon> = "glucagon secretion": Endocrine, Glucagon, IntoBody;
    Clearance<Glucagon> = "glucagon clearance": Endocrine, Glucagon, OutOfBody;
    Secretion<Renin> = "renin secretion": Endocrine, Renin, IntoBody;
    Clearance<Renin> = "renin clearance": Endocrine, Renin, OutOfBody;
    AngiotensinIIMade = "angiotensin II made": Endocrine, AngiotensinII, IntoBody;
    Clearance<AngiotensinII> = "angiotensin II clearance": Endocrine, AngiotensinII, OutOfBody;
    Secretion<Aldosterone> = "aldosterone secretion": Endocrine, Aldosterone, IntoBody;
    Clearance<Aldosterone> = "aldosterone clearance": Endocrine, Aldosterone, OutOfBody;
    Secretion<NatriureticPeptide> = "natriuretic peptide secretion": Endocrine, NatriureticPeptide, IntoBody;
    Clearance<NatriureticPeptide> = "natriuretic peptide clearance": Endocrine, NatriureticPeptide, OutOfBody;
    Secretion<ParathyroidHormone> = "parathyroid hormone secretion": Endocrine, ParathyroidHormone, IntoBody;
    Clearance<ParathyroidHormone> = "parathyroid hormone clearance": Endocrine, ParathyroidHormone, OutOfBody;
    Secretion<Cortisol> = "cortisol secretion": Endocrine, Cortisol, IntoBody;
    Clearance<Cortisol> = "cortisol clearance": Endocrine, Cortisol, OutOfBody;
    Secretion<Vasopressin> = "vasopressin secretion": Endocrine, Vasopressin, IntoBody;
    Clearance<Vasopressin> = "vasopressin clearance": Endocrine, Vasopressin, OutOfBody;
    InsulinFirstPass = "insulin first pass": Hepatic, Insulin, OutOfBody;
    AlbuminSynthesis = "albumin synthesis": Hepatic, Albumin, IntoBody;
    AlbuminLoss = "albumin loss": Hepatic, Albumin, OutOfBody;
    FixedAcidMade = "fixed acid made": Hepatic, FixedAcid, IntoBody;
    HemoglobinProduction = "hemoglobin production": Hematology, Hemoglobin, IntoBody;
    HemoglobinDestruction = "hemoglobin destruction": Hematology, Hemoglobin, OutOfBody;
    Feces<Water> = "feces water": Gastrointestinal, Water, OutOfBody;
    Feces<Sodium> = "feces sodium": Gastrointestinal, Sodium, OutOfBody;
    Feces<Potassium> = "feces potassium": Gastrointestinal, Potassium, OutOfBody;
    Feces<Chloride> = "feces chloride": Gastrointestinal, Chloride, OutOfBody;
    Feces<Glucose> = "feces glucose": Gastrointestinal, Glucose, OutOfBody;
    Feces<Triglyceride> = "feces triglyceride": Gastrointestinal, Triglyceride, OutOfBody;
    Feces<AminoAcids> = "feces amino acids": Gastrointestinal, AminoAcids, OutOfBody;
    Feces<Calcium> = "feces calcium": Gastrointestinal, Calcium, OutOfBody;
    Feces<Phosphate> = "feces phosphate": Gastrointestinal, Phosphate, OutOfBody;
    Feces<Magnesium> = "feces magnesium": Gastrointestinal, Magnesium, OutOfBody;
    Feces<Iron> = "feces iron": Gastrointestinal, Iron, OutOfBody;
    SkinEvaporation = "skin evaporation": Metabolism, Water, OutOfBody;
    Sweat<Water> = "sweat water": Metabolism, Water, OutOfBody;
    Sweat<Sodium> = "sweat sodium": Metabolism, Sodium, OutOfBody;
    Sweat<Chloride> = "sweat chloride": Metabolism, Chloride, OutOfBody;
    DryHeatLoss = "dry heat loss": Metabolism, Energy, OutOfBody;
    SkinEvaporativeHeat = "skin evaporative heat": Metabolism, Energy, OutOfBody;
    Inspired<Oxygen> = "inspired oxygen": Respiratory, Oxygen, IntoBody;
    Inspired<CarbonDioxide> = "inspired carbon dioxide": Respiratory, CarbonDioxide, IntoBody;
    Inspired<NitrogenGas> = "inspired nitrogen gas": Respiratory, NitrogenGas, IntoBody;
    Expired<Oxygen> = "expired oxygen": Respiratory, Oxygen, OutOfBody;
    Expired<CarbonDioxide> = "expired carbon dioxide": Respiratory, CarbonDioxide, OutOfBody;
    Expired<NitrogenGas> = "expired nitrogen gas": Respiratory, NitrogenGas, OutOfBody;
    Expired<Water> = "expired water": Respiratory, Water, OutOfBody;
    AirwayHeat = "airway heat": Respiratory, Energy, OutOfBody;
    Urine = "urine": Renal, Water, OutOfBody, drains;
}
