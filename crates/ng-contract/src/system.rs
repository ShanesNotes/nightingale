mod sealed {
    pub trait Owner {}
}
/// An owner brand: which laws a capability may serve. Sealed to the 13 atlas cards, the receptor
/// layer and the boundary. No owner owns a pool.
pub trait Owner: sealed::Owner {
    /// The name a law's name starts with.
    const CARD: &'static str;
    const IN_SLICE: bool;
}
/// Only a card outside the slice can be held.
pub trait OutsideSlice: Owner {}

macro_rules! owners {
    ($($(#[$doc:meta])* $name:ident = $card:literal, $place:ident;)*) => {
        $(
            $(#[$doc])*
            pub enum $name {}
            impl sealed::Owner for $name {}
            impl Owner for $name {
                const CARD: &'static str = $card;
                const IN_SLICE: bool = owners!(@in_slice $place);
            }
            owners!(@outside $place $name);
        )*
        /// How many cards stand outside the slice, so a composition root can show it held them all.
        pub const OUTSIDE_SLICE: usize = 0 $(+ owners!(@count $place))*;
    };
    (@in_slice slice) => { true };
    (@in_slice held) => { false };
    (@outside slice $name:ident) => {};
    (@outside held $name:ident) => { impl OutsideSlice for $name {} };
    (@count slice) => { 0 };
    (@count held) => { 1 };
}
owners! {
    Cardiovascular = "CV", slice;
    Respiratory = "RS", held;
    Nervous = "NS", slice;
    Renal = "RN", slice;
    Hepatic = "HP", held;
    Gastrointestinal = "GI", held;
    Endocrine = "EN", held;
    BloodChemistry = "BC", held;
    Hematology = "HC", held;
    Immune = "IM", held;
    Fluids = "FL", slice;
    Metabolism = "MT", held;
    Musculoskeletal = "MS", held;
    /// The receptor layer: it owns laws like a system but is not one of the 13.
    Receptor = "Receptor", slice;
    /// The body's exterior and the routes the boundary port drives.
    Boundary = "Boundary", slice;
}
