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
    (@marker [$($attr:tt)*] $name:ident uncharged()) => {
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
    (@borne water_borne) => {
        true
    };
    (@borne not_water_borne) => {
        false
    };
    (@spaces watery $space:ident) => {
        $space.holds_water()
    };
    (@spaces outside $space:ident) => {
        $space.holds_water() && !matches!($space, Space::Cells)
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
    (@charged uncharged $name:ident) => {};
    (@osmole osmotic $name:ident) => {
        species!(@sum Osmoles $name);
    };
    (@osmole inert $name:ident) => {};
    (@osmotic osmotic) => {
        true
    };
    (@osmotic inert) => {
        false
    };
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
            pub(crate) const fn names(self) -> [&'static str; 16] {
                match self {
                    $(Species::$name => [
                        concat!("reclaim ", $text),
                        concat!("release ", $text),
                        concat!("reuptake ", $text),
                        concat!("binding ", $text),
                        concat!("effect ", $text),
                        concat!("exchange ", $text),
                        concat!("cell membrane ", $text),
                        concat!("alveolar ", $text),
                        concat!("lining ", $text),
                        concat!("endothelium ", $text),
                        concat!("airway ", $text),
                        concat!("transit ", $text),
                        concat!("absorb ", $text),
                        concat!("chyle ", $text),
                        concat!("shed ", $text),
                        concat!("secrete ", $text),
                    ],)*
                }
            }
            pub(crate) fn dimension(self) -> String {
                match self {
                    $(Species::$name => String::from($text),)*
                }
            }
            pub(crate) const fn water_borne(self) -> bool {
                match self {
                    $(Species::$name => species!(@borne $carry),)*
                }
            }
            pub(crate) const fn may_hold(self, space: Space) -> bool {
                match self {
                    $(Species::$name => $(species!(@spaces $kinds space))||+,)*
                }
            }
            #[cfg(test)]
            const fn osmotic(self) -> bool {
                match self {
                    $(Species::$name => species!(@osmotic $osmole),)*
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
    Water = "water" => atoms(H 2, O 1), water_borne, [watery, gas], uncharged(), inert;
    Sodium = "sodium" => carried(1), water_borne, [watery], fixed(), osmotic;
    Potassium = "potassium" => carried(1), water_borne, [watery], fixed(), osmotic;
    Chloride = "chloride" => carried(-1), water_borne, [watery], fixed(), osmotic;
    Albumin = "albumin" => carried(0), water_borne, [outside], weak(), inert;
    Vasopressin = "vasopressin" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    Norepinephrine = "norepinephrine" => carried(0), water_borne, [synapse, outside], uncharged(), inert;
    Acetylcholine = "acetylcholine" => carried(0), water_borne, [synapse, outside], uncharged(), inert;
    Energy = "energy" => energy(), not_water_borne, [stores], uncharged(), inert;
    Oxygen = "oxygen" => atoms(O 2), water_borne, [watery, gas], uncharged(), inert;
    /// Total carbon dioxide: dissolved, bicarbonate and carbamino, one moiety.
    CarbonDioxide = "carbon dioxide" => atoms(C 1, O 2), water_borne, [watery, gas], weak(), osmotic;
    Glucose = "glucose" => fuel(C 6, H 12, O 6), water_borne, [watery], uncharged(), osmotic;
    NitrogenGas = "nitrogen gas" => carried(0), not_water_borne, [gas], uncharged(), inert;
    /// One glucosyl residue stands for a unit of glycogen.
    Glycogen = "glycogen" => fuel(C 6, H 10, O 5), not_water_borne, [cells], uncharged(), inert;
    /// Tripalmitin stands for every triglyceride.
    Triglyceride = "triglyceride" => fuel(C 51, H 98, O 6), water_borne, [watery], uncharged(), inert;
    /// Palmitic acid stands for every fatty acid.
    FattyAcids = "fatty acids" => fuel(C 16, H 32, O 2), water_borne, [watery], uncharged(), inert;
    Glycerol = "glycerol" => fuel(C 3, H 8, O 3), water_borne, [watery], uncharged(), inert;
    /// Alanine stands for every amino acid.
    AminoAcids = "amino acids" => fuel(C 3, H 7, N 1, O 2), water_borne, [watery], uncharged(), osmotic;
    /// An alanine residue, the amino acid less a water, stands for a unit of body protein.
    BodyProtein = "body protein" => fuel(C 3, H 5, N 1, O 1), not_water_borne, [cells], uncharged(), inert;
    /// Lactic acid's atoms stand for lactate.
    Lactate = "lactate" => fuel(C 3, H 6, O 3), water_borne, [watery], strong(-1), osmotic;
    /// Acetoacetic acid stands for every ketone.
    Ketones = "ketones" => fuel(C 4, H 6, O 3), water_borne, [watery], strong(-1), osmotic;
    Urea = "urea" => fuel(C 1, H 4, N 2, O 1), water_borne, [watery], uncharged(), osmotic;
    /// Ammonia's atoms, its uncharged form, stand for ammonium.
    Ammonium = "ammonium" => fuel(N 1, H 3), water_borne, [watery], strong(1), osmotic;
    Creatinine = "creatinine" => fuel(C 4, H 7, N 3, O 1), water_borne, [watery], uncharged(), osmotic;
    /// The strong anion the diet's protein sulfur leaves. A synthesis route makes it, never a reaction.
    FixedAcid = "fixed acid" => carried(0), water_borne, [watery], strong(-1), osmotic;
    /// Acetic acid stands for the diet's organic anions: one unit is one equivalent of base.
    DietBase = "diet base" => fuel(C 2, H 4, O 2), water_borne, [watery], strong(-1), osmotic;
    /// The cells' own anions, which cross no membrane.
    CellAnion = "cell anion" => carried(-1), not_water_borne, [cells], fixed(), osmotic;
    Calcium = "calcium" => carried(2), water_borne, [watery], fixed(), osmotic;
    Magnesium = "magnesium" => carried(2), water_borne, [watery], fixed(), osmotic;
    Phosphate = "phosphate" => carried(0), water_borne, [watery], weak(), osmotic;
    Hemoglobin = "hemoglobin" => carried(0), water_borne, [blood], weak(), inert;
    Iron = "iron" => carried(0), water_borne, [watery], uncharged(), inert;
    Insulin = "insulin" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    Glucagon = "glucagon" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    Renin = "renin" => carried(0), water_borne, [blood], uncharged(), inert;
    AngiotensinII = "angiotensin II" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    Aldosterone = "aldosterone" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    NatriureticPeptide = "natriuretic peptide" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    ParathyroidHormone = "parathyroid hormone" => carried(0), water_borne, [outside, bound], uncharged(), inert;
    Cortisol = "cortisol" => carried(0), water_borne, [outside, bound], uncharged(), inert;
}

const WATER_BORNE_COUNT: usize = {
    let (mut count, mut at) = (0, 0);
    while at < Species::ALL.len() {
        if Species::ALL[at].water_borne() {
            count += 1;
        }
        at += 1;
    }
    count
};

impl Species {
    pub(crate) const WATER_BORNE: [Species; WATER_BORNE_COUNT] = {
        let mut carried = [Species::Water; WATER_BORNE_COUNT];
        let (mut at, mut next) = (0, 0);
        while at < Species::ALL.len() {
            if Species::ALL[at].water_borne() {
                carried[next] = Species::ALL[at];
                next += 1;
            }
            at += 1;
        }
        carried
    };

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

    #[test]
    fn the_first_twelve_species_keep_what_the_slice_model_hash_reads() {
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
            assert_eq!(
                species.names()[..6],
                PREFIXES.map(|prefix| format!("{prefix} {text}"))
            );
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
        assert_eq!(Species::WATER_BORNE[..carried.len()], carried);
    }

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

    #[test]
    fn only_the_osmotic_rows_count_in_osmolality() {
        let osmotic: Vec<Species> = Species::ALL
            .into_iter()
            .filter(|species| species.osmotic())
            .collect();
        assert_eq!(
            osmotic,
            [
                Species::Sodium,
                Species::Potassium,
                Species::Chloride,
                Species::CarbonDioxide,
                Species::Glucose,
                Species::AminoAcids,
                Species::Lactate,
                Species::Ketones,
                Species::Urea,
                Species::Ammonium,
                Species::Creatinine,
                Species::FixedAcid,
                Species::DietBase,
                Species::CellAnion,
                Species::Calcium,
                Species::Magnesium,
                Species::Phosphate,
            ]
        );
    }

    #[test]
    fn the_new_rows_keep_the_tables_makeup_carriage_and_spaces() {
        use super::{
            Ammonium, Calcium, CellAnion, Chloride, Creatinine, DietBase, FixedAcid, Glucose, Ketones,
            Lactate, Magnesium, Phosphate, Potassium, Sodium, Urea,
        };
        use super::{Charge, Concentration, Element, Makeup, Osmoles, SameDimension, Space};
        use Element::{Carbon as C, Hydrogen as H, Nitrogen as N, Oxygen as O};
        use Space::{
            Bound, Cells, Cleft, Elastic, Fluid, Gas, Interstitium, Lumen, Plasma, Thermal, Vesicle,
        };
        const WATERY: &[Space] = &[Plasma, Interstitium, Cells, Lumen, Fluid];
        const HORMONE: &[Space] = &[Plasma, Interstitium, Lumen, Fluid, Bound];
        const BLOOD: &[Space] = &[Plasma, Lumen];
        let carried = |charge: i64| Makeup::Carried { charge };
        let fuel = |elements: &'static [(Element, i64)]| Makeup::Atoms { elements, fuel: true };
        let rows: [(Species, Makeup, bool, &[Space]); 28] = [
            (Species::NitrogenGas, carried(0), false, &[Gas]),
            (
                Species::Glycogen,
                fuel(&[(C, 6), (H, 10), (O, 5)]),
                false,
                &[Cells],
            ),
            (
                Species::Triglyceride,
                fuel(&[(C, 51), (H, 98), (O, 6)]),
                true,
                WATERY,
            ),
            (
                Species::FattyAcids,
                fuel(&[(C, 16), (H, 32), (O, 2)]),
                true,
                WATERY,
            ),
            (Species::Glycerol, fuel(&[(C, 3), (H, 8), (O, 3)]), true, WATERY),
            (
                Species::AminoAcids,
                fuel(&[(C, 3), (H, 7), (N, 1), (O, 2)]),
                true,
                WATERY,
            ),
            (
                Species::BodyProtein,
                fuel(&[(C, 3), (H, 5), (N, 1), (O, 1)]),
                false,
                &[Cells],
            ),
            (Species::Lactate, fuel(&[(C, 3), (H, 6), (O, 3)]), true, WATERY),
            (Species::Ketones, fuel(&[(C, 4), (H, 6), (O, 3)]), true, WATERY),
            (
                Species::Urea,
                fuel(&[(C, 1), (H, 4), (N, 2), (O, 1)]),
                true,
                WATERY,
            ),
            (Species::Ammonium, fuel(&[(N, 1), (H, 3)]), true, WATERY),
            (
                Species::Creatinine,
                fuel(&[(C, 4), (H, 7), (N, 3), (O, 1)]),
                true,
                WATERY,
            ),
            (Species::FixedAcid, carried(0), true, WATERY),
            (Species::DietBase, fuel(&[(C, 2), (H, 4), (O, 2)]), true, WATERY),
            (Species::CellAnion, carried(-1), false, &[Cells]),
            (Species::Calcium, carried(2), true, WATERY),
            (Species::Magnesium, carried(2), true, WATERY),
            (Species::Phosphate, carried(0), true, WATERY),
            (Species::Hemoglobin, carried(0), true, BLOOD),
            (Species::Iron, carried(0), true, WATERY),
            (Species::Insulin, carried(0), true, HORMONE),
            (Species::Glucagon, carried(0), true, HORMONE),
            (Species::Renin, carried(0), true, BLOOD),
            (Species::AngiotensinII, carried(0), true, HORMONE),
            (Species::Aldosterone, carried(0), true, HORMONE),
            (Species::NatriureticPeptide, carried(0), true, HORMONE),
            (Species::ParathyroidHormone, carried(0), true, HORMONE),
            (Species::Cortisol, carried(0), true, HORMONE),
        ];
        let every = [
            Plasma,
            Interstitium,
            Cells,
            Lumen,
            Fluid,
            Vesicle,
            Cleft,
            Bound,
            Elastic,
            Thermal,
            Gas,
        ];
        assert_eq!(rows.map(|row| row.0), Species::ALL[12..]);
        for (species, makeup, water_borne, spaces) in rows {
            assert_eq!(species.makeup(), makeup, "{species:?}");
            assert_eq!(species.water_borne(), water_borne, "{species:?}");
            let held: Vec<Space> = every
                .into_iter()
                .filter(|&space| species.may_hold(space))
                .collect();
            assert_eq!(held, spaces, "{species:?}");
        }
        fn charge<A: SameDimension<Charge>, C: SameDimension<Concentration<Charge>>>() {}
        fn osmoles<A: SameDimension<Osmoles>, C: SameDimension<Concentration<Osmoles>>>() {}
        charge::<Sodium, Concentration<Sodium>>();
        charge::<Potassium, Concentration<Potassium>>();
        charge::<Chloride, Concentration<Chloride>>();
        charge::<Lactate, Concentration<Lactate>>();
        charge::<Ketones, Concentration<Ketones>>();
        charge::<Ammonium, Concentration<Ammonium>>();
        charge::<FixedAcid, Concentration<FixedAcid>>();
        charge::<DietBase, Concentration<DietBase>>();
        charge::<CellAnion, Concentration<CellAnion>>();
        charge::<Calcium, Concentration<Calcium>>();
        charge::<Magnesium, Concentration<Magnesium>>();
        osmoles::<Glucose, Concentration<Glucose>>();
        osmoles::<Urea, Concentration<Urea>>();
        osmoles::<Creatinine, Concentration<Creatinine>>();
        osmoles::<Phosphate, Concentration<Phosphate>>();
        osmoles::<CellAnion, Concentration<CellAnion>>();
    }
}
