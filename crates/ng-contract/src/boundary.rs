use crate::identity::{Canonical, Seal};
use crate::landing::Bracket;
use crate::pinned::turn;
use crate::{
    AdmissionError, AminoAcids, ByteReader, Calcium, Chloride, CompiledBody, CoupledProblem, Cycle, DietBase,
    Digest, Dimensionless, EngineFault, EngineFaultKind, Glucose, Interval, Iron, Magnesium, Malformed,
    Measure, Phosphate, Potassium, Pressure, Refused, SimTime, Sodium, Species, StepTicket, Temperature,
    TrialFailure, Triglyceride, VerifiedStep, Water,
};
use core::num::NonZeroU64;

mod day;
pub use day::Day;

const NANOSECONDS_PER_SECOND: f64 = 1e9;
const MINUTE_NS: u64 = 60_000_000_000;
const DAY_NS: u64 = 24 * 60 * MINUTE_NS;
const EIGHTH_OF_A_DAY: NonZeroU64 = NonZeroU64::new(180).unwrap();

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputId {
    digest: Digest,
}
impl InputId {
    pub fn new(digest: Digest) -> Self {
        Self { digest }
    }
}
impl Seal for InputId {
    fn seal(&self, out: &mut Canonical) {
        let Self { digest } = self;
        digest.seal(out);
    }
}

/// `amount` through the named boundary, from the event's instant until `ends`, at the constant rate that delivers
/// it: "applied once" shows as a total received. Overlapping deliveries add. `check_delivery` refuses a boundary that
/// carries another species.
pub struct Delivered<Q> {
    pub boundary: &'static str,
    pub amount: Measure<Q>,
    pub ends: SimTime,
}
pub type WaterDelivery = Delivered<Water>;
pub enum BoundaryInput {
    Water(WaterDelivery),
    Sodium(Delivered<Sodium>),
    Potassium(Delivered<Potassium>),
    Chloride(Delivered<Chloride>),
    /// How the body lies from this instant until the next one. An event is one instant, so a ramp is a train of events.
    /// Each ends a step, and every site's head follows from its place.
    Posture(Posture),
    Clock(Clock),
    Sleep(Sleep),
    Room(Room),
    Void(Gate),
    Meal(Meal),
}
pub struct Meal {
    ends: SimTime,
    amounts: [f64; Meal::SPECIES.len()],
}
impl Meal {
    pub const SPECIES: [Species; 12] = [
        Species::Water,
        Species::Sodium,
        Species::Potassium,
        Species::Chloride,
        Species::Glucose,
        Species::Triglyceride,
        Species::AminoAcids,
        Species::DietBase,
        Species::Calcium,
        Species::Phosphate,
        Species::Magnesium,
        Species::Iron,
    ];
    pub const BOUNDARIES: [&'static str; 12] = [
        "gut water",
        "gut sodium",
        "gut potassium",
        "gut chloride",
        "gut glucose",
        "gut triglyceride",
        "gut amino acids",
        "gut diet base",
        "gut calcium",
        "gut phosphate",
        "gut magnesium",
        "gut iron",
    ];
    pub fn new(ends: SimTime) -> Self {
        Self {
            ends,
            amounts: [0.0; Meal::SPECIES.len()],
        }
    }
    pub fn with<Q: Eaten>(mut self, amount: Measure<Q>) -> Self {
        self.amounts[<Q as eaten::Sealed>::PLACE] = amount.value();
        self
    }
}
const fn place(species: Species) -> usize {
    let mut at = 0;
    while Meal::SPECIES[at] as usize != species as usize {
        at += 1;
    }
    at
}
mod eaten {
    pub trait Sealed: crate::Substance {
        const PLACE: usize = super::place(Self::SPECIES);
    }
}
pub trait Eaten: eaten::Sealed {}
macro_rules! eaten {
    ($($species:ident),*) => {$(
        impl eaten::Sealed for $species {}
        impl Eaten for $species {}
    )*};
}
eaten!(
    Water,
    Sodium,
    Potassium,
    Chloride,
    Glucose,
    Triglyceride,
    AminoAcids,
    DietBase,
    Calcium,
    Phosphate,
    Magnesium,
    Iron
);
fn part_id(meal: InputId, place: usize) -> InputId {
    let mut out = Canonical::new("ng-contract meal part 1");
    meal.seal(&mut out);
    out.number(place as u64);
    InputId::new(out.digest())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub since_midnight_ns: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sleep {
    Awake,
    Nrem,
}
impl Sleep {
    const fn value(self) -> f64 {
        match self {
            Sleep::Awake => 0.0,
            Sleep::Nrem => 1.0,
        }
    }
    fn of_value(value: f64) -> Result<Self, Malformed> {
        let found = [Sleep::Awake, Sleep::Nrem];
        (found.into_iter())
            .find(|sleep| sleep.value().to_bits() == value.to_bits())
            .ok_or(Malformed)
    }
}
/// Air speed is its ratio to 0.1 m/s and clothing or bedding a count of clo, because the table has no speed and
/// insulation has no dimension.
pub struct Room {
    pub air: Measure<Temperature>,
    pub radiant: Measure<Temperature>,
    pub humidity: Measure<Dimensionless>,
    pub air_speed: Measure<Dimensionless>,
    pub pressure: Measure<Pressure>,
    pub oxygen: Measure<Dimensionless>,
    pub carbon_dioxide: Measure<Dimensionless>,
    pub clothing: Measure<Dimensionless>,
}
impl Room {
    fn values(&self) -> [f64; ROOM.len()] {
        let Self {
            air,
            radiant,
            humidity,
            air_speed,
            pressure,
            oxygen,
            carbon_dioxide,
            clothing,
        } = self;
        [
            air.value(),
            radiant.value(),
            humidity.value(),
            air_speed.value(),
            pressure.value(),
            oxygen.value(),
            carbon_dioxide.value(),
            clothing.value(),
        ]
    }
}
fn room_in_range(values: [f64; ROOM.len()]) -> bool {
    let [
        air,
        radiant,
        humidity,
        air_speed,
        pressure,
        oxygen,
        carbon_dioxide,
        clothing,
    ] = values;
    let fraction = |value: f64| (0.0..=1.0).contains(&value);
    values.iter().all(|value| value.is_finite())
        && air > 0.0
        && radiant > 0.0
        && fraction(humidity)
        && air_speed >= 0.0
        && pressure > 0.0
        && fraction(oxygen)
        && fraction(carbon_dioxide)
        && oxygen + carbon_dioxide <= 1.0
        && clothing >= 0.0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Open,
    Shut,
}
impl Gate {
    const fn value(self) -> f64 {
        match self {
            Gate::Open => 1.0,
            Gate::Shut => 0.0,
        }
    }
    fn of_value(value: f64) -> Result<Self, Malformed> {
        let found = [Gate::Open, Gate::Shut];
        (found.into_iter())
            .find(|gate| gate.value().to_bits() == value.to_bits())
            .ok_or(Malformed)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Held {
    ClockCosine,
    ClockSine,
    Sleep,
    AirTemperature,
    RadiantTemperature,
    Humidity,
    AirSpeed,
    Pressure,
    InspiredOxygen,
    InspiredCarbonDioxide,
    Clothing,
    Void,
}
impl Seal for Held {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}
const HELD: usize = 12;
const ROOM: [Held; 8] = [
    Held::AirTemperature,
    Held::RadiantTemperature,
    Held::Humidity,
    Held::AirSpeed,
    Held::Pressure,
    Held::InspiredOxygen,
    Held::InspiredCarbonDioxide,
    Held::Clothing,
];
/// `trunk` is the sine of the trunk's angle from flat: 0 lying flat, 1 sitting up or standing, a negative
/// value head down.
pub struct Posture {
    trunk: f64,
    legs: Legs,
}
impl Posture {
    /// A trunk past straight up or straight down has no angle.
    pub fn new(trunk: Measure<Dimensionless>, legs: Legs) -> Result<Self, AdmissionError> {
        let trunk = trunk.value();
        if trunk.abs() > 1.0 {
            return Err(AdmissionError::PostureOutOfRange);
        }
        Ok(Self { trunk, legs })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legs {
    Horizontal,
    SeatedDependent,
    Standing,
}
impl Legs {
    /// How much of gravity runs along the legs: seated, the thigh lies flat and the shin hangs. Model default.
    const fn share(self) -> f64 {
        match self {
            Legs::Horizontal => 0.0,
            Legs::SeatedDependent => 0.5,
            Legs::Standing => 1.0,
        }
    }
    fn of_share(share: f64) -> Result<Self, Malformed> {
        let found = [Legs::Horizontal, Legs::SeatedDependent, Legs::Standing];
        (found
            .into_iter()
            .find(|legs| legs.share().to_bits() == share.to_bits()))
        .ok_or(Malformed)
    }
}
pub struct BoundaryEvent {
    id: InputId,
    at: SimTime,
    payload: BoundaryInput,
}
impl BoundaryEvent {
    pub fn new(id: InputId, at: SimTime, payload: BoundaryInput) -> Self {
        Self { id, at, payload }
    }
    pub fn id(&self) -> InputId {
        self.id
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Delivery {
    pub(crate) id: InputId,
    pub(crate) at: SimTime,
    pub(crate) boundary: &'static str,
    amount: f64,
    pub(crate) ends: SimTime,
    rate: f64,
    kind: Kind,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Delivered(usize),
    Posture,
    Clock(u64),
    Sleep(Sleep),
    Room([f64; ROOM.len()]),
    Void(Gate),
}
impl Kind {
    /// After the fields every payload shares, so a delivery and a posture keep the bytes they had.
    fn seal_setting(&self, out: &mut Canonical) {
        match *self {
            Kind::Delivered(_) | Kind::Posture => {}
            Kind::Clock(reading) => out.number(reading),
            Kind::Sleep(sleep) => sleep.value().seal(out),
            Kind::Room(values) => values.iter().for_each(|value| value.seal(out)),
            Kind::Void(gate) => gate.value().seal(out),
        }
    }
}
impl Seal for Delivery {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            id,
            at,
            boundary,
            amount,
            ends,
            rate,
            kind,
        } = self;
        id.seal(out);
        at.seal(out);
        out.text(boundary);
        amount.seal(out);
        ends.seal(out);
        rate.seal(out);
        out.number(payload_tag(*kind));
        kind.seal_setting(out);
    }
}

/// Boundary names are checked against the compiled body at `CompiledBody::initialization`.
pub struct BoundaryPort {
    pub(crate) deliveries: Vec<Delivery>,
}
impl BoundaryPort {
    /// Refuses a delivery ending at or before its event, a negative amount, a rate past the finite range
    /// and one id with two payloads. Collapses one id repeated with one payload.
    pub fn new(events: Vec<BoundaryEvent>) -> Result<Self, AdmissionError> {
        let mut deliveries: Vec<Delivery> = Vec::with_capacity(events.len());
        for event in events {
            let arrival = Delivery::parsed(event)?;
            if arrival.new_to(&deliveries)? {
                deliveries.extend(arrival.parts);
            }
        }
        deliveries.sort_by_key(|delivery| (delivery.at, delivery.id));
        Ok(Self { deliveries })
    }
    pub fn digest(&self) -> Digest {
        let Self { deliveries } = self;
        let mut out = Canonical::new("ng-contract boundary port 1");
        deliveries.seal(&mut out);
        out.digest()
    }
    /// The first delivery that starts or ends inside the window between two points, or the first clock whose
    /// minute turns there in a body that reads it. A point's frame holds one interval, so such an edge would read
    /// a drive the run never gives.
    pub(crate) fn off_grid(&self, start: SimTime, cycle: Cycle, reads_clock: bool) -> Option<AdmissionError> {
        let (from, step) = (start.nanoseconds(), cycle.step_ns());
        let window = SimTime::from_ns(from.saturating_add(cycle.period_ns()));
        let off = |edge: SimTime| {
            let reached = edge.nanoseconds().saturating_sub(from);
            edge.nanoseconds() > from && reached < cycle.period_ns() && !reached.is_multiple_of(step)
        };
        let clocks: Vec<&Delivery> = (self.deliveries.iter())
            .filter(|delivery| matches!(delivery.kind, Kind::Clock(_)))
            .collect();
        let turns_off = |delivery: &Delivery| {
            let Kind::Clock(reading) = delivery.kind else {
                return false;
            };
            let next = clocks.iter().map(|clock| clock.at).find(|at| *at > delivery.at);
            let held_until = next.unwrap_or(window).min(window);
            let mut edge = next_minute(delivery.at, reading, delivery.at.max(start));
            while edge < held_until {
                if off(edge) {
                    return true;
                }
                edge = SimTime::from_ns(edge.nanoseconds().saturating_add(MINUTE_NS));
            }
            false
        };
        self.deliveries.iter().find_map(|delivery| {
            let id = delivery.id;
            if off(delivery.at) || off(delivery.ends) {
                Some(AdmissionError::DeliveryOffGrid { id })
            } else if reads_clock && turns_off(delivery) {
                Some(AdmissionError::ClockOffGrid { id })
            } else {
                None
            }
        })
    }
}
struct Arrival {
    id: InputId,
    part_ids: Vec<InputId>,
    parts: Vec<Delivery>,
}
impl Arrival {
    fn one(delivery: Delivery) -> Self {
        Self {
            id: delivery.id,
            part_ids: vec![delivery.id],
            parts: vec![delivery],
        }
    }
    fn new_to(&self, known: &[Delivery]) -> Result<bool, AdmissionError> {
        let found: Vec<&Delivery> = (known.iter())
            .filter(|known| self.part_ids.contains(&known.id))
            .collect();
        if found.is_empty() {
            return Ok(true);
        }
        let same = found.len() == self.parts.len() && self.parts.iter().all(|part| found.contains(&part));
        if same {
            Ok(false)
        } else {
            Err(AdmissionError::InputConflict { id: self.id })
        }
    }
    fn payload(&self) -> Digest {
        match self.parts.as_slice() {
            [part] => part.payload(),
            parts => {
                let mut out = Canonical::new("ng-contract meal 1");
                parts.seal(&mut out);
                out.digest()
            }
        }
    }
    fn named(&self, error: AdmissionError) -> AdmissionError {
        match error {
            AdmissionError::EventBeforeHead { .. } => AdmissionError::EventBeforeHead { id: self.id },
            other => other,
        }
    }
}
impl Delivery {
    fn meal(id: InputId, at: SimTime, meal: Meal) -> Result<Arrival, AdmissionError> {
        let Meal { ends, amounts } = meal;
        if ends <= at || amounts.iter().all(|amount| *amount == 0.0) {
            return Err(AdmissionError::EmptyDelivery { id });
        }
        if amounts.iter().any(|amount| *amount < 0.0) {
            return Err(AdmissionError::NegativeDelivery { id });
        }
        let mut parts = Vec::new();
        for (place, amount) in amounts
            .into_iter()
            .enumerate()
            .filter(|(_, amount)| *amount > 0.0)
        {
            let boundary = Meal::BOUNDARIES[place];
            let part = Delivery::checked(part_id(id, place), at, boundary, amount, ends, place);
            parts.push(part.map_err(|_| AdmissionError::UnboundedDelivery { id })?);
        }
        let part_ids = (0..Meal::SPECIES.len()).map(|place| part_id(id, place)).collect();
        Ok(Arrival { id, part_ids, parts })
    }
    fn parsed(event: BoundaryEvent) -> Result<Arrival, AdmissionError> {
        let BoundaryEvent { id, at, payload } = event;
        fn delivered<Q: Eaten>(
            id: InputId,
            at: SimTime,
            delivery: Delivered<Q>,
        ) -> Result<Delivery, AdmissionError> {
            let Delivered {
                boundary,
                amount,
                ends,
            } = delivery;
            Delivery::checked(
                id,
                at,
                boundary,
                amount.value(),
                ends,
                <Q as eaten::Sealed>::PLACE,
            )
        }
        let single = match payload {
            BoundaryInput::Meal(meal) => return Self::meal(id, at, meal),
            BoundaryInput::Water(delivery) => delivered(id, at, delivery),
            BoundaryInput::Sodium(delivery) => delivered(id, at, delivery),
            BoundaryInput::Potassium(delivery) => delivered(id, at, delivery),
            BoundaryInput::Chloride(delivery) => delivered(id, at, delivery),
            BoundaryInput::Posture(Posture { trunk, legs }) => Ok(Self::posture(id, at, trunk, legs)),
            BoundaryInput::Clock(Clock { since_midnight_ns }) if since_midnight_ns >= DAY_NS => {
                Err(AdmissionError::ClockOutOfRange { id })
            }
            BoundaryInput::Clock(Clock { since_midnight_ns }) => {
                Ok(Self::held(id, at, Kind::Clock(since_midnight_ns)))
            }
            BoundaryInput::Sleep(sleep) => Ok(Self::held(id, at, Kind::Sleep(sleep))),
            BoundaryInput::Room(room) if !room_in_range(room.values()) => {
                Err(AdmissionError::RoomOutOfRange { id })
            }
            BoundaryInput::Room(room) => Ok(Self::held(id, at, Kind::Room(room.values()))),
            BoundaryInput::Void(gate) => Ok(Self::held(id, at, Kind::Void(gate))),
        };
        single.map(Arrival::one)
    }
    fn posture(id: InputId, at: SimTime, trunk: f64, legs: Legs) -> Self {
        Self {
            id,
            at,
            boundary: POSTURE,
            amount: trunk,
            ends: SimTime::from_ns(u64::MAX),
            rate: legs.share(),
            kind: Kind::Posture,
        }
    }
    fn held(id: InputId, at: SimTime, kind: Kind) -> Self {
        let boundary = match kind {
            Kind::Delivered(_) | Kind::Posture => POSTURE,
            Kind::Clock(_) => "clock",
            Kind::Sleep(_) => "sleep",
            Kind::Room(_) => "room",
            Kind::Void(_) => "void",
        };
        Self {
            id,
            at,
            boundary,
            amount: 0.0,
            ends: SimTime::from_ns(u64::MAX),
            rate: 0.0,
            kind,
        }
    }
    fn checked(
        id: InputId,
        at: SimTime,
        boundary: &'static str,
        amount: f64,
        ends: SimTime,
        place: usize,
    ) -> Result<Self, AdmissionError> {
        if ends <= at {
            return Err(AdmissionError::EmptyDelivery { id });
        }
        if amount < 0.0 {
            return Err(AdmissionError::NegativeDelivery { id });
        }
        let seconds = (ends.nanoseconds() - at.nanoseconds()) as f64 / NANOSECONDS_PER_SECOND;
        let rate = amount / seconds;
        if !rate.is_finite() {
            return Err(AdmissionError::UnboundedDelivery { id });
        }
        Ok(Self {
            id,
            at,
            boundary,
            amount,
            ends,
            rate,
            kind: Kind::Delivered(place),
        })
    }
    pub(crate) fn held_event(&self) -> bool {
        !matches!(self.kind, Kind::Delivered(_))
    }
    pub(crate) fn carries(&self, species: Species) -> bool {
        matches!(self.kind, Kind::Delivered(place) if Meal::SPECIES[place] == species)
    }
    fn payload(&self) -> Digest {
        let mut out = Canonical::new("ng-contract delivery 1");
        self.seal(&mut out);
        out.digest()
    }
}

pub(crate) struct Plan {
    pub(crate) due: Vec<InputId>,
    pub(crate) frame: DriveFrame,
    pub(crate) after_breakpoint: bool,
    /// The earliest end of a delivery running or due here, or the next minute of a clock the body reads. That
    /// input has applied or applies with this step, so every run at this head knows this instant.
    pub(crate) running_end: Option<SimTime>,
}

#[derive(Clone, Copy)]
pub enum Attempt<'a> {
    First { span_ns: u64 },
    After(&'a Refused),
}

/// The timeline: record state, and the one way to a ticket. A core holds a problem, and neither a problem
/// nor an accepted view lends this value, so a waiting input shows to a core only in a cut.
pub struct PendingInputs {
    timeline: Vec<Delivery>,
    past: AppliedInputs,
}

/// What has applied of the timeline: the part an accepted image seals and one publish replaces. It reads no
/// input that still waits, so two runs with one past seal the same bytes whatever waits.
pub struct AppliedInputs {
    cursor: usize,
    active: Vec<Delivery>,
    chain: Digest,
    fence: SimTime,
}
impl AppliedInputs {
    pub fn digest(&self) -> Digest {
        self.canonical().digest()
    }
    /// Every input applied so far, payload and time, in order: equal for two runs of one timeline.
    pub(crate) fn chain(&self) -> Digest {
        self.chain
    }
    pub(crate) fn canonical(&self) -> Canonical {
        let Self {
            cursor,
            active,
            chain,
            fence,
        } = self;
        let mut out = Canonical::new("ng-contract applied inputs 1");
        cursor.seal(&mut out);
        active.seal(&mut out);
        chain.seal(&mut out);
        fence.seal(&mut out);
        out
    }
}

fn chained(chain: Digest, delivery: &Delivery) -> Digest {
    let mut out = Canonical::new("ng-contract applied chain 1");
    chain.seal(&mut out);
    delivery.seal(&mut out);
    out.digest()
}

impl PendingInputs {
    pub(crate) fn copied(&self) -> Self {
        let AppliedInputs {
            cursor,
            active,
            chain,
            fence,
        } = &self.past;
        Self {
            timeline: self.timeline.clone(),
            past: AppliedInputs {
                cursor: *cursor,
                active: active.clone(),
                chain: *chain,
                fence: *fence,
            },
        }
    }
    /// The fence opens at the first instant admission did not read: an input stamped before it cannot arrive
    /// after the start state that read its drive.
    pub(crate) fn opening(port: BoundaryPort, fence: SimTime) -> Self {
        Self {
            timeline: port.deliveries,
            past: AppliedInputs {
                cursor: 0,
                active: Vec::new(),
                chain: Digest::of_canonical(b"ng-contract no input applied"),
                fence,
            },
        }
    }
    /// The drives running across `[at, at + 1 ns)`, as a ticket starting at `at` would freeze them. Unlike `plan`
    /// it reads the whole timeline, because nothing has applied yet and a delivery can end inside a window.
    pub(crate) fn frame_at(&self, at: SimTime, reads: &[Held]) -> DriveFrame {
        let mut frame = DriveFrame::default();
        for delivery in &self.timeline {
            if delivery.at <= at && at < delivery.ends {
                frame.take(delivery, at, reads);
            }
        }
        frame
    }
    fn due_at(&self, start: SimTime) -> &[Delivery] {
        let waiting = &self.timeline[self.past.cursor..];
        &waiting[..waiting.partition_point(|delivery| delivery.at <= start)]
    }
    fn running_at(&self, start: SimTime) -> impl Iterator<Item = &Delivery> {
        let running = (self.past.active.iter()).filter(move |delivery| delivery.ends > start);
        running.chain(self.due_at(start))
    }
    pub(crate) fn plan(&self, start: SimTime, fired: bool, reads: &[Held]) -> Plan {
        let due = self.due_at(start);
        let ended = self.past.active.iter().any(|delivery| delivery.ends == start);
        let mut frame = DriveFrame::default();
        for delivery in self.running_at(start) {
            frame.take(delivery, start, reads);
        }
        let clock = (self.running_at(start))
            .filter_map(|delivery| match delivery.kind {
                Kind::Clock(reading) => Some((delivery.at, reading)),
                _ => None,
            })
            .last()
            .filter(|_| reads_clock(reads));
        let turned = clock.is_some_and(|(at, reading)| into_minute(at, reading, start) == 0);
        let next_turn = clock.map(|(at, reading)| next_minute(at, reading, start));
        Plan {
            due: due.iter().map(|delivery| delivery.id).collect(),
            frame,
            after_breakpoint: !due.is_empty() || ended || fired || turned,
            running_end: (self.running_at(start).map(|delivery| delivery.ends))
                .chain(next_turn)
                .min(),
        }
    }
    /// One attempt of a step: issues its ticket and hands it to `solved`, which solves and validates. It finds
    /// the blind step first: the span's result, or, when that crosses a guard, the landing inside it, fired. When
    /// a waiting input lies inside the blind step, it drops that result and hands `solved` the same ticket
    /// ended at that input. A cut can be refused like any ticket, and a cut that crosses lands in turn.
    ///
    /// This order is the causal step rule. A zero span, or one past the end of time, is `BindingMismatch`, as
    /// is a refusal of another base.
    pub fn attempt<T>(
        &self,
        problem: &CoupledProblem<'_>,
        attempt: Attempt<'_>,
        mut solved: impl FnMut(&StepTicket) -> Result<(VerifiedStep, T), TrialFailure>,
    ) -> Result<(VerifiedStep, T), TrialFailure> {
        let start = problem.accepted.time();
        let (span_ns, number, fence) = match attempt {
            Attempt::First { span_ns } => (span_ns, 0, start),
            Attempt::After(refused) if refused.base == problem.base => {
                (refused.retry_ns, refused.attempt + 1, refused.fence)
            }
            Attempt::After(refused) => {
                return Err(TrialFailure::EngineFault(EngineFault {
                    kind: EngineFaultKind::BindingMismatch,
                    at: refused.at,
                    attempted_ns: refused.attempted_ns,
                    last_refusal: None,
                }));
            }
        };
        let mismatch = EngineFault {
            kind: EngineFaultKind::BindingMismatch,
            at: start,
            attempted_ns: span_ns,
            last_refusal: None,
        };
        let plan = self.plan(start, problem.accepted.state().fired, problem.body.held_reads());
        let capped = span_ns.min(problem.longest_span_ns());
        let wanted = start.plus_ns(capped).ok_or(mismatch)?;
        let end = plan.running_end.map_or(wanted, |ends| ends.min(wanted));
        let interval = Interval::new(start, end).ok_or(mismatch)?;
        let resolution = problem.identity.resolution();
        let identity = problem.identity.digest();
        let mut ticket = StepTicket {
            base: problem.base,
            interval,
            span_ns,
            due: plan.due,
            frame: plan.frame,
            after_breakpoint: plan.after_breakpoint,
            attempt: number,
            fence,
            floor_ns: resolution.minimum_interval_ns(),
            budget: resolution.retry_budget(),
            digest: problem.base,
        };
        ticket.digest = ticket.sealed(identity);
        let passed = solved(&ticket)?;
        let blind = land(problem, &ticket, passed, &mut solved)?;
        let end = blind.0.ticket().interval().end();
        let cut = (self.timeline[self.past.cursor + ticket.due.len()..].first())
            .map(|delivery| delivery.at)
            .filter(|next| *next < end)
            .and_then(|next| Some((next, Interval::new(start, next)?)));
        let Some((next, interval)) = cut else {
            return Ok(blind);
        };
        drop(blind);
        ticket.interval = interval;
        ticket.fence = next;
        ticket.digest = ticket.sealed(identity);
        let passed = solved(&ticket)?;
        land(problem, &ticket, passed, &mut solved)
    }
    /// Adds a late input to what waits, and returns its payload's digest. No accepted part moves: no
    /// revision, no seal, no ticket. The same id with the same payload is the same request whenever it
    /// comes, even after it applied.
    pub fn admit(
        &mut self,
        body: &CompiledBody,
        head: SimTime,
        event: BoundaryEvent,
    ) -> Result<Digest, AdmissionError> {
        let arrival = Delivery::parsed(event)?;
        let (id, payload) = (arrival.id, arrival.payload());
        if !arrival.new_to(&self.timeline)? {
            return Ok(payload);
        }
        let fence = self.past.fence;
        for part in &arrival.parts {
            body.check_delivery(part, head)
                .map_err(|error| arrival.named(error))?;
            if part.at < fence {
                return Err(AdmissionError::EventBeforeFence { id, fence });
            }
        }
        for part in arrival.parts {
            let waiting = &self.timeline[self.past.cursor..];
            let key = (part.at, part.id);
            let at = waiting.partition_point(|known| (known.at, known.id) < key);
            self.timeline.insert(self.past.cursor + at, part);
        }
        Ok(payload)
    }
    /// The earliest stamp `admit` takes at `head`.
    pub fn earliest_stamp(&self, head: SimTime) -> SimTime {
        self.past.fence.max(head)
    }
    /// What has applied once `ticket`'s interval publishes.
    pub fn after(&self, ticket: &StepTicket) -> AppliedInputs {
        let start = ticket.interval().start();
        let due = self.due_at(start);
        AppliedInputs {
            cursor: self.past.cursor + due.len(),
            active: self.running_at(start).copied().collect(),
            chain: due.iter().fold(self.past.chain, chained),
            fence: ticket.fence.max(self.past.fence),
        }
    }
    pub fn advance(&mut self, past: AppliedInputs) {
        self.past = past;
    }
    /// Admission only adds to the timeline, so this moves exactly when a new input arrives.
    pub(crate) fn known(&self) -> usize {
        self.timeline.len()
    }
    pub fn applied(&self) -> &AppliedInputs {
        &self.past
    }
    pub fn waiting_digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract waiting inputs 1");
        self.timeline[self.past.cursor..].seal(&mut out);
        out.digest()
    }
    /// `chain` and every `rate` but a posture's are derived, so `read` never takes them on trust.
    pub(crate) fn write(&self, out: &mut Canonical) {
        let Self { timeline, past } = self;
        let AppliedInputs {
            cursor,
            active,
            chain: _,
            fence,
        } = past;
        out.length(timeline.len());
        for delivery in timeline {
            let Delivery {
                id,
                at,
                boundary,
                amount,
                ends,
                rate,
                kind,
            } = delivery;
            out.raw(id.digest.as_bytes());
            at.seal(out);
            out.number(payload_tag(*kind));
            out.text(boundary);
            amount.seal(out);
            ends.seal(out);
            if *kind == Kind::Posture {
                rate.seal(out);
            }
            kind.seal_setting(out);
        }
        cursor.seal(out);
        out.length(active.len());
        for delivery in active {
            let place = timeline.iter().position(|known| known.id == delivery.id);
            place.expect("a running delivery is on the timeline").seal(out);
        }
        fence.seal(out);
    }
    pub(crate) fn read(
        read: &mut ByteReader<'_>,
        body: &CompiledBody,
        time: SimTime,
    ) -> Result<Self, Malformed> {
        let mut timeline: Vec<Delivery> = Vec::new();
        for _ in 0..read.count(DELIVERY_BYTES)? {
            let id = InputId::new(read.digest()?);
            let at = read.time()?;
            let tag = read.number()?;
            let name = read.text()?;
            let amount = read.float()?;
            let ends = read.time()?;
            let delivery = match tag {
                POSTURE_PAYLOAD if amount.abs() > 1.0 => return Err(Malformed),
                POSTURE_PAYLOAD => Delivery::posture(id, at, amount, Legs::of_share(read.float()?)?),
                CLOCK_PAYLOAD => match read.number()? {
                    reading if reading >= DAY_NS => return Err(Malformed),
                    reading => Delivery::held(id, at, Kind::Clock(reading)),
                },
                SLEEP_PAYLOAD => Delivery::held(id, at, Kind::Sleep(Sleep::of_value(read.float()?)?)),
                ROOM_PAYLOAD => {
                    let mut values = [0.0; ROOM.len()];
                    for value in &mut values {
                        *value = read.float()?;
                    }
                    if !room_in_range(values) {
                        return Err(Malformed);
                    }
                    Delivery::held(id, at, Kind::Room(values))
                }
                VOID_PAYLOAD => Delivery::held(id, at, Kind::Void(Gate::of_value(read.float()?)?)),
                tag => {
                    let place = delivered_place(tag)?;
                    let boundary = body.boundary_name(name).ok_or(Malformed)?;
                    Delivery::checked(id, at, boundary, amount, ends, place).map_err(|_| Malformed)?
                }
            };
            let start_of_time = SimTime::from_ns(0);
            body.check_delivery(&delivery, start_of_time)
                .map_err(|_| Malformed)?;
            let ordered = (timeline.last()).is_none_or(|last| (last.at, last.id) < (at, id));
            if !ordered || timeline.iter().any(|known| known.id == id) {
                return Err(Malformed);
            }
            timeline.push(delivery);
        }
        let cursor = usize::try_from(read.number()?).map_err(|_| Malformed)?;
        let applied = timeline.get(..cursor).ok_or(Malformed)?;
        if applied.iter().any(|delivery| delivery.at >= time)
            || timeline[cursor..].iter().any(|delivery| delivery.at < time)
        {
            return Err(Malformed);
        }
        let mut active: Vec<Delivery> = Vec::new();
        for _ in 0..read.count(8)? {
            let place = usize::try_from(read.number()?).map_err(|_| Malformed)?;
            let delivery = *applied.get(place).ok_or(Malformed)?;
            if delivery.ends < time || active.iter().any(|known| known.id == delivery.id) {
                return Err(Malformed);
            }
            active.push(delivery);
        }
        let fence = read.time()?;
        let start = Digest::of_canonical(b"ng-contract no input applied");
        let chain = applied.iter().fold(start, chained);
        Ok(Self {
            timeline,
            past: AppliedInputs {
                cursor,
                active,
                chain,
                fence,
            },
        })
    }
}

/// A cut's ticket carries its fence into every trial, so a landing short of a shown input leaves that fence
/// when it publishes. A landing comes back fired, so a firing with no state to publish refuses before
/// anything reads the timeline, in a run that knows an input as in one that does not.
fn land<T>(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
    passed: (VerifiedStep, T),
    solved: &mut impl FnMut(&StepTicket) -> Result<(VerifiedStep, T), TrialFailure>,
) -> Result<(VerifiedStep, T), TrialFailure> {
    let start = ticket.interval().start();
    let Some(guard) = passed.0.crossed(problem)? else {
        return Ok(passed);
    };
    let floor = problem.identity.resolution().minimum_interval_ns();
    let identity = problem.identity.digest();
    let base = problem.gap(ticket, problem.accepted.state(), guard)?;
    let mut bracket = Bracket::new(base, ticket.length_ns(), passed.0.gap(problem, guard)?, floor);
    let mut top = passed;
    while !bracket.closed() {
        let length = bracket.aim();
        let end = start.plus_ns(length).ok_or(ticket.mismatch())?;
        let mut trial = ticket.clone();
        trial.interval = Interval::new(start, end).ok_or(ticket.mismatch())?;
        trial.digest = trial.sealed(identity);
        // A refusal drops the bracket: a shorter ticket can be the refused one, because a stage's error is
        // not monotone in its length.
        let step = solved(&trial)?;
        let crossed = step.0.crossed(problem)?.is_some();
        bracket.note(length, step.0.gap(problem, guard)?, crossed);
        if crossed {
            top = step;
        }
    }
    Ok((top.0.fire(problem)?, top.1))
}

const POSTURE: &str = "posture";
const DELIVERED_PAYLOADS: [u64; Meal::SPECIES.len()] = [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12];
const POSTURE_PAYLOAD: u64 = 4;
const CLOCK_PAYLOAD: u64 = 13;
const SLEEP_PAYLOAD: u64 = 14;
const ROOM_PAYLOAD: u64 = 15;
const VOID_PAYLOAD: u64 = 16;

fn payload_tag(kind: Kind) -> u64 {
    match kind {
        Kind::Delivered(place) => DELIVERED_PAYLOADS[place],
        Kind::Posture => POSTURE_PAYLOAD,
        Kind::Clock(_) => CLOCK_PAYLOAD,
        Kind::Sleep(_) => SLEEP_PAYLOAD,
        Kind::Room(_) => ROOM_PAYLOAD,
        Kind::Void(_) => VOID_PAYLOAD,
    }
}
fn delivered_place(tag: u64) -> Result<usize, Malformed> {
    (DELIVERED_PAYLOADS.iter())
        .position(|known| *known == tag)
        .ok_or(Malformed)
}
const DELIVERY_BYTES: usize = 32 + 8 + 8 + 8 + 8 + 8;

fn reads_clock(reads: &[Held]) -> bool {
    reads.contains(&Held::ClockCosine) || reads.contains(&Held::ClockSine)
}
fn into_minute(at: SimTime, reading: u64, now: SimTime) -> u64 {
    let elapsed = now.nanoseconds().saturating_sub(at.nanoseconds());
    (reading % MINUTE_NS + elapsed % MINUTE_NS) % MINUTE_NS
}
fn next_minute(at: SimTime, reading: u64, now: SimTime) -> SimTime {
    let left = MINUTE_NS - into_minute(at, reading, now);
    SimTime::from_ns(now.nanoseconds().saturating_add(left))
}
fn time_of_day(at: SimTime, reading: u64, now: SimTime) -> u64 {
    let elapsed = now.nanoseconds().saturating_sub(at.nanoseconds());
    (reading + elapsed % DAY_NS) % DAY_NS
}
fn minute_of_day(at: SimTime, reading: u64, now: SimTime) -> u64 {
    time_of_day(at, reading, now) / MINUTE_NS
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DriveFrame {
    drives: Vec<(&'static str, f64)>,
    trunk: f64,
    legs: f64,
    /// Only the held inputs the body reads, so a ticket never seals one no law sees.
    held: [Option<f64>; HELD],
}
impl DriveFrame {
    /// A site's height in gravity from its place in the body frame, so posture reaches every law through one read and no
    /// law carries gravity. The hip is the origin: what lies above it turns with the trunk, what lies below with the legs.
    pub(crate) fn height(&self, axial: f64) -> f64 {
        axial * if axial < 0.0 { self.legs } else { self.trunk }
    }
    /// Deliveries add, and a posture replaces the one before it: both arrive here in `(time, id)` order.
    fn take(&mut self, delivery: &Delivery, now: SimTime, reads: &[Held]) {
        match delivery.kind {
            Kind::Delivered(_) => self.add(delivery.boundary, delivery.rate),
            Kind::Posture => (self.trunk, self.legs) = (delivery.amount, delivery.rate),
            Kind::Clock(reading) => {
                let (cosine, sine) = turn(minute_of_day(delivery.at, reading, now), EIGHTH_OF_A_DAY);
                self.set(Held::ClockCosine, cosine, reads);
                self.set(Held::ClockSine, sine, reads);
            }
            Kind::Sleep(sleep) => self.set(Held::Sleep, sleep.value(), reads),
            Kind::Room(values) => {
                for (held, value) in ROOM.into_iter().zip(values) {
                    self.set(held, value, reads);
                }
            }
            Kind::Void(gate) => self.set(Held::Void, gate.value(), reads),
        }
    }
    fn set(&mut self, held: Held, value: f64, reads: &[Held]) {
        if reads.contains(&held) {
            self.held[held as usize] = Some(value);
        }
    }
    pub(crate) fn states(&self, held: Held) -> bool {
        self.held[held as usize].is_some()
    }
    /// Admission refuses a body that reads a held input with no value, so a run never reads this NaN.
    pub(crate) fn held(&self, held: Held) -> f64 {
        self.held[held as usize].unwrap_or(f64::NAN)
    }
    fn add(&mut self, boundary: &'static str, rate: f64) {
        match self.drives.binary_search_by_key(&boundary, |drive| drive.0) {
            Ok(at) => self.drives[at].1 += rate,
            Err(at) => self.drives.insert(at, (boundary, rate)),
        }
    }
    pub(crate) fn rate(&self, boundary: &str) -> f64 {
        match self.drives.binary_search_by_key(&boundary, |drive| drive.0) {
            Ok(at) => self.drives[at].1,
            Err(_) => 0.0,
        }
    }
}
impl Seal for DriveFrame {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            drives,
            trunk,
            legs,
            held,
        } = self;
        out.length(drives.len());
        for (boundary, rate) in drives {
            out.text(boundary);
            rate.seal(out);
        }
        trunk.seal(out);
        legs.seal(out);
        // A frame that holds no held input seals the bytes it did before there were any.
        if held.iter().any(Option::is_some) {
            out.number(u64::MAX);
            held.seal(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BoundaryEvent, BoundaryInput, BoundaryPort, Clock, Cycle, DriveFrame, EIGHTH_OF_A_DAY, Gate, Held,
        InputId, MINUTE_NS, PendingInputs, Room, Sleep, turn,
    };
    use crate::identity::{Canonical, Seal};
    use crate::{AdmissionError, Digest, Measure, SimTime};
    use core::num::{NonZeroU32, NonZeroU64};

    const SECOND: u64 = 1_000_000_000;
    const HOUR: u64 = 60 * MINUTE_NS;
    const CLOCK: [Held; 2] = [Held::ClockCosine, Held::ClockSine];
    type Change = fn(&mut [f64; 8]);

    fn event(name: &[u8], at: u64, payload: BoundaryInput) -> BoundaryEvent {
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(name)),
            SimTime::from_ns(at),
            payload,
        )
    }
    fn clock(at: u64, since_midnight_ns: u64) -> BoundaryEvent {
        event(b"clock", at, BoundaryInput::Clock(Clock { since_midnight_ns }))
    }
    fn opened(events: Vec<BoundaryEvent>) -> Result<PendingInputs, AdmissionError> {
        Ok(PendingInputs::opening(
            BoundaryPort::new(events)?,
            SimTime::from_ns(0),
        ))
    }
    fn applied(events: Vec<BoundaryEvent>) -> Result<PendingInputs, AdmissionError> {
        let mut pending = opened(events)?;
        pending.past.cursor = pending.timeline.len();
        pending.past.active = pending.timeline.clone();
        Ok(pending)
    }
    fn bits(frame: &DriveFrame) -> (u64, u64) {
        (
            frame.held(Held::ClockCosine).to_bits(),
            frame.held(Held::ClockSine).to_bits(),
        )
    }
    fn turned(minute: u64) -> (u64, u64) {
        let (cosine, sine) = turn(minute, EIGHTH_OF_A_DAY);
        (cosine.to_bits(), sine.to_bits())
    }

    #[test]
    fn the_clock_is_zero_at_midnight_turns_once_a_day_and_moves_each_whole_minute()
    -> Result<(), AdmissionError> {
        let half_past_seven = 7 * HOUR + 30 * MINUTE_NS;
        let pending = opened(vec![clock(0, half_past_seven)])?;
        let frame = |at: u64| pending.frame_at(SimTime::from_ns(at), &CLOCK);
        let (cosine, sine) = (frame(0).held(Held::ClockCosine), frame(0).held(Held::ClockSine));
        assert!((cosine + 0.382_683_432_365_089_8).abs() < 1e-15, "{cosine}");
        assert!((sine - 0.923_879_532_511_286_7).abs() < 1e-15, "{sine}");
        assert_eq!(bits(&frame(0)), turned(450));
        assert_eq!(bits(&frame(MINUTE_NS - 1)), turned(450));
        assert_eq!(bits(&frame(MINUTE_NS)), turned(451));
        let midnight = opened(vec![clock(0, 24 * HOUR - 30 * SECOND)])?;
        let frame = |at: u64| midnight.frame_at(SimTime::from_ns(at), &CLOCK);
        assert_eq!(bits(&frame(29 * SECOND)), turned(1439));
        assert_eq!(bits(&frame(30 * SECOND)), (1.0_f64.to_bits(), 0.0_f64.to_bits()));
        assert_eq!(bits(&frame(30 * SECOND + 24 * HOUR)), turned(0), "a day later");
        Ok(())
    }

    #[test]
    fn a_held_input_replaces_the_one_before_it_and_a_frame_holds_only_what_the_body_reads()
    -> Result<(), AdmissionError> {
        let pending = opened(vec![
            event(b"awake", 0, BoundaryInput::Sleep(Sleep::Awake)),
            event(b"asleep", 10 * SECOND, BoundaryInput::Sleep(Sleep::Nrem)),
            event(b"open", 0, BoundaryInput::Void(Gate::Open)),
            event(b"shut", 10 * SECOND, BoundaryInput::Void(Gate::Shut)),
        ])?;
        let reads = [Held::Sleep, Held::Void];
        let early = pending.frame_at(SimTime::from_ns(5 * SECOND), &reads);
        let late = pending.frame_at(SimTime::from_ns(10 * SECOND), &reads);
        assert_eq!((early.held(Held::Sleep), early.held(Held::Void)), (0.0, 1.0));
        assert_eq!((late.held(Held::Sleep), late.held(Held::Void)), (1.0, 0.0));
        let unread = pending.frame_at(SimTime::from_ns(10 * SECOND), &[Held::Void]);
        assert!(!unread.states(Held::Sleep) && unread.held(Held::Sleep).is_nan());
        Ok(())
    }

    #[test]
    fn a_frame_seals_its_held_inputs_only_when_it_holds_one() -> Result<(), AdmissionError> {
        let sealed = |frame: &DriveFrame| {
            let mut out = Canonical::new("frame");
            frame.seal(&mut out);
            out.digest()
        };
        let mut before = Canonical::new("frame");
        before.length(0);
        0.0_f64.seal(&mut before);
        0.0_f64.seal(&mut before);
        let pending = opened(vec![event(b"open", 0, BoundaryInput::Void(Gate::Open))])?;
        let unread = pending.frame_at(SimTime::from_ns(0), &[Held::Sleep]);
        let read = pending.frame_at(SimTime::from_ns(0), &[Held::Void]);
        assert_eq!(sealed(&DriveFrame::default()), before.digest());
        assert_eq!(sealed(&unread), before.digest());
        assert_ne!(sealed(&read), before.digest());
        Ok(())
    }

    #[test]
    fn in_a_body_that_reads_the_clock_each_whole_minute_ends_a_step_and_starts_one_after_a_breakpoint()
    -> Result<(), AdmissionError> {
        let pending = applied(vec![clock(0, 7 * HOUR + 20 * SECOND)])?;
        let edge = SimTime::from_ns(40 * SECOND);
        let inside = pending.plan(SimTime::from_ns(SECOND), false, &CLOCK);
        assert_eq!(inside.running_end, Some(edge));
        assert!(!inside.after_breakpoint);
        let at_edge = pending.plan(edge, false, &CLOCK);
        assert!(at_edge.after_breakpoint);
        assert_eq!(
            at_edge.running_end,
            SimTime::from_ns(40 * SECOND).plus_ns(MINUTE_NS)
        );
        let blind = pending.plan(edge, false, &[Held::Sleep]);
        assert!(!blind.after_breakpoint);
        assert_eq!(blind.running_end, Some(SimTime::from_ns(u64::MAX)));
        Ok(())
    }

    #[test]
    fn a_clock_whose_minute_turns_between_two_points_is_off_the_grid_only_in_a_body_that_reads_it()
    -> Result<(), AdmissionError> {
        let id = InputId::new(Digest::of_canonical(b"clock"));
        let port = BoundaryPort::new(vec![clock(0, 7 * HOUR + 20 * SECOND)])?;
        let cycle = |seconds: u64, intervals: u32| {
            let period = NonZeroU64::new(seconds * SECOND).expect("a period");
            Cycle::new(period, NonZeroU32::new(intervals).expect("intervals"))
        };
        let start = SimTime::from_ns(0);
        assert_eq!(
            port.off_grid(start, cycle(48, 4)?, true),
            Some(AdmissionError::ClockOffGrid { id }),
            "40 s between 36 and 48"
        );
        assert_eq!(port.off_grid(start, cycle(48, 4)?, false), None);
        assert_eq!(port.off_grid(start, cycle(50, 5)?, true), None, "40 s is a point");
        assert_eq!(port.off_grid(start, cycle(30, 4)?, true), None, "past the window");
        Ok(())
    }

    #[test]
    fn a_clock_of_a_day_or_more_and_a_room_out_of_range_are_refused() -> Result<(), AdmissionError> {
        let late = clock(0, 24 * HOUR);
        let id = late.id();
        assert_eq!(
            BoundaryPort::new(vec![late]).err(),
            Some(AdmissionError::ClockOutOfRange { id })
        );
        assert!(BoundaryPort::new(vec![clock(0, 24 * HOUR - 1)]).is_ok());
        let room = |change: Change| -> Result<BoundaryEvent, AdmissionError> {
            let mut values = [293.0, 293.0, 0.5, 1.0, 760.0, 0.21, 0.0004, 1.0];
            change(&mut values);
            let [
                air,
                radiant,
                humidity,
                air_speed,
                pressure,
                oxygen,
                carbon_dioxide,
                clothing,
            ] = values;
            Ok(event(
                b"room",
                0,
                BoundaryInput::Room(Room {
                    air: Measure::new(air)?,
                    radiant: Measure::new(radiant)?,
                    humidity: Measure::new(humidity)?,
                    air_speed: Measure::new(air_speed)?,
                    pressure: Measure::new(pressure)?,
                    oxygen: Measure::new(oxygen)?,
                    carbon_dioxide: Measure::new(carbon_dioxide)?,
                    clothing: Measure::new(clothing)?,
                }),
            ))
        };
        let id = InputId::new(Digest::of_canonical(b"room"));
        assert!(BoundaryPort::new(vec![room(|_| {})?]).is_ok());
        let edges: [Change; 6] = [
            |values| values[2] = 0.0,
            |values| values[2] = 1.0,
            |values| values[3] = 0.0,
            |values| values[7] = 0.0,
            |values| (values[5], values[6]) = (0.75, 0.25),
            |values| (values[5], values[6]) = (1.0, 0.0),
        ];
        for edge in edges {
            assert!(BoundaryPort::new(vec![room(edge)?]).is_ok());
        }
        let breaks: [(&str, Change); 11] = [
            ("air at absolute zero", |values| values[0] = 0.0),
            ("radiant at absolute zero", |values| values[1] = 0.0),
            ("humidity below 0", |values| values[2] = -0.1),
            ("humidity past 1", |values| values[2] = 1.1),
            ("negative air speed", |values| values[3] = -0.1),
            ("no pressure", |values| values[4] = 0.0),
            ("oxygen past 1", |values| values[5] = 1.1),
            ("negative carbon dioxide", |values| values[6] = -0.1),
            ("carbon dioxide past 1", |values| values[6] = 1.1),
            ("fractions past 1 together", |values| {
                (values[5], values[6]) = (0.75, 0.3)
            }),
            ("negative clothing", |values| values[7] = -0.1),
        ];
        for (name, change) in breaks {
            let refused = BoundaryPort::new(vec![room(change)?]).err();
            assert_eq!(refused, Some(AdmissionError::RoomOutOfRange { id }), "{name}");
        }
        Ok(())
    }
}
