mod sealed {
    pub trait Owner {}
}
/// An owner brand: which laws a capability may serve. Sealed to the 13 atlas cards, the receptor
/// layer and the boundary. No owner owns a pool.
pub trait Owner: sealed::Owner {
    /// The name a law's name starts with.
    const CARD: &'static str;
}

macro_rules! owners {
    ($($(#[$doc:meta])* $name:ident = $card:literal;)*) => {$(
        $(#[$doc])*
        pub enum $name {}
        impl sealed::Owner for $name {}
        impl Owner for $name {
            const CARD: &'static str = $card;
        }
    )*};
}
owners! {
    Cardiovascular = "CV";
    Respiratory = "RS";
    Nervous = "NS";
    Renal = "RN";
    Hepatic = "HP";
    Gastrointestinal = "GI";
    Endocrine = "EN";
    BloodChemistry = "BC";
    Hematology = "HC";
    Immune = "IM";
    Fluids = "FL";
    Metabolism = "MT";
    Musculoskeletal = "MS";
    /// The receptor layer: it owns laws like a system but is not one of the 13.
    Receptor = "Receptor";
    /// The body's exterior and the routes the boundary port drives.
    Boundary = "Boundary";
}
