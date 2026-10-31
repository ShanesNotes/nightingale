use crate::identity::{Canonical, Seal};
use crate::landing::Bracket;
use crate::{
    AdmissionError, ByteReader, CompiledBody, CoupledProblem, Digest, EngineFault, EngineFaultKind, Interval,
    Malformed, Measure, Refused, SimTime, StepTicket, TrialFailure, VerifiedStep, Water,
};

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
        Self::checked(id, at, boundary, amount.value(), ends)
    }
    fn checked(
        id: InputId,
        at: SimTime,
        boundary: &'static str,
        amount: f64,
        ends: SimTime,
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
        })
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
    /// The earliest end of a delivery running or due here. That input has applied or applies with this
    /// step, so every run at this head knows this instant.
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
        out.digest()
    }
}

fn chained(chain: Digest, delivery: &Delivery) -> Digest {
    let mut out = Canonical::new("ng-contract applied chain 1");
    chain.seal(&mut out);
    delivery.seal(&mut out);
    out.digest()
}

impl PendingInputs {
    /// The fence opens one nanosecond past `head`, because initialization has read `head`'s drive: an input
    /// stamped there cannot arrive after the start state that read it.
    pub(crate) fn opening(port: BoundaryPort, head: SimTime) -> Self {
        Self {
            timeline: port.deliveries,
            past: AppliedInputs {
                cursor: 0,
                active: Vec::new(),
                chain: Digest::of_canonical(b"ng-contract no input applied"),
                fence: head.plus_ns(1).unwrap_or(head),
            },
        }
    }
    fn due_at(&self, start: SimTime) -> &[Delivery] {
        let waiting = &self.timeline[self.past.cursor..];
        &waiting[..waiting.partition_point(|delivery| delivery.at <= start)]
    }
    fn running_at(&self, start: SimTime) -> impl Iterator<Item = &Delivery> {
        let running = (self.past.active.iter()).filter(move |delivery| delivery.ends > start);
        running.chain(self.due_at(start))
    }
    /// `fired` is the head's: a guard that fired at `start` is a breakpoint, read from sealed state, so a fresh
    /// session at this head reaches the same ticket.
    pub(crate) fn plan(&self, start: SimTime, fired: bool) -> Plan {
        let due = self.due_at(start);
        let ended = self.past.active.iter().any(|delivery| delivery.ends == start);
        let mut frame = DriveFrame::default();
        for delivery in self.running_at(start) {
            frame.add(delivery.boundary, delivery.rate);
        }
        Plan {
            due: due.iter().map(|delivery| delivery.id).collect(),
            frame,
            after_breakpoint: !due.is_empty() || ended || fired,
            running_end: self.running_at(start).map(|delivery| delivery.ends).min(),
        }
    }
    /// One attempt of a step: issues its ticket and hands it to `solved`, which solves and validates. It finds
    /// the blind step first: the span's result, or, when that crosses a guard, the landing inside it. When a
    /// waiting input lies inside the blind step, it drops that result and hands `solved` the same ticket
    /// ended at that input. A cut can be refused like any ticket, and a cut that crosses lands in turn.
    ///
    /// This order is the causal step rule. Every ticket of the blind step is a function of the sealed past,
    /// the span, the refusals of this base and published integrals, so it is the same ticket whatever waits.
    /// A waiting input shows only to a step the record would have accepted past it, and then no late input
    /// can still arrive before it, except behind the fence a cut leaves when it does not publish at its own
    /// end. An input past the landing shows to no one, so a run that takes it at the landing's head equals the
    /// run that knew it. A zero span, or one past the end of time, is `BindingMismatch`, as is a refusal of
    /// another base.
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
        let plan = self.plan(start, problem.accepted.state().fired);
        let wanted = start.plus_ns(span_ns).ok_or(mismatch)?;
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
            return Ok((blind.0.fire(problem), blind.1));
        };
        drop(blind);
        ticket.interval = interval;
        ticket.fence = next;
        ticket.digest = ticket.sealed(identity);
        let passed = solved(&ticket)?;
        let (step, kept) = land(problem, &ticket, passed, &mut solved)?;
        Ok((step.fire(problem), kept))
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
        let delivery = Delivery::parsed(event)?;
        let id = delivery.id;
        if let Some(known) = self.timeline.iter().find(|known| known.id == id) {
            if *known == delivery {
                return Ok(known.payload());
            }
            return Err(AdmissionError::InputConflict { id });
        }
        body.check_delivery(&delivery, head)?;
        let fence = self.past.fence;
        if delivery.at < fence {
            return Err(AdmissionError::EventBeforeFence { id, fence });
        }
        let waiting = &self.timeline[self.past.cursor..];
        let key = (delivery.at, delivery.id);
        let at = waiting.partition_point(|known| (known.at, known.id) < key);
        self.timeline.insert(self.past.cursor + at, delivery);
        Ok(delivery.payload())
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
    pub fn applied(&self) -> &AppliedInputs {
        &self.past
    }
    pub fn waiting_digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract waiting inputs 1");
        self.timeline[self.past.cursor..].seal(&mut out);
        out.digest()
    }
    /// `rate` and `chain` are derived, so `read` never takes them on trust.
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
                rate: _,
            } = delivery;
            out.raw(id.digest.as_bytes());
            at.seal(out);
            out.number(WATER_PAYLOAD);
            out.text(boundary);
            amount.seal(out);
            ends.seal(out);
        }
        cursor.seal(out);
        out.length(active.len());
        for delivery in active {
            let place = timeline.iter().position(|known| known.id == delivery.id);
            place.expect("a running delivery is on the timeline").seal(out);
        }
        fence.seal(out);
    }
    /// Refuses bytes no run writes.
    pub(crate) fn read(
        read: &mut ByteReader<'_>,
        body: &CompiledBody,
        time: SimTime,
    ) -> Result<Self, Malformed> {
        let mut timeline: Vec<Delivery> = Vec::new();
        for _ in 0..read.count(DELIVERY_BYTES)? {
            let id = InputId::new(read.digest()?);
            let at = read.time()?;
            if read.number()? != WATER_PAYLOAD {
                return Err(Malformed);
            }
            let boundary = body.boundary_name(read.text()?).ok_or(Malformed)?;
            let amount = read.float()?;
            let ends = read.time()?;
            let delivery = Delivery::checked(id, at, boundary, amount, ends).map_err(|_| Malformed)?;
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

/// The step a ticket's result stands for: the result itself when it crosses no guard, else the landing inside
/// it. It takes no timeline, so when `ticket` is blind this is blind, and a cut's ticket carries its fence into
/// every trial, so a landing short of a shown input leaves that fence when it publishes. Every trial is an
/// ordinary ticket through `solved`, so `verify` proves each one, and a core sees only a shorter ticket.
fn land<T>(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
    passed: (VerifiedStep, T),
    solved: &mut impl FnMut(&StepTicket) -> Result<(VerifiedStep, T), TrialFailure>,
) -> Result<(VerifiedStep, T), TrialFailure> {
    let start = ticket.interval().start();
    let gap = passed.0.gap(problem);
    if !gap.crossed() {
        return Ok(passed);
    }
    let floor = problem.identity.resolution().minimum_interval_ns();
    let identity = problem.identity.digest();
    let base = problem.chance.gap(problem.body, problem.accepted.state());
    let mut bracket = Bracket::new(base, ticket.length_ns(), gap, floor);
    let mut top = passed;
    while !bracket.closed() {
        let length = bracket.aim();
        let end = start.plus_ns(length).ok_or(ticket.mismatch())?;
        let mut trial = ticket.clone();
        trial.interval = Interval::new(start, end).ok_or(ticket.mismatch())?;
        trial.digest = trial.sealed(identity);
        // A refusal leaves through `?` and drops the bracket: a shorter span is a new attempt, and a shorter
        // ticket can be the refused one, because a stage's error is not monotone in its length.
        let step = solved(&trial)?;
        let gap = step.0.gap(problem);
        bracket.note(length, gap);
        if gap.crossed() {
            top = step;
        }
    }
    Ok(top)
}

const WATER_PAYLOAD: u64 = 0;
/// An id, a stamp, a tag, a name's length, an amount and an end.
const DELIVERY_BYTES: usize = 32 + 8 + 8 + 8 + 8 + 8;

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
