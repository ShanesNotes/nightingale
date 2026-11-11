//! The coupling catalog: one row per arrow of the "Direct couplings" graph in
//! `docs/atlas/coupling-map.md` (lines 77-114).
//!
//! Mapped means the route has a declared home, not that the outside system is simulated.
use crate::law::{Brand, Carried, Output, ReadPort};
use crate::{
    Ammonia, BloodChemistry, CapillaryBed, Cardiovascular, Concentration, Dimensionless, Endocrine, Energy,
    Fluids, Gastrointestinal, Hematology, Hepatic, Immune, LawName, Metabolism, Musculoskeletal, Nervous,
    Owner, Oxygen, PerSite, Phase, Pressure, Rate, Renal, Renin, Respiratory, Temperature, Viscosity, Volume,
};
use core::marker::PhantomData;

pub(crate) use private::Field;
use private::Token;

/// Only this module makes a `Token`, so a coupling's ports come only from `emit` and `receive`, and no type
/// outside the contract can be a `Side` or a `Coupling`.
mod private {
    use crate::law::{Carried, Over};

    #[derive(Clone, Copy)]
    pub struct Token(pub(super) ());
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

pub trait Side {
    type Field<U>;
    fn field<U>(token: Token, sender: &'static str, field: &'static str) -> Self::Field<U>;
}
pub struct Reads<'b, O>(Brand<'b>, PhantomData<O>);
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

/// Only the generated payloads implement this.
pub trait Coupling: Sized {
    type From: Owner;
    type To: Owner;
    type Side: Side;
    fn issue(token: Token) -> (Self, Vec<Field>);
}
pub(crate) fn issued<P: Coupling>() -> (P, Vec<Field>) {
    P::issue(Token(()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Live,
    ExternalInput,
    ExternalSink,
    /// Realized by an anatomy relation, not a wire.
    AnatomyRelation {
        reason: &'static str,
    },
    OutsideSlice {
        reason: &'static str,
    },
    Unresolved {
        reason: &'static str,
    },
    Held,
}
impl Disposition {
    pub const fn is_mapped(&self) -> bool {
        matches!(
            self,
            Disposition::Live
                | Disposition::ExternalInput
                | Disposition::ExternalSink
                | Disposition::AnatomyRelation { .. }
                | Disposition::Held
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CensusRow {
    pub line: u16,
    pub disposition: Disposition,
    pub sender_runs: bool,
    pub receiver_runs: bool,
}
impl CensusRow {
    const fn incident(&self) -> bool {
        self.sender_runs || self.receiver_runs
    }
    const fn ends_fit(&self) -> bool {
        let (sender, receiver) = (self.sender_runs, self.receiver_runs);
        match self.disposition {
            Disposition::Live | Disposition::Held => sender && receiver,
            Disposition::ExternalInput => !sender && receiver,
            Disposition::ExternalSink => sender && !receiver,
            Disposition::OutsideSlice { .. } => !sender && !receiver,
            Disposition::AnatomyRelation { .. } | Disposition::Unresolved { .. } => self.incident(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WiredField {
    pub name: &'static str,
    pub unit: &'static str,
    pub ticket: u8,
}
/// A held field is no member of its arrow's payload: EN cannot read the ventricular wall stress arrow 79 holds.
/// ```compile_fail,E0609
/// use ng_contract::{catalog::*, *};
/// fn read<'b>(stress: WallStress<Reads<'b, Endocrine>>) -> ReadPort<'b, Endocrine, Pressure> {
///     stress.ventricular_wall_stress
/// }
/// ```
/// ```
/// use ng_contract::{catalog::*, *};
/// fn read<'b>(stress: WallStress<Reads<'b, Endocrine>>) -> ReadPort<'b, Endocrine, Pressure> {
///     stress.atrial_wall_stress
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldField {
    pub name: &'static str,
    pub unit: &'static str,
    pub holder: &'static str,
    pub reason: &'static str,
}
/// No arrow is outside the whole body or unresolved in it.
/// ```compile_fail,E0599
/// use ng_contract::catalog::*;
/// const ICP: Carries = Carries::Unresolved { reason: "ICP ownership conflict" };
/// ```
/// ```
/// use ng_contract::catalog::*;
/// const ICP: Carries = Carries::Fields {
///     wired: &[],
///     held: &[HeldField { name: "intracranial_pressure", unit: "Pressure", holder: "NS", reason: "no skull" }],
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carries {
    Relation {
        reason: &'static str,
    },
    Fields {
        wired: &'static [WiredField],
        held: &'static [HeldField],
    },
}
impl Carries {
    pub const fn wired(&self) -> &'static [WiredField] {
        match *self {
            Carries::Fields { wired, .. } => wired,
            Carries::Relation { .. } => &[],
        }
    }
    pub const fn held(&self) -> &'static [HeldField] {
        match *self {
            Carries::Fields { held, .. } => held,
            Carries::Relation { .. } => &[],
        }
    }
    const fn disposition(&self) -> Disposition {
        match *self {
            Carries::Relation { reason } => Disposition::AnatomyRelation { reason },
            Carries::Fields { wired: [], .. } => Disposition::Held,
            Carries::Fields { .. } => Disposition::Live,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrow {
    pub line: u16,
    pub from: &'static str,
    pub to: &'static str,
    pub sender_in_slice: bool,
    pub receiver_in_slice: bool,
    pub slice: Disposition,
    pub carries: Carries,
}
impl Arrow {
    pub const fn accounted(&self) -> bool {
        match self.carries {
            Carries::Relation { .. } => true,
            Carries::Fields { wired, held } => !wired.is_empty() || !held.is_empty(),
        }
    }
    pub const fn held_at_its_ends(&self) -> bool {
        let held = self.carries.held();
        let mut i = 0;
        while i < held.len() {
            if !same(held[i].holder, self.from) && !same(held[i].holder, self.to) {
                return false;
            }
            i += 1;
        }
        true
    }
    pub const fn wired_by_body_tickets(&self) -> bool {
        let wired = self.carries.wired();
        let mut i = 0;
        while i < wired.len() {
            if wired[i].ticket < FIRST_WIRING_TICKET || wired[i].ticket > LAST_WIRING_TICKET {
                return false;
            }
            i += 1;
        }
        true
    }
    pub const fn every_reason_given(&self) -> bool {
        let slice = match self.slice {
            Disposition::AnatomyRelation { reason }
            | Disposition::OutsideSlice { reason }
            | Disposition::Unresolved { reason } => !reason.is_empty(),
            Disposition::Live
            | Disposition::ExternalInput
            | Disposition::ExternalSink
            | Disposition::Held => true,
        };
        let relation = match self.carries {
            Carries::Relation { reason } => !reason.is_empty(),
            Carries::Fields { .. } => true,
        };
        let held = self.carries.held();
        let mut i = 0;
        while i < held.len() {
            if held[i].reason.is_empty() {
                return false;
            }
            i += 1;
        }
        slice && relation
    }
    pub const fn one_holder_per_field(&self) -> bool {
        let (wired, held) = (self.carries.wired(), self.carries.held());
        let fields = wired.len() + held.len();
        let mut i = 0;
        while i < fields {
            let mut j = i + 1;
            while j < fields {
                if same(field_name(wired, held, i), field_name(wired, held, j)) {
                    return false;
                }
                j += 1;
            }
            i += 1;
        }
        true
    }
}
const fn field_name(wired: &[WiredField], held: &[HeldField], i: usize) -> &'static str {
    if i < wired.len() {
        wired[i].name
    } else {
        held[i - wired.len()].name
    }
}
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Census {
    Slice,
    WholeBody,
}
impl Census {
    pub const fn row(self, arrow: &Arrow) -> CensusRow {
        match self {
            Census::Slice => CensusRow {
                line: arrow.line,
                disposition: arrow.slice,
                sender_runs: arrow.sender_in_slice,
                receiver_runs: arrow.receiver_in_slice,
            },
            Census::WholeBody => CensusRow {
                line: arrow.line,
                disposition: arrow.carries.disposition(),
                sender_runs: true,
                receiver_runs: true,
            },
        }
    }
    pub const fn count(self) -> Count {
        let mut count = Count {
            live: 0,
            input: 0,
            sink: 0,
            relation: 0,
            held: 0,
            outside: 0,
            unresolved: 0,
            incident_mapped: 0,
            incident_unmapped: 0,
        };
        let mut i = 0;
        while i < MAP_EDGES {
            let row = self.row(&ARROWS[i]);
            match row.disposition {
                Disposition::Live => count.live += 1,
                Disposition::ExternalInput => count.input += 1,
                Disposition::ExternalSink => count.sink += 1,
                Disposition::AnatomyRelation { .. } => count.relation += 1,
                Disposition::Held => count.held += 1,
                Disposition::OutsideSlice { .. } => count.outside += 1,
                Disposition::Unresolved { .. } => count.unresolved += 1,
            }
            if row.incident() && row.disposition.is_mapped() {
                count.incident_mapped += 1;
            } else if row.incident() {
                count.incident_unmapped += 1;
            }
            i += 1;
        }
        count
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    pub live: usize,
    pub input: usize,
    pub sink: usize,
    pub relation: usize,
    pub held: usize,
    pub outside: usize,
    pub unresolved: usize,
    pub incident_mapped: usize,
    pub incident_unmapped: usize,
}
impl Count {
    pub const fn mapped(&self) -> usize {
        self.live + self.input + self.sink + self.relation + self.held
    }
    pub const fn unmapped(&self) -> usize {
        self.outside + self.unresolved
    }
}

const fn carried_unit<U: Carried>(written: &'static str) -> &'static str {
    written
}

const MAP_EDGES: usize = 38;
const FIRST_WIRING_TICKET: u8 = 6;
const LAST_WIRING_TICKET: u8 = 25;

macro_rules! coupling_map {
    ($($line:literal $from:ident => $to:ident, $label:literal, $slice:ident $($why:literal)?,
        $plan:ident $carried:tt $(held $held:tt)?;)*) => {
        $(coupling_map!(@payload $plan $carried $line $from $to $label);)*
        pub const ARROWS: [Arrow; MAP_EDGES] = [$(Arrow {
            line: $line,
            from: <$from as Owner>::CARD,
            to: <$to as Owner>::CARD,
            sender_in_slice: <$from as Owner>::IN_SLICE,
            receiver_in_slice: <$to as Owner>::IN_SLICE,
            slice: coupling_map!(@slice $slice $($why)?),
            carries: coupling_map!(@carries $plan $carried $($held)?),
        },)*];
    };
    (@payload wired ($name:ident { $($field:ident: $unit:ty => $ticket:literal),+ $(,)? })
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
    (@payload $plan:ident $carried:tt $line:literal $from:ident $to:ident $label:literal) => {};
    (@slice live) => { Disposition::Live };
    (@slice input) => { Disposition::ExternalInput };
    (@slice sink) => { Disposition::ExternalSink };
    (@slice relation $why:literal) => { Disposition::AnatomyRelation { reason: $why } };
    (@slice outside $why:literal) => { Disposition::OutsideSlice { reason: $why } };
    (@slice unresolved $why:literal) => { Disposition::Unresolved { reason: $why } };
    (@carries relation $why:literal) => { Carries::Relation { reason: $why } };
    (@carries wired ($name:ident { $($field:ident: $unit:ty => $ticket:literal),+ $(,)? }) $($held:tt)?) => {
        Carries::Fields {
            wired: &[$(WiredField {
                name: stringify!($field),
                unit: carried_unit::<$unit>(stringify!($unit)),
                ticket: $ticket,
            }),+],
            held: coupling_map!(@held $($held)?),
        }
    };
    (@carries held $held:tt) => { Carries::Fields { wired: &[], held: coupling_map!(@held $held) } };
    (@held) => { &[] };
    (@held [$($field:ident: $unit:ty => $holder:ident $reason:literal),+ $(,)?]) => {
        &[$(HeldField {
            name: stringify!($field),
            unit: carried_unit::<$unit>(stringify!($unit)),
            holder: <$holder as Owner>::CARD,
            reason: $reason,
        }),+]
    };
}

coupling_map! {
    77 Cardiovascular => Nervous, "baroreceptor afferents", live,
        wired (BaroreceptorAfferents { carotid_sinus_stretch: Dimensionless => 6, aortic_arch_stretch: Dimensionless => 6 });
    78 Cardiovascular => Metabolism, "myocardial work, flow per bed", sink,
        wired (MyocardialWorkAndBedFlow { myocardial_work: Rate<Energy> => 9,
            flow_per_bed: PerSite<CapillaryBed, Rate<Volume>> => 9 });
    79 Cardiovascular => Endocrine, "wall stress", sink,
        wired (WallStress { atrial_wall_stress: Pressure => 17 })
        held [ventricular_wall_stress: Pressure => Cardiovascular "one natriuretic peptide is secreted, on atrial stretch"];
    80 Cardiovascular => Fluids, "capillary and venous pressure", live,
        wired (CapillaryAndVenousPressure { capillary_pressure: PerSite<CapillaryBed, Pressure> => 6,
            venous_pressure: PerSite<CapillaryBed, Pressure> => 6 });
    81 Cardiovascular => Respiratory, "pulmonary capillary flow", sink,
        wired (PulmonaryCapillaryFlow { pulmonary_capillary_flow: Rate<Volume> => 10 });
    82 Respiratory => Cardiovascular, "pleural pressure",
        relation "the pleural space's pressure, felt by every bed it encloses",
        relation "the pleural space's pressure, felt by every bed it encloses, defined by the breath from ticket 10";
    83 Respiratory => Nervous, "lung afferents", input,
        wired (LungAfferents { pulmonary_stretch: Dimensionless => 11 });
    84 Respiratory => Metabolism, "work of breathing", outside "neither end in the arena slice",
        wired (WorkOfBreathing { work_of_breathing: Rate<Energy> => 10 });
    85 Respiratory => Musculoskeletal, "pump load", outside "neither end in the arena slice",
        wired (PumpLoad { pump_load: Pressure => 10 });
    86 Nervous => Respiratory, "respiratory drive", sink,
        wired (RespiratoryDrive { respiratory_drive: Dimensionless => 11 });
    87 Nervous => Metabolism, "thermostat, shiver and sweat", sink,
        wired (ThermostatShiverSweat { shiver_drive: Dimensionless => 12 })
        held [thermostat_set_point: Temperature => Nervous "the set point is NS memory and no MT law reads it",
            sweat_drive: Dimensionless => Nervous "sweat answers release at the skin junction (S10)"];
    88 Nervous => Musculoskeletal, "motor drive", sink,
        wired (MotorDrive { motor_drive: Dimensionless => 11 });
    89 Nervous => Endocrine, "CRH, vasopressin, circadian phase", sink,
        wired (HypothalamicDrive { vasopressin_drive: Dimensionless => 17 })
        held [crh_drive: Dimensionless => Nervous "cortisol's profile follows the clock input directly (ticket 20)",
            circadian_phase: Phase => Nervous "the clock is a boundary input every law reads (ticket 05)"];
    90 Metabolism => Cardiovascular, "O2 debt per bed, temperature", input,
        wired (OxygenDebtAndTemperature { oxygen_debt_per_bed: PerSite<CapillaryBed, Oxygen> => 9 })
        held [temperature: Temperature => Cardiovascular "CV's laws run at a reference temperature"];
    91 Metabolism => Nervous, "temperature", input,
        wired (CoreTemperature { core_temperature: Temperature => 12 });
    92 Metabolism => Hematology, "temperature", outside "neither end in the arena slice",
        held [temperature: Temperature => Hematology "HC's laws run at a reference temperature (ticket 12)"];
    93 Metabolism => Hepatic, "temperature", outside "neither end in the arena slice",
        held [temperature: Temperature => Hepatic "HP's laws run at a reference temperature (ticket 12)"];
    94 Metabolism => BloodChemistry, "temperature", outside "neither end in the arena slice",
        held [temperature: Temperature => BloodChemistry "BC's speciation runs at a reference temperature (ticket 12)"];
    95 Musculoskeletal => Respiratory, "pump force and fatigue", outside "neither end in the arena slice",
        wired (PumpForceAndFatigue { pump_force: Pressure => 10 })
        held [fatigue: Dimensionless => Musculoskeletal "a resting day has no fatigue law"];
    96 Musculoskeletal => Cardiovascular, "position and gravity",
        relation "site positions under the boundary's posture",
        relation "site positions under the boundary's posture, and the hydrostatic head those positions give";
    97 Renal => Cardiovascular, "tubuloglomerular feedback", live,
        wired (TubuloglomerularFeedback { afferent_tone: Dimensionless => 6 });
    98 Renal => Endocrine, "renin", sink,
        wired (ReninSecretion { renin_secretion: Rate<Renin> => 17 });
    99 Hepatic => Cardiovascular, "portal pressure", input,
        relation
            "the liver bed on the portal circuit, so CV's one solve gives portal pressure through HP's sinusoidal edge law (ticket 14)";
    100 Hepatic => Nervous, "ammonia", input,
        held [plasma_ammonia: Concentration<Ammonia> => Hepatic
            "urea is made with no ammonia pool and no rung 0 law reads ammonia (ticket 14)"];
    101 Gastrointestinal => Cardiovascular, "intra-abdominal pressure",
        relation "the abdominal space's pressure, felt by every bed it encloses",
        relation "the abdominal space's pressure, felt by every bed it encloses, whose compliance GI holds";
    102 Gastrointestinal => Renal, "intra-abdominal pressure", input,
        held [intra_abdominal_pressure: Pressure => Gastrointestinal "a held constant on a healthy day (ticket 13)"];
    103 Gastrointestinal => Respiratory, "intra-abdominal pressure", outside "neither end in the arena slice",
        held [intra_abdominal_pressure: Pressure => Gastrointestinal
            "the same held constant, and the resting lung volume follows posture directly (ticket 10)"];
    104 Gastrointestinal => Hepatic, "portal delivery", outside "neither end in the arena slice",
        relation "the portal vein carries what the gut absorbs to the liver bed by CV's advection (ticket 13)";
    105 Immune => Cardiovascular, "iNOS, myocardial depression", input,
        held [inos_activity: Dimensionless => Immune "immunity at rest has zero flux (ticket 21)",
            myocardial_depression: Dimensionless => Immune "zero flux"];
    106 Immune => Fluids, "permeability, glycocalyx", input,
        held [permeability: PerSite<CapillaryBed, Dimensionless> => Immune "the healthy wall (ticket 21)",
            glycocalyx: PerSite<CapillaryBed, Dimensionless> => Immune "the healthy wall"];
    107 Immune => Hematology, "tissue factor, hepcidin", outside "neither end in the arena slice",
        held [tissue_factor: Dimensionless => Immune "zero flux (ticket 21)",
            hepcidin: Dimensionless => Immune "zero flux, and iron's store follows its own loss"];
    108 Immune => Nervous, "set point shift", input,
        held [set_point_shift: Temperature => Immune "no fever (ticket 21)"];
    109 Immune => Metabolism, "mitochondrial efficiency", outside "neither end in the arena slice",
        held [mitochondrial_efficiency: Dimensionless => Immune "the healthy value (ticket 21)"];
    110 Fluids => Respiratory, "lung water", sink,
        held [lung_water: Volume => Fluids "no rung 0 lung law reads it (ticket 10's non-goal)"];
    111 Fluids => Nervous, "brain water and ICP", unresolved
        "ICP ownership conflict (FL card 24 vs NS card 11); the minimal baroreflex needs neither",
        held [brain_water: Volume => Fluids "no skull enclosure",
            intracranial_pressure: Pressure => Nervous "no skull enclosure, and NS owns it when one is earned"];
    112 Fluids => Gastrointestinal, "gut wall water", sink,
        held [gut_wall_water: Volume => Fluids "no rung 0 gut law reads it"];
    113 Fluids => Musculoskeletal, "skin and compartment water", sink,
        held [skin_water: Volume => Fluids "pressure injury is absent",
            compartment_water: Volume => Fluids "limb compartments are absent"];
    114 Hematology => Cardiovascular, "bleeding competence, viscosity, thrombosis", input,
        wired (BleedingViscosityThrombosis { viscosity: Viscosity => 8 })
        held [bleeding_competence: Dimensionless => Hematology "no bleeding edge (ticket 21)",
            thrombosis: Dimensionless => Hematology "zero flux (ticket 21)"];
}

const SLICE: Count = Census::Slice.count();
const WHOLE_BODY: Count = Census::WholeBody.count();
const _: () = assert!(SLICE.mapped() + SLICE.unmapped() == MAP_EDGES);
const _: () = assert!(SLICE.mapped() == 27 && SLICE.unmapped() == 11);
const _: () = assert!(SLICE.incident_mapped == 27 && SLICE.incident_unmapped == 1);
const _: () = assert!(WHOLE_BODY.live == 18 && WHOLE_BODY.relation == 5 && WHOLE_BODY.held == 15);
const _: () = assert!(WHOLE_BODY.mapped() == 38 && WHOLE_BODY.unmapped() == 0);
const FIELDS: FieldCount = field_count();
const _: () = assert!(FIELDS.wired == 21 && FIELDS.held == 29 && FIELDS.wire_and_hold == 6);
const _: () = {
    let mut i = 0;
    while i < MAP_EDGES {
        let arrow = &ARROWS[i];
        assert!(arrow.line as usize == 77 + i);
        assert!(Census::Slice.row(arrow).ends_fit() && Census::WholeBody.row(arrow).ends_fit());
        assert!(
            arrow.accounted(),
            "an arrow carries no field and is no anatomy relation"
        );
        assert!(
            arrow.held_at_its_ends(),
            "a held field's holder is neither end of its arrow"
        );
        assert!(
            arrow.wired_by_body_tickets(),
            "a wired field names no body ticket from 06 to 25"
        );
        assert!(arrow.one_holder_per_field(), "a field has a second holder");
        assert!(arrow.every_reason_given(), "a reason is empty");
        i += 1;
    }
};
const _: () = {
    let relations = [82, 96, 99, 101, 104];
    let wire_and_hold = [79, 87, 89, 90, 95, 114];
    let mut i = 0;
    while i < relations.len() {
        assert!(matches!(
            ARROWS[relations[i] - 77].carries,
            Carries::Relation { .. }
        ));
        i += 1;
    }
    let mut i = 0;
    while i < wire_and_hold.len() {
        let carries = ARROWS[wire_and_hold[i] - 77].carries;
        assert!(!carries.wired().is_empty() && !carries.held().is_empty());
        i += 1;
    }
};

macro_rules! slice_lines {
    ($kind:pat, $lines:expr) => {{
        let lines: &[usize] = &$lines;
        let (mut i, mut n) = (0, 0);
        while i < MAP_EDGES {
            if matches!(ARROWS[i].slice, $kind) {
                assert!(n < lines.len() && lines[n] == 77 + i);
                n += 1;
            }
            i += 1;
        }
        assert!(n == lines.len());
    }};
}
const _: () = {
    slice_lines!(Disposition::Live, [77, 80, 97]);
    slice_lines!(
        Disposition::ExternalInput,
        [83, 90, 91, 99, 100, 102, 105, 106, 108, 114]
    );
    slice_lines!(
        Disposition::ExternalSink,
        [78, 79, 81, 86, 87, 88, 89, 98, 110, 112, 113]
    );
    slice_lines!(Disposition::AnatomyRelation { .. }, [82, 96, 101]);
    slice_lines!(
        Disposition::OutsideSlice { .. },
        [84, 85, 92, 93, 94, 95, 103, 104, 107, 109]
    );
    slice_lines!(Disposition::Unresolved { .. }, [111]);
};

struct FieldCount {
    wired: usize,
    held: usize,
    wire_and_hold: usize,
}
const fn field_count() -> FieldCount {
    let mut count = FieldCount {
        wired: 0,
        held: 0,
        wire_and_hold: 0,
    };
    let mut i = 0;
    while i < MAP_EDGES {
        let carries = ARROWS[i].carries;
        count.wired += carries.wired().len();
        count.held += carries.held().len();
        if !carries.wired().is_empty() && !carries.held().is_empty() {
            count.wire_and_hold += 1;
        }
        i += 1;
    }
    count
}
