//! The coupling catalog: one declaration per arrow of the "Direct couplings" graph in
//! `docs/atlas/coupling-map.md` (lines 77-114). Each row emits BOTH its census entry and, when
//! mapped, its port type: a structured payload carrying every field the map's label names. There
//! is no second hand-written list of ports.
//!
//! A payload is generic over a [`Side`]: the emitting owner receives `Payload<Defines<..>>` (one
//! [`Output`] per field, each defined once), the receiving owner `Payload<Reads<..>>` (one
//! [`ReadPort`] per field). `FragmentBuilder::receive` requires `To = Owner`, so a wire the map
//! does not draw cannot be read.
//!
//! Dispositions: `live` (both ends in the arena slice), `input` (a held stand-in emits into the
//! slice), `sink` (the slice emits to a held stand-in that consumes no physiology), `outside`
//! (neither end in the slice) and `unresolved` (an ownership conflict kept visible). Mapped means
//! the route has a declared home, not that the outside system is simulated.
//!
//! The counts are compiler-checked below. A repository gate must later compare these rows,
//! line by line, with `docs/atlas/coupling-map.md`, so an added, removed or relabelled arrow fails
//! the build rather than waiting for a review; today that comparison is evidence only
//! (`solver-contract-sketch/evidence/census.txt`).
use super::*;
use core::marker::PhantomData;

/// What a payload field grants on one side of an edge.
pub trait Side {
    type Field<U>;
}
/// The receiving owner's view: one read port per field.
pub struct Reads<'b, O>(Brand<'b>, PhantomData<O>);
/// The emitting owner's view: one definition capability per field.
pub struct Defines<'b, O>(Brand<'b>, PhantomData<O>);
impl<'b, O> Side for Reads<'b, O> {
    type Field<U> = ReadPort<'b, O, U>;
}
impl<'b, O> Side for Defines<'b, O> {
    type Field<U> = Output<'b, O, U>;
}

mod sealed {
    pub trait Coupling {}
}
/// A mapped coupling-map arrow. Implemented only by the generated payloads below.
pub trait Coupling: sealed::Coupling {
    type From: System;
    type To: System;
    type Side: Side;
    const LINE: u16;
    const LABEL: &'static str;
    const DISPOSITION: Disposition;
}

#[derive(Clone, Copy)]
pub enum Disposition {
    Live,
    ExternalInput,
    ExternalSink,
    /// Realized by an anatomy relation, not a wire: the enclosing space's pressure, or site
    /// positions under the boundary's posture.
    AnatomyRelation { reason: &'static str },
    OutsideSlice { reason: &'static str },
    Unresolved { reason: &'static str },
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
/// One census row: a map arrow and its disposition, with the payload's field names.
#[derive(Clone, Copy)]
pub struct CensusRow {
    pub line: u16,
    pub from: &'static str,
    pub to: &'static str,
    pub label: &'static str,
    pub disposition: Disposition,
    pub payload: Option<&'static str>,
    pub fields: &'static [&'static str],
    pub slice_incident: bool,
}

/// Arrows in the map's "Direct couplings" graph. The census array's type has this length.
pub const MAP_EDGES: usize = 38;

macro_rules! coupling_map {
    ($($line:literal $from:ident => $to:ident, $label:literal, $kind:ident $body:tt;)*) => {
        $(coupling_map!(@payload $kind $body $line $from $to $label);)*
        /// The census: the only list of coupling-map arrows. Port types above are generated from it.
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
        impl<S: Side> sealed::Coupling for $name<S> {}
        impl<S: Side> Coupling for $name<S> {
            type From = $from;
            type To = $to;
            type Side = S;
            const LINE: u16 = $line;
            const LABEL: &'static str = $label;
            const DISPOSITION: Disposition = coupling_map!(@disposition $kind);
        }
    };
    (@row relation $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to $label, Disposition::AnatomyRelation { reason: $why }, None, &[])
    };
    (@row outside $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to $label, Disposition::OutsideSlice { reason: $why }, None, &[])
    };
    (@row unresolved $why:literal $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to $label, Disposition::Unresolved { reason: $why }, None, &[])
    };
    (@row $kind:ident ($name:ident { $($field:ident: $unit:ty),+ $(,)? })
        $line:literal $from:ident $to:ident $label:literal) => {
        coupling_map!(@census $line $from $to $label, coupling_map!(@disposition $kind),
            Some(stringify!($name)), &[$(stringify!($field)),+])
    };
    (@census $line:literal $from:ident $to:ident $label:literal, $disposition:expr, $payload:expr, $fields:expr) => {
        CensusRow {
            line: $line,
            from: <$from as System>::CARD,
            to: <$to as System>::CARD,
            label: $label,
            disposition: $disposition,
            payload: $payload,
            fields: $fields,
            slice_incident: <$from as System>::IN_SLICE || <$to as System>::IN_SLICE,
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
        if row.disposition.is_mapped() == mapped && (row.slice_incident || !slice_only) {
            n += 1;
        }
        i += 1;
    }
    n
}
pub const MAPPED: usize = count(true, false);
pub const UNMAPPED: usize = count(false, false);
pub const SLICE_INCIDENT_MAPPED: usize = count(true, true);
pub const SLICE_INCIDENT_UNMAPPED: usize = count(false, true);
// 38 = mapped + listed unmapped; the figures the design doc states; rows are the map's
// consecutive arrow lines 77..=114 in order.
const _: () = assert!(MAPPED + UNMAPPED == MAP_EDGES);
const _: () = assert!(MAPPED == 27 && UNMAPPED == 11);
const _: () = assert!(SLICE_INCIDENT_MAPPED == 27 && SLICE_INCIDENT_UNMAPPED == 1);
const _: () = {
    let mut i = 0;
    while i < MAP_EDGES {
        assert!(CENSUS[i].line as usize == 77 + i);
        i += 1;
    }
};
