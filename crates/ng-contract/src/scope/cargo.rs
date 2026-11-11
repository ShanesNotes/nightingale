use crate::{
    AminoAcids, Beds, Calcium, CarbonDioxide, Chloride, DietBase, Fat, FattyAcids, Glucose, Glycerol, Iron,
    Ketones, Lactate, Magnesium, Muscle, NitrogenGas, Oxygen, Phosphate, Potassium, SiteSet, Sodium,
    Substance, Triglyceride, Water,
};

mod membership {
    pub trait Of<C> {}
    pub enum Respired {}
    pub enum Breathed {}
    pub enum CellFuel {}
    pub enum Mineral {}
    pub enum Ingested {}
    pub enum Absorbed {}
    pub enum FatStore {}
}
macro_rules! classes {
    ($($(#[$doc:meta])* $class:ident: $bound:path = $($member:ty),+;)*) => {$(
        $(#[$doc])*
        pub trait $class: $bound + membership::Of<membership::$class> {}
        $(
            impl membership::Of<membership::$class> for $member {}
            impl $class for $member {}
        )+
    )*};
}
classes! {
    Respired: Substance = Oxygen, CarbonDioxide;
    Breathed: Substance = Oxygen, CarbonDioxide, NitrogenGas, Water;
    CellFuel: Substance = Glucose, FattyAcids, Glycerol, AminoAcids, Lactate, Ketones;
    Mineral: Substance = Calcium, Phosphate, Magnesium;
    Absorbed: Substance = Water, Sodium, Potassium, Chloride, Glucose, AminoAcids, DietBase, Calcium, Phosphate,
        Magnesium, Iron;
    Ingested: Substance = Triglyceride;
    FatStore: SiteSet = Beds<Fat>, Beds<Muscle>;
}
impl<Q: Absorbed> membership::Of<membership::Ingested> for Q {}
impl<Q: Absorbed> Ingested for Q {}
