use crate::quantity::{dimensions, identities};
use crate::{Charge, Concentration, Dimension, Element, Makeup, Osmoles, SameDimension, Space};

mod substances {
    pub trait Sealed {}
}
/// A dimension a pool can hold: the type a law reads, tied to the name a row carries.
pub trait Substance: substances::Sealed + Dimension {
    const SPECIES: Species;
}

mod reactants {
    pub trait Sealed {}
}
/// A species a reaction may use or make: one that states its atoms. A carried species states none to balance and is
/// the only kind that holds a fixed charge, so no reaction moves a fixed charge. Energy follows from what a reaction
/// burns.
pub trait Reactant: reactants::Sealed + Substance {}

/// A typed action on one species. Code that needs a species' marker type reaches it from a `Species` value with one
/// call, so a new species needs no arm there.
pub trait Visit {
    type Output;
    fn visit<Q: Substance>(self) -> Self::Output;
}

/// Each row is the only place a species is stated: its name, its makeup, whether water carries it, the kinds of space
/// that may hold it, its charge after speciation, and whether it counts in osmolality.
macro_rules! species {
    (@marker [$($attr:tt)*] $name:ident fixed()) => {
        $($attr)*
        #[doc = "Its fixed charge is its charge after speciation."]
        pub enum $name {}
    };
    (@marker [$($attr:tt)*] $name:ident strong($charge:literal)) => {
        $($attr)*
        #[doc = concat!("After speciation each unit holds a charge of ", stringify!($charge), ".")]
        pub enum $name {}
    };
    (@marker [$($attr:tt)*] $name:ident weak()) => {
        $($attr)*
        #[doc = "After speciation its charge follows its space's acidity."]
        pub enum $name {}
    };
    (@marker [$($attr:tt)*] $name:ident none()) => {
        $($attr)*
        pub enum $name {}
    };
    (@makeup carried($charge:literal)) => {
        Makeup::Carried { charge: $charge }
    };
    (@makeup atoms($($element:ident $count:literal),+)) => {
        Makeup::Atoms { elements: &[$((species!(@element $element), $count)),+], fuel: false }
    };
    (@makeup fuel($($element:ident $count:literal),+)) => {
        Makeup::Atoms { elements: &[$((species!(@element $element), $count)),+], fuel: true }
    };
    (@makeup energy()) => {
        Makeup::Energy
    };
    (@element C) => {
        Element::Carbon
    };
    (@element H) => {
        Element::Hydrogen
    };
    (@element O) => {
        Element::Oxygen
    };
    (@element N) => {
        Element::Nitrogen
    };
    (@reactant atoms $name:ident) => {
        impl reactants::Sealed for $name {}
        impl Reactant for $name {}
    };
    (@reactant fuel $name:ident) => {
        species!(@reactant atoms $name);
    };
    (@reactant carried $name:ident) => {};
    (@reactant energy $name:ident) => {};
    (@carried water) => {
        true
    };
    (@carried still) => {
        false
    };
    (@spaces watery $space:ident) => {
        matches!($space, Space::Plasma | Space::Interstitium | Space::Cells | Space::Lumen | Space::Fluid)
    };
    (@spaces outside $space:ident) => {
        matches!($space, Space::Plasma | Space::Interstitium | Space::Lumen | Space::Fluid)
    };
    (@spaces blood $space:ident) => {
        matches!($space, Space::Plasma | Space::Lumen)
    };
    (@spaces cells $space:ident) => {
        matches!($space, Space::Cells)
    };
    (@spaces bound $space:ident) => {
        matches!($space, Space::Bound)
    };
    (@spaces synapse $space:ident) => {
        matches!($space, Space::Vesicle | Space::Cleft | Space::Bound)
    };
    (@spaces gas $space:ident) => {
        matches!($space, Space::Gas)
    };
    (@spaces stores $space:ident) => {
        matches!($space, Space::Elastic | Space::Thermal)
    };
    (@charged fixed $name:ident) => {
        species!(@sum Charge $name);
    };
    (@charged strong $name:ident) => {
        species!(@sum Charge $name);
    };
    (@charged weak $name:ident) => {};
    (@charged none $name:ident) => {};
    (@osmole osmole $name:ident) => {
        species!(@sum Osmoles $name);
    };
    (@osmole inert $name:ident) => {};
    (@sum $total:ident $name:ident) => {
        impl identities::Sealed<$total> for $name {}
        impl SameDimension<$total> for $name {}
        impl identities::Sealed<Concentration<$total>> for Concentration<$name> {}
        impl SameDimension<Concentration<$total>> for Concentration<$name> {}
    };
    ($(
        $(#[$doc:meta])*
        $name:ident = $text:literal => $kind:ident($($made:tt)*), $carry:ident, [$($kinds:ident),+],
            $after:ident($($charge:tt)*), $osmole:ident;
    )*) => {
        $(
            species!(@marker [$(#[$doc])*] $name $after($($charge)*));
            impl dimensions::Sealed for $name {
                fn name() -> String {
                    String::from($text)
                }
            }
            impl Dimension for $name {}
            impl substances::Sealed for $name {}
            impl Substance for $name {
                const SPECIES: Species = Species::$name;
            }
            species!(@reactant $kind $name);
            species!(@charged $after $name);
            species!(@osmole $osmole $name);
        )*

        /// What a pool holds. One closed list, so a row, a route's carried columns, a makeup and a `Book` all read one
        /// name. A new species goes after the last, so no discriminant a seal holds moves.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Species {
            $($name),*
        }

        impl Species {
            pub(crate) const ALL: [Species; [$(Species::$name),*].len()] = [$(Species::$name),*];
            /// What one unit is made of.
            pub const fn makeup(self) -> Makeup {
                match self {
                    $(Species::$name => species!(@makeup $kind($($made)*)),)*
                }
            }
            /// The names of its per-species routes and outputs: reclaim, release, reuptake, binding, effect and
            /// exchange.
            pub(crate) const fn names(self) -> [&'static str; 6] {
                match self {
                    $(Species::$name => [
                        concat!("reclaim ", $text),
                        concat!("release ", $text),
                        concat!("reuptake ", $text),
                        concat!("binding ", $text),
                        concat!("effect ", $text),
                        concat!("exchange ", $text),
                    ],)*
                }
            }
            pub(crate) fn dimension(self) -> String {
                match self {
                    $(Species::$name => String::from($text),)*
                }
            }
            /// Whether water carries it, so a fluid route moves it as a share of its flow.
            pub(crate) const fn carried(self) -> bool {
                match self {
                    $(Species::$name => species!(@carried $carry),)*
                }
            }
            /// Whether a space of this kind may hold it.
            pub(crate) const fn may_hold(self, space: Space) -> bool {
                match self {
                    $(Species::$name => $(species!(@spaces $kinds space))||+,)*
                }
            }
            /// Calls the visitor with this species' marker type.
            pub fn visit<V: Visit>(self, visitor: V) -> V::Output {
                match self {
                    $(Species::$name => visitor.visit::<$name>(),)*
                }
            }
        }
    };
}

species! {
    /// Water, whose amount is in the canonical amount unit; its volume derives from it. In a gas space it is vapor.
    Water = "water" => atoms(H 2, O 1), water, [watery, gas], none(), inert;
    Sodium = "sodium" => carried(1), water, [watery], fixed(), osmole;
    Potassium = "potassium" => carried(1), water, [watery], fixed(), osmole;
    Chloride = "chloride" => carried(-1), water, [watery], fixed(), osmole;
    Albumin = "albumin" => carried(0), water, [outside], weak(), inert;
    Vasopressin = "vasopressin" => carried(0), water, [outside, bound], none(), inert;
    Norepinephrine = "norepinephrine" => carried(0), water, [synapse, outside], none(), inert;
    Acetylcholine = "acetylcholine" => carried(0), water, [synapse, outside], none(), inert;
    Energy = "energy" => energy(), still, [stores], none(), inert;
    Oxygen = "oxygen" => atoms(O 2), water, [watery, gas], none(), inert;
    /// Total carbon dioxide: dissolved, bicarbonate and carbamino, one moiety.
    CarbonDioxide = "carbon dioxide" => atoms(C 1, O 2), water, [watery, gas], weak(), osmole;
    Glucose = "glucose" => fuel(C 6, H 12, O 6), water, [watery], none(), osmole;
    NitrogenGas = "nitrogen gas" => carried(0), still, [gas], none(), inert;
    /// One glucosyl residue stands for a unit of glycogen.
    Glycogen = "glycogen" => fuel(C 6, H 10, O 5), still, [cells], none(), inert;
    /// Tripalmitin stands for every triglyceride.
    Triglyceride = "triglyceride" => fuel(C 51, H 98, O 6), water, [watery], none(), inert;
    /// Palmitic acid stands for every fatty acid.
    FattyAcids = "fatty acids" => fuel(C 16, H 32, O 2), water, [watery], none(), inert;
    Glycerol = "glycerol" => fuel(C 3, H 8, O 3), water, [watery], none(), osmole;
    /// Alanine stands for every amino acid.
    AminoAcids = "amino acids" => fuel(C 3, H 7, N 1, O 2), water, [watery], none(), osmole;
    /// An alanine residue, the amino acid less a water, stands for a unit of body protein.
    BodyProtein = "body protein" => fuel(C 3, H 5, N 1, O 1), still, [cells], none(), inert;
    /// Lactic acid's atoms stand for lactate.
    Lactate = "lactate" => fuel(C 3, H 6, O 3), water, [watery], strong(-1), osmole;
    /// Acetoacetic acid stands for every ketone.
    Ketones = "ketones" => fuel(C 4, H 6, O 3), water, [watery], strong(-1), osmole;
    Urea = "urea" => fuel(C 1, H 4, N 2, O 1), water, [watery], none(), osmole;
    /// Ammonia's atoms, its uncharged form, stand for ammonium.
    Ammonium = "ammonium" => fuel(N 1, H 3), water, [watery], strong(1), osmole;
    Creatinine = "creatinine" => fuel(C 4, H 7, N 3, O 1), water, [watery], none(), osmole;
    /// The strong anion the diet's protein sulfur leaves. A synthesis route makes it, never a reaction.
    FixedAcid = "fixed acid" => carried(0), water, [watery], strong(-1), osmole;
    /// Acetic acid stands for the diet's organic anions: one unit is one equivalent of base.
    DietBase = "diet base" => fuel(C 2, H 4, O 2), water, [watery], strong(-1), osmole;
    /// The cells' own anions, which cross no membrane.
    CellAnion = "cell anion" => carried(-1), still, [cells], fixed(), osmole;
    Calcium = "calcium" => carried(2), water, [watery], fixed(), osmole;
    Magnesium = "magnesium" => carried(2), water, [watery], fixed(), osmole;
    Phosphate = "phosphate" => carried(0), water, [watery], weak(), osmole;
    Hemoglobin = "hemoglobin" => carried(0), water, [blood], weak(), inert;
    Iron = "iron" => carried(0), water, [watery], none(), inert;
    Insulin = "insulin" => carried(0), water, [outside, bound], none(), inert;
    Glucagon = "glucagon" => carried(0), water, [outside, bound], none(), inert;
    Renin = "renin" => carried(0), water, [blood], none(), inert;
    AngiotensinII = "angiotensin II" => carried(0), water, [outside, bound], none(), inert;
    Aldosterone = "aldosterone" => carried(0), water, [outside, bound], none(), inert;
    NatriureticPeptide = "natriuretic peptide" => carried(0), water, [outside, bound], none(), inert;
    ParathyroidHormone = "parathyroid hormone" => carried(0), water, [outside, bound], none(), inert;
    Cortisol = "cortisol" => carried(0), water, [outside, bound], none(), inert;
}

const CARRIED_COUNT: usize = {
    let (mut count, mut at) = (0, 0);
    while at < Species::ALL.len() {
        if Species::ALL[at].carried() {
            count += 1;
        }
        at += 1;
    }
    count
};

impl Species {
    /// What water carries, water first, in the table's order, which is the order of a fluid route's carried columns.
    pub(crate) const CARRIED: [Species; CARRIED_COUNT] = {
        let mut carried = [Species::Water; CARRIED_COUNT];
        let (mut at, mut next) = (0, 0);
        while at < Species::ALL.len() {
            if Species::ALL[at].carried() {
                carried[next] = Species::ALL[at];
                next += 1;
            }
            at += 1;
        }
        carried
    };

    /// Charge in units of the amount: a carried species' fixed charge, and none for one that states atoms.
    pub(crate) const fn charge(self) -> i64 {
        match self.makeup() {
            Makeup::Carried { charge } => charge,
            Makeup::Atoms { .. } | Makeup::Energy => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Species;

    const PREFIXES: [&str; 6] = ["reclaim", "release", "reuptake", "binding", "effect", "exchange"];

    /// A seal writes a species' discriminant and its names, and a fluid route's columns follow `CARRIED`, so the slice's
    /// model hash holds all three.
    #[test]
    fn the_first_twelve_species_keep_their_places_and_names() {
        let pinned = [
            (Species::Water, "water"),
            (Species::Sodium, "sodium"),
            (Species::Potassium, "potassium"),
            (Species::Chloride, "chloride"),
            (Species::Albumin, "albumin"),
            (Species::Vasopressin, "vasopressin"),
            (Species::Norepinephrine, "norepinephrine"),
            (Species::Acetylcholine, "acetylcholine"),
            (Species::Energy, "energy"),
            (Species::Oxygen, "oxygen"),
            (Species::CarbonDioxide, "carbon dioxide"),
            (Species::Glucose, "glucose"),
        ];
        for (at, (species, text)) in pinned.into_iter().enumerate() {
            assert_eq!(species as usize, at, "{text}");
            assert_eq!(species.dimension(), text);
            assert_eq!(species.names(), PREFIXES.map(|prefix| format!("{prefix} {text}")));
        }
        let carried = [
            Species::Water,
            Species::Sodium,
            Species::Potassium,
            Species::Chloride,
            Species::Albumin,
            Species::Vasopressin,
            Species::Norepinephrine,
            Species::Acetylcholine,
            Species::Oxygen,
            Species::CarbonDioxide,
            Species::Glucose,
        ];
        assert_eq!(Species::CARRIED[..carried.len()], carried);
    }

    /// The basis's charge combination weighs each amount by its species' fixed charge, so these six ions are the only
    /// terms it can hold.
    #[test]
    fn only_carried_species_hold_a_fixed_charge() {
        let charged: Vec<(Species, i64)> = (Species::ALL.into_iter())
            .map(|species| (species, species.charge()))
            .filter(|&(_, charge)| charge != 0)
            .collect();
        assert_eq!(
            charged,
            [
                (Species::Sodium, 1),
                (Species::Potassium, 1),
                (Species::Chloride, -1),
                (Species::CellAnion, -1),
                (Species::Calcium, 2),
                (Species::Magnesium, 2),
            ]
        );
    }
}
