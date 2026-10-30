//! The boundary port: ONE timestamped timeline of the day, partitioned by payload and ordered by
//! (time, id). Pending events and active continuous deliveries are record state and checkpoint.
use super::{AdmissionError, Digest, Measure, SimTime, StepTicket, Water};

pub struct BoundaryPort {
    _events: Vec<BoundaryEvent>,
}
impl BoundaryPort {
    /// Parses and orders a schedule; duplicate ids with different payloads are refused, and so is
    /// a continuous delivery that ends at or before its event.
    pub fn new(_events: Vec<BoundaryEvent>) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct BoundaryEvent {
    _id: InputId,
    _at: SimTime,
    _payload: BoundaryInput,
}
impl BoundaryEvent {
    pub fn new(_id: InputId, _at: SimTime, _payload: BoundaryInput) -> Self {
        todo!()
    }
}
#[derive(Clone, Copy)]
pub struct InputId {
    _digest: Digest,
}
impl InputId {
    pub fn new(_digest: Digest) -> Self {
        todo!()
    }
}
pub enum BoundaryInput {
    /// A bench's plain delivery. `Intake` adds composition and routes to the same envelope.
    Water(WaterDelivery),
    Intake(Intake),
    Posture(Posture),
    Environment(Environment),
    Clock(ClockPhase),
    Sleep(SleepState),
}
/// `amount` of water through the named boundary, from the event's instant until `ends`, at the rate
/// that delivers it exactly, so "applied once" shows as a total received. Overlapping deliveries add.
pub struct WaterDelivery {
    pub boundary: &'static str,
    pub amount: Measure<Water>,
    pub ends: SimTime,
}
/// Oral, tube and IV deliveries share one envelope with distinct physical routes.
pub struct Intake {
    _route: IntakeRoute,
    _composition: Composition,
    _delivery: Delivery,
}
impl Intake {
    pub fn new(_route: IntakeRoute, _composition: Composition, _delivery: Delivery) -> Self {
        todo!()
    }
}
pub enum IntakeRoute {
    Oral,
    Tube,
    Intravenous,
}
pub enum Delivery {
    Bolus,
    Continuous { ends: SimTime },
}
/// One finite, nonnegative amount per entry of an intake schema. Every payload producer here refuses
/// non-finite values; units, ranges and the schema itself are checked against the compiled body
/// when the port is bound at initialization and admission.
pub struct Composition {
    _schema: Digest,
    _amounts: Vec<f64>,
}
impl Composition {
    pub fn new(_schema: Digest, _amounts: Vec<f64>) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// Continuous trunk angle plus a leg descriptor and subject bed offsets; never a two-value enum.
pub struct Posture {
    _trunk_angle: f64,
    _legs: Legs,
}
impl Posture {
    /// The angle may be negative (head down); only a non-finite one is refused here.
    pub fn new(_trunk_angle: f64, _legs: Legs) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub enum Legs {
    Horizontal,
    SeatedDependent,
    Standing,
}
pub struct Environment {
    _air_kelvin: f64,
    _radiant_kelvin: f64,
    _humidity: f64,
    _air_speed: f64,
    _clothing: f64,
    _bedding: f64,
    _pressure: f64,
    _inspired_fractions: Vec<f64>,
}
impl Environment {
    pub fn new(
        _air_kelvin: f64,
        _radiant_kelvin: f64,
        _humidity: f64,
        _air_speed: f64,
        _clothing: f64,
        _bedding: f64,
        _pressure: f64,
        _inspired_fractions: Vec<f64>,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct ClockPhase {
    _admission_phase: u64,
    _period_ns: u64,
}
impl ClockPhase {
    /// Refuses a zero period or an admission phase at or past it.
    pub fn new(_admission_phase: u64, _period_ns: u64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub enum SleepState {
    Awake,
    Nrem,
}
/// Pending timeline plus cursor and active deliveries: record state, checkpointed
/// (nightingale-pulse#238). Applying an input moves the cursor past it and adds its delivery in one
/// accepted image, so a refusal cannot half apply it. Its fields are private and `CoupledProblem`
/// lends no view of them: the contract clips each ticket at the next physical breakpoint (a timeline
/// event or a delivery's end; never a UI refresh, pace change or checkpoint request), so no core
/// sees a later input.
pub struct PendingInputs {
    _port: BoundaryPort,
    _cursor: u64,
    _active_deliveries: Vec<Intake>,
}
impl PendingInputs {
    /// The pending inputs once `ticket`'s interval publishes: its due inputs applied, and deliveries
    /// that ended at its start dropped.
    pub fn after(&self, _ticket: &StepTicket) -> Self {
        todo!()
    }
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
/// What the boundary supplies over one ticket, frozen when the ticket is issued. A law reads this and
/// never the stage's instant, so a stage at the ticket's end reads the left limit even when the
/// next delivery starts exactly there. A boundary with no delivery running supplies zero.
#[derive(Clone)]
pub struct DriveFrame {
    _drives: Vec<(&'static str, f64)>,
}
