use super::table::Release::{self, Kept, Ticket};
use ng_contract::*;

/// A fluid route whose law keeps species back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FluidRoute {
    Starling,
    Lymph,
    Filtration,
}
impl FluidRoute {
    /// The origin of the route's columns.
    pub const fn law(self) -> LawName {
        match self {
            FluidRoute::Starling => LawName {
                owner: Fluids::CARD,
                local: "starling",
            },
            FluidRoute::Lymph => LawName {
                owner: Fluids::CARD,
                local: "lymph",
            },
            FluidRoute::Filtration => LawName {
                owner: Renal::CARD,
                local: "filtration",
            },
        }
    }
}

/// One `keeps_back` line of a fluid route's law module, and what releases it. An inert line keeps back a species
/// neither end of the route holds in any assembly.
#[derive(Clone, Copy, Debug)]
pub struct KeepBack {
    pub route: FluidRoute,
    pub species: Species,
    pub release: Release,
    pub inert: bool,
}

const fn line<Q: Substance>(route: FluidRoute, release: Release) -> KeepBack {
    let species = Q::SPECIES;
    KeepBack {
        route,
        species,
        release,
        inert: false,
    }
}

const fn inert<Q: Substance>(route: FluidRoute) -> KeepBack {
    let species = Q::SPECIES;
    KeepBack {
        route,
        species,
        release: Kept,
        inert: true,
    }
}

const FL: &str = Fluids::CARD;
const RN: &str = Renal::CARD;
const MT: &str = Metabolism::CARD;
use FluidRoute::{Filtration, Lymph, Starling};

/// `crates/ng-fluids/src/starling.rs`, `lymph.rs` and `crates/ng-renal/src/filtration.rs`, line for line, with the
/// ticket body 01 gives each crossing. Filtration's are kept where body 01 puts no urinary pool of the species.
pub const KEEP_BACKS: &[KeepBack] = &[
    line::<Vasopressin>(Starling, Kept),
    line::<Oxygen>(Starling, Ticket(8, MT)),
    line::<CarbonDioxide>(Starling, Ticket(8, MT)),
    line::<Glucose>(Starling, Ticket(9, FL)),
    line::<Triglyceride>(Starling, Kept),
    line::<FattyAcids>(Starling, Ticket(9, FL)),
    line::<Glycerol>(Starling, Ticket(14, FL)),
    line::<AminoAcids>(Starling, Ticket(13, FL)),
    line::<Lactate>(Starling, Ticket(9, FL)),
    line::<Ketones>(Starling, Ticket(16, FL)),
    line::<Urea>(Starling, Ticket(14, FL)),
    inert::<Ammonium>(Starling),
    line::<Creatinine>(Starling, Ticket(16, FL)),
    line::<FixedAcid>(Starling, Ticket(18, FL)),
    line::<DietBase>(Starling, Ticket(18, FL)),
    line::<Calcium>(Starling, Ticket(19, FL)),
    line::<Magnesium>(Starling, Ticket(19, FL)),
    line::<Phosphate>(Starling, Ticket(19, FL)),
    line::<Hemoglobin>(Starling, Kept),
    line::<Iron>(Starling, Ticket(21, FL)),
    line::<Insulin>(Starling, Ticket(15, FL)),
    line::<Glucagon>(Starling, Ticket(15, FL)),
    line::<Renin>(Starling, Kept),
    line::<AngiotensinII>(Starling, Ticket(17, FL)),
    line::<Aldosterone>(Starling, Ticket(17, FL)),
    line::<NatriureticPeptide>(Starling, Ticket(17, FL)),
    line::<ParathyroidHormone>(Starling, Ticket(19, FL)),
    line::<Cortisol>(Starling, Ticket(20, FL)),
    line::<Vasopressin>(Lymph, Kept),
    line::<Oxygen>(Lymph, Ticket(8, FL)),
    line::<CarbonDioxide>(Lymph, Ticket(8, FL)),
    line::<Glucose>(Lymph, Ticket(9, FL)),
    line::<Triglyceride>(Lymph, Ticket(13, FL)),
    line::<FattyAcids>(Lymph, Ticket(9, FL)),
    line::<Glycerol>(Lymph, Ticket(14, FL)),
    line::<AminoAcids>(Lymph, Ticket(13, FL)),
    line::<Lactate>(Lymph, Ticket(9, FL)),
    line::<Ketones>(Lymph, Ticket(16, FL)),
    line::<Urea>(Lymph, Ticket(14, FL)),
    inert::<Ammonium>(Lymph),
    line::<Creatinine>(Lymph, Ticket(16, FL)),
    line::<FixedAcid>(Lymph, Ticket(18, FL)),
    line::<DietBase>(Lymph, Ticket(18, FL)),
    line::<Calcium>(Lymph, Ticket(19, FL)),
    line::<Magnesium>(Lymph, Ticket(19, FL)),
    line::<Phosphate>(Lymph, Ticket(19, FL)),
    line::<Hemoglobin>(Lymph, Kept),
    line::<Iron>(Lymph, Ticket(21, FL)),
    line::<Insulin>(Lymph, Ticket(15, FL)),
    line::<Glucagon>(Lymph, Ticket(15, FL)),
    line::<Renin>(Lymph, Kept),
    line::<AngiotensinII>(Lymph, Ticket(17, FL)),
    line::<Aldosterone>(Lymph, Ticket(17, FL)),
    line::<NatriureticPeptide>(Lymph, Ticket(17, FL)),
    line::<ParathyroidHormone>(Lymph, Ticket(19, FL)),
    line::<Cortisol>(Lymph, Ticket(20, FL)),
    line::<Albumin>(Filtration, Kept),
    line::<Vasopressin>(Filtration, Kept),
    line::<Oxygen>(Filtration, Kept),
    line::<CarbonDioxide>(Filtration, Ticket(18, RN)),
    line::<Glucose>(Filtration, Ticket(15, RN)),
    line::<Triglyceride>(Filtration, Kept),
    line::<FattyAcids>(Filtration, Kept),
    line::<Glycerol>(Filtration, Kept),
    line::<AminoAcids>(Filtration, Kept),
    line::<Lactate>(Filtration, Kept),
    line::<Ketones>(Filtration, Kept),
    line::<Urea>(Filtration, Ticket(14, RN)),
    line::<Ammonium>(Filtration, Kept),
    line::<Creatinine>(Filtration, Ticket(16, RN)),
    line::<FixedAcid>(Filtration, Ticket(18, RN)),
    line::<DietBase>(Filtration, Kept),
    line::<Calcium>(Filtration, Ticket(19, RN)),
    line::<Magnesium>(Filtration, Ticket(19, RN)),
    line::<Phosphate>(Filtration, Ticket(19, RN)),
    line::<Hemoglobin>(Filtration, Kept),
    line::<Iron>(Filtration, Kept),
    line::<Insulin>(Filtration, Kept),
    line::<Glucagon>(Filtration, Kept),
    line::<Renin>(Filtration, Kept),
    line::<AngiotensinII>(Filtration, Kept),
    line::<Aldosterone>(Filtration, Kept),
    line::<NatriureticPeptide>(Filtration, Kept),
    line::<ParathyroidHormone>(Filtration, Kept),
    line::<Cortisol>(Filtration, Kept),
];
