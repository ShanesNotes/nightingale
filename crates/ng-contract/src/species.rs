use crate::quantity::dimensions;
use crate::{Dimension, Element, Makeup};

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
/// A species a reaction may use or make: every substance but energy, whose share a reaction derives from what it burns.
pub trait Reactant: reactants::Sealed + Substance {}

/// A typed action on one species. Code that needs a species' marker type reaches it from a `Species` value with one
/// call, so a new species needs no arm there.
pub trait Visit {
    type Output;
    fn visit<Q: Substance>(self) -> Self::Output;
}

/// Each row is the only place a species is stated: its marker type, its name, its makeup and whether water carries it.
macro_rules! species {
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
    (@reactant atoms $name:ident) => {
        impl reactants::Sealed for $name {}
        impl Reactant for $name {}
    };
    (@reactant fuel $name:ident) => {
        species!(@reactant atoms $name);
    };
    (@reactant carried $name:ident) => {
        species!(@reactant atoms $name);
    };
    (@reactant energy $name:ident) => {};
    (@carried water) => {
        true
    };
    (@carried still) => {
        false
    };
    ($(
        $(#[$doc:meta])*
        $name:ident = $text:literal => $kind:ident($($made:tt)*), $carry:ident;
    )*) => {
        $(
            $(#[$doc])*
            pub enum $name {}
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
    /// Water, whose amount is in the canonical amount unit; its volume derives from it.
    Water = "water" => atoms(H 2, O 1), water;
    Sodium = "sodium" => carried(1), water;
    Potassium = "potassium" => carried(1), water;
    Chloride = "chloride" => carried(-1), water;
    Albumin = "albumin" => carried(0), water;
    Vasopressin = "vasopressin" => carried(0), water;
    Norepinephrine = "norepinephrine" => carried(0), water;
    Acetylcholine = "acetylcholine" => carried(0), water;
    Energy = "energy" => energy(), still;
    Oxygen = "oxygen" => atoms(O 2), water;
    CarbonDioxide = "carbon dioxide" => atoms(C 1, O 2), water;
    Glucose = "glucose" => fuel(C 6, H 12, O 6), water;
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
}
