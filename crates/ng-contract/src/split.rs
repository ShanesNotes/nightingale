//! A split is trusted only on measurement. `SplitMeasurement::qualify` is the only producer of a
//! `SplitQualification`, over the four-family metric (state, exchange integral, observable, event time;
//! CB numerics.rs 122-260) with the reference's own self-convergence error included. A `SplitTrace` comes
//! only from a draw over accepted heads, each step read through the extents `verify` made for it.
use crate::body::RowKind;
use crate::identity::{Canonical, Seal};
use crate::{
    AcceptedStateView, AdmissionError, Compensated, CompiledBody, Digest, Dimensionless, ErrorScales, Fired,
    InputId, Measure, ModelHash, ObservationLens, PacketHash, RowId, RunIdentity, SimTime, StepTicket,
};
use core::marker::PhantomData;
use core::num::NonZeroU64;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

mod sealed {
    pub trait MetricFamily {}
}
pub trait MetricFamily: sealed::MetricFamily {}
pub enum StateFamily {}
pub enum ExchangeFamily {}
pub enum ObservableFamily {}
pub enum EventTimeFamily {}
impl sealed::MetricFamily for StateFamily {}
impl MetricFamily for StateFamily {}
impl sealed::MetricFamily for ExchangeFamily {}
impl MetricFamily for ExchangeFamily {}
impl sealed::MetricFamily for ObservableFamily {}
impl MetricFamily for ObservableFamily {}
impl sealed::MetricFamily for EventTimeFamily {}
impl MetricFamily for EventTimeFamily {}

/// Instants `start + k * period` for `k` below `samples`; both traces of a comparison use the same one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceGrid {
    start: SimTime,
    period_ns: NonZeroU64,
    samples: u32,
}
impl TraceGrid {
    /// Refuses fewer than two instants and a last instant past the clock.
    pub fn new(start: SimTime, period_ns: NonZeroU64, samples: u32) -> Result<Self, AdmissionError> {
        let span = (period_ns.get()).checked_mul(u64::from(samples).saturating_sub(1));
        match span.and_then(|span| start.plus_ns(span)) {
            Some(_) if samples >= 2 => Ok(Self {
                start,
                period_ns,
                samples,
            }),
            _ => Err(AdmissionError::EmptyTraceGrid),
        }
    }
    pub fn end(&self) -> SimTime {
        self.instant(self.samples - 1)
    }
    fn instant(&self, index: u32) -> SimTime {
        SimTime::from_ns(self.start.nanoseconds() + self.period_ns.get() * u64::from(index))
    }
}
impl Seal for TraceGrid {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            start,
            period_ns,
            samples,
        } = self;
        start.seal(out);
        out.number(period_ns.get());
        out.number(u64::from(*samples));
    }
}

/// One family's keys and values. A grid family holds one value per key at each instant, instant by
/// instant; the event family one firing time per key, in seconds.
pub struct FamilySamples<F: MetricFamily> {
    keys: Vec<Digest>,
    values: Vec<Compensated>,
    family: PhantomData<F>,
}
impl<F: MetricFamily> FamilySamples<F> {
    fn new(keys: Vec<Digest>, values: Vec<Compensated>) -> Self {
        Self {
            keys,
            values,
            family: PhantomData,
        }
    }
}
impl<F: MetricFamily> Seal for FamilySamples<F> {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            keys,
            values,
            family: _,
        } = self;
        keys.seal(out);
        values.seal(out);
    }
}

/// One verified step's gross extents per column and the ticket they were made for. Only `verify` makes one,
/// so a trace's exchanges are a solve's own.
#[derive(Debug)]
pub struct StepExtents {
    pub(crate) ticket: StepTicket,
    pub(crate) forward: Vec<Compensated>,
    pub(crate) reverse: Vec<Compensated>,
}

/// One run's four families on a grid, bound to its identity, its lens and the inputs it applied inside the
/// grid. There is no constructor from samples: `TraceDraw::finish` makes one from accepted heads.
pub struct SplitTrace {
    identity: RunIdentity,
    grid: TraceGrid,
    lens: Digest,
    schedule: Vec<(SimTime, InputId)>,
    state: FamilySamples<StateFamily>,
    exchange: FamilySamples<ExchangeFamily>,
    observable: FamilySamples<ObservableFamily>,
    event_time: FamilySamples<EventTimeFamily>,
}
impl SplitTrace {
    pub fn digest(&self) -> Digest {
        let Self {
            identity,
            grid,
            lens,
            schedule,
            state,
            exchange,
            observable,
            event_time,
        } = self;
        let mut out = Canonical::new("ng-contract split trace 1");
        identity.digest().seal(&mut out);
        grid.seal(&mut out);
        lens.seal(&mut out);
        seal_schedule(&mut out, schedule);
        state.seal(&mut out);
        exchange.seal(&mut out);
        observable.seal(&mut out);
        event_time.seal(&mut out);
        out.digest()
    }
    fn families(&self) -> [(&[Digest], &[Compensated]); 4] {
        [
            (&self.state.keys, &self.state.values),
            (&self.exchange.keys, &self.exchange.values),
            (&self.observable.keys, &self.observable.values),
            (&self.event_time.keys, &self.event_time.values),
        ]
    }
    /// The same patient, seed, grid, lens, inputs and events under another resolution.
    fn comparable(&self, other: &Self) -> Result<(), SplitRejected> {
        if !self.identity.same_course(&other.identity) {
            return Err(SplitRejected::OtherCourse);
        }
        if self.identity.resolution().digest() == other.identity.resolution().digest() {
            return Err(SplitRejected::SameRun);
        }
        if (self.grid, self.lens) != (other.grid, other.lens) {
            return Err(SplitRejected::OtherGrid);
        }
        if self.schedule != other.schedule {
            return Err(SplitRejected::OtherSchedule);
        }
        let keyed = (self.families().into_iter())
            .zip(other.families())
            .all(|((first, _), (second, _))| first == second);
        if !keyed {
            return Err(SplitRejected::OtherEvents);
        }
        Ok(())
    }
}

fn seal_schedule(out: &mut Canonical, schedule: &[(SimTime, InputId)]) {
    out.length(schedule.len());
    for (at, input) in schedule {
        at.seal(out);
        input.seal(out);
    }
}

struct Opened {
    identity: RunIdentity,
    run: Digest,
    head: SimTime,
    seal: Digest,
    net: Vec<Compensated>,
    origin: Option<Vec<Compensated>>,
    filled: u32,
    state: Vec<Compensated>,
    exchange: Vec<Compensated>,
    observable: Vec<Compensated>,
    events: Vec<(Digest, Compensated)>,
    schedule: Vec<(SimTime, InputId)>,
}

struct Layout {
    grid: TraceGrid,
    rows: Vec<RowId>,
    columns: usize,
    channels: Vec<RowId>,
}
impl Layout {
    fn open(&self, identity: &RunIdentity, head: AcceptedStateView<'_>, seal: Digest) -> Result<Opened, SplitRejected> {
        let time = head.time();
        if self.grid.start < time {
            return Err(SplitRejected::PastStart);
        }
        let mut opened = Opened {
            identity: identity.clone(),
            run: identity.digest(),
            head: time,
            seal,
            net: vec![Compensated::ZERO; self.columns],
            origin: None,
            filled: 0,
            state: Vec::new(),
            exchange: Vec::new(),
            observable: Vec::new(),
            events: Vec::new(),
            schedule: Vec::new(),
        };
        if self.grid.start == time {
            let net = opened.net.clone();
            self.sample(&mut opened, net, |row| head.value(row));
        }
        Ok(opened)
    }
    /// Checks everything before it changes anything, so a refused head leaves the draw as it was.
    fn advance(
        &self,
        opened: &mut Opened,
        identity: &RunIdentity,
        head: AcceptedStateView<'_>,
        seal: Digest,
        (extents, fired): (&StepExtents, &[Fired]),
    ) -> Result<(), SplitRejected> {
        let ticket = &extents.ticket;
        let run = opened.run;
        if identity.digest() != run || ticket.sealed(run) != ticket.digest() {
            return Err(SplitRejected::OtherRun);
        }
        let interval = ticket.interval();
        let follows = ticket.base == opened.seal
            && interval.start() == opened.head
            && interval.end() == head.time()
            && head.last_segment().interval() == Some(interval);
        if !follows {
            return Err(SplitRejected::Gap);
        }
        let end: Vec<Compensated> = (opened.net.iter().zip(&extents.forward).zip(&extents.reverse))
            .map(|((net, forward), reverse)| net.plus(*forward).minus(*reverse))
            .collect();
        let (first, last) = (self.grid.start, self.grid.end());
        if first <= interval.start() && interval.start() < last {
            opened.schedule.extend(ticket.due.iter().map(|input| (interval.start(), *input)));
        }
        for firing in fired.iter().filter(|firing| first <= firing.at() && firing.at() <= last) {
            let mut key = Canonical::new("ng-contract split event 1");
            firing.stream().seal(&mut key);
            key.number(firing.ordinal());
            let seconds = firing.at().nanoseconds() as f64 / NANOSECONDS_PER_SECOND;
            opened.events.push((key.digest(), Compensated::exact(seconds)));
        }
        while opened.filled < self.grid.samples {
            let at = self.grid.instant(opened.filled);
            if head.time() < at {
                break;
            }
            let reached = at.nanoseconds() - interval.start().nanoseconds();
            let net = (opened.net.iter().zip(&end))
                .map(|(from, to)| Compensated::between(*from, *to, reached, interval.length_ns()))
                .collect();
            if at == head.time() {
                self.sample(opened, net, |row| head.value(row));
            } else {
                let segment = head.last_segment();
                self.sample(opened, net, |row| {
                    segment.sample(row, at).expect("a grid instant inside the step lies on its chord")
                });
            }
        }
        opened.net = end;
        opened.head = head.time();
        opened.seal = seal;
        Ok(())
    }
    fn sample(&self, opened: &mut Opened, net: Vec<Compensated>, value: impl Fn(RowId) -> Compensated) {
        let origin = opened.origin.get_or_insert_with(|| net.clone());
        let exchanges = net.iter().zip(origin.iter()).map(|(at, from)| at.minus(*from));
        opened.exchange.extend(exchanges);
        opened.state.extend(self.rows.iter().map(|row| value(*row)));
        opened.observable.extend(self.channels.iter().map(|row| value(*row)));
        opened.filled += 1;
    }
}

/// Reads one course's accepted heads in order and keeps the grid's samples. `PatientRecord::split_trace`
/// feeds it: the head a draw opens on, then each accepted step with the extents and firings `verify` made.
/// An instant between two heads reads the step's chord; one at a head reads what the head holds, after any
/// firing there.
pub struct TraceDraw {
    lens: Digest,
    model: ModelHash,
    layout: Layout,
    opened: Option<Opened>,
}
impl TraceDraw {
    /// The state family is every amount and memory row, the exchange family every column's net extent since
    /// the grid's first instant and the observable family every channel of `lens`, so a body with no column or
    /// a lens with no channel is refused.
    pub fn new(body: &CompiledBody, lens: &ObservationLens, grid: TraceGrid) -> Result<Self, SplitRejected> {
        if lens.model() != body.model_hash() {
            return Err(SplitRejected::OtherBody);
        }
        let rows: Vec<RowId> = (body.rows().iter())
            .filter(|row| !matches!(row.kind(), RowKind::Algebraic(_)))
            .map(|row| row.id())
            .collect();
        let channels: Vec<RowId> = lens.channels().iter().map(|channel| channel.row()).collect();
        if rows.is_empty() || body.columns().is_empty() || channels.is_empty() {
            return Err(SplitRejected::MissingFamily);
        }
        Ok(Self {
            lens: lens.digest(),
            model: body.model_hash(),
            layout: Layout {
                grid,
                rows,
                columns: body.columns().len(),
                channels,
            },
            opened: None,
        })
    }
    /// A head with no step opens the draw and must not be past the grid's first instant; every later head comes
    /// with the step that made it, from the head read last, of the same run.
    pub fn read(
        &mut self,
        identity: &RunIdentity,
        head: AcceptedStateView<'_>,
        seal: Digest,
        step: Option<(&StepExtents, &[Fired])>,
    ) -> Result<(), SplitRejected> {
        if identity.model() != self.model {
            return Err(SplitRejected::OtherBody);
        }
        match (&mut self.opened, step) {
            (None, None) => {
                self.opened = Some(self.layout.open(identity, head, seal)?);
                Ok(())
            }
            (Some(opened), Some(step)) => self.layout.advance(opened, identity, head, seal, step),
            _ => Err(SplitRejected::Gap),
        }
    }
    /// Refuses a draw that has not reached the grid's last instant.
    pub fn finish(self) -> Result<SplitTrace, SplitRejected> {
        let Self {
            lens,
            model: _,
            layout,
            opened,
        } = self;
        let Some(opened) = opened.filter(|opened| opened.filled == layout.grid.samples) else {
            return Err(SplitRejected::Unfinished);
        };
        let Opened {
            identity,
            mut events,
            schedule,
            state,
            exchange,
            observable,
            ..
        } = opened;
        let keyed = |tag: &str, count: usize| -> Vec<Digest> {
            (0..count)
                .map(|index| {
                    let mut key = Canonical::new(tag);
                    key.length(index);
                    key.digest()
                })
                .collect()
        };
        events.sort_by_key(|(key, _)| *key.as_bytes());
        let (event_keys, times) = events.into_iter().unzip();
        let Layout {
            grid,
            rows,
            columns,
            channels,
        } = layout;
        Ok(SplitTrace {
            identity,
            grid,
            lens,
            schedule,
            state: FamilySamples::new(keyed("ng-contract split state 1", rows.len()), state),
            exchange: FamilySamples::new(keyed("ng-contract split exchange 1", columns), exchange),
            observable: FamilySamples::new(keyed("ng-contract split observable 1", channels.len()), observable),
            event_time: FamilySamples::new(event_keys, times),
        })
    }
}

/// The coupled reference's own error, measured against a refined run of it on the same grid; never assumed
/// zero. It keeps the digest of the reference trace it was measured on, so an easier run's small error cannot
/// stand in for this reference's.
pub struct ReferenceError {
    reference: Digest,
    differences: [Vec<Compensated>; 4],
}
impl ReferenceError {
    pub fn self_convergence(reference: &SplitTrace, refined: &SplitTrace) -> Result<Self, SplitRejected> {
        reference.comparable(refined)?;
        let (coarse, fine) = (reference.families(), refined.families());
        Ok(Self {
            reference: reference.digest(),
            differences: core::array::from_fn(|family| {
                (coarse[family].1.iter().zip(fine[family].1))
                    .map(|(coarse, fine)| coarse.minus(*fine))
                    .collect()
            }),
        })
    }
}

/// One error scale per family, in each key's own unit (seconds for event times), and the share of the budget
/// the reference's own error may take.
pub struct SplitBudget {
    scales: [ErrorScales; 4],
    reference_fraction: Measure<Dimensionless>,
}
impl SplitBudget {
    /// Refuses a reference share outside zero to one, open at both ends.
    pub fn new(
        state: ErrorScales,
        exchange: ErrorScales,
        observable: ErrorScales,
        event_time: ErrorScales,
        reference_fraction: Measure<Dimensionless>,
    ) -> Result<Self, AdmissionError> {
        let fraction = reference_fraction.value();
        if !(0.0 < fraction && fraction < 1.0) {
            return Err(AdmissionError::ReferenceShare);
        }
        Ok(Self {
            scales: [state, exchange, observable, event_time],
            reference_fraction,
        })
    }
}

/// A split run's trace against the coupled reference's on one grid, with the reference's own error.
pub struct SplitMeasurement {
    reference: SplitTrace,
    split: SplitTrace,
    reference_error: ReferenceError,
    budget: SplitBudget,
}
impl SplitMeasurement {
    /// Refuses traces of another patient, seed, grid, lens, input schedule or event set, a trace compared with
    /// its own run, and a reference error not measured on `reference`.
    pub fn new(
        reference: SplitTrace,
        split: SplitTrace,
        reference_error: ReferenceError,
        budget: SplitBudget,
    ) -> Result<Self, SplitRejected> {
        if reference_error.reference != reference.digest() {
            return Err(SplitRejected::OtherReference);
        }
        reference.comparable(&split)?;
        Ok(Self {
            reference,
            split,
            reference_error,
            budget,
        })
    }
    /// The only producer of `SplitQualification`: each family's worst normalized error plus the reference's
    /// own must fit the budget, and the reference's own must fit its share. Each error is normalized by its
    /// family's scales at the reference's value.
    pub fn qualify(self) -> Result<SplitQualification, SplitRejected> {
        let Self {
            reference,
            split,
            reference_error,
            budget,
        } = self;
        let mut maxima = [0.0; 4];
        let mut own = 0.0;
        let families = (reference.families().into_iter())
            .zip(split.families())
            .zip(&reference_error.differences)
            .zip(&budget.scales);
        for (family, ((((_, expected), (_, measured)), differences), scales)) in families.enumerate() {
            for ((expected, measured), difference) in expected.iter().zip(measured).zip(differences) {
                let scale = scales.at(expected.rounded(), 1.0);
                let error = normalized(measured.minus(*expected), scale)?;
                if maxima[family] < error {
                    maxima[family] = error;
                }
                let error = normalized(*difference, scale)?;
                if own < error {
                    own = error;
                }
            }
        }
        let worst = (maxima.iter()).fold(0.0, |worst, &maximum| if worst < maximum { maximum } else { worst });
        let assessment = SplitAssessment {
            maxima: maxima.map(Compensated::exact),
            reference_error: Compensated::exact(own),
            worst_total: Compensated::exact(worst + own),
        };
        if !assessment.worst_total.is_finite() {
            return Err(SplitRejected::NotFinite);
        }
        if budget.reference_fraction.value() < own {
            return Err(SplitRejected::ReferenceTooCoarse(assessment));
        }
        if 1.0 < worst + own {
            return Err(SplitRejected::OverBudget(assessment));
        }
        let mut domain = Canonical::new("ng-contract split domain 1");
        reference.grid.seal(&mut domain);
        reference.lens.seal(&mut domain);
        seal_schedule(&mut domain, &reference.schedule);
        Ok(SplitQualification {
            plan: split.identity.resolution().digest(),
            model: split.identity.model(),
            packet: split.identity.packet(),
            domain: domain.digest(),
            assessment,
        })
    }
}

fn normalized(difference: Compensated, scale: f64) -> Result<f64, SplitRejected> {
    let error = difference.abs().rounded() / scale;
    if error.is_finite() { Ok(error) } else { Err(SplitRejected::NotFinite) }
}

/// Each family's worst normalized error in family order (state, exchange integral, observable, event time),
/// the reference's own, and the worst family plus the reference's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitAssessment {
    maxima: [Compensated; 4],
    reference_error: Compensated,
    worst_total: Compensated,
}
impl SplitAssessment {
    pub fn maxima(&self) -> [Compensated; 4] {
        self.maxima
    }
    pub fn reference_error(&self) -> Compensated {
        self.reference_error
    }
    pub fn worst_total(&self) -> Compensated {
        self.worst_total
    }
}

/// Bound to the measured plan (the split run's resolution), model, packet and validated domain (grid, lens and
/// inputs).
#[derive(Debug)]
pub struct SplitQualification {
    plan: Digest,
    model: ModelHash,
    packet: PacketHash,
    domain: Digest,
    assessment: SplitAssessment,
}
impl SplitQualification {
    pub fn plan(&self) -> Digest {
        self.plan
    }
    pub fn model(&self) -> ModelHash {
        self.model
    }
    pub fn packet(&self) -> PacketHash {
        self.packet
    }
    pub fn domain(&self) -> Digest {
        self.domain
    }
    pub fn assessment(&self) -> &SplitAssessment {
        &self.assessment
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitRejected {
    /// A lens or a run of another body.
    OtherBody,
    /// A body with no column or a lens with no channel, so a family would be empty.
    MissingFamily,
    /// The head a draw opens on is past the grid's first instant.
    PastStart,
    /// A head that does not follow the last one read, or that comes without its step.
    Gap,
    /// A head or a step of another run.
    OtherRun,
    /// A receipt of another head than the record's own.
    OtherHead,
    /// A draw that has not reached the grid's last instant.
    Unfinished,
    /// Another patient, packet or seed.
    OtherCourse,
    /// A run compared with itself.
    SameRun,
    OtherGrid,
    OtherSchedule,
    /// The two runs fired different events.
    OtherEvents,
    /// A reference error measured on another reference trace.
    OtherReference,
    NotFinite,
    ReferenceTooCoarse(SplitAssessment),
    OverBudget(SplitAssessment),
}

/// Which blocks a stepper evaluates apart. A split plan holds the qualification its measurement made.
#[derive(Clone, Copy)]
pub enum EvaluationPlan<'a> {
    Coupled,
    QualifiedSplit(&'a SplitQualification),
    /// Only in a run whose declared resolution is a split measurement; such runs feed `SplitMeasurement` and
    /// are never scored by the arena.
    UnderMeasurement,
}
