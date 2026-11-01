//! The coupling catalog: one row per arrow of the "Direct couplings" graph in
//! `docs/atlas/coupling-map.md` (lines 77-114). Each row gives its census entry and, when mapped, a payload
//! with every field the map's label names, so there is no second list of ports.
//!
//! A payload is generic over a [`Side`]. `FragmentBuilder::emit` gives the map's sender one [`Output`] per
//! field and records a law owed on each. `FragmentBuilder::receive` gives the map's receiver one [`ReadPort`]
//! per field. A wire the map does not draw has no sender and no receiver, so it does not type-check.
//!
//! Dispositions: `live` (both ends in the slice), `input` (a held stand-in sends into the slice), `sink` (the
//! slice sends to a held stand-in that consumes no physiology), `relation` (an anatomy relation, not a wire),
//! `outside` (neither end in the slice) and `unresolved` (an ownership conflict kept visible). Mapped means the
//! route has a declared home, not that the outside system is simulated.
//!
//! The counts are compiler-checked below. `tools/verify` compares these rows with the map and with the
//! sketch's copy, line by line (`evidence/census-check.py`).
use crate::law::{Brand, Output, ReadPort};
use crate::{
    Ammonia, BloodChemistry, CapillaryBed, Cardiovascular, Concentration, Dimensionless, Endocrine, Energy,
    Fluids, Gastrointestinal, Hematology, Hepatic, Immune, LawName, Metabolism, Musculoskeletal, Nervous,
    Owner, Oxygen, PerSite, Phase, Pressure, Rate, Renal, Renin, Respiratory, Temperature, Viscosity, Volume,
};
use core::marker::PhantomData;

pub(crate) use private::Field;
use private::Token;

/// Only this module makes a `Token`, so only `emit` and `receive` issue a port, and no type outside the
/// contract can be a `Side` or a `Coupling`.
mod private {
    use crate::law::{Carried, Over};

    #[derive(Clone, Copy)]
    pub struct Token(pub(super) ());
    /// A field `emit` records: a law owed on one row, or on one row per site.
    pub struct Field {
        pub(crate) name: &'static str,
        pub(crate) over: Over,
        pub(crate) dimension: String,
    }
    impl Field {
        pub(super) fn of<F: Carried>(name: &'static str) -> Self {
            Self {
                name,
                over: F::over(),
                dimension: F::dimension(),
            }
        }
    }
}

/// What a payload field grants on one side of an arrow.
pub trait Side {
    type Field<U>;
    fn field<U>(token: Token, sender: &'static str, field: &'static str) -> Self::Field<U>;
}
/// The receiver's side: one read port per field.
pub struct Reads<'b, O>(Brand<'b>, PhantomData<O>);
/// The sender's side: one output per field.
pub struct Defines<'b, O>(Brand<'b>, PhantomData<O>);
impl<'b, O> Side for Reads<'b, O> {
    type Field<U> = ReadPort<'b, O, U>;
    fn field<U>(_: Token, owner: &'static str, local: &'static str) -> ReadPort<'b, O, U> {
        ReadPort::field(LawName { owner, local })
    }
}
impl<'b, O> Side for Defines<'b, O> {
    type Field<U> = Output<'b, O, U>;
    fn field<U>(_: Token, _: &'static str, field: &'static str) -> Output<'b, O, U> {
        Output::named(field)
    }
}

/// A mapped arrow. Only the generated payloads implement it.
pub trait Coupling: Sized {
    type From: Owner;
    type To: Owner;
    type Side: Side;
    fn issue(token: Token) -> (Self, Vec<Field>);
}
/// The ports of one arrow and the fields its sender owes.
pub(crate) fn issued<P: Coupling>() -> (P, Vec<Field>) {
    P::issue(Token(()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Live,
    ExternalInput,
    ExternalSink,
    /// Realized by an anatomy relation, not a wire: an enclosing space's pressure, or site positions
    /// under the boundary's posture.
    AnatomyRelation {
        reason: &'static str,
    },
    OutsideSlice {
        reason: &'static str,
    },
    Unresolved {
        reason: &'static str,
    },
}
impl Disposition {
    pub const fn is_mapped(&self) -> bool {
        matches!(
            self,
            Disposition::Live
                | Disposition::ExternalInput
                | Disposition::ExternalSink
                | Disposition::AnatomyRelation { .. }
        )
    }
}
/// One arrow of the map and what the slice does with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CensusRow {
    pub line: u16,
    pub disposition: Disposition,
    pub sender_in_slice: bool,
    pub receiver_in_slice: bool,
}
impl CensusRow {
    const fn incident(&self) -> bool {
        self.sender_in_slice || self.receiver_in_slice
    }
    /// A disposition says which ends of its arrow run in the slice.
    const fn ends_fit(&self) -> bool {
        let (sender, receiver) = (self.sender_in_slice, self.receiver_in_slice);
        match self.disposition {
            Disposition::Live => sender && receiver,
            Disposition::ExternalInput => !sender && receiver,
            Disposition::ExternalSink => sender && !receiver,
            Disposition::OutsideSlice { .. } => !sender && !receiver,
            Disposition::AnatomyRelation { .. } | Disposition::Unresolved { .. } => self.incident(),
        }
    }
}

const MAP_EDGES: usize = 38;

macro_rules! coupling_map {
    ($($line:literal $from:ident => $to:ident, $label:literal, $kind:ident $body:tt;)*) => {
        $(coupling_map!(@payload $kind $body $line $from $to $label);)*
        /// The census: the only list of the map's arrows. The payloads above come from it.
        pub const CENSUS: [CensusRow; MAP_EDGES] = [$(
            coupling_map!(@row $kind $body $line $from $to $label),
        )*];
    };
    (@payload relation $why:literal $line:literal $from:ident $to:ident $label:literal) => {};
    (@payload outside $why:literal $line:literal $from:ident $to:ident $label:literal) => {};
    (@payload unresolved $why:literal $line:literal $from:ident $to:ident $label:literal) => {};
    (@payload $kind:ident ($name:ident { $($field:ident: $unit:ty),+ $(,)? })
        $line:literal $from:ident $to:ident $label:literal) => {
        #[doc = concat!("Coupling-map line ", stringify!($line), ", ", stringify!($from), " to ",
            stringify!($to), ": \"", $label, "\".")]
        pub struct $name<S: Side> {
            $(pub $field: S::Field<$unit>,)+
        }
        impl<S: Side> Coupling for $name<S> {
            type From = $from;
            type To = $to;
            type Side = S;
            fn issue(token: Token) -> (Self, Vec<Field>) {
                let sender = <$from as Owner>::CARD;
                let payload = Self {
                    $($field: S::field::<$unit>(token, sender, stringify!($field)),)+
                };
                (payload, vec![$(Field::of::<$unit>(stringify!($field))),+])
            }
        }
    };
    (@row relation $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to, Disposition::AnatomyRelation { reason: $why })
    };
    (@row outside $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to, Disposition::OutsideSlice { reason: $why })
    };
    (@row unresolved $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to, Disposition::Unresolved { reason: $why })
    };
    (@row $kind:ident ($name:ident { $($field:ident: $unit:ty),+ $(,)? })
        $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to, coupling_map!(@disposition $kind))
    };
    (@census $line:literal $from:ident $to:ident, $disposition:expr) => {
        CensusRow {
            line: $line,
            disposition: $disposition,
            sender_in_slice: <$from as Owner>::IN_SLICE,
            receiver_in_slice: <$to as Owner>::IN_SLICE,
        }
    };
    (@disposition live) => { Disposition::Live };
    (@disposition input) => { Disposition::ExternalInput };
    (@disposition sink) => { Disposition::ExternalSink };
}

coupling_map! {
    77 Cardiovascular => Nervous, "baroreceptor afferents",
        live (BaroreceptorAfferents { carotid_sinus_stretch: Dimensionless, aortic_arch_stretch: Dimensionless });
    78 Cardiovascular => Metabolism, "myocardial work, flow per bed",
        sink (MyocardialWorkAndBedFlow { myocardial_work: Rate<Energy>, flow_per_bed: PerSite<CapillaryBed, Rate<Volume>> });
    79 Cardiovascular => Endocrine, "wall stress",
        sink (WallStress { atrial_wall_stress: Pressure, ventricular_wall_stress: Pressure });
    80 Cardiovascular => Fluids, "capillary and venous pressure",
        live (CapillaryAndVenousPressure { capillary_pressure: PerSite<CapillaryBed, Pressure>, venous_pressure: PerSite<CapillaryBed, Pressure> });
    81 Cardiovascular => Respiratory, "pulmonary capillary flow",
        sink (PulmonaryCapillaryFlow { pulmonary_capillary_flow: Rate<Volume> });
    82 Respiratory => Cardiovascular, "pleural pressure",
        relation "the pleural space's pressure, felt by every bed it encloses";
    83 Respiratory => Nervous, "lung afferents",
        input (LungAfferents { pulmonary_stretch: Dimensionless });
    84 Respiratory => Metabolism, "work of breathing", outside "neither end in the arena slice";
    85 Respiratory => Musculoskeletal, "pump load", outside "neither end in the arena slice";
    86 Nervous => Respiratory, "respiratory drive",
        sink (RespiratoryDrive { respiratory_drive: Dimensionless });
    87 Nervous => Metabolism, "thermostat, shiver and sweat",
        sink (ThermostatShiverSweat { thermostat_set_point: Temperature, shiver_drive: Dimensionless,
            sweat_drive: Dimensionless });
    88 Nervous => Musculoskeletal, "motor drive",
        sink (MotorDrive { motor_drive: Dimensionless });
    89 Nervous => Endocrine, "CRH, vasopressin, circadian phase",
        sink (HypothalamicDrive { crh_drive: Dimensionless, vasopressin_drive: Dimensionless,
            circadian_phase: Phase });
    90 Metabolism => Cardiovascular, "O2 debt per bed, temperature",
        input (OxygenDebtAndTemperature { oxygen_debt_per_bed: PerSite<CapillaryBed, Oxygen>, temperature: Temperature });
    91 Metabolism => Nervous, "temperature",
        input (CoreTemperature { core_temperature: Temperature });
    92 Metabolism => Hematology, "temperature", outside "neither end in the arena slice";
    93 Metabolism => Hepatic, "temperature", outside "neither end in the arena slice";
    94 Metabolism => BloodChemistry, "temperature", outside "neither end in the arena slice";
    95 Musculoskeletal => Respiratory, "pump force and fatigue", outside "neither end in the arena slice";
    96 Musculoskeletal => Cardiovascular, "position and gravity",
        relation "site positions under the boundary's posture";
    97 Renal => Cardiovascular, "tubuloglomerular feedback",
        live (TubuloglomerularFeedback { afferent_tone: Dimensionless });
    98 Renal => Endocrine, "renin",
        sink (ReninSecretion { renin_secretion: Rate<Renin> });
    99 Hepatic => Cardiovascular, "portal pressure",
        input (PortalPressure { portal_pressure: Pressure });
    100 Hepatic => Nervous, "ammonia",
        input (PlasmaAmmonia { plasma_ammonia: Concentration<Ammonia> });
    101 Gastrointestinal => Cardiovascular, "intra-abdominal pressure",
        relation "the abdominal space's pressure, felt by every bed it encloses";
    102 Gastrointestinal => Renal, "intra-abdominal pressure",
        input (AbdominalPressureOnKidney { intra_abdominal_pressure: Pressure });
    103 Gastrointestinal => Respiratory, "intra-abdominal pressure", outside "neither end in the arena slice";
    104 Gastrointestinal => Hepatic, "portal delivery", outside "neither end in the arena slice";
    105 Immune => Cardiovascular, "iNOS, myocardial depression",
        input (InosAndMyocardialDepression { inos_activity: Dimensionless, myocardial_depression: Dimensionless });
    106 Immune => Fluids, "permeability, glycocalyx",
        input (PermeabilityAndGlycocalyx { permeability: PerSite<CapillaryBed, Dimensionless>,
            glycocalyx: PerSite<CapillaryBed, Dimensionless> });
    107 Immune => Hematology, "tissue factor, hepcidin", outside "neither end in the arena slice";
    108 Immune => Nervous, "set point shift",
        input (SetPointShift { set_point_shift: Temperature });
    109 Immune => Metabolism, "mitochondrial efficiency", outside "neither end in the arena slice";
    110 Fluids => Respiratory, "lung water",
        sink (LungWater { lung_water: Volume });
    111 Fluids => Nervous, "brain water and ICP", unresolved
        "ICP ownership conflict (FL card 24 vs NS card 11); the minimal baroreflex needs neither";
    112 Fluids => Gastrointestinal, "gut wall water",
        sink (GutWallWater { gut_wall_water: Volume });
    113 Fluids => Musculoskeletal, "skin and compartment water",
        sink (SkinAndCompartmentWater { skin_water: Volume, compartment_water: Volume });
    114 Hematology => Cardiovascular, "bleeding competence, viscosity, thrombosis",
        input (BleedingViscosityThrombosis { bleeding_competence: Dimensionless, viscosity: Viscosity,
            thrombosis: Dimensionless });
}

const fn count(mapped: bool, slice_only: bool) -> usize {
    let mut i = 0;
    let mut n = 0;
    while i < MAP_EDGES {
        let row = &CENSUS[i];
        if row.disposition.is_mapped() == mapped && (row.incident() || !slice_only) {
            n += 1;
        }
        i += 1;
    }
    n
}
const _: () = assert!(count(true, false) + count(false, false) == MAP_EDGES);
const _: () = assert!(count(true, false) == 27 && count(false, false) == 11);
const _: () = assert!(count(true, true) == 27 && count(false, true) == 1);
const _: () = {
    let mut i = 0;
    while i < MAP_EDGES {
        assert!(CENSUS[i].line as usize == 77 + i);
        assert!(CENSUS[i].ends_fit());
        i += 1;
    }
};
