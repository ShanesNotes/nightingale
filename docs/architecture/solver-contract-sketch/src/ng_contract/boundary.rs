//! The boundary port: ONE timestamped timeline of the day, partitioned by payload and ordered by
//! (time, id). Pending events and active continuous deliveries are record state and checkpoint.
use super::{AdmissionError, Digest, SimTime};

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
pub struct InputId {
    _digest: Digest,
}
impl InputId {
    pub fn new(_digest: Digest) -> Self {
        todo!()
    }
}
pub enum BoundaryInput {
    Intake(Intake),
    Posture(Posture),
    Environment(Environment),
    Clock(ClockPhase),
    Sleep(SleepState),
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
/// Pending timeline plus cursor and active deliveries: record state, checkpointed (#238).
pub struct PendingInputs {
    _port: BoundaryPort,
    _cursor: u64,
    _active_deliveries: Vec<Intake>,
}
/// Read-only view of the pending timeline inside one problem. Only the contract reads it: it
/// clips each ticket at the next physical breakpoint (a timeline event or a delivery's end; never a
/// UI refresh, pace change or checkpoint request), so no core sees a later input.
pub struct BoundaryView<'a> {
    _pending: &'a PendingInputs,
}
