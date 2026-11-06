use crate::body::RowKind;
use crate::identity::{Canonical, Seal};
use crate::{
    AcceptedStateView, AdmissionError, BoundParameters, Compensated, CompiledBody, Digest, Dimensionless,
    ErrorScales, Fired, ImageParts, InputId, Measure, ModelHash, ObservationLens, PacketHash, RecordHash,
    RowId, RunIdentity, SimTime, Time,
};
use core::marker::PhantomData;
use core::num::NonZeroU64;

const NANOSECONDS_PER_SECOND: f64 = 1e9;

mod sealed {
    pub trait MetricFamily {
        const INDEX: usize;
    }
}
pub trait MetricFamily: sealed::MetricFamily {}
pub enum StateFamily {}
pub enum ExchangeFamily {}
pub enum ObservableFamily {}
pub enum EventTimeFamily {}
impl sealed::MetricFamily for StateFamily {
    const INDEX: usize = 0;
}
impl MetricFamily for StateFamily {}
impl sealed::MetricFamily for ExchangeFamily {
    const INDEX: usize = 1;
}
impl MetricFamily for ExchangeFamily {}
impl sealed::MetricFamily for ObservableFamily {
    const INDEX: usize = 2;
}
impl MetricFamily for ObservableFamily {}
impl sealed::MetricFamily for EventTimeFamily {
    const INDEX: usize = 3;
}
impl MetricFamily for EventTimeFamily {}

/// Instants `start + k * period` for `k` below `samples`.
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
            _ => Err(AdmissionError::TraceGridOutOfRange),
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

/// One family's keys and values: a grid family's are instant by instant, and the event family's are one firing
/// time per key, in seconds.
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
    fn parts(&self) -> (&[Digest], &[Compensated]) {
        (&self.keys, &self.values)
    }
}

fn indexed(tag: &str, count: usize) -> Vec<Digest> {
    (0..count)
        .map(|index| {
            let mut key = Canonical::new(tag);
            key.length(index);
            key.digest()
        })
        .collect()
}
impl<F: MetricFamily> Seal for FamilySamples<F> {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            keys,
            values,
            family: PhantomData,
        } = self;
        keys.seal(out);
        values.seal(out);
    }
}

/// Only `verify` makes one, so a trace's exchanges and events are a solve's own, and none can be left out.
#[derive(Debug)]
pub struct StepExtents {
    pub(crate) base: Digest,
    pub(crate) applied: Vec<InputId>,
    pub(crate) forward: Vec<Compensated>,
    pub(crate) reverse: Vec<Compensated>,
    pub(crate) fired: Vec<Fired>,
}
impl StepExtents {
    /// Each input this step applied, once, in `(time, id)` order.
    pub fn applied(&self) -> &[InputId] {
        &self.applied
    }
    pub fn fired(&self) -> &[Fired] {
        &self.fired
    }
}

/// The draw recomputes this head's seal from its image and the given revision, seal before and ledger digest, and
/// keeps it only once the next step's extents name that seal. The bound values a draw reads at admission reach the
/// seal only through the ledger digest given here.
pub struct TraceHead<'r> {
    parts: &'r ImageParts,
    revision: u64,
    previous: Digest,
    ledger: Digest,
    hash: RecordHash,
}
impl<'r> TraceHead<'r> {
    pub fn new(
        parts: &'r ImageParts,
        revision: u64,
        previous: Digest,
        ledger: Digest,
        hash: RecordHash,
    ) -> Self {
        Self {
            parts,
            revision,
            previous,
            ledger,
            hash,
        }
    }
    fn view(&self) -> AcceptedStateView<'r> {
        let parts = self.parts;
        AcceptedStateView::new(parts.time, &parts.state, &parts.continuation, &parts.dense)
    }
    fn seal(&self) -> Digest {
        let parts = self.parts;
        let applied = parts.pending.applied();
        let image = (self.hash).image(
            parts.time,
            &parts.state,
            &parts.continuation,
            &parts.dense,
            applied,
        );
        self.hash.link(self.previous, self.revision, image, self.ledger)
    }
}

pub struct SplitTrace {
    identity: RunIdentity,
    patient: Digest,
    grid: TraceGrid,
    lens: Digest,
    inputs: Digest,
    state: FamilySamples<StateFamily>,
    exchange: FamilySamples<ExchangeFamily>,
    observable: FamilySamples<ObservableFamily>,
    event_time: FamilySamples<EventTimeFamily>,
}
impl SplitTrace {
    pub fn digest(&self) -> Digest {
        let Self {
            identity,
            patient,
            grid,
            lens,
            inputs,
            state,
            exchange,
            observable,
            event_time,
        } = self;
        let mut out = Canonical::new("ng-contract split trace 1");
        identity.digest().seal(&mut out);
        patient.seal(&mut out);
        grid.seal(&mut out);
        lens.seal(&mut out);
        inputs.seal(&mut out);
        state.seal(&mut out);
        exchange.seal(&mut out);
        observable.seal(&mut out);
        event_time.seal(&mut out);
        out.digest()
    }
    fn families(&self) -> [(&[Digest], &[Compensated]); 4] {
        [
            self.state.parts(),
            self.exchange.parts(),
            self.observable.parts(),
            self.event_time.parts(),
        ]
    }
    fn comparable(&self, other: &Self) -> Result<(), SplitRejected> {
        if self.patient != other.patient || !self.identity.same_seed(&other.identity) {
            return Err(SplitRejected::OtherCourse);
        }
        if self.identity.resolution().digest() == other.identity.resolution().digest() {
            return Err(SplitRejected::SameRun);
        }
        if (self.grid, self.lens) != (other.grid, other.lens) {
            return Err(SplitRejected::OtherGrid);
        }
        if self.inputs != other.inputs {
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

/// The bound patient whatever core runs it. The packet hash also covers the core's method history at admission, so
/// two cores' runs of one patient never share it.
fn patient(parameters: &BoundParameters) -> Digest {
    let mut out = Canonical::new("ng-contract split patient 1");
    parameters.model.digest().seal(&mut out);
    parameters.values.seal(&mut out);
    parameters.starts.seal(&mut out);
    parameters.sizes.seal(&mut out);
    parameters.modes.seal(&mut out);
    out.digest()
}

struct Opened {
    identity: RunIdentity,
    patient: Digest,
    head: SimTime,
    unconfirmed: Digest,
    net: Vec<Compensated>,
    filled: u32,
    confirmed: u32,
    inputs: Digest,
    state: Vec<Compensated>,
    exchange: Vec<Compensated>,
    observable: Vec<Compensated>,
    events: Vec<(Digest, Compensated)>,
}

struct Layout {
    grid: TraceGrid,
    rows: Vec<RowId>,
    columns: usize,
    channels: Vec<RowId>,
}
impl Layout {
    /// The run's first seal chains its identity, so a draw opened at admission is bound to that run.
    fn open(&self, identity: &RunIdentity, head: &TraceHead<'_>) -> Result<Opened, SplitRejected> {
        if head.revision != 0 || head.previous != identity.digest() {
            return Err(SplitRejected::NotAdmission);
        }
        let view = head.view();
        let time = view.time();
        if self.grid.start < time {
            return Err(SplitRejected::PastStart);
        }
        let mut opened = Opened {
            identity: identity.clone(),
            patient: patient(head.parts.parameters()),
            head: time,
            unconfirmed: head.seal(),
            net: vec![Compensated::ZERO; self.columns],
            filled: 0,
            confirmed: 0,
            inputs: head.parts.pending.applied().chain(),
            state: Vec::new(),
            exchange: Vec::new(),
            observable: Vec::new(),
            events: Vec::new(),
        };
        if self.grid.start == time {
            let net = opened.net.clone();
            self.sample(&mut opened, net, |row| view.value(row));
        }
        Ok(opened)
    }
    /// A refused head leaves the draw as it was, so every check comes first. The step's extents name the seal of
    /// the head read before, which confirms that head's samples.
    fn advance(
        &self,
        opened: &mut Opened,
        head: &TraceHead<'_>,
        extents: &StepExtents,
    ) -> Result<(), SplitRejected> {
        let view = head.view();
        let follows = extents.base == opened.unconfirmed;
        let interval = (view.last_segment().interval()).filter(|interval| interval.start() == opened.head);
        let Some(interval) = interval.filter(|_| follows) else {
            return Err(SplitRejected::Gap);
        };
        opened.confirmed = opened.filled;
        let end: Vec<Compensated> = (opened.net.iter().zip(&extents.forward).zip(&extents.reverse))
            .map(|((net, forward), reverse)| net.plus(*forward).minus(*reverse))
            .collect();
        let (first, last) = (self.grid.start, self.grid.end());
        for firing in (extents.fired.iter()).filter(|firing| first < firing.at() && firing.at() <= last) {
            let mut key = Canonical::new("ng-contract split event 1");
            firing.stream().seal(&mut key);
            key.number(firing.ordinal());
            let seconds = firing.at().nanoseconds() as f64 / NANOSECONDS_PER_SECOND;
            opened.events.push((key.digest(), Compensated::exact(seconds)));
        }
        let before = opened.filled;
        while opened.filled < self.grid.samples {
            let at = self.grid.instant(opened.filled);
            if view.time() < at {
                break;
            }
            let reached = at.nanoseconds() - interval.start().nanoseconds();
            let net = (opened.net.iter().zip(&end))
                .map(|(from, to)| Compensated::between(*from, *to, reached, interval.length_ns()))
                .collect();
            if at == view.time() {
                self.sample(opened, net, |row| view.value(row));
            } else {
                let segment = view.last_segment();
                self.sample(opened, net, |row| {
                    segment
                        .sample(row, at)
                        .expect("a grid instant inside the step lies on its chord")
                });
            }
        }
        if opened.filled > before {
            opened.inputs = head.parts.pending.applied().chain();
        }
        opened.net = end;
        opened.head = view.time();
        opened.unconfirmed = head.seal();
        Ok(())
    }
    fn confirmed(&self, opened: &Opened) -> bool {
        opened.confirmed == self.grid.samples
    }
    fn sample(&self, opened: &mut Opened, net: Vec<Compensated>, value: impl Fn(RowId) -> Compensated) {
        opened.exchange.extend(net);
        opened.state.extend(self.rows.iter().map(|row| value(*row)));
        opened
            .observable
            .extend(self.channels.iter().map(|row| value(*row)));
        opened.filled += 1;
    }
}

/// It recomputes each head's seal, and keeps a head's samples only once the next step's extents, which only
/// `verify` makes, name that seal. It cannot tell which step made a head: `PatientRecord::split_trace` pairs each
/// accepted head with its own step's receipt. An instant between two heads reads the step's chord; one at a head
/// reads what the head holds, after any firing there.
pub struct TraceDraw {
    lens: Digest,
    model: ModelHash,
    layout: Layout,
    opened: Option<Opened>,
}
impl TraceDraw {
    /// The state family is every amount and memory row, the exchange family every column's net extent since
    /// admission and the observable family every channel of `lens`. Refuses a lens of another body and a family
    /// that would be empty.
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
    /// Opens the draw, once, on a head that names itself the run's admission and is not past the grid's first
    /// instant. The first read proves the claim.
    pub fn open(&mut self, identity: &RunIdentity, head: TraceHead<'_>) -> Result<(), SplitRejected> {
        if identity.model() != self.model {
            return Err(SplitRejected::OtherBody);
        }
        if self.opened.is_some() {
            return Err(SplitRejected::Gap);
        }
        self.opened = Some(self.layout.open(identity, &head)?);
        Ok(())
    }
    /// Reads a head with the extents of the step before it, which must name the seal of the head read last.
    pub fn read(&mut self, head: TraceHead<'_>, step: &StepExtents) -> Result<(), SplitRejected> {
        let Some(opened) = &mut self.opened else {
            return Err(SplitRejected::Gap);
        };
        self.layout.advance(opened, &head, step)
    }
    /// Every instant is drawn and its head confirmed by the step after it.
    pub fn done(&self) -> bool {
        (self.opened.as_ref()).is_some_and(|opened| self.layout.confirmed(opened))
    }
    /// Refuses a draw that is not done.
    pub fn finish(self) -> Result<SplitTrace, SplitRejected> {
        let Self {
            lens,
            model: _,
            layout,
            opened,
        } = self;
        let Some(opened) = opened.filter(|opened| layout.confirmed(opened)) else {
            return Err(SplitRejected::Unfinished);
        };
        let Opened {
            identity,
            patient,
            inputs,
            mut events,
            state,
            exchange,
            observable,
            ..
        } = opened;
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
            patient,
            grid,
            lens,
            inputs,
            state: FamilySamples::new(indexed("ng-contract split state 1", rows.len()), state),
            exchange: FamilySamples::new(indexed("ng-contract split exchange 1", columns), exchange),
            observable: FamilySamples::new(
                indexed("ng-contract split observable 1", channels.len()),
                observable,
            ),
            event_time: FamilySamples::new(event_keys, times),
        })
    }
}

/// The coupled reference's own error, measured against a second run of it on the same grid and never assumed
/// zero.
pub struct ReferenceError {
    reference: Digest,
    differences: [Vec<Compensated>; 4],
}
impl ReferenceError {
    pub fn self_convergence(reference: &SplitTrace, refined: &SplitTrace) -> Result<Self, SplitRejected> {
        reference.comparable(refined)?;
        let (coarse, fine) = (reference.families(), refined.families());
        let differences: [Vec<Compensated>; 4] = core::array::from_fn(|family| {
            (coarse[family].1.iter().zip(fine[family].1))
                .map(|(coarse, fine)| coarse.minus(*fine))
                .collect()
        });
        if differences
            .iter()
            .flatten()
            .all(|difference| difference.abs().at_most(Compensated::ZERO))
        {
            return Err(SplitRejected::NotRefined);
        }
        Ok(Self {
            reference: reference.digest(),
            differences,
        })
    }
}

/// A grid family's scales are in each key's own unit.
pub struct SplitBudget {
    scales: [ErrorScales; 4],
    reference_fraction: Measure<Dimensionless>,
}
impl SplitBudget {
    /// Refuses an event time scale that is not positive and a reference share outside zero to one, open at both
    /// ends.
    pub fn new(
        state: ErrorScales,
        exchange: ErrorScales,
        observable: ErrorScales,
        event_time: Measure<Time>,
        reference_fraction: Measure<Dimensionless>,
    ) -> Result<Self, AdmissionError> {
        let event_time = ErrorScales::uniform(Measure::new(event_time.value())?, Measure::new(0.0)?)?;
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
    /// Each family's worst normalized error plus the reference's own must fit the budget, and the reference's
    /// own must fit its share. Each error is normalized by its family's scales at the reference's value.
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
        let worst = (maxima.iter()).fold(
            0.0,
            |worst, &maximum| if worst < maximum { maximum } else { worst },
        );
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
        reference.inputs.seal(&mut domain);
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
    if error.is_finite() {
        Ok(error)
    } else {
        Err(SplitRejected::NotFinite)
    }
}

/// The worst total is the worst family's error plus the reference's own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitAssessment {
    maxima: [Compensated; 4],
    reference_error: Compensated,
    worst_total: Compensated,
}
impl SplitAssessment {
    pub fn maximum<F: MetricFamily>(&self) -> Compensated {
        self.maxima[F::INDEX]
    }
    pub fn reference_error(&self) -> Compensated {
        self.reference_error
    }
    pub fn worst_total(&self) -> Compensated {
        self.worst_total
    }
}

/// Bound to the measured plan (the split run's resolution), model, packet and validated domain (grid, lens and
/// the inputs applied through the grid).
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
    OtherBody,
    MissingFamily,
    NotAdmission,
    PastStart,
    /// A head that does not follow the head read last, a step that does not name its seal, a read before the draw
    /// opened, or a second opening.
    Gap,
    OtherHead,
    Unfinished,
    /// Another patient or seed.
    OtherCourse,
    SameRun,
    OtherGrid,
    OtherSchedule,
    OtherEvents,
    OtherReference,
    NotRefined,
    NotFinite,
    ReferenceTooCoarse(SplitAssessment),
    OverBudget(SplitAssessment),
}

/// Which blocks a stepper evaluates apart.
#[derive(Clone, Copy)]
pub enum EvaluationPlan<'a> {
    Coupled,
    QualifiedSplit(&'a SplitQualification),
}
