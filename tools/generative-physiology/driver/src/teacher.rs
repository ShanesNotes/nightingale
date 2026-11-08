use crate::archive::{
    BranchSummary, Course, FaultKind, FaultSummary, PartialRun, Recorded, RunState, RunSummary, SUMMARY,
    Settled, write_synced,
};
use crate::capture::{BeatReading, Capture, Catalogue, CatalogueError, STEP_COLUMNS, step_row};
use crate::link_file::LinkFile;
use crate::npy::NpyWriter;
use crate::spec::{CAPTURE_VERSION, CoreChoice, PostureEvent, RunSpec, SpecError, Stamp, Variant};
use core::fmt;
use ng_contract::{
    AdmissionError, BoundaryPort, CompiledBody, CoupledSolver, Digest, InitializationFailure,
    ObservationLens, ObservationSeed, SimTime,
};
use ng_observe::Observer;
use ng_record::{
    AuthorizedInput, BranchId, Checkpoint, CheckpointError, Faulted, ObservationRefusal, RecordDigests,
    Running, Session, Step, StepReceipt,
};
use ng_validate::{Admitted, admit_at, anchor_moment, arena_body, arena_lens, arena_packet};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const NS_PER_MS: u64 = 1_000_000;
const STEPS_BETWEEN_READS: u64 = 10_000;
/// `crates/ng-validate/src/run.rs:219` observes the slice with this seed; the arena lens has no error, so any
/// seed reads the same.
const OBSERVATION_SEED: [u64; 4] = [5, 5, 5, 5];

#[allow(
    clippy::disallowed_types,
    reason = "cost evidence for the run summary; no wall clock reaches a seal or a row"
)]
mod clock {
    use std::time::Instant;

    pub(crate) struct Stopwatch(Instant);
    impl Stopwatch {
        pub(crate) fn start() -> Self {
            Self(Instant::now())
        }
        pub(crate) fn seconds(&self) -> f64 {
            self.0.elapsed().as_secs_f64()
        }
    }
}
use clock::Stopwatch;

/// The resident-memory limit a run stops at. The summary records it, and the spec id does not cover it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryCap {
    gigabytes: u32,
}
impl MemoryCap {
    pub fn gb(gigabytes: u32) -> Self {
        Self { gigabytes }
    }
    pub fn gigabytes(self) -> u32 {
        self.gigabytes
    }
    fn bytes(self) -> u64 {
        u64::from(self.gigabytes) * 1_000_000_000
    }
}

/// What a run records that no spec says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub cap: MemoryCap,
    pub source_commit: String,
}

#[derive(Debug)]
pub enum TeachError {
    Io(io::Error),
    Admission(Box<AdmissionError>),
    Initialization(Box<InitializationFailure>),
    Catalogue(CatalogueError),
    Checkpoint(CheckpointError),
    Observation(ObservationRefusal),
    Spec(SpecError),
    /// The engine refused a variant's event: a lead shorter than a step, or a stamp a step had passed.
    Enqueue {
        event: usize,
        error: Box<AdmissionError>,
    },
    /// A live process is writing this spec's run.
    Held(PathBuf),
    /// `run` takes an open-loop spec and `interact` a closed-loop one.
    Loop,
    /// The teacher was built for another core.
    OtherCore,
    /// A checkpoint variant reached its end with no checkpoint.
    NoCheckpoint,
}
impl fmt::Display for TeachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Admission(error) => write!(f, "admission refused: {error:?}"),
            Self::Initialization(failure) => write!(f, "initialization failed: {failure:?}"),
            Self::Catalogue(error) => write!(f, "catalogue: {error}"),
            Self::Checkpoint(error) => write!(f, "checkpoint: {error:?}"),
            Self::Observation(refusal) => write!(f, "observer: {refusal:?}"),
            Self::Spec(error) => write!(f, "spec: {error}"),
            Self::Enqueue { event, error } => write!(f, "event {event} refused: {error:?}"),
            Self::Held(path) => write!(f, "a live run holds {}", path.display()),
            Self::Loop => write!(f, "run takes a spec without closed-loop, interact one with it"),
            Self::OtherCore => write!(f, "the spec names another core than the teacher's"),
            Self::NoCheckpoint => write!(f, "the run ended without its checkpoint"),
        }
    }
}
impl From<io::Error> for TeachError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<AdmissionError> for TeachError {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(Box::new(error))
    }
}
impl From<InitializationFailure> for TeachError {
    fn from(failure: InitializationFailure) -> Self {
        Self::Initialization(Box::new(failure))
    }
}
impl From<CatalogueError> for TeachError {
    fn from(error: CatalogueError) -> Self {
        Self::Catalogue(error)
    }
}
impl From<CheckpointError> for TeachError {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}
impl From<ObservationRefusal> for TeachError {
    fn from(refusal: ObservationRefusal) -> Self {
        Self::Observation(refusal)
    }
}
impl From<SpecError> for TeachError {
    fn from(error: SpecError) -> Self {
        Self::Spec(error)
    }
}

/// Where a run's directory stands after a command.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Complete already, so nothing was written.
    Present(PathBuf),
    /// Faulted already, so nothing was written.
    FaultFound(PathBuf),
    Complete(PathBuf),
    Faulted(PathBuf),
}

#[derive(Debug)]
pub enum LinkError {
    Io(io::Error),
    Closed,
    Answer(SpecError),
}
impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "the link failed: {error}"),
            Self::Closed => write!(f, "the policy closed its link"),
            Self::Answer(error) => write!(f, "an answer that is not an event line: {error}"),
        }
    }
}

pub trait PolicyLink {
    /// The beat that just ended, and the earliest stamp an answer may take. A beat that ends at or after the
    /// spec's end is not asked about.
    fn after_beat(&mut self, beat: &BeatReading, earliest: Stamp) -> Result<Option<PostureEvent>, LinkError>;
}

/// One JSON line per beat out, one answer line back: `none`, or an event line of the spec grammar.
pub struct LineLink {
    input: Box<dyn BufRead>,
    output: Box<dyn Write>,
}
impl LineLink {
    pub fn stdio() -> Self {
        Self {
            input: Box::new(io::stdin().lock()),
            output: Box::new(io::stdout()),
        }
    }
}
impl PolicyLink for LineLink {
    fn after_beat(&mut self, beat: &BeatReading, earliest: Stamp) -> Result<Option<PostureEvent>, LinkError> {
        writeln!(self.output, "{}", beat.json(earliest)).map_err(LinkError::Io)?;
        self.output.flush().map_err(LinkError::Io)?;
        let mut answer = String::new();
        if self.input.read_line(&mut answer).map_err(LinkError::Io)? == 0 {
            return Err(LinkError::Closed);
        }
        match answer.strip_suffix('\n').unwrap_or(&answer) {
            "none" => Ok(None),
            line => PostureEvent::parse_line(line)
                .map(Some)
                .map_err(LinkError::Answer),
        }
    }
}

fn status_bytes(field: &str) -> io::Result<u64> {
    let status = fs::read_to_string("/proc/self/status")?;
    let kb = (status.lines())
        .find_map(|line| line.strip_prefix(field))
        .and_then(|kb| kb.trim().strip_suffix("kB"))
        .and_then(|kb| kb.trim().parse::<u64>().ok());
    kb.map(|kb| kb * 1024)
        .ok_or_else(|| io::Error::other(format!("no {field} line in kB")))
}

fn over_cap(cap: MemoryCap) -> io::Result<Option<FaultKind>> {
    let resident_bytes = status_bytes("VmRSS:")?;
    Ok((resident_bytes > cap.bytes()).then_some(FaultKind::MemoryCap { resident_bytes }))
}

fn authorized(event: &PostureEvent) -> Result<AuthorizedInput, AdmissionError> {
    let command = Digest::of_canonical(b"gp-driver");
    Ok(AuthorizedInput::from_command_receipt(
        command,
        event.boundary_event()?,
    ))
}

fn enqueue(session: &mut Session<'_, Running>, event: &PostureEvent, index: usize) -> Result<(), TeachError> {
    let refused = |error: AdmissionError| TeachError::Enqueue {
        event: index,
        error: Box::new(error),
    };
    session
        .enqueue(authorized(event).map_err(refused)?)
        .map_err(refused)?;
    Ok(())
}

enum Stepped<'m> {
    Accepted {
        session: Session<'m, Running>,
        receipt: Box<StepReceipt>,
        refusals: u64,
    },
    Faulted {
        session: Session<'m, Faulted>,
        refusals: u64,
    },
}

/// One accepted step, retrying each refusal as `crates/ng-validate/src/run.rs:23-46` does, with the count kept.
fn step(session: Session<'_, Running>) -> Stepped<'_> {
    let mut refusals = 0;
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, receipt) => {
                return Stepped::Accepted {
                    session,
                    receipt: Box::new(receipt),
                    refusals,
                };
            }
            Step::Refused(retrying) => {
                refusals += 1;
                retrying.retry()
            }
            Step::Faulted(session) => return Stepped::Faulted { session, refusals },
        };
    }
}

fn engine_fault(course: Course, session: &Session<'_, Faulted>, refusals: u64) -> FaultSummary {
    let fault = session.fault();
    FaultSummary {
        course,
        kind: FaultKind::Engine {
            kind: fault.kind(),
            last_refusal: fault.last_refusal(),
        },
        at: fault.at(),
        attempted_ns: fault.attempted_ns(),
        refusals,
    }
}

fn halt<S>(course: Course, session: &Session<'_, S>, kind: FaultKind) -> FaultSummary {
    FaultSummary {
        course,
        kind,
        at: session.record().accepted().time(),
        attempted_ns: 0,
        refusals: 0,
    }
}

#[derive(Clone, Copy)]
struct Head {
    revision: u64,
    seal: Digest,
    digests: RecordDigests,
}
impl Head {
    fn of<S>(session: &Session<'_, S>) -> Self {
        let record = session.record();
        Self {
            revision: record.revision(),
            seal: record.seal().digest(),
            digests: record.digests(),
        }
    }
}

/// When the events reach the session, chosen once from the spec.
enum Feed<'l> {
    /// Every event in the admission port.
    Port,
    /// None in the port; each enqueued at the first accepted head at or after its stamp minus the lead.
    Late { lead_ns: u64, next: usize },
    /// The events before `withheld` in the port; the rest enqueued right after the checkpoint at the first head at
    /// or after `at`, and again on the copy or branch.
    AfterCheckpoint {
        at: SimTime,
        withheld: usize,
        saved: Option<Checkpoint>,
    },
    /// The spec's events in the port, and after each beat whatever the policy answers.
    Policy(Policy<'l>),
}

struct Policy<'l> {
    link: &'l mut dyn PolicyLink,
    end: Stamp,
    known: BTreeSet<Stamp>,
    actions: Vec<PostureEvent>,
}
impl Policy<'_> {
    /// The driver never moves a stamp: an answer it cannot keep as given ends the run.
    fn answer(
        &mut self,
        session: &mut Session<'_, Running>,
        beat: &BeatReading,
        spec_events: usize,
        capture: &mut Capture<'_>,
    ) -> Result<Option<FaultKind>, TeachError> {
        let refused = |reason: String| -> Result<Option<FaultKind>, TeachError> {
            Ok(Some(FaultKind::Protocol(format!(
                "beat {}: {reason}",
                beat.ordinal
            ))))
        };
        let Some(earliest) = Stamp::at_or_after(session.record().earliest_stamp()) else {
            return refused("no whole-ms stamp follows the head".to_string());
        };
        let action = match self.link.after_beat(beat, earliest) {
            Ok(None) => return Ok(None),
            Ok(Some(action)) => action,
            Err(error) => return refused(error.to_string()),
        };
        let line = action.line();
        if action.at < earliest {
            return refused(format!(
                "{line} is before the earliest stamp, {} ms",
                earliest.ms()
            ));
        }
        if action.at > self.end {
            return refused(format!("{line} is after the end"));
        }
        if self.known.contains(&action.at) {
            return refused(format!("{line} has the stamp of another event"));
        }
        capture.register(action.id(), (spec_events + self.actions.len()) as u64);
        if let Err(error) = session.enqueue(authorized(&action)?) {
            return refused(format!("the engine refused {line}: {error:?}"));
        }
        self.known.insert(action.at);
        self.actions.push(action);
        Ok(None)
    }
}

impl<'l> Feed<'l> {
    fn choose(spec: &RunSpec, longest_step_ns: u64, link: Option<&'l mut dyn PolicyLink>) -> Self {
        if let Some(link) = link {
            return Self::Policy(Policy {
                link,
                end: spec.end(),
                known: spec.events().iter().map(|event| event.at).collect(),
                actions: Vec::new(),
            });
        }
        match spec.variant() {
            Variant::None | Variant::ObserverOff | Variant::Replica(_) => Self::Port,
            Variant::Late { lead_ms } => Self::Late {
                lead_ns: u64::from(lead_ms.get()) * NS_PER_MS,
                next: 0,
            },
            Variant::CheckpointResume { at } | Variant::Rewind { at } => {
                // The step that reaches `at` is at most the body's longest step, so an event that far past it is
                // after the checkpoint's head however that step lands.
                let past = at.sim_time().nanoseconds().saturating_add(longest_step_ns);
                let withheld = (spec.events().iter())
                    .position(|event| event.at.sim_time().nanoseconds() >= past)
                    .unwrap_or(spec.events().len());
                Self::AfterCheckpoint {
                    at: at.sim_time(),
                    withheld,
                    saved: None,
                }
            }
        }
    }
    fn port<'e>(&self, events: &'e [PostureEvent]) -> &'e [PostureEvent] {
        match self {
            Self::Port | Self::Policy(_) => events,
            Self::Late { .. } => &[],
            Self::AfterCheckpoint { withheld, .. } => &events[..*withheld],
        }
    }
    fn at_head(
        &mut self,
        session: &mut Session<'_, Running>,
        events: &[PostureEvent],
        beat: Option<&BeatReading>,
        capture: &mut Capture<'_>,
    ) -> Result<Option<FaultKind>, TeachError> {
        let head = session.record().accepted().time();
        match self {
            Self::Port => {}
            Self::Late { lead_ns, next } => {
                while let Some(event) = events.get(*next) {
                    if event.at.sim_time().nanoseconds().saturating_sub(*lead_ns) > head.nanoseconds() {
                        break;
                    }
                    enqueue(session, event, *next)?;
                    *next += 1;
                }
            }
            Self::AfterCheckpoint { at, withheld, saved } => {
                if saved.is_none() && head >= *at {
                    *saved = Some(session.checkpoint()?);
                    for (index, event) in events.iter().enumerate().skip(*withheld) {
                        enqueue(session, event, index)?;
                    }
                }
            }
            Self::Policy(policy) => {
                if let Some(beat) = beat {
                    return policy.answer(session, beat, events.len(), capture);
                }
            }
        }
        Ok(None)
    }
}

/// What every course stops at: the spec's end, the memory cap, and a history store that failed.
#[derive(Clone, Copy)]
struct Bounds<'r> {
    end: SimTime,
    cap: MemoryCap,
    links: &'r LinkFile,
}

struct Stop {
    fault: FaultSummary,
    head: Head,
}

enum Ended<'m> {
    Reached(Session<'m, Running>),
    Stopped(Box<Stop>),
}

/// The run's own course, which writes every file but the branch's.
struct Original<'r, 'l> {
    events: &'r [PostureEvent],
    bounds: Bounds<'r>,
    feed: Feed<'l>,
    capture: Capture<'r>,
    lens: &'r ObservationLens,
    observer: Option<Observer<'r>>,
    refusals: u64,
}
impl Original<'_, '_> {
    fn drive<'m>(&mut self, mut session: Session<'m, Running>) -> Result<Ended<'m>, TeachError> {
        self.observe(&session)?;
        if let Some(kind) = self
            .feed
            .at_head(&mut session, self.events, None, &mut self.capture)?
        {
            return self.stopped(&session, halt(Course::Original, &session, kind));
        }
        if let Some(kind) = over_cap(self.bounds.cap)? {
            return self.stopped(&session, halt(Course::Original, &session, kind));
        }
        let mut accepted: u64 = 0;
        while session.record().accepted().time() < self.bounds.end {
            let (next, receipt, refusals) = match step(session) {
                Stepped::Accepted {
                    session,
                    receipt,
                    refusals,
                } => (session, receipt, refusals),
                Stepped::Faulted { session, refusals } => {
                    self.refusals += refusals;
                    return self.stopped(&session, engine_fault(Course::Original, &session, refusals));
                }
            };
            session = next;
            self.refusals += refusals;
            let beat = self.capture.accepted(&session, &receipt, refusals)?;
            self.observe(&session)?;
            self.bounds.links.check()?;
            // Past the end no step follows, so no answer could apply.
            let continuing = session.record().accepted().time() < self.bounds.end;
            let asked = beat.as_ref().filter(|_| continuing);
            if let Some(kind) = self
                .feed
                .at_head(&mut session, self.events, asked, &mut self.capture)?
            {
                return self.stopped(&session, halt(Course::Original, &session, kind));
            }
            accepted += 1;
            if accepted.is_multiple_of(STEPS_BETWEEN_READS)
                && let Some(kind) = over_cap(self.bounds.cap)?
            {
                return self.stopped(&session, halt(Course::Original, &session, kind));
            }
        }
        self.capture.end(&session)?;
        Ok(Ended::Reached(session))
    }
    fn observe(&mut self, session: &Session<'_, Running>) -> Result<(), TeachError> {
        let Some(observer) = &mut self.observer else {
            return Ok(());
        };
        let observations = observer.advance(&session.observation(self.lens)?)?;
        self.capture.observed(&observations)?;
        Ok(())
    }
    fn stopped<'m, S>(
        &mut self,
        session: &Session<'_, S>,
        fault: FaultSummary,
    ) -> Result<Ended<'m>, TeachError> {
        self.capture.end(session)?;
        let head = Head::of(session);
        Ok(Ended::Stopped(Box::new(Stop { fault, head })))
    }
}

struct Followed {
    from_revision: u64,
    head: Head,
    refusals: u64,
    fault: Option<FaultSummary>,
}

/// A copy or branch of a checkpoint variant: the withheld events again, then steps to the end, rows only.
fn follow(
    mut session: Session<'_, Running>,
    withheld: &[(usize, PostureEvent)],
    bounds: Bounds<'_>,
    steps: &mut NpyWriter<u64>,
) -> Result<Followed, TeachError> {
    for (index, event) in withheld {
        enqueue(&mut session, event, *index)?;
    }
    let from_revision = session.record().revision();
    let (mut refusals, mut accepted) = (0, 0u64);
    while session.record().accepted().time() < bounds.end {
        match step(session) {
            Stepped::Accepted {
                session: next,
                receipt,
                refusals: these,
            } => {
                session = next;
                refusals += these;
                let end = session.record().accepted().time();
                steps.push(&step_row(receipt.revision(), end, these, receipt.seal().digest()))?;
                bounds.links.check()?;
                accepted += 1;
                if accepted.is_multiple_of(STEPS_BETWEEN_READS)
                    && let Some(kind) = over_cap(bounds.cap)?
                {
                    let fault = Some(halt(Course::Branch, &session, kind));
                    let head = Head::of(&session);
                    return Ok(Followed {
                        from_revision,
                        head,
                        refusals,
                        fault,
                    });
                }
            }
            Stepped::Faulted {
                session: stopped,
                refusals: these,
            } => {
                refusals += these;
                let fault = Some(engine_fault(Course::Branch, &stopped, these));
                let head = Head::of(&stopped);
                return Ok(Followed {
                    from_revision,
                    head,
                    refusals,
                    fault,
                });
            }
        }
    }
    Ok(Followed {
        from_revision,
        head: Head::of(&session),
        refusals,
        fault: None,
    })
}

/// One admission, one run.
pub struct Teacher {
    choice: CoreChoice,
    body: CompiledBody,
    core: Box<dyn CoupledSolver>,
    catalogue: Catalogue,
}
impl Teacher {
    /// Compiles the arena body once.
    pub fn new(core: &CoreChoice) -> Result<Self, TeachError> {
        let body = arena_body()?;
        let catalogue = Catalogue::of(&body)?;
        Ok(Self {
            choice: *core,
            core: core.build()?,
            body,
            catalogue,
        })
    }
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }
    /// Writes `out/<spec id>/`, or `out/<spec id>.fault/` when the run faults or passes its cap.
    pub fn run(&self, spec: &RunSpec, options: &Options, out: &Path) -> Result<Outcome, TeachError> {
        if spec.closed_loop().is_some() {
            return Err(TeachError::Loop);
        }
        self.teach(spec, None, options, out)
    }
    /// A closed-loop run: after each beat edge the driver asks the link and enqueues what comes back.
    pub fn interact(
        &self,
        spec: &RunSpec,
        link: &mut dyn PolicyLink,
        options: &Options,
        out: &Path,
    ) -> Result<Outcome, TeachError> {
        if spec.closed_loop().is_none() {
            return Err(TeachError::Loop);
        }
        self.teach(spec, Some(link), options, out)
    }
    fn teach(
        &self,
        spec: &RunSpec,
        link: Option<&mut dyn PolicyLink>,
        options: &Options,
        out: &Path,
    ) -> Result<Outcome, TeachError> {
        if *spec.core() != self.choice {
            return Err(TeachError::OtherCore);
        }
        let id = spec.id();
        fs::create_dir_all(out)?;
        match RunState::of(out, id, None)? {
            RunState::Present(_) | RunState::Stale(_) => {
                return Ok(Outcome::Present(out.join(id.to_string())));
            }
            RunState::Fault(path) => return Ok(Outcome::FaultFound(path)),
            RunState::Partial { paths, held: true } => {
                return Err(TeachError::Held(paths.into_iter().next().unwrap_or_default()));
            }
            RunState::Partial { paths, held: false } => {
                for path in paths {
                    fs::remove_dir_all(path)?;
                }
            }
            RunState::Missing => {}
        }
        let partial = PartialRun::create(out, id)?;
        write_synced(&partial.file("spec.txt"), &spec.canonical())?;
        let links = LinkFile::create(&partial.file("links"))?;
        let summary = self.courses(spec, link, options, &partial, &links)?;
        links.remove()?;
        write_synced(&partial.file(SUMMARY), &summary.render())?;
        Ok(match summary.fault {
            None => match partial.complete()? {
                Settled::Renamed(path) => Outcome::Complete(path),
                Settled::Lost(path) => Outcome::Present(path),
            },
            Some(_) => match partial.fault()? {
                Settled::Renamed(path) => Outcome::Faulted(path),
                Settled::Lost(path) => Outcome::FaultFound(path),
            },
        })
    }
    fn courses(
        &self,
        spec: &RunSpec,
        link: Option<&mut dyn PolicyLink>,
        options: &Options,
        partial: &PartialRun,
        links: &LinkFile,
    ) -> Result<RunSummary, TeachError> {
        let lens = arena_lens(&self.body)?;
        let feed = Feed::choose(spec, self.body.longest_step_ns(), link);
        let port: Result<Vec<_>, _> = (feed.port(spec.events()).iter())
            .map(PostureEvent::boundary_event)
            .collect();
        let port = BoundaryPort::new(port?)?;
        let packet = arena_packet(&self.body)?;
        let clock = Stopwatch::start();
        let admitted = admit_at(
            &self.body,
            packet,
            port,
            self.core.as_ref(),
            &lens,
            anchor_moment(),
            links,
        )?;
        let admission_s = clock.seconds();
        let Admitted { session, .. } = admitted;
        let identity = session.record().identity().clone();
        let recorded = Recorded {
            model: identity.model().digest(),
            packet: identity.packet().digest(),
            resolution: identity.resolution().digest(),
            run_identity: identity.digest(),
        };
        let first_seal = session.record().seal().digest();
        let inputs: BTreeMap<_, _> = (spec.events().iter().enumerate())
            .map(|(index, event)| (event.id(), index as u64))
            .collect();
        let capture = Capture::create(partial.directory(), &self.body, &self.catalogue, &session, inputs)?;
        let bounds = Bounds {
            end: spec.end().sim_time(),
            cap: options.cap,
            links,
        };
        let observe = spec.variant() != Variant::ObserverOff;
        let mut original = Original {
            events: spec.events(),
            bounds,
            feed,
            capture,
            lens: &lens,
            observer: observe.then(|| Observer::new(&lens, ObservationSeed::new(OBSERVATION_SEED))),
            refusals: 0,
        };
        let clock = Stopwatch::start();
        let ended = original.drive(session)?;
        let Original {
            feed,
            capture,
            refusals,
            ..
        } = original;
        let (head, mut fault, reached) = match ended {
            Ended::Reached(session) => (Head::of(&session), None, Some(session)),
            Ended::Stopped(stop) => (stop.head, Some(stop.fault), None),
        };
        let mut branch = None;
        if let (Some(session), Feed::AfterCheckpoint { withheld, saved, .. }) = (reached, &feed) {
            let saved = saved.as_ref().ok_or(TeachError::NoCheckpoint)?;
            let later: Vec<(usize, PostureEvent)> = (spec.events().iter().copied().enumerate())
                .skip(*withheld)
                .collect();
            let mut steps = NpyWriter::create(&partial.file("branch-steps.npy"), STEP_COLUMNS)?;
            let followed = match spec.variant() {
                Variant::Rewind { .. } => {
                    let named = BranchId::new(Digest::of_canonical(b"gp-driver rewind"));
                    let rewound = session.rewind(saved, named)?;
                    drop(session);
                    follow(rewound, &later, bounds, &mut steps)?
                }
                _ => {
                    drop(session);
                    let body = arena_body()?;
                    let core = self.choice.build()?;
                    let copy = Session::resume(saved, &identity, &body, core.as_ref(), links)?;
                    follow(copy, &later, bounds, &mut steps)?
                }
            };
            steps.finish()?;
            fault = followed.fault;
            branch = Some(BranchSummary {
                from_revision: followed.from_revision,
                revisions: followed.head.revision,
                refusals: followed.refusals,
                last_seal: followed.head.seal,
                end: followed.head.digests,
            });
        }
        let step_wall_s = clock.seconds();
        let beats = capture.beats();
        capture.finish()?;
        let mut files = vec![
            "spec.txt",
            "edges.npy",
            "beats.npy",
            "steps.npy",
            "applied.npy",
            "lens.npy",
        ];
        if branch.is_some() {
            files.push("branch-steps.npy");
        }
        if let Feed::Policy(policy) = &feed {
            let realized = spec.realized(&policy.actions)?;
            write_synced(&partial.file("realized.txt"), &realized.canonical())?;
            files.push("realized.txt");
        }
        let files = (files.into_iter())
            .map(|name| -> io::Result<(String, Digest)> {
                Ok((
                    name.to_string(),
                    Digest::of_canonical(&fs::read(partial.file(name))?),
                ))
            })
            .collect::<io::Result<Vec<_>>>()?;
        Ok(RunSummary {
            id: spec.id(),
            capture_version: CAPTURE_VERSION,
            source_commit: options.source_commit.clone(),
            identity: recorded,
            first_seal,
            last_seal: head.seal,
            revisions: head.revision,
            refusals,
            beats,
            fault,
            admission_s,
            step_wall_s,
            peak_rss_bytes: status_bytes("VmHWM:")?,
            files,
            memory_cap_gb: options.cap.gigabytes(),
            variant: spec.variant().words(),
            end: head.digests,
            branch,
        })
    }
}
