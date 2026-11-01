//! The boundary port: ONE timestamped timeline of the day, partitioned by payload and ordered by
//! (time, id). Pending events and active continuous deliveries are record state and checkpoint.
use super::{AdmissionError, CompiledBody, CoupledProblem, Digest, Measure, Refused, SimTime, StepTicket, TrialFailure, VerifiedStep, Water};

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
    pub fn id(&self) -> InputId {
        todo!()
    }
    pub fn at(&self) -> SimTime {
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
/// A continuous trunk tilt plus a leg descriptor and subject bed offsets; never a two-value enum.
pub struct Posture {
    _trunk: f64,
    _legs: Legs,
}
impl Posture {
    /// `trunk` is the sine of the trunk's angle from flat, so it may be negative (head down); one past 1
    /// has no angle and is refused here.
    pub fn new(_trunk: f64, _legs: Legs) -> Result<Self, AdmissionError> {
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
#[derive(Clone, Copy)]
pub enum Attempt<'a> {
    First { span_ns: u64 },
    After(&'a Refused),
}
/// The timeline: record state, checkpointed (nightingale-pulse#238), and the one way to a ticket. A
/// core holds a problem, and neither a problem nor an accepted view lends this value, so a waiting
/// input shows to a core only in a cut.
pub struct PendingInputs {
    _timeline: BoundaryPort,
    _past: AppliedInputs,
}
/// What has applied of the timeline: the part an accepted image seals and one publish replaces. It
/// reads no input that still waits, so two runs with one past seal the same bytes whatever waits.
pub struct AppliedInputs {
    _cursor: u64,
    _active_deliveries: Vec<Intake>,
    _chain: Digest,
    _fence: SimTime,
}
impl AppliedInputs {
    pub fn digest(&self) -> Digest {
        todo!()
    }
}
impl PendingInputs {
    /// One attempt of a step: issues its ticket and hands it to `solved`, which solves and validates.
    /// It finds the blind step first: the span's result, or, when that crosses a guard, the landing
    /// inside it. When a waiting input lies inside the blind step, it drops that result and hands
    /// `solved` the same ticket ended at that input. This order is the causal step rule: every ticket of
    /// the blind step is a function of the sealed past, the span, the refusals of this base and
    /// published integrals, so it is the same ticket whatever waits. A waiting input shows only to a
    /// step the record would have accepted past it, and then no late input can still arrive before it,
    /// except behind the fence a cut leaves when it does not publish at its own end.
    pub fn attempt<T>(
        &self,
        _problem: &CoupledProblem<'_>,
        _attempt: Attempt<'_>,
        _solved: impl FnMut(&StepTicket) -> Result<(VerifiedStep, T), TrialFailure>,
    ) -> Result<(VerifiedStep, T), TrialFailure> {
        todo!()
    }
    /// Adds a late input to what waits, and returns its payload's digest. No accepted part moves. The
    /// same id with the same payload is the same request whenever it comes, even after it applied.
    pub fn admit(
        &mut self,
        _body: &CompiledBody,
        _head: SimTime,
        _event: BoundaryEvent,
    ) -> Result<Digest, AdmissionError> {
        todo!()
    }
    /// The earliest stamp `admit` takes at `head`.
    pub fn earliest_stamp(&self, _head: SimTime) -> SimTime {
        todo!()
    }
    /// What has applied once `ticket`'s interval publishes.
    pub fn after(&self, _ticket: &StepTicket) -> AppliedInputs {
        todo!()
    }
    pub fn advance(&mut self, _past: AppliedInputs) {
        todo!()
    }
    pub fn applied(&self) -> &AppliedInputs {
        todo!()
    }
    pub fn waiting_digest(&self) -> Digest {
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
