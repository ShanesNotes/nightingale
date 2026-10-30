use crate::identity::{Canonical, Seal};
use crate::{AdmissionError, Digest, Measure, SimTime, StepTicket, Water};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

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

/// `amount` of water through the named boundary, from the event's instant until `ends`, at the constant
/// rate that delivers it: "applied once" shows as a total received. Overlapping deliveries add.
pub struct WaterDelivery {
    pub boundary: &'static str,
    pub amount: Measure<Water>,
    pub ends: SimTime,
}
pub enum BoundaryInput {
    Water(WaterDelivery),
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Delivery {
    pub(crate) id: InputId,
    pub(crate) at: SimTime,
    pub(crate) boundary: &'static str,
    amount: f64,
    pub(crate) ends: SimTime,
    rate: f64,
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
        } = self;
        id.seal(out);
        at.seal(out);
        out.text(boundary);
        amount.seal(out);
        ends.seal(out);
        rate.seal(out);
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
            let delivery = Delivery::parsed(event)?;
            match deliveries.iter().find(|known| known.id == delivery.id) {
                Some(known) if *known == delivery => {}
                Some(_) => return Err(AdmissionError::InputConflict { id: delivery.id }),
                None => deliveries.push(delivery),
            }
        }
        deliveries.sort_by_key(|delivery| (delivery.at, delivery.id));
        Ok(Self { deliveries })
    }
}
impl Delivery {
    fn parsed(event: BoundaryEvent) -> Result<Self, AdmissionError> {
        let BoundaryEvent {
            id,
            at,
            payload:
                BoundaryInput::Water(WaterDelivery {
                    boundary,
                    amount,
                    ends,
                }),
        } = event;
        if ends <= at {
            return Err(AdmissionError::EmptyDelivery { id });
        }
        let amount = amount.value();
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
        })
    }
}

pub(crate) struct Plan {
    pub(crate) due: Vec<InputId>,
    pub(crate) frame: DriveFrame,
    pub(crate) after_breakpoint: bool,
    pub(crate) next_breakpoint: Option<SimTime>,
}

/// The future of the timeline and the deliveries running now: record state.
/// Applying an input moves the cursor past it and adds its delivery in one accepted image, so a refusal
/// cannot half apply it.
pub struct PendingInputs {
    deliveries: Vec<Delivery>,
    cursor: usize,
    active: Vec<Delivery>,
}
impl PendingInputs {
    pub(crate) fn opening(port: BoundaryPort) -> Self {
        Self {
            deliveries: port.deliveries,
            cursor: 0,
            active: Vec::new(),
        }
    }
    fn due_at(&self, start: SimTime) -> &[Delivery] {
        let waiting = &self.deliveries[self.cursor..];
        &waiting[..waiting.partition_point(|delivery| delivery.at <= start)]
    }
    fn running_at(&self, start: SimTime) -> impl Iterator<Item = &Delivery> {
        let running = self.active.iter().filter(move |delivery| delivery.ends > start);
        running.chain(self.due_at(start))
    }
    pub(crate) fn plan(&self, start: SimTime) -> Plan {
        let due = self.due_at(start);
        let later = self.deliveries[self.cursor + due.len()..].first();
        let ends = self.running_at(start).map(|delivery| delivery.ends);
        let ended = self.active.iter().any(|delivery| delivery.ends == start);
        let mut frame = DriveFrame::default();
        for delivery in self.running_at(start) {
            frame.add(delivery.boundary, delivery.rate);
        }
        Plan {
            due: due.iter().map(|delivery| delivery.id).collect(),
            frame,
            after_breakpoint: !due.is_empty() || ended,
            next_breakpoint: later.map(|delivery| delivery.at).into_iter().chain(ends).min(),
        }
    }
    /// The pending inputs once `ticket`'s interval publishes: its due inputs applied, and deliveries that
    /// ended at its start dropped.
    pub fn after(&self, ticket: &StepTicket) -> Self {
        let start = ticket.interval().start();
        let due = self.due_at(start);
        let active = self.running_at(start).copied().collect();
        Self {
            deliveries: self.deliveries.clone(),
            cursor: self.cursor + due.len(),
            active,
        }
    }
    pub fn digest(&self) -> Digest {
        let Self {
            deliveries,
            cursor,
            active,
        } = self;
        let mut out = Canonical::new("ng-contract pending inputs 1");
        deliveries.seal(&mut out);
        cursor.seal(&mut out);
        active.seal(&mut out);
        out.digest()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DriveFrame {
    drives: Vec<(&'static str, f64)>,
}
impl DriveFrame {
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
        let Self { drives } = self;
        out.length(drives.len());
        for (boundary, rate) in drives {
            out.text(boundary);
            rate.seal(out);
        }
    }
}
