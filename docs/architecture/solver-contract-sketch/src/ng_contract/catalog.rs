//! The coupling catalog: one declaration per arrow of the "Direct couplings" graph in
//! `docs/atlas/coupling-map.md` (lines 77-114). Each row emits its census entry in two assemblies and, when it wires
//! a field, its port type: a structured payload carrying every field a body ticket wires. There is no second
//! hand-written list of ports.
//!
//! A payload is generic over a [`Side`]: the emitting owner receives `Payload<Defines<..>>` (one
//! [`Output`] per field, each defined once), the receiving owner `Payload<Reads<..>>` (one
//! [`ReadPort`] per field). `FragmentBuilder::receive` requires `To = Owner`, so a wire the map
//! does not draw cannot be read.
//!
//! After its label a row gives the arrow's disposition in the slice, which stays as the slice was built: `live`
//! (both ends in the arena slice), `input` (a held stand-in emits into the slice), `sink` (the slice emits to a held
//! stand-in that consumes no physiology), `relation`, `outside` (neither end in the slice) and `unresolved` (an
//! ownership conflict kept visible). Then it gives what the arrow carries in the whole body as body ticket 01 plans
//! rung 0: a `relation`, or fields. A wired field, `name: Unit => ticket`, names the body ticket that wires it. A held
//! field, `name: Unit => Holder "reason"`, names the one card that holds it and why, and is no member of the payload.
//! Mapped means the route has a declared home, not that the outside system is simulated.
//!
//! Both censuses' counts are compiler-checked below. `tools/verify` reruns `evidence/census-check.py`, which compares
//! the crate's rows line by line with `docs/atlas/coupling-map.md` and these rows with the crate's byte for byte, and
//! counts both censuses again, so an added, removed or relabelled arrow or field fails the build rather than waiting
//! for a review.
//!
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
/// A coupling-map arrow that wires a field. Implemented only by the generated payloads below.
pub trait Coupling: sealed::Coupling {
    type From: System;
    type To: System;
    type Side: Side;
}

/// An arrow's disposition in one census.
#[derive(Clone, Copy)]
pub enum Disposition {
    Live,
    ExternalInput,
    ExternalSink,
    /// Realized by an anatomy relation, not a wire: the enclosing space's pressure, site
    /// positions under the boundary's posture, or the portal circuit.
    AnatomyRelation { reason: &'static str },
    OutsideSlice { reason: &'static str },
    Unresolved { reason: &'static str },
    /// Both ends run and every field is held, each by one end with its reason.
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
        )
    }
}
/// An arrow's row in one census, with whether the assembly runs each end's card.
#[derive(Clone, Copy)]
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
    /// A disposition says which ends of its arrow run in the assembly.
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

/// A field a body ticket wires: the sender's law defines it and the receiver's law reads it.
#[derive(Clone, Copy)]
pub struct WiredField {
    pub name: &'static str,
    pub unit: &'static str,
    pub ticket: u8,
}
/// A field no rung 0 law defines or reads. One end of its arrow holds it with its reason, and it
/// keeps its unit so a later rung can wire it. It is no member of its arrow's payload.
#[derive(Clone, Copy)]
pub struct HeldField {
    pub name: &'static str,
    pub unit: &'static str,
    pub holder: &'static str,
    pub reason: &'static str,
}
/// What an arrow carries in the whole body at rung 0. No arrow is outside the whole body or
/// unresolved in it.
#[derive(Clone, Copy)]
pub enum Carries {
    Relation { reason: &'static str },
    Fields { wired: &'static [WiredField], held: &'static [HeldField] },
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

/// One arrow of the map, with its disposition in the slice and what it carries in the whole body.
#[derive(Clone, Copy)]
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
    /// It is an anatomy relation or carries a field, so the whole body accounts for it.
    pub const fn accounted(&self) -> bool {
        match self.carries {
            Carries::Relation { .. } => true,
            Carries::Fields { wired, held } => !wired.is_empty() || !held.is_empty(),
        }
    }
    /// Each field it holds has one of its two ends as its holder.
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
    /// Each field it wires names a body ticket from 06 to 25, the tickets that build the whole
    /// body's systems.
    pub const fn wired_by_body_tickets(&self) -> bool {
        let wired = self.carries.wired();
        let mut i = 0;
        while i < wired.len() {
            if wired[i].ticket < 6 || wired[i].ticket > 25 {
                return false;
            }
            i += 1;
        }
        true
    }
    /// No field name appears twice, wired or held, so no field has a second holder.
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
    if i < wired.len() { wired[i].name } else { held[i - wired.len()].name }
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

/// Which assembly a census counts.
#[derive(Clone, Copy)]
pub enum Census {
    Slice,
    /// Every card runs, as body ticket 01 plans rung 0.
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
/// How many of the map's arrows stand in each disposition in one census, and how many touch a
/// card it runs.
#[derive(Clone, Copy)]
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
        self.live + self.input + self.sink + self.relation
    }
    pub const fn unmapped(&self) -> usize {
        self.held + self.outside + self.unresolved
    }
}

/// The unit as the table writes it, which compiles only for a type in scope, held fields' included.
const fn unit<U>(written: &'static str) -> &'static str {
    written
}

/// Arrows in the map's "Direct couplings" graph. The table's type has this length.
pub const MAP_EDGES: usize = 38;

macro_rules! coupling_map {
    ($($line:literal $from:ident => $to:ident, $label:literal, $slice:ident $($why:literal)?,
        $plan:ident $carried:tt $(held $held:tt)?;)*) => {
        $(coupling_map!(@payload $plan $carried $line $from $to $label);)*
        /// The only list of coupling-map arrows. Port types above are generated from it.
        pub const ARROWS: [Arrow; MAP_EDGES] = [$(Arrow {
            line: $line,
            from: <$from as System>::CARD,
            to: <$to as System>::CARD,
            sender_in_slice: <$from as System>::IN_SLICE,
            receiver_in_slice: <$to as System>::IN_SLICE,
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
        impl<S: Side> sealed::Coupling for $name<S> {}
        impl<S: Side> Coupling for $name<S> {
            type From = $from;
            type To = $to;
            type Side = S;
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
                unit: unit::<$unit>(stringify!($unit)),
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
            unit: unit::<$unit>(stringify!($unit)),
            holder: <$holder as System>::CARD,
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

/// The slice's census: the arrows as the slice was built.
pub const SLICE: Count = Census::Slice.count();
/// The whole body's census at rung 0.
pub const WHOLE_BODY: Count = Census::WholeBody.count();
// The slice: 38 = mapped + listed unmapped, the figures the design doc states. The whole body:
// every arrow accounted for, field by field. Rows are the map's consecutive arrow lines 77..=114.
const _: () = assert!(SLICE.mapped() + SLICE.unmapped() == MAP_EDGES);
const _: () = assert!(SLICE.mapped() == 27 && SLICE.unmapped() == 11);
const _: () = assert!(SLICE.incident_mapped == 27 && SLICE.incident_unmapped == 1);
const _: () = assert!(WHOLE_BODY.live == 18 && WHOLE_BODY.relation == 5 && WHOLE_BODY.held == 15);
const _: () = assert!(matches!(fields(), (21, 29, 6)));
const _: () = {
    let mut i = 0;
    while i < MAP_EDGES {
        let arrow = &ARROWS[i];
        assert!(arrow.line as usize == 77 + i);
        assert!(Census::Slice.row(arrow).ends_fit() && Census::WholeBody.row(arrow).ends_fit());
        assert!(arrow.accounted(), "an arrow carries no field and is no anatomy relation");
        assert!(arrow.held_at_its_ends(), "a held field's holder is neither end of its arrow");
        assert!(arrow.wired_by_body_tickets(), "a wired field names no body ticket from 06 to 25");
        assert!(arrow.one_holder_per_field(), "a field has a second holder");
        i += 1;
    }
};
const _: () = {
    let relations = [82, 96, 99, 101, 104];
    let mixed = [79, 87, 89, 90, 95, 114];
    let mut i = 0;
    while i < relations.len() {
        assert!(matches!(ARROWS[relations[i] - 77].carries, Carries::Relation { .. }));
        i += 1;
    }
    let mut i = 0;
    while i < mixed.len() {
        let carries = ARROWS[mixed[i] - 77].carries;
        assert!(!carries.wired().is_empty() && !carries.held().is_empty());
        i += 1;
    }
};

/// How many fields the whole body wires and holds, and how many arrows do both.
const fn fields() -> (usize, usize, usize) {
    let (mut wired, mut held, mut mixed) = (0, 0, 0);
    let mut i = 0;
    while i < MAP_EDGES {
        let carries = ARROWS[i].carries;
        wired += carries.wired().len();
        held += carries.held().len();
        if !carries.wired().is_empty() && !carries.held().is_empty() {
            mixed += 1;
        }
        i += 1;
    }
    (wired, held, mixed)
}
