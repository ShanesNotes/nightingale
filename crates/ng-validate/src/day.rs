mod judge;
mod table;
mod trace;

#[cfg(test)]
mod tests;

use crate::initialize::{admit_start, solved_start};
use crate::run::accept;
use core::fmt;
use core::num::NonZeroU32;
use judge::{Shown, Uncovered};
use ng_contract::{
    AdmissionError, BoundaryEvent, BoundaryInput, BoundaryPort, CompiledBody, Context, CoupledSolver,
    Delivered, Digest, ErrorScales, InitializationFailure, InputId, Interval, Legs, Measure, ObservationLens,
    ParameterPacket, Posture, RowKind, RunIdentity, Seed, SimTime, SyntheticFixture,
};
use ng_record::{AuthorizedInput, Checkpoint, CheckpointError, FaultReport, Faulted, HistoryStore, Session};
use table::{Bound, Course, Lean, Row, Scoring, Span};
use trace::{DayTrace, Fold, Reading, Readings};

const SECOND: u64 = 1_000_000_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const MEAL_WINDOW: u64 = 4 * HOUR;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventKind {
    SitUp,
    Meal,
    LieDownToSleep,
    Wake,
}

#[derive(Clone, Copy, Debug)]
enum Nutrient {
    Water,
    Sodium,
    Potassium,
    Chloride,
}
impl Nutrient {
    /// Synthetic fixture amounts.
    fn delivered(self, ends: SimTime) -> Result<BoundaryInput, AdmissionError> {
        Ok(match self {
            Nutrient::Water => BoundaryInput::Water(Delivered {
                boundary: "gut water",
                amount: Measure::new(500.0)?,
                ends,
            }),
            Nutrient::Sodium => BoundaryInput::Sodium(Delivered {
                boundary: "gut sodium",
                amount: Measure::new(50.0)?,
                ends,
            }),
            Nutrient::Potassium => BoundaryInput::Potassium(Delivered {
                boundary: "gut potassium",
                amount: Measure::new(2.0)?,
                ends,
            }),
            Nutrient::Chloride => BoundaryInput::Chloride(Delivered {
                boundary: "gut chloride",
                amount: Measure::new(52.0)?,
                ends,
            }),
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Payload {
    Posture { twentieths: u8, legs: Legs },
    Gut { nutrient: Nutrient, ends: SimTime },
}

/// A boundary event is not `Clone`, and each counterfactual run enqueues the plan again.
#[derive(Clone, Copy, Debug)]
struct Planned {
    id: InputId,
    at: SimTime,
    payload: Payload,
}
impl Planned {
    fn event(&self) -> Result<BoundaryEvent, AdmissionError> {
        let payload = match self.payload {
            Payload::Posture { twentieths, legs } => {
                let trunk = Measure::new(f64::from(twentieths) / 20.0)?;
                BoundaryInput::Posture(Posture::new(trunk, legs)?)
            }
            Payload::Gut { nutrient, ends } => nutrient.delivered(ends)?,
        };
        Ok(BoundaryEvent::new(self.id, self.at, payload))
    }
}

#[derive(Clone, Debug)]
struct Occurrence {
    kind: EventKind,
    day: NonZeroU32,
    at: SimTime,
    inputs: Vec<Planned>,
    missing_payload: bool,
}

fn instant(day: NonZeroU32, clock: u64) -> SimTime {
    SimTime::from_ns(u64::from(day.get() - 1) * DAY + clock)
}

fn input_id(kind: EventKind, day: NonZeroU32, slot: u8, ordinal: u8) -> InputId {
    let mut tag = b"day plan input".to_vec();
    tag.push(kind as u8);
    tag.extend_from_slice(&day.get().to_be_bytes());
    tag.extend_from_slice(&[slot, ordinal]);
    InputId::new(Digest::of_canonical(&tag))
}

const RAMP: u8 = 20;

fn ramp(kind: EventKind, day: NonZeroU32, clock: u64, posture: impl Fn(u8) -> Payload) -> Vec<Planned> {
    let at = instant(day, clock);
    let event = |step: u8| Planned {
        id: input_id(kind, day, 0, step),
        at: SimTime::from_ns(at.nanoseconds() + u64::from(step) * SECOND / 2),
        payload: posture(step),
    };
    (0..=RAMP).map(event).collect()
}

impl Occurrence {
    fn sit_up(day: NonZeroU32) -> Self {
        let clock = 7 * HOUR;
        let posture = |step| Payload::Posture {
            twentieths: step,
            legs: if step < 10 {
                Legs::Horizontal
            } else {
                Legs::SeatedDependent
            },
        };
        Self {
            kind: EventKind::SitUp,
            day,
            at: instant(day, clock),
            inputs: ramp(EventKind::SitUp, day, clock, posture),
            missing_payload: false,
        }
    }
    fn meal(day: NonZeroU32, slot: u8, clock: u64) -> Self {
        let at = instant(day, clock);
        let ends = SimTime::from_ns(at.nanoseconds() + HOUR);
        let nutrients = [
            Nutrient::Water,
            Nutrient::Sodium,
            Nutrient::Potassium,
            Nutrient::Chloride,
        ];
        let inputs = (0..).zip(nutrients).map(|(ordinal, nutrient)| Planned {
            id: input_id(EventKind::Meal, day, slot, ordinal),
            at,
            payload: Payload::Gut { nutrient, ends },
        });
        Self {
            kind: EventKind::Meal,
            day,
            at,
            inputs: inputs.collect(),
            missing_payload: false,
        }
    }
    fn lie_down(day: NonZeroU32) -> Self {
        let clock = 23 * HOUR;
        let posture = |step| Payload::Posture {
            twentieths: RAMP - step,
            legs: if step < 10 {
                Legs::SeatedDependent
            } else {
                Legs::Horizontal
            },
        };
        Self {
            kind: EventKind::LieDownToSleep,
            day,
            at: instant(day, clock),
            inputs: ramp(EventKind::LieDownToSleep, day, clock, posture),
            missing_payload: true,
        }
    }
    fn wake(day: NonZeroU32) -> Self {
        Self {
            kind: EventKind::Wake,
            day,
            at: instant(day, 6 * HOUR + 30 * MINUTE),
            inputs: Vec::new(),
            missing_payload: true,
        }
    }
}

/// The contract's day (§6) from the 06:55 admission, for each of `days` days: sit up at 07:00, meals at 07:30,
/// 12:30 and 18:30, lie down to sleep at 23:00, wake at 06:30; then the next morning's sit-up, which cuts the last
/// wake's window.
#[derive(Clone, Debug)]
pub struct DayPlan {
    occurrences: Vec<Occurrence>,
}
impl DayPlan {
    /// Sitting up is a 21-event posture ramp, and lying down is that ramp reversed. No payload carries sleep
    /// onset or offset yet.
    pub fn reference(days: NonZeroU32) -> Self {
        let meals = [
            7 * HOUR + 30 * MINUTE,
            12 * HOUR + 30 * MINUTE,
            18 * HOUR + 30 * MINUTE,
        ];
        let mut occurrences = Vec::new();
        for day in (1..=days.get()).filter_map(NonZeroU32::new) {
            occurrences.push(Occurrence::sit_up(day));
            let meal = |(slot, clock)| Occurrence::meal(day, slot, clock);
            occurrences.extend((0..).zip(meals).map(meal));
            occurrences.push(Occurrence::lie_down(day));
            occurrences.push(Occurrence::wake(day.saturating_add(1)));
        }
        occurrences.push(Occurrence::sit_up(days.saturating_add(1)));
        Self { occurrences }
    }
    /// Keeps the meal as a marker, so every other window still ends where it did.
    fn without(&self, meal: usize) -> DayPlan {
        let mut plan = self.clone();
        if let Some(withheld) = plan.occurrences.get_mut(meal) {
            withheld.inputs.clear();
        }
        plan
    }
    fn next_after(&self, at: SimTime) -> Option<SimTime> {
        let later = self.occurrences.iter().map(|occurrence| occurrence.at);
        later.filter(|&later| later > at).min()
    }
    fn meal_end(&self, at: SimTime) -> SimTime {
        let cap = SimTime::from_ns(at.nanoseconds().saturating_add(MEAL_WINDOW));
        self.next_after(at).map_or(cap, |next| next.min(cap))
    }
}

/// Admission at the window's start; the run steps until the head reaches its end.
pub struct DaySpec {
    pub window: Interval,
    pub packet: ParameterPacket<SyntheticFixture>,
    pub lens: ObservationLens,
    pub seed: Seed,
    pub plan: DayPlan,
}

/// What a day check reads, each a produced quantity of the slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayQuantity {
    HeartRate,
    ArterialPressure,
    StrokeVolume,
    CardiacOutput,
    PlasmaVolume,
    PlasmaAlbumin,
    SodiumExcretion,
    Filtration,
    PlasmaOsmolality,
    PlasmaSodium,
    Vasopressin,
    PotassiumExcretion,
    ChlorideExcretion,
    PlasmaPotassium,
    UrineFlow,
}

/// Not available yet never passes or fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail(Miss),
    NotAvailableYet(Unavailable),
}

/// The part of a row's course the trace did not show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Miss {
    /// The change was not the row's direction, or not a response at all.
    Direction,
    /// The direction held, but no onset met the row's bound.
    Onset,
    /// No dip past the band in time.
    Dip,
    /// No return to the band in time after the dip.
    Recovery,
    /// The pressure from 30 s on fell below its baseline.
    Held,
    /// No rise past the band in time.
    Rise,
    /// No fall from 5 min on, or none that began in time.
    Fall,
    /// The night before the wake did not dip below the awake hour.
    NoDip,
}

/// Why a check cannot score yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The row has no sourced direction or timing.
    Unscored,
    /// No boundary payload carries the event yet, such as sleep.
    NoPayload,
    /// The run holds no occurrence of the event, or not every input of it applied.
    NotReached,
    /// The run does not reach a window's end, or starts after a window's start.
    NotCovered,
    /// A window holds fewer than 2 whole beats.
    TooFewBeats,
    /// No run without this meal stands beside the run.
    NoCounterfactual,
}

/// One quantity of one row around one occurrence of its event.
#[derive(Clone, Debug)]
pub struct DayCheck {
    row: &'static Row,
    quantity: DayQuantity,
    occurrence: Option<(NonZeroU32, SimTime)>,
    verdict: Verdict,
    shown: Shown,
}
impl DayCheck {
    pub fn row(&self) -> u8 {
        self.row.number
    }
    pub fn quantity(&self) -> DayQuantity {
        self.quantity
    }
    /// The occurrence's instant, or `None` when the run holds none.
    pub fn at(&self) -> Option<SimTime> {
        self.occurrence.map(|(_, at)| at)
    }
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }
}

pub enum DayFailure {
    Initialization(InitializationFailure),
    Faulted(FaultReport),
    /// A start the store would not checkpoint or resume, or a fault it would not export.
    Checkpoint(CheckpointError),
    /// An event the plan schedules that the session refuses to take.
    Input(AdmissionError),
    /// A row the day trace reads that this body lacks.
    MissingRow(RowKind),
    /// The sodium transfer along this edge, which the day trace reads from the ledger, absent from this body.
    MissingTransfer(Context),
}

fn exported(stopped: Session<'_, Faulted>) -> DayFailure {
    match stopped.export() {
        Ok(report) => DayFailure::Faulted(report),
        Err(error) => DayFailure::Checkpoint(error),
    }
}

struct Start<'m> {
    saved: Checkpoint,
    identity: RunIdentity,
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    store: &'m dyn HistoryStore,
    readings: Readings,
    scales: ErrorScales,
    sizes: Reading,
}
impl Start<'_> {
    fn run(&self, plan: &DayPlan, end: SimTime) -> Result<DayTrace, DayFailure> {
        let Self { saved, identity, .. } = self;
        let resumed = Session::resume(saved, identity, self.body, self.core, self.store);
        let mut session = resumed.map_err(DayFailure::Checkpoint)?;
        let head = session.record().accepted();
        let (start, (phase, _)) = (head.time(), self.readings.read(head));
        let command = Digest::of_canonical(b"the reference day");
        let inputs = plan.occurrences.iter().flat_map(|occurrence| &occurrence.inputs);
        for input in inputs.filter(|input| input.at >= start) {
            let event = input.event().map_err(DayFailure::Input)?;
            let enqueued = session.enqueue(AuthorizedInput::from_command_receipt(command, event));
            enqueued.map_err(DayFailure::Input)?;
        }
        let mut fold = Fold::new(plan, self.scales, self.sizes, start, phase);
        while session.record().accepted().time() < end {
            let before = session.record().accepted().time();
            let (next, receipt) = accept(session).map_err(exported)?;
            session = next;
            let view = session.record().accepted();
            let (phase, reading) = self.readings.read(view);
            let length_ns = view.time().nanoseconds() - before.nanoseconds();
            let ledger = session.conservation();
            let head = ledger.head().expect("an accepted step heads the ledger");
            fold.step(
                view.time(),
                length_ns,
                phase,
                &reading,
                self.readings.sodium_excreted_per_second(&head),
            );
            fold.applied(receipt.applied());
        }
        let record = session.record();
        Ok(fold.finish(
            identity.digest(),
            record.seal().digest(),
            record.accepted().time(),
        ))
    }
}

/// Admits once, checkpoints, and runs the plan from that start; for each meal whose window the run covers, also
/// runs the plan without that meal from the same start. Then every row of the day table is checked.
pub fn day_checks<'m>(
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    spec: DaySpec,
    store: &'m dyn HistoryStore,
) -> Result<DayReport, DayFailure> {
    let DaySpec {
        window,
        packet,
        lens,
        seed,
        plan,
    } = spec;
    let readings = Readings::bind(body)?;
    let port = BoundaryPort::new(Vec::new()).map_err(|error| DayFailure::Initialization(error.into()))?;
    let (saved, identity, scales, sizes) = {
        let start = solved_start(body, packet, port, core, &lens, window.start());
        let session = start.and_then(|start| admit_start(body, core, start, seed, store));
        let session = session.map_err(DayFailure::Initialization)?;
        let saved = session.checkpoint().map_err(DayFailure::Checkpoint)?;
        let record = session.record();
        let identity = record.identity().clone();
        let scales = *identity.resolution().scales();
        (saved, identity, scales, readings.sizes(&record.problem()))
    };
    let start = Start {
        saved,
        identity,
        body,
        core,
        store,
        readings,
        scales,
        sizes,
    };
    let fed = start.run(&plan, window.end())?;
    let mut controls = Vec::new();
    for (index, meal) in plan.occurrences.iter().enumerate() {
        let end = plan.meal_end(meal.at);
        if meal.kind == EventKind::Meal && meal.at >= window.start() && end <= window.end() {
            controls.push((index, start.run(&plan.without(index), end)?));
        }
    }
    let checks = judge::judge(&plan, &fed, &controls);
    Ok(DayReport {
        run: fed.run,
        seal: fed.seal,
        start: fed.start,
        end: fed.end,
        beats: fed.beats.len(),
        checks,
    })
}

/// Every row of the day table, checked on one run and its counterfactuals.
pub struct DayReport {
    run: Digest,
    seal: Digest,
    start: SimTime,
    end: SimTime,
    beats: usize,
    checks: Vec<DayCheck>,
}
impl DayReport {
    /// The run identity's digest.
    pub fn run(&self) -> Digest {
        self.run
    }
    /// Every row from 1 to 17 appears, in table order.
    pub fn checks(&self) -> &[DayCheck] {
        &self.checks
    }
}

struct Hex(Digest);
impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.as_bytes()[..8]
            .iter()
            .try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

struct Clock(SimTime);
impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.nanoseconds() % DAY / SECOND;
        write!(
            f,
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    }
}

struct When(SimTime);
impl fmt::Display for When {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.nanoseconds() / DAY + 1 {
            1 => write!(f, "{}", Clock(self.0)),
            day => write!(f, "day {day} {}", Clock(self.0)),
        }
    }
}

struct Figures(f64);
impl fmt::Display for Figures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(value) = *self;
        let (mut size, mut decimals) = (value.abs(), 3usize);
        if size.is_finite() && size > 0.0 {
            while size >= 10.0 && decimals > 0 {
                size /= 10.0;
                decimals -= 1;
            }
            while size < 1.0 && decimals < 12 {
                size *= 10.0;
                decimals += 1;
            }
        }
        if f.sign_plus() {
            write!(f, "{value:+.decimals$}")
        } else {
            write!(f, "{value:.decimals$}")
        }
    }
}

struct Seconds(Option<u64>);
impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(ns) => write!(f, "{} s", Figures(ns as f64 / SECOND as f64)),
            None => write!(f, "none"),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Span::Seconds(count) => write!(f, "{count} s"),
            Span::Minutes(count) => write!(f, "{count} min"),
            Span::Hours(count) => write!(f, "{count} h"),
        }
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bound::Under(span) => write!(f, "under {span}"),
            Bound::By(span) => write!(f, "by {span}"),
        }
    }
}

impl fmt::Display for judge::Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let way = match self.sign() {
            Some(table::Direction::Up) => "up",
            Some(table::Direction::Down) => "down",
            None => "unchanged",
        };
        let (delta, noise, tolerance) = (Figures(self.delta), Figures(self.noise), Figures(self.tolerance));
        write!(
            f,
            "{way} {delta:+} against 3 SD {noise} and tolerance {tolerance}"
        )
    }
}

struct Change(Option<judge::Change>);
impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(change) => write!(f, "{change}"),
            None => write!(f, "no change measured"),
        }
    }
}

fn quantity_words(quantity: DayQuantity) -> &'static str {
    match quantity {
        DayQuantity::HeartRate => "heart rate",
        DayQuantity::ArterialPressure => "arterial pressure",
        DayQuantity::StrokeVolume => "stroke volume",
        DayQuantity::CardiacOutput => "cardiac output",
        DayQuantity::PlasmaVolume => "plasma volume",
        DayQuantity::PlasmaAlbumin => "plasma albumin",
        DayQuantity::SodiumExcretion => "sodium excretion",
        DayQuantity::Filtration => "filtration",
        DayQuantity::PlasmaOsmolality => "plasma osmolality",
        DayQuantity::PlasmaSodium => "plasma sodium",
        DayQuantity::Vasopressin => "vasopressin",
        DayQuantity::PotassiumExcretion => "potassium excretion",
        DayQuantity::ChlorideExcretion => "chloride excretion",
        DayQuantity::PlasmaPotassium => "plasma potassium",
        DayQuantity::UrineFlow => "urine flow",
    }
}

fn event_words(kind: EventKind) -> &'static str {
    match kind {
        EventKind::SitUp => "sit up",
        EventKind::Meal => "meal",
        EventKind::LieDownToSleep => "lie down to sleep",
        EventKind::Wake => "wake",
    }
}

fn miss_words(miss: Miss) -> &'static str {
    match miss {
        Miss::Direction => "wrong direction",
        Miss::Onset => "no onset within its bound",
        Miss::Dip => "no dip",
        Miss::Recovery => "no recovery",
        Miss::Held => "not held",
        Miss::Rise => "no rise",
        Miss::Fall => "no fall",
        Miss::NoDip => "no night dip",
    }
}

impl DayCheck {
    fn unavailable(&self, why: Unavailable, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not available yet: ")?;
        match (why, self.row.scoring, self.shown.uncovered) {
            (Unavailable::Unscored, Scoring::Unscored(_, reason), _) => write!(f, "unscored, {reason}"),
            (Unavailable::Unscored, ..) => write!(f, "unscored"),
            (Unavailable::NoPayload, ..) => write!(f, "no payload carries sleep"),
            (Unavailable::NotReached, ..) if self.occurrence.is_some() => {
                write!(f, "its inputs did not all apply")
            }
            (Unavailable::NotReached, ..) => write!(f, "not reached"),
            (Unavailable::NotCovered, _, Some(Uncovered::EndsBefore(end))) => {
                write!(f, "the run ends before {}", When(end))
            }
            (Unavailable::NotCovered, _, Some(Uncovered::StartsAfter(start))) => {
                write!(f, "the run starts after {}", When(start))
            }
            (Unavailable::NotCovered, ..) => write!(f, "the plan holds no event to bound its window"),
            (Unavailable::TooFewBeats, ..) => write!(f, "fewer than 2 whole beats in a window"),
            (Unavailable::NoCounterfactual, ..) => write!(f, "no run without this meal"),
        }
    }
    fn numbers(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Shown {
            change,
            dip,
            first,
            second,
            ..
        } = self.shown;
        let (change, dip, first, second) = (Change(change), Change(dip), Seconds(first), Seconds(second));
        match self.row.scoring {
            Scoring::Scored(_, Course::OneWay(_, bound)) => write!(f, "{change}, onset {first}, {bound}"),
            Scoring::Scored(_, Course::DipThenBack { dip, back, held_from }) => write!(
                f,
                "dip {first}, {dip}, back {second}, {back}, then {change} from {held_from}"
            ),
            Scoring::Scored(
                _,
                Course::RiseThenFall {
                    rise,
                    fall_from,
                    fall,
                },
            ) => write!(
                f,
                "rise {first}, {rise}, then {change} from {fall_from}, fall onset {second}, {fall}"
            ),
            Scoring::Scored(_, Course::AgainstControl(_, bound)) => {
                write!(f, "{change} against the run without it, onset {first}, {bound}")
            }
            Scoring::DirectionOnly(_, Lean::Down) => write!(f, "{change}, timing not available yet"),
            Scoring::DirectionOnly(_, Lean::UpAfterNightDip) => write!(
                f,
                "night {dip} below the awake hour, then {change}, timing not available yet"
            ),
            Scoring::Unscored(..) => write!(f, "nothing read"),
        }
    }
}

struct Line<'c>(&'c DayCheck);
impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(check) = self;
        let row = check.row;
        write!(
            f,
            "row {:>2} {}, {}",
            row.number,
            quantity_words(check.quantity),
            event_words(row.event)
        )?;
        match check.occurrence {
            Some((day, at)) => write!(f, " day {day} {}: ", Clock(at))?,
            None => write!(f, ", none in the run: ")?,
        }
        match check.verdict {
            Verdict::Pass => {
                write!(f, "pass, ")?;
                check.numbers(f)?;
            }
            Verdict::Fail(miss) => {
                write!(f, "fail, {}, ", miss_words(miss))?;
                check.numbers(f)?;
            }
            Verdict::NotAvailableYet(why) => check.unavailable(why, f)?,
        }
        write!(f, " ({})", row.source)
    }
}

impl fmt::Display for DayReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "day checks run {} seal {} from {} to {}, {} beats",
            Hex(self.run),
            Hex(self.seal),
            When(self.start),
            When(self.end),
            self.beats
        )?;
        self.checks
            .iter()
            .try_for_each(|check| write!(f, "\n{}", Line(check)))
    }
}
