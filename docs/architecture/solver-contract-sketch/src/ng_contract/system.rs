//! Owner brands: the thirteen atlas cards, plus the receptor layer and the boundary.
//! An owner brand says which LAWS a capability may serve; no owner owns a pool.

mod sealed {
    pub trait System {}
}
/// An atlas card (`docs/atlas/README.md`). Sealed: the set of systems is the atlas's.
pub trait System: sealed::System {
    const CARD: &'static str;
    /// Whether the arena slice (CV, FL, RN, NS) implements this card's laws.
    const IN_SLICE: bool;
}
/// A card outside the arena slice. The contract holds it as a labeled stand-in generated from
/// the coupling catalog (`Assembly::hold`), never as hidden physiology inside a slice crate.
pub trait OutsideSlice: System {}

macro_rules! systems {
    ($($name:ident = $card:literal, $slice:ident;)*) => {$(
        pub enum $name {}
        impl sealed::System for $name {}
        impl System for $name {
            const CARD: &'static str = $card;
            const IN_SLICE: bool = systems!(@in $slice);
        }
        systems!(@outside $slice $name);
    )*};
    (@in slice) => { true };
    (@in held) => { false };
    (@outside slice $name:ident) => {};
    (@outside held $name:ident) => { impl OutsideSlice for $name {} };
}
systems! {
    Cardiovascular = "CV", slice;
    Fluids = "FL", slice;
    Renal = "RN", slice;
    Nervous = "NS", slice;
    Respiratory = "RS", held;
    Metabolism = "MT", held;
    Musculoskeletal = "MS", held;
    Endocrine = "EN", held;
    Hepatic = "HP", held;
    Gastrointestinal = "GI", held;
    Immune = "IM", held;
    Hematology = "HC", held;
    BloodChemistry = "BC", held;
}
/// The shared ligand-binding layer: reflex transmitters and future drugs meet the same receptors.
pub enum Receptor {}
/// The body's exterior and the routes the boundary timeline drives.
pub enum Boundary {}
