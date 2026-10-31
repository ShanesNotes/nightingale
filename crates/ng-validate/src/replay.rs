use crate::ReferenceIntegrator;
use ng_contract::{
    AcceptedStateView, AdmissionError, BoundaryEvent, CompiledBody, CoupledProblem, CoupledSolver,
    DeclaredResolution, Digest, EngineFault, Fired, InitializationFailure, InputId, LawName, SimTime,
    StepProposal, StepTicket, TrialFailure,
};
use ng_record::{
    AuthorizedInput, BranchId, CheckpointError, HistoryStore, RecordDigests, Running, Seal, Session, Step,
};

pub trait ReplayCase {
    fn body(&self) -> &CompiledBody;
    fn end(&self) -> SimTime;
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError>;
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure>;
    /// The same run with no timeline input known at admission, so each must come through `Session::enqueue`.
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure>;
}

/// Builds a core with no state from earlier runs. A gate calls it once per run, its reference run first, so
/// a core that must differ between two runs can differ only through what its factory gives it.
pub trait CoreFactory {
    fn fresh(&self) -> Box<dyn CoupledSolver + '_>;
}
impl<F, C> CoreFactory for F
where
    F: Fn() -> C,
    C: CoupledSolver + 'static,
{
    fn fresh(&self) -> Box<dyn CoupledSolver + '_> {
        Box::new(self())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockSource {
    /// A list of readings, which proves pace cannot reach a run and is no live clock.
    Scripted,
    Host,
}

/// No crate here reads a clock, so a host outside the workspace passes one in. A host's clock may wait for
/// its next tick before it answers.
pub trait WallClock {
    /// Wall ns since the run began, or `None` when this clock has no more to say.
    fn elapsed_ns(&self) -> Option<u64>;
    fn source(&self) -> ClockSource;
}

/// `reference` is typed so a caller cannot name another core.
pub struct ChancePlan<'c> {
    pub second_policy: &'c dyn CoreFactory,
    pub reference: &'c dyn Fn() -> ReferenceIntegrator,
    /// How far from the reference a policy may fire one event: the landing tolerance plus what a quadrature of
    /// the hazard rate gets wrong. A model default.
    pub event_tolerance_ns: u64,
}

pub struct ReplayPlan<'c> {
    pub clock: &'c dyn WallClock,
    /// Simulated ns per wall ns under `clock`.
    pub simulated_per_wall: u64,
    /// Lengths of simulated time, each after the last, repeating to the end of the case.
    pub chunks: Vec<u64>,
    /// Resume at each pair of neighbouring revisions this many apart, and on each side of a breakpoint or a
    /// refusal.
    pub resume_every: u64,
    /// `None` only for a body with no hazard guard.
    pub chance: Option<ChancePlan<'c>>,
}

enum Pace<'c> {
    Unbounded,
    Live {
        clock: &'c dyn WallClock,
        simulated_per_wall: u64,
    },
    Chunked(&'c [u64]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayGate {
    Pace,
    Resume,
    LateAdmission,
    Rewind,
    /// Every ticket solved twice and the second kept: same bits, or a value moved with an evaluation (I8). The
    /// attempt is the same both times, so dice keyed on it answer to `StepPolicy`.
    ExtraEvaluation,
    /// Two step policies, each the same events as the reference within the plan's distance (I8).
    StepPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayFailure {
    /// The first accepted instant at which a run left its reference.
    Diverged {
        gate: ReplayGate,
        first_divergence: SimTime,
    },
    /// The core could not finish the run the gate needed.
    Faulted { gate: ReplayGate, fault: EngineFault },
    /// A case or a core a session would not admit.
    Admission {
        gate: ReplayGate,
        failure: InitializationFailure,
    },
    /// A timeline input the case could not build, or `enqueue` refused at the head the gate chose.
    InputRefused { gate: ReplayGate, error: AdmissionError },
    /// A checkpoint that could not be saved or loaded.
    Checkpoint {
        gate: ReplayGate,
        error: CheckpointError,
    },
    /// An input with no head after admission to arrive at, or no step left to apply it: the run would be the
    /// known run by construction, and a gate that tests nothing passes nothing.
    NoLateBoundary { id: InputId },
    /// One event, by its stream and ordinal, further from the reference than the plan allows.
    EventMoved {
        stream: LawName,
        ordinal: u64,
        apart_ns: u64,
    },
    /// A policy that fired a different list of events than the reference.
    EventsDiffer { reference: usize, policy: usize },
    /// A plan or a case that gives the gate nothing to compare, a pace that never stopped a run short of its
    /// end, a clock that stopped before it, a gate that measured another run or another course, a body with a
    /// hazard guard whose run fired nothing, or two step policies on one grid.
    MissingEvidence { gate: ReplayGate },
}

/// `runs` holds the reference run first, then each run the gate compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateEvidence {
    gate: ReplayGate,
    run: Digest,
    runs: Vec<RecordDigests>,
    clock: Option<ClockSource>,
    course: Seal,
    fired: Vec<Fired>,
    tolerance_ns: Option<u64>,
}
impl GateEvidence {
    pub fn gate(&self) -> ReplayGate {
        self.gate
    }
    pub fn run(&self) -> Digest {
        self.run
    }
    pub fn runs(&self) -> &[RecordDigests] {
        &self.runs
    }
    /// What paced the live run, for the pace gate.
    pub fn clock(&self) -> Option<ClockSource> {
        self.clock
    }
    pub fn fired(&self) -> &[Fired] {
        &self.fired
    }
    pub fn tolerance_ns(&self) -> Option<u64> {
        self.tolerance_ns
    }
}

/// No public constructor, so evidence comes only from `replay_gates`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayEvidence {
    run: Digest,
    pace: GateEvidence,
    resume: GateEvidence,
    late_admission: GateEvidence,
    rewind: GateEvidence,
    extra_evaluation: GateEvidence,
    step_policy: GateEvidence,
}
impl ReplayEvidence {
    pub fn run(&self) -> Digest {
        self.run
    }
    pub fn of(&self, gate: ReplayGate) -> &GateEvidence {
        match gate {
            ReplayGate::Pace => &self.pace,
            ReplayGate::Resume => &self.resume,
            ReplayGate::LateAdmission => &self.late_admission,
            ReplayGate::Rewind => &self.rewind,
            ReplayGate::ExtraEvaluation => &self.extra_evaluation,
            ReplayGate::StepPolicy => &self.step_policy,
        }
    }
}

/// A seal covers every accepted part, so comparing seals per step needs no rehash of the ledger.
struct Row {
    time: SimTime,
    seal: Seal,
    earliest: SimTime,
    near_a_break: bool,
    fired: Vec<Fired>,
}
impl Row {
    fn of(session: &Session<'_, Running>, near_a_break: bool) -> Self {
        let record = session.record();
        Self {
            time: record.accepted().time(),
            seal: record.seal(),
            earliest: record.earliest_stamp(),
            near_a_break,
            fired: Vec::new(),
        }
    }
}

fn fired(rows: &[Row]) -> Vec<Fired> {
    rows.iter().flat_map(|row| row.fired.iter().copied()).collect()
}

struct Trace {
    rows: Vec<Row>,
    last: RecordDigests,
    seal: Seal,
}

/// Two runs to one end with equal seals have equal rows, so a shared prefix is the whole of each.
fn first_difference(reference: &[Row], run: &[Row]) -> Option<SimTime> {
    (reference.iter().zip(run))
        .find(|(known, seen)| known.seal != seen.seal)
        .map(|(known, seen)| known.time.min(seen.time))
}

/// The end's digests also read what waits and are recomputed from the live fields, so a part a seal failed
/// to cover still shows.
fn first_divergence(
    known: &[Row],
    expected: &RecordDigests,
    rows: &[Row],
    last: &RecordDigests,
) -> Option<SimTime> {
    first_difference(known, rows).or_else(|| (expected != last).then_some(last.time))
}

fn advance<'m>(
    gate: ReplayGate,
    session: Session<'m, Running>,
    rows: &mut Vec<Row>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    let mut refused = false;
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, receipt) => {
                let fired = receipt.fired().to_vec();
                let near_a_break = refused || receipt.after_breakpoint() || !fired.is_empty();
                let mut row = Row::of(&session, near_a_break);
                row.fired = fired;
                rows.push(row);
                return Ok(session);
            }
            Step::Refused(session) => {
                refused = true;
                session.retry()
            }
            Step::Faulted(stopped) => {
                let fault = *stopped.fault();
                return Err(ReplayFailure::Faulted { gate, fault });
            }
        };
    }
}

fn step_to<'m>(
    gate: ReplayGate,
    mut session: Session<'m, Running>,
    until: SimTime,
    rows: &mut Vec<Row>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    while session.record().accepted().time() < until {
        session = advance(gate, session, rows)?;
    }
    Ok(session)
}

fn revision_to<'m>(
    gate: ReplayGate,
    mut session: Session<'m, Running>,
    revision: u64,
    rows: &mut Vec<Row>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    while session.record().revision() < revision {
        session = advance(gate, session, rows)?;
    }
    Ok(session)
}

fn traced<'m>(
    gate: ReplayGate,
    mut session: Session<'m, Running>,
    mut rows: Vec<Row>,
    end: SimTime,
    pace: &Pace<'_>,
) -> Result<(Session<'m, Running>, Trace), ReplayFailure> {
    let head = |session: &Session<'_, Running>| session.record().accepted().time();
    let mut stopped_short = false;
    match pace {
        Pace::Unbounded => session = step_to(gate, session, end, &mut rows)?,
        Pace::Chunked(lengths) => {
            let mut stop = head(&session).nanoseconds();
            for length in lengths.iter().cycle() {
                if head(&session) >= end {
                    break;
                }
                stop = stop.saturating_add(*length);
                let before = rows.len();
                session = step_to(gate, session, SimTime::from_ns(stop).min(end), &mut rows)?;
                stopped_short |= rows.len() > before && head(&session) < end;
            }
        }
        Pace::Live {
            clock,
            simulated_per_wall,
        } => {
            let origin = head(&session).nanoseconds();
            while head(&session) < end {
                let reading = clock
                    .elapsed_ns()
                    .ok_or(ReplayFailure::MissingEvidence { gate })?;
                let allowed = origin.saturating_add(reading.saturating_mul(*simulated_per_wall));
                let before = rows.len();
                session = step_to(gate, session, SimTime::from_ns(allowed).min(end), &mut rows)?;
                stopped_short |= rows.len() > before && head(&session) < end;
            }
        }
    }
    if !stopped_short && !matches!(pace, Pace::Unbounded) {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let (last, seal) = (session.record().digests(), session.record().seal());
    Ok((session, Trace { rows, last, seal }))
}

fn admitted<'m>(
    gate: ReplayGate,
    session: Result<Session<'m, Running>, InitializationFailure>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    session.map_err(|failure| ReplayFailure::Admission { gate, failure })
}

fn reference(
    gate: ReplayGate,
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
) -> Result<(Trace, Digest), ReplayFailure> {
    let core = cores.fresh();
    let session = admitted(gate, case.admit(&*core))?;
    let rows = vec![Row::of(&session, false)];
    let (session, trace) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    let fired_nothing = case.body().hazards().next().is_some() && fired(&trace.rows).is_empty();
    if trace.rows.len() < 2 || fired_nothing {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    Ok((trace, session.record().identity().digest()))
}

fn same_or_diverged(
    gate: ReplayGate,
    known: &[Row],
    expected: &RecordDigests,
    run: &Trace,
) -> Result<(), ReplayFailure> {
    match first_divergence(known, expected, &run.rows, &run.last) {
        None => Ok(()),
        Some(first_divergence) => Err(ReplayFailure::Diverged {
            gate,
            first_divergence,
        }),
    }
}

fn pace_gate(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::Pace;
    if plan.chunks.is_empty() || plan.chunks.contains(&0) || plan.simulated_per_wall == 0 {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let (reference, run) = reference(gate, case, cores)?;
    let mut runs = vec![reference.last];
    let live = Pace::Live {
        clock: plan.clock,
        simulated_per_wall: plan.simulated_per_wall,
    };
    for pace in [Pace::Chunked(&plan.chunks), live] {
        let core = cores.fresh();
        let session = admitted(gate, case.admit(&*core))?;
        let rows = vec![Row::of(&session, false)];
        let (_, paced) = traced(gate, session, rows, case.end(), &pace)?;
        same_or_diverged(gate, &reference.rows, &reference.last, &paced)?;
        runs.push(paced.last);
    }
    Ok(GateEvidence {
        gate,
        run,
        runs,
        clock: Some(plan.clock.source()),
        course: reference.seal,
        fired: fired(&reference.rows),
        tolerance_ns: None,
    })
}

#[derive(Default)]
struct MemoryStore {
    links: Vec<(Digest, Vec<u8>)>,
}
impl HistoryStore for MemoryStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let at = self.links.binary_search_by_key(seal, |link| link.0).ok()?;
        Some(self.links[at].1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        if let Err(at) = self.links.binary_search_by_key(&seal, |link| link.0) {
            self.links.insert(at, (seal, link));
        }
        Ok(())
    }
}

/// Each side of a step that started at a breakpoint, met a refusal or fired is a boundary a resume is likeliest
/// to lose. Neighbouring revisions are never both multiples of a hidden state's period, whatever `every` is.
fn boundaries(rows: &[Row], every: u64) -> Vec<u64> {
    let last = rows.len() as u64 - 1;
    let mut revisions = Vec::new();
    for (revision, row) in (0..).zip(rows) {
        if revision % every == 0 {
            revisions.extend([revision, (revision + 1).min(last)]);
        }
        if row.near_a_break {
            revisions.extend([revision - 1, revision, (revision + 1).min(last)]);
        }
    }
    revisions.sort_unstable();
    revisions.dedup();
    revisions
}

/// Every load runs to the end, because a lost state can show many steps after it. `resume_every` sets the
/// cost: each load costs the rest of a run.
fn resume_gate(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::Resume;
    let checkpointed = |error| ReplayFailure::Checkpoint { gate, error };
    if plan.resume_every == 0 {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let (reference, run) = reference(gate, case, cores)?;
    let core = cores.fresh();
    let mut store = MemoryStore::default();
    let mut session = admitted(gate, case.admit(&*core))?;
    let mut rows = Vec::new();
    let mut saved = Vec::new();
    for revision in boundaries(&reference.rows, plan.resume_every) {
        session = revision_to(gate, session, revision, &mut rows)?;
        saved.push((revision, session.checkpoint(&mut store).map_err(checkpointed)?));
    }
    let identity = session.record().identity();
    let mut runs = vec![reference.last];
    for (revision, checkpoint) in &saved {
        let fresh = cores.fresh();
        let resumed = Session::resume(checkpoint, identity, case.body(), &*fresh, &store);
        let resumed = resumed.map_err(checkpointed)?;
        let rows = vec![Row::of(&resumed, false)];
        let (_, rest) = traced(gate, resumed, rows, case.end(), &Pace::Unbounded)?;
        let known = &reference.rows[*revision as usize..];
        same_or_diverged(gate, known, &reference.last, &rest)?;
        runs.push(rest.last);
    }
    Ok(GateEvidence {
        gate,
        run,
        runs,
        clock: None,
        course: reference.seal,
        fired: fired(&reference.rows),
        tolerance_ns: None,
    })
}

/// The last head that still takes `event`, read off a run that does not know it: its own earliest stamps, not
/// the known run's. A fence in the known run comes from a cut only it showed, so its heads hide an input the
/// late run would have taken after a landing, which is the arrival a landing puts at risk. Admission is no
/// arrival, so the first head is the earliest, and an input at that head's stamp can still arrive there.
fn arrival(rows: &[Row], event: &BoundaryEvent) -> u64 {
    let last = rows.iter().rposition(|row| row.earliest <= event.at());
    last.unwrap_or(0).max(1) as u64
}

fn late_run<'m>(
    gate: ReplayGate,
    case: &'m dyn ReplayCase,
    core: &'m dyn CoupledSolver,
    arrivals: &[u64],
    rows: &mut Vec<Row>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    let mut session = admitted(gate, case.admit_withheld(core))?;
    rows.push(Row::of(&session, false));
    let events = stamped(gate, case)?;
    for (revision, event) in arrivals.iter().zip(events) {
        session = revision_to(gate, session, *revision, rows)?;
        if session.record().earliest_stamp() > event.at() {
            return Err(ReplayFailure::NoLateBoundary { id: event.id() });
        }
        let command = Digest::of_canonical(b"replay gate");
        let input = AuthorizedInput::from_command_receipt(command, event);
        let refused = |error| ReplayFailure::InputRefused { gate, error };
        session.enqueue(input).map_err(refused)?;
    }
    Ok(session)
}

fn stamped(gate: ReplayGate, case: &dyn ReplayCase) -> Result<Vec<BoundaryEvent>, ReplayFailure> {
    let mut events = case
        .events()
        .map_err(|error| ReplayFailure::InputRefused { gate, error })?;
    events.sort_by_key(|event| (event.at(), event.id()));
    Ok(events)
}

/// Costs one run per input and one more: each input's arrival is read off the run that holds every input
/// before it at its own arrival.
fn late_admission_gate(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::LateAdmission;
    let (reference, run) = reference(gate, case, cores)?;
    let events = stamped(gate, case)?;
    if events.is_empty() {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let mut arrivals: Vec<u64> = Vec::new();
    for event in &events {
        if event.at() >= reference.last.time {
            return Err(ReplayFailure::NoLateBoundary { id: event.id() });
        }
        let core = cores.fresh();
        let mut rows = Vec::new();
        let session = late_run(gate, case, &*core, &arrivals, &mut rows)?;
        let (_, withheld) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
        arrivals.push(arrival(&withheld.rows, event));
    }
    let core = cores.fresh();
    let mut rows = Vec::new();
    let session = late_run(gate, case, &*core, &arrivals, &mut rows)?;
    let (_, late) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    same_or_diverged(gate, &reference.rows, &reference.last, &late)?;
    Ok(GateEvidence {
        gate,
        run,
        runs: vec![reference.last, late.last],
        clock: None,
        course: reference.seal,
        fired: fired(&reference.rows),
        tolerance_ns: None,
    })
}

/// The rewind keeps the parent's core object on purpose: a state it kept while the parent ran ahead has
/// nowhere to hide. It runs from two neighbouring revisions, so a hidden period cannot land on both.
fn rewind_gate(case: &dyn ReplayCase, cores: &dyn CoreFactory) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::Rewind;
    let checkpointed = |error| ReplayFailure::Checkpoint { gate, error };
    let (reference, run) = reference(gate, case, cores)?;
    let middle = (reference.rows.len() - 1) / 2;
    let core = cores.fresh();
    let mut store = MemoryStore::default();
    let session = admitted(gate, case.admit(&*core))?;
    let mut rows = vec![Row::of(&session, false)];
    let session = revision_to(gate, session, middle as u64, &mut rows)?;
    let first = session.checkpoint(&mut store).map_err(checkpointed)?;
    let session = revision_to(gate, session, middle as u64 + 1, &mut rows)?;
    let second = session.checkpoint(&mut store).map_err(checkpointed)?;
    let (parent, ahead) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    same_or_diverged(gate, &reference.rows, &reference.last, &ahead)?;
    let identity = parent.record().identity();
    let mut runs = vec![reference.last, ahead.last];
    let forks: [(&[u8], _, _); 2] = [
        (b"replay gate fork", &first, middle),
        (b"replay gate later fork", &second, middle + 1),
    ];
    for (name, saved, revision) in forks {
        let branch = BranchId::new(Digest::of_canonical(name));
        let forked = parent.rewind(saved, branch).map_err(checkpointed)?;
        let rows = vec![Row::of(&forked, false)];
        let (_, fork) = traced(gate, forked, rows, case.end(), &Pace::Unbounded)?;
        let fresh = cores.fresh();
        let loaded = Session::resume(saved, identity, case.body(), &*fresh, &store);
        let loaded = loaded.map_err(checkpointed)?;
        let rows = vec![Row::of(&loaded, false)];
        let (_, load) = traced(gate, loaded, rows, case.end(), &Pace::Unbounded)?;
        same_or_diverged(gate, &load.rows, &load.last, &fork)?;
        same_or_diverged(gate, &ahead.rows[revision..], &ahead.last, &fork)?;
        runs.extend([fork.last, load.last]);
    }
    Ok(GateEvidence {
        gate,
        run,
        runs,
        clock: None,
        course: reference.seal,
        fired: fired(&reference.rows),
        tolerance_ns: None,
    })
}

/// Its resolution is its inner core's, so the run identity holds and seals compare.
struct SolvesTwice<'c> {
    inner: Box<dyn CoupledSolver + 'c>,
}
impl CoupledSolver for SolvesTwice<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let _ = self.inner.solve(problem, ticket);
        self.inner.solve(problem, ticket)
    }
}

fn extra_evaluation_gate(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::ExtraEvaluation;
    let (reference, run) = reference(gate, case, cores)?;
    let forced = SolvesTwice { inner: cores.fresh() };
    let session = admitted(gate, case.admit(&forced))?;
    let rows = vec![Row::of(&session, false)];
    let (_, twice) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    same_or_diverged(gate, &reference.rows, &reference.last, &twice)?;
    Ok(GateEvidence {
        gate,
        run,
        runs: vec![reference.last, twice.last],
        clock: None,
        course: reference.seal,
        fired: fired(&reference.rows),
        tolerance_ns: None,
    })
}

fn pair<'f>(event: &Fired, list: &'f [Fired]) -> Option<&'f Fired> {
    (list.iter()).find(|other| other.stream() == event.stream() && other.ordinal() == event.ordinal())
}

/// Pairs by stream and ordinal, because two unrelated events a floor apart can share a landing under a coarser
/// policy, and then the lists differ in order and in nothing else. An event with no pair within the tolerance of
/// the case's end is an honest straddle: one policy fired it just before the end.
fn same_events(
    reference: &[Fired],
    policy: &[Fired],
    tolerance_ns: u64,
    end: SimTime,
) -> Result<(), ReplayFailure> {
    let differ = ReplayFailure::EventsDiffer {
        reference: reference.len(),
        policy: policy.len(),
    };
    let unpaired = |event: &Fired| event.at().nanoseconds().saturating_add(tolerance_ns) < end.nanoseconds();
    for known in reference {
        match pair(known, policy) {
            Some(seen) => {
                let apart_ns = known.at().nanoseconds().abs_diff(seen.at().nanoseconds());
                if apart_ns > tolerance_ns {
                    return Err(ReplayFailure::EventMoved {
                        stream: known.stream(),
                        ordinal: known.ordinal(),
                        apart_ns,
                    });
                }
            }
            None if unpaired(known) => return Err(differ),
            None => {}
        }
    }
    let extra = |seen: &&Fired| pair(seen, reference).is_none();
    if policy.iter().filter(extra).any(unpaired) {
        return Err(differ);
    }
    Ok(())
}

/// Two policies have two identities, so nothing here compares bits. Each is held to the reference integrator,
/// because a core that biases every step alike agrees with itself.
fn step_policy_gate(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::StepPolicy;
    let (first, run) = reference(gate, case, cores)?;
    let Some(chance) = plan.chance.as_ref() else {
        if case.body().hazards().next().is_some() {
            return Err(ReplayFailure::MissingEvidence { gate });
        }
        return Ok(GateEvidence {
            gate,
            run,
            runs: vec![first.last],
            clock: None,
            course: first.seal,
            fired: Vec::new(),
            tolerance_ns: None,
        });
    };
    let (second, _) = reference(gate, case, chance.second_policy)?;
    let grid = |rows: &[Row]| rows.iter().map(|row| row.time).collect::<Vec<_>>();
    if grid(&first.rows) == grid(&second.rows) {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let (known, _) = reference(gate, case, &chance.reference)?;
    let events = fired(&known.rows);
    for policy in [&first, &second] {
        same_events(
            &events,
            &fired(&policy.rows),
            chance.event_tolerance_ns,
            case.end(),
        )?;
    }
    Ok(GateEvidence {
        gate,
        run,
        runs: vec![first.last, second.last, known.last],
        clock: None,
        course: first.seal,
        fired: fired(&first.rows),
        tolerance_ns: Some(chance.event_tolerance_ns),
    })
}

pub fn replay_gate(
    gate: ReplayGate,
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<GateEvidence, ReplayFailure> {
    match gate {
        ReplayGate::Pace => pace_gate(case, cores, plan),
        ReplayGate::Resume => resume_gate(case, cores, plan),
        ReplayGate::LateAdmission => late_admission_gate(case, cores),
        ReplayGate::Rewind => rewind_gate(case, cores),
        ReplayGate::ExtraEvaluation => extra_evaluation_gate(case, cores),
        ReplayGate::StepPolicy => step_policy_gate(case, cores, plan),
    }
}

pub fn replay_gates(
    case: &dyn ReplayCase,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<ReplayEvidence, ReplayFailure> {
    let pace = replay_gate(ReplayGate::Pace, case, cores, plan)?;
    let (run, course) = (pace.run, pace.course);
    let of_this_run = |gate| {
        let evidence = replay_gate(gate, case, cores, plan)?;
        if evidence.course == course {
            Ok(evidence)
        } else {
            Err(ReplayFailure::MissingEvidence { gate })
        }
    };
    Ok(ReplayEvidence {
        run,
        pace,
        resume: of_this_run(ReplayGate::Resume)?,
        late_admission: of_this_run(ReplayGate::LateAdmission)?,
        rewind: of_this_run(ReplayGate::Rewind)?,
        extra_evaluation: of_this_run(ReplayGate::ExtraEvaluation)?,
        step_policy: of_this_run(ReplayGate::StepPolicy)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Delivery, ReferenceIntegrator, TwoReservoirs};
    use ng_contract::{BoundaryInput, Measure, WaterDelivery};

    #[test]
    fn a_trace_that_differs_only_in_what_waits_has_diverged() {
        let delivery = Delivery {
            id: b"waits past the end",
            from: SimTime::from_ns(10),
            to: SimTime::from_ns(20),
            amount: Measure::new(0.0).expect("finite"),
        };
        let start = || Measure::new(1_000.0).expect("finite");
        let case = TwoReservoirs::delivering([start(), start()], vec![delivery]).expect("a body");
        let core = ReferenceIntegrator::new(1_000_000_000);
        let session = case.admit(&core).expect("admitted");
        let withheld = case.admit_withheld(&core).expect("admitted");
        let trace = |session: &Session<'_, Running>| Trace {
            rows: vec![Row::of(session, false)],
            last: session.record().digests(),
            seal: session.record().seal(),
        };
        let (reference, run) = (trace(&session), trace(&withheld));
        let compare = |run: &Trace| first_divergence(&reference.rows, &reference.last, &run.rows, &run.last);
        assert_eq!(compare(&reference), None);
        assert_eq!(compare(&run), Some(run.last.time));
    }

    #[test]
    fn an_input_arrives_at_the_last_head_that_still_takes_it_and_never_at_admission() {
        let start = || Measure::new(1_000.0).expect("finite");
        let case = TwoReservoirs::delivering([start(), start()], Vec::new()).expect("a body");
        let core = ReferenceIntegrator::new(1_000_000_000);
        let session = case.admit(&core).expect("admitted");
        let head = |earliest| Row {
            earliest: SimTime::from_ns(earliest),
            ..Row::of(&session, false)
        };
        let rows = [head(1), head(10), head(20)];
        let event = |stamp| {
            let water = WaterDelivery {
                boundary: "exterior",
                amount: Measure::new(1.0).expect("finite"),
                ends: SimTime::from_ns(stamp + 1),
            };
            let id = InputId::new(Digest::of_canonical(b"an input"));
            BoundaryEvent::new(id, SimTime::from_ns(stamp), BoundaryInput::Water(water))
        };
        let arrivals = [1, 9, 10, 19, 20, 21].map(|stamp| arrival(&rows, &event(stamp)));
        assert_eq!(arrivals, [1, 1, 1, 1, 2, 2]);
    }
}
