//! The coupling catalog: one row per arrow of the "Direct couplings" graph in
//! `docs/atlas/coupling-map.md` (lines 77-114).
//!
//! Mapped means the route has a declared home, not that the outside system is simulated.
use crate::law::{Brand, Carried, Output, ReadPort};
use crate::{
    Acidity, Ammonia, Beds, BloodChemistry, Brain, CapillaryBed, CarbonDioxidePartialPressure,
    Cardiovascular, Class, Concentration, Dimensionless, Endocrine, Energy, Fluids, Gastrointestinal,
    Hematology, Hepatic, Immune, Includes, LawName, Lung, Metabolism, Musculoskeletal, Nervous, Owner,
    Oxygen, OxygenPartialPressure, PerSite, Phase, Pressure, Rate, Renal, Renin, Respiratory, SiteSet, Sites,
    Speciated, Speciation, Temperature, Tissue, Tubule, Viscosity, Volume,
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
    pub body_ticket: u8,
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
            if wired[i].body_ticket < FIRST_WIRING_TICKET || wired[i].body_ticket > LAST_WIRING_TICKET {
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
                body_ticket: $ticket,
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
        wired (MyocardialWorkAndBedFlow { myocardial_work: Rate<Energy> => 9 })
        held [flow_per_bed: PerSite<CapillaryBed, Rate<Volume>> => Cardiovascular
            "no law reads a bed's flow at rung 0: a bed's oxygen arrives by the blood's own transport into its plasma"];
    79 Cardiovascular => Endocrine, "wall stress", sink,
        wired (WallStress { atrial_wall_stress: Pressure => 17 })
        held [ventricular_wall_stress: Pressure => Cardiovascular "one natriuretic peptide is secreted, on atrial stretch"];
    80 Cardiovascular => Fluids, "capillary and venous pressure", live,
        wired (CapillaryAndVenousPressure { capillary_pressure: PerSite<CapillaryBed, Pressure> => 6,
            venous_pressure: PerSite<CapillaryBed, Pressure> => 6 });
    81 Cardiovascular => Respiratory, "pulmonary capillary flow", sink,
        held [pulmonary_capillary_flow: Rate<Volume> => Cardiovascular
            "no law reads the lung's capillary flow at rung 0: the exchange reads the plasma's own partial pressures, which the blood's own transport into its plasma sets"];
    82 Respiratory => Cardiovascular, "pleural pressure",
        relation "the pleural space's pressure, felt by every bed it encloses",
        relation "the pleural space's pressure, felt by every bed it encloses, defined by the breath from ticket 10";
    83 Respiratory => Nervous, "lung afferents", input,
        held [pulmonary_stretch: Dimensionless => Respiratory
            "a resting day's breaths stay under the volume at which stretch slows the breath (ticket 11)"];
    84 Respiratory => Metabolism, "work of breathing", outside "neither end in the arena slice",
        wired (WorkOfBreathing { work_of_breathing: Rate<Energy> => 10 });
    85 Respiratory => Musculoskeletal, "pump load", outside "neither end in the arena slice",
        held [pump_load: Pressure => Respiratory "no law reads the pump's load at rung 0: the pump's fatigue is held"];
    86 Nervous => Respiratory, "respiratory drive", sink,
        wired (RespiratoryDrive { mixing_share: Dimensionless => 11 });
    87 Nervous => Metabolism, "thermostat, shiver and sweat", sink,
        wired (ShiverDrive { shiver_drive: Dimensionless => 12 })
        held [thermostat_set_point: Temperature => Nervous "the set point is NS memory and no MT law reads it",
            sweat_drive: Dimensionless => Nervous "sweat answers release at the skin junction (S10)"];
    88 Nervous => Musculoskeletal, "motor drive", sink,
        relation "release at the respiratory pump's junction";
    89 Nervous => Endocrine, "CRH, vasopressin, circadian phase", sink,
        wired (HypothalamicDrive { vasopressin_drive: Dimensionless => 17 })
        held [crh_drive: Dimensionless => Nervous "cortisol's profile follows the clock input directly (ticket 20)",
            circadian_phase: Phase => Nervous "the clock is a boundary input every law reads (ticket 05)"];
    90 Metabolism => Cardiovascular, "O2 debt per bed, temperature", input,
        wired (OxygenDebtPerBed { oxygen_debt_per_bed: PerSite<CapillaryBed, Oxygen> => 9 })
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
        wired (PumpForce { pump_force: Pressure => 10 })
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
        wired (BloodViscosity { viscosity: Viscosity => 8 })
        held [bleeding_competence: Dimensionless => Hematology "no bleeding edge (ticket 21)",
            thrombosis: Dimensionless => Hematology "zero flux (ticket 21)"];
}

const SLICE: Count = Census::Slice.count();
const WHOLE_BODY: Count = Census::WholeBody.count();
const _: () = assert!(SLICE.mapped() + SLICE.unmapped() == MAP_EDGES);
const _: () = assert!(SLICE.mapped() == 27 && SLICE.unmapped() == 11);
const _: () = assert!(SLICE.incident_mapped == 27 && SLICE.incident_unmapped == 1);
const _: () = assert!(WHOLE_BODY.live == 14 && WHOLE_BODY.relation == 6 && WHOLE_BODY.held == 18);
const FIELDS: FieldCount = field_count();
const _: () = assert!(FIELDS.wired == 16 && FIELDS.held == 33 && FIELDS.wire_and_hold == 7);
const _: () = {
    let mut i = 0;
    while i < MAP_EDGES {
        let arrow = &ARROWS[i];
        assert!(arrow.line as usize == 77 + i);
        assert!(Census::Slice.row(arrow).ends_fit());
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
    let wire_and_hold = [78, 79, 87, 89, 90, 95, 114];
    let mut i = 0;
    while i < wire_and_hold.len() {
        let carries = ARROWS[wire_and_hold[i] - 77].carries;
        assert!(!carries.wired().is_empty() && !carries.held().is_empty());
        i += 1;
    }
};

macro_rules! census_lines {
    ($census:expr, $kind:pat, $lines:expr) => {{
        let lines: &[usize] = &$lines;
        let (mut i, mut n) = (0, 0);
        while i < MAP_EDGES {
            if matches!($census.row(&ARROWS[i]).disposition, $kind) {
                assert!(n < lines.len() && lines[n] == 77 + i);
                n += 1;
            }
            i += 1;
        }
        assert!(n == lines.len());
    }};
}
const _: () = {
    census_lines!(Census::Slice, Disposition::Live, [77, 80, 97]);
    census_lines!(
        Census::Slice,
        Disposition::ExternalInput,
        [83, 90, 91, 99, 100, 102, 105, 106, 108, 114]
    );
    census_lines!(
        Census::Slice,
        Disposition::ExternalSink,
        [78, 79, 81, 86, 87, 88, 89, 98, 110, 112, 113]
    );
    census_lines!(Census::Slice, Disposition::AnatomyRelation { .. }, [82, 96, 101]);
    census_lines!(
        Census::Slice,
        Disposition::OutsideSlice { .. },
        [84, 85, 92, 93, 94, 95, 103, 104, 107, 109]
    );
    census_lines!(Census::Slice, Disposition::Unresolved { .. }, [111]);
    census_lines!(
        Census::WholeBody,
        Disposition::Live,
        [77, 78, 79, 80, 84, 86, 87, 89, 90, 91, 95, 97, 98, 114]
    );
    census_lines!(
        Census::WholeBody,
        Disposition::AnatomyRelation { .. },
        [82, 88, 96, 99, 101, 104]
    );
    census_lines!(
        Census::WholeBody,
        Disposition::Held,
        [
            81, 83, 85, 92, 93, 94, 100, 102, 103, 105, 106, 107, 108, 109, 110, 111, 112, 113
        ]
    );
};

const _: () = {
    let plan: [(usize, &str, &str); 33] = [
        (78, "flow_per_bed", "CV"),
        (79, "ventricular_wall_stress", "CV"),
        (81, "pulmonary_capillary_flow", "CV"),
        (83, "pulmonary_stretch", "RS"),
        (85, "pump_load", "RS"),
        (87, "thermostat_set_point", "NS"),
        (87, "sweat_drive", "NS"),
        (89, "crh_drive", "NS"),
        (89, "circadian_phase", "NS"),
        (90, "temperature", "CV"),
        (92, "temperature", "HC"),
        (93, "temperature", "HP"),
        (94, "temperature", "BC"),
        (95, "fatigue", "MS"),
        (100, "plasma_ammonia", "HP"),
        (102, "intra_abdominal_pressure", "GI"),
        (103, "intra_abdominal_pressure", "GI"),
        (105, "inos_activity", "IM"),
        (105, "myocardial_depression", "IM"),
        (106, "permeability", "IM"),
        (106, "glycocalyx", "IM"),
        (107, "tissue_factor", "IM"),
        (107, "hepcidin", "IM"),
        (108, "set_point_shift", "IM"),
        (109, "mitochondrial_efficiency", "IM"),
        (110, "lung_water", "FL"),
        (111, "brain_water", "FL"),
        (111, "intracranial_pressure", "NS"),
        (112, "gut_wall_water", "FL"),
        (113, "skin_water", "FL"),
        (113, "compartment_water", "FL"),
        (114, "bleeding_competence", "HC"),
        (114, "thrombosis", "HC"),
    ];
    let (mut i, mut n) = (0, 0);
    while i < MAP_EDGES {
        let held = ARROWS[i].carries.held();
        let mut j = 0;
        while j < held.len() {
            let (line, name, holder) = plan[n];
            assert!(
                line == 77 + i && same(held[j].name, name) && same(held[j].holder, holder),
                "a held field's holder is not the plan's"
            );
            j += 1;
            n += 1;
        }
        i += 1;
    }
    assert!(n == plan.len());
};
const _: () = {
    let plan: [(usize, &str, u8); 16] = [
        (77, "carotid_sinus_stretch", 6),
        (77, "aortic_arch_stretch", 6),
        (78, "myocardial_work", 9),
        (79, "atrial_wall_stress", 17),
        (80, "capillary_pressure", 6),
        (80, "venous_pressure", 6),
        (84, "work_of_breathing", 10),
        (86, "mixing_share", 11),
        (87, "shiver_drive", 12),
        (89, "vasopressin_drive", 17),
        (90, "oxygen_debt_per_bed", 9),
        (91, "core_temperature", 12),
        (95, "pump_force", 10),
        (97, "afferent_tone", 6),
        (98, "renin_secretion", 17),
        (114, "viscosity", 8),
    ];
    let (mut i, mut n) = (0, 0);
    while i < MAP_EDGES {
        let wired = ARROWS[i].carries.wired();
        let mut j = 0;
        while j < wired.len() {
            let (line, name, ticket) = plan[n];
            assert!(
                line == 77 + i && same(wired[j].name, name) && wired[j].body_ticket == ticket,
                "a wired field's body ticket is not the plan's"
            );
            j += 1;
            n += 1;
        }
        i += 1;
    }
    assert!(n == plan.len());
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

/// One reader of speciation: a card whose laws read what blood chemistry states, at the sites of one set. Speciation is
/// a shared substrate's derived property, so its readers are this second table and not arrows. Each quantity a row
/// reads is wired by the body ticket that builds the reading law, or held by the reader with a reason. Blood chemistry
/// states every quantity whoever reads it, and compile refuses a read no row wires (`UnwiredRead`).
///
/// A reader reads only what its row lists: metabolism reads no acidity.
/// ```compile_fail,E0609
/// use ng_contract::{catalog::*, *};
/// fn read<'b>(gases: PlasmaGases<Reads<'b, Metabolism>>) -> ReadPort<'b, Metabolism, PerSite<CapillaryBed, Dimensionless>> {
///     gases.acidity
/// }
/// ```
/// ```
/// use ng_contract::{catalog::*, *};
/// fn read<'b>(gases: PlasmaGases<Reads<'b, Metabolism>>) -> ReadPort<'b, Metabolism, PerSite<CapillaryBed, Pressure>> {
///     gases.carbon_dioxide_partial_pressure
/// }
/// ```
/// Only the row's reader receives it.
/// ```compile_fail,E0271
/// use ng_contract::{catalog::*, *};
/// fn received<'b>(ns: &FragmentBuilder<'b, Nervous>) -> PlasmaGases<Reads<'b, Nervous>> {
///     ns.receive()
/// }
/// ```
/// ```
/// use ng_contract::{catalog::*, *};
/// fn received<'b>(mt: &FragmentBuilder<'b, Metabolism>) -> PlasmaGases<Reads<'b, Metabolism>> {
///     mt.receive()
/// }
/// ```
/// Blood chemistry states through its port and emits no reader's row.
/// ```compile_fail,E0277
/// use ng_contract::{catalog::*, *};
/// fn emitted<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>) -> PlasmaGases<Defines<'b, BloodChemistry>> {
///     bc.emit()
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn stated<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>, ports: &BloodChemistryPorts<'b>) -> SpeciationOutputs<'b> {
///     ports.speciation(bc)
/// }
/// ```
/// A held row has no payload: the nervous system reads no arterial gases until its row is wired.
/// ```compile_fail,E0425
/// use ng_contract::{catalog::*, *};
/// fn received<'b>(ns: &FragmentBuilder<'b, Nervous>) -> ArterialGases<Reads<'b, Nervous>> {
///     ns.receive()
/// }
/// ```
/// ```
/// use ng_contract::catalog::*;
/// fn held() -> &'static [HeldRead] {
///     SPECIATION_READERS[3].held
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeciationReader {
    pub reader: &'static str,
    pub sites: Sites,
    pub label: &'static str,
    pub wired: &'static [WiredRead],
    pub held: &'static [HeldRead],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WiredRead {
    pub quantity: &'static str,
    pub body_ticket: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldRead {
    pub quantity: &'static str,
    pub reason: &'static str,
}

impl SpeciationReader {
    pub const fn accounted(&self) -> bool {
        !self.wired.is_empty() || !self.held.is_empty()
    }
    pub const fn read_by_another_card(&self) -> bool {
        !same(self.reader, BloodChemistry::CARD)
    }
    pub const fn each_quantity_once(&self) -> bool {
        let reads = self.wired.len() + self.held.len();
        let mut i = 0;
        while i < reads {
            let mut j = i + 1;
            while j < reads {
                if same(self.quantity(i), self.quantity(j)) {
                    return false;
                }
                j += 1;
            }
            i += 1;
        }
        true
    }
    pub const fn wired_by_body_tickets(&self) -> bool {
        let mut i = 0;
        while i < self.wired.len() {
            let ticket = self.wired[i].body_ticket;
            if ticket < FIRST_WIRING_TICKET || ticket > LAST_WIRING_TICKET {
                return false;
            }
            i += 1;
        }
        true
    }
    pub const fn every_reason_given(&self) -> bool {
        let mut i = 0;
        while i < self.held.len() {
            let reason = self.held[i].reason;
            if reason.is_empty() {
                return false;
            }
            i += 1;
        }
        true
    }
    /// No two rows of `table` share a reader and a set.
    pub const fn one_row_per_reader_and_set(table: &[SpeciationReader]) -> bool {
        let mut i = 0;
        while i < table.len() {
            let mut j = i + 1;
            while j < table.len() {
                if same(table[i].reader, table[j].reader) && same_sites(table[i].sites, table[j].sites) {
                    return false;
                }
                j += 1;
            }
            i += 1;
        }
        true
    }
    const fn quantity(&self, i: usize) -> &'static str {
        if i < self.wired.len() {
            self.wired[i].quantity
        } else {
            self.held[i - self.wired.len()].quantity
        }
    }
}
const fn same_sites(a: Sites, b: Sites) -> bool {
    match (a, b) {
        (Sites::Class(a), Sites::Class(b)) => a as u8 == b as u8,
        (Sites::Tissue(a), Sites::Tissue(b)) | (Sites::Besides(a), Sites::Besides(b)) => a as u8 == b as u8,
        (Sites::Blood, Sites::Blood)
        | (Sites::Urinary, Sites::Urinary)
        | (Sites::Alimentary, Sites::Alimentary)
        | (Sites::Airway, Sites::Airway)
        | (Sites::Veins, Sites::Veins)
        | (Sites::Speciated, Sites::Speciated) => true,
        _ => false,
    }
}

const fn inside_speciation<C: SiteSet>()
where
    Speciated: Includes<C>,
{
}

macro_rules! speciation_readers {
    ($($reader:ident at $set:ty, $label:literal, $plan:ident $carried:tt $(held $held:tt)?;)*) => {
        $(speciation_readers!(@payload $plan $carried $reader $set);)*
        pub const SPECIATION_READERS: [SpeciationReader; READERS] = [$(SpeciationReader {
            reader: <$reader as Owner>::CARD,
            sites: <$set as SiteSet>::SET,
            label: $label,
            wired: speciation_readers!(@wired $plan $carried),
            held: speciation_readers!(@held $plan $carried $($held)?),
        },)*];
        $(const _: () = inside_speciation::<$set>();)*
    };
    (@payload wired ($name:ident { $($field:ident: $quantity:ident => $ticket:literal),+ $(,)? }) $reader:ident $set:ty) => {
        #[doc = concat!("What ", stringify!($reader), " reads of speciation.")]
        pub struct $name<S: Side> {
            $(pub $field: S::Field<PerSite<$set, <$quantity as Speciation>::Unit>>,)+
        }
        impl<'b, O> Coupling for $name<Reads<'b, O>> {
            type From = BloodChemistry;
            type To = $reader;
            type Side = Reads<'b, O>;
            fn issue(token: Token) -> (Self, Vec<Field>) {
                let stater = <BloodChemistry as Owner>::CARD;
                let payload = Self {
                    $($field: <Reads<'b, O> as Side>::field::<PerSite<$set, <$quantity as Speciation>::Unit>>(
                        token,
                        stater,
                        <$quantity as Speciation>::NAME,
                    ),)+
                };
                (payload, Vec::new())
            }
        }
    };
    (@payload held $carried:tt $reader:ident $set:ty) => {};
    (@wired wired ($name:ident { $($field:ident: $quantity:ident => $ticket:literal),+ $(,)? })) => {
        &[$(WiredRead { quantity: <$quantity as Speciation>::NAME, body_ticket: $ticket }),+]
    };
    (@wired held $carried:tt) => {
        &[]
    };
    (@held wired $carried:tt) => {
        &[]
    };
    (@held wired $carried:tt [$($quantity:ident $reason:literal),+ $(,)?]) => {
        &[$(HeldRead { quantity: <$quantity as Speciation>::NAME, reason: $reason }),+]
    };
    (@held held [$($quantity:ident $reason:literal),+ $(,)?]) => {
        &[$(HeldRead { quantity: <$quantity as Speciation>::NAME, reason: $reason }),+]
    };
}

const READERS: usize = 5;

speciation_readers! {
    Metabolism at CapillaryBed, "each bed's plasma gases, for wall diffusion",
        wired (PlasmaGases {
            oxygen_partial_pressure: OxygenPartialPressure => 8,
            carbon_dioxide_partial_pressure: CarbonDioxidePartialPressure => 8,
        });
    Respiratory at Beds<Lung>, "the pulmonary bed's gases, for alveolar exchange",
        wired (LungGases {
            oxygen_partial_pressure: OxygenPartialPressure => 10,
            carbon_dioxide_partial_pressure: CarbonDioxidePartialPressure => 10,
        });
    Nervous at Beds<Brain>, "the brain bed's acidity, at the central chemoreceptor",
        held [Acidity "no central sensor until ticket 11"];
    Nervous at Speciated, "arterial gases, at the peripheral chemoreceptor",
        held [OxygenPartialPressure "no peripheral sensor until ticket 11, which samples the systemic arteries",
            CarbonDioxidePartialPressure "the same sensor"];
    Renal at Beds<Tubule>, "the peritubular bed's acidity, for the kidney's acid feedback",
        held [Acidity "ticket 18's text names the strong ion difference as what the kidney's feedback reads"];
}

#[derive(Clone, Copy)]
struct ReadCount {
    wired: usize,
    held: usize,
}
const fn read_count() -> ReadCount {
    let mut count = ReadCount { wired: 0, held: 0 };
    let mut i = 0;
    while i < READERS {
        count.wired += SPECIATION_READERS[i].wired.len();
        count.held += SPECIATION_READERS[i].held.len();
        i += 1;
    }
    count
}
const READ: ReadCount = read_count();
const _: () = assert!(READ.wired == 4 && READ.held == 4);
const _: () = assert!(SpeciationReader::one_row_per_reader_and_set(&SPECIATION_READERS));
const _: () = {
    let mut i = 0;
    while i < READERS {
        let row = &SPECIATION_READERS[i];
        assert!(row.accounted(), "a reader reads nothing");
        assert!(
            row.read_by_another_card(),
            "blood chemistry is no reader of its own speciation"
        );
        assert!(row.each_quantity_once(), "a reader names one quantity twice");
        assert!(
            row.wired_by_body_tickets(),
            "a wired read names no body ticket from 06 to 25"
        );
        assert!(row.every_reason_given(), "a reason is empty");
        i += 1;
    }
};
const _: () = {
    let plan: [(&str, Sites, &str, u8); 4] = [
        (
            "MT",
            Sites::Class(Class::CapillaryBed),
            "oxygen partial pressure",
            8,
        ),
        (
            "MT",
            Sites::Class(Class::CapillaryBed),
            "carbon dioxide partial pressure",
            8,
        ),
        ("RS", Sites::Tissue(Tissue::Lung), "oxygen partial pressure", 10),
        (
            "RS",
            Sites::Tissue(Tissue::Lung),
            "carbon dioxide partial pressure",
            10,
        ),
    ];
    let (mut i, mut n) = (0, 0);
    while i < READERS {
        let row = &SPECIATION_READERS[i];
        let mut j = 0;
        while j < row.wired.len() {
            let (reader, sites, quantity, ticket) = plan[n];
            assert!(
                same(row.reader, reader)
                    && same_sites(row.sites, sites)
                    && same(row.wired[j].quantity, quantity)
                    && row.wired[j].body_ticket == ticket,
                "a wired read is not the plan's"
            );
            j += 1;
            n += 1;
        }
        i += 1;
    }
    assert!(n == plan.len());
};
const _: () = {
    let plan: [(&str, Sites, &str); 4] = [
        ("NS", Sites::Tissue(Tissue::Brain), "acidity"),
        ("NS", Sites::Speciated, "oxygen partial pressure"),
        ("NS", Sites::Speciated, "carbon dioxide partial pressure"),
        ("RN", Sites::Tissue(Tissue::Tubule), "acidity"),
    ];
    let (mut i, mut n) = (0, 0);
    while i < READERS {
        let row = &SPECIATION_READERS[i];
        let mut j = 0;
        while j < row.held.len() {
            let (reader, sites, quantity) = plan[n];
            assert!(
                same(row.reader, reader)
                    && same_sites(row.sites, sites)
                    && same(row.held[j].quantity, quantity),
                "a held read is not the plan's"
            );
            j += 1;
            n += 1;
        }
        i += 1;
    }
    assert!(n == plan.len());
};
