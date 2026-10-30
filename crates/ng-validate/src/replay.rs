use crate::TwoReservoirs;
use ng_contract::{
    AdmissionError, BoundaryEvent, CoupledSolver, Digest, EngineFault, InitializationFailure, InputId,
    SimTime,
};
use ng_record::{
    AuthorizedInput, BranchId, CheckpointError, HistoryStore, RecordDigests, Running, Seal, Session, Step,
};

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

pub struct ReplayPlan<'c> {
    pub clock: &'c dyn WallClock,
    /// Simulated ns per wall ns under `clock`.
    pub simulated_per_wall: u64,
    /// Lengths of simulated time, each after the last, repeating to the end of the case.
    pub chunks: Vec<u64>,
    /// Resume at each pair of neighbouring revisions this many apart, and on each side of a breakpoint or a
    /// refusal.
    pub resume_every: u64,
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
    /// A plan or a case that gives the gate nothing to compare, a pace that never stopped a run short of its
    /// end, a clock that stopped before it, or a gate that measured another run.
    MissingEvidence { gate: ReplayGate },
}

/// `runs` holds the reference run first, then each run the gate compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateEvidence {
    gate: ReplayGate,
    run: Digest,
    runs: Vec<RecordDigests>,
    clock: Option<ClockSource>,
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
}

/// No public constructor, so evidence comes only from `replay_gates`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayEvidence {
    run: Digest,
    pace: GateEvidence,
    resume: GateEvidence,
    late_admission: GateEvidence,
    rewind: GateEvidence,
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
        }
    }
}

/// A seal covers every accepted part, so comparing seals per step needs no rehash of the ledger.
struct Row {
    time: SimTime,
    seal: Seal,
    earliest: SimTime,
    near_a_break: bool,
}
impl Row {
    fn of(session: &Session<'_, Running>, near_a_break: bool) -> Self {
        let record = session.record();
        Self {
            time: record.accepted().time(),
            seal: record.seal(),
            earliest: record.earliest_stamp(),
            near_a_break,
        }
    }
}

struct Trace {
    rows: Vec<Row>,
    last: RecordDigests,
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
                rows.push(Row::of(&session, refused || receipt.after_breakpoint()));
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
            while head(&session) < end {
                let reading = clock
                    .elapsed_ns()
                    .ok_or(ReplayFailure::MissingEvidence { gate })?;
                let allowed = SimTime::from_ns(reading.saturating_mul(*simulated_per_wall));
                let before = rows.len();
                session = step_to(gate, session, allowed.min(end), &mut rows)?;
                stopped_short |= rows.len() > before && head(&session) < end;
            }
        }
    }
    if !stopped_short && !matches!(pace, Pace::Unbounded) {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let last = session.record().digests();
    Ok((session, Trace { rows, last }))
}

fn admitted<'m>(
    gate: ReplayGate,
    session: Result<Session<'m, Running>, InitializationFailure>,
) -> Result<Session<'m, Running>, ReplayFailure> {
    session.map_err(|failure| ReplayFailure::Admission { gate, failure })
}

fn reference(
    gate: ReplayGate,
    case: &TwoReservoirs,
    cores: &dyn CoreFactory,
) -> Result<(Trace, Digest), ReplayFailure> {
    let core = cores.fresh();
    let session = admitted(gate, case.admit(&*core))?;
    let rows = vec![Row::of(&session, false)];
    let (session, trace) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    if trace.rows.len() < 2 {
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
    case: &TwoReservoirs,
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

/// Each side of a step that started at a breakpoint or met a refusal is a boundary a resume is likeliest to
/// lose. Neighbouring revisions are never both multiples of a hidden state's period, whatever `every` is.
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

/// The first load runs to the end and each other to the next boundary, so the cost stays near two runs and
/// a slow leak of hidden state still shows.
fn resume_gate(
    case: &TwoReservoirs,
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
        let checkpoint = session.checkpoint(&mut store).map_err(checkpointed)?;
        saved.push((revision, checkpoint, session.record().digests()));
    }
    let identity = session.record().identity();
    let mut runs = vec![reference.last];
    for (index, (revision, checkpoint, _)) in saved.iter().enumerate() {
        let fresh = cores.fresh();
        let resumed = Session::resume(checkpoint, identity, case.body(), &*fresh, &store);
        let resumed = resumed.map_err(checkpointed)?;
        let mut rows = vec![Row::of(&resumed, false)];
        let known = &reference.rows[*revision as usize..];
        let next = saved.get(index + 1).filter(|_| index > 0);
        let resumed = match next {
            Some((next, ..)) => revision_to(gate, resumed, *next, &mut rows)?,
            None => step_to(gate, resumed, case.end(), &mut rows)?,
        };
        let last = resumed.record().digests();
        let expected = next.map_or(reference.last, |(_, _, digests)| *digests);
        let leg = Trace { rows, last };
        same_or_diverged(gate, known, &expected, &leg)?;
        runs.push(last);
    }
    Ok(GateEvidence {
        gate,
        run,
        runs,
        clock: None,
    })
}

/// The last head a run can still take `event`, so every step before it was blind: the last reference row
/// whose earliest stamp is before the input's. The next reference step reached the stamp, or was refused
/// short of it after a cut there, and a run that did not know the input would have gone past. Admission is
/// no arrival, so the first head is the earliest, and an input at that head's stamp can still arrive there.
fn arrival(rows: &[Row], event: &BoundaryEvent) -> u64 {
    let last = rows.iter().rposition(|row| row.earliest < event.at());
    last.unwrap_or(0).max(1) as u64
}

fn late_admission_gate(case: &TwoReservoirs, cores: &dyn CoreFactory) -> Result<GateEvidence, ReplayFailure> {
    let gate = ReplayGate::LateAdmission;
    let (reference, run) = reference(gate, case, cores)?;
    let events = case
        .events()
        .map_err(|error| ReplayFailure::InputRefused { gate, error })?;
    if events.is_empty() {
        return Err(ReplayFailure::MissingEvidence { gate });
    }
    let mut arrivals: Vec<(u64, BoundaryEvent)> = Vec::new();
    for event in events {
        if event.at() >= reference.last.time {
            return Err(ReplayFailure::NoLateBoundary { id: event.id() });
        }
        arrivals.push((arrival(&reference.rows, &event), event));
    }
    arrivals.sort_by_key(|arrival| arrival.0);
    let core = cores.fresh();
    let mut session = admitted(gate, case.admit_withheld(&*core))?;
    let mut rows = vec![Row::of(&session, false)];
    for (revision, event) in arrivals {
        session = revision_to(gate, session, revision, &mut rows)?;
        if session.record().earliest_stamp() > event.at() {
            return Err(ReplayFailure::NoLateBoundary { id: event.id() });
        }
        let command = Digest::of_canonical(b"replay gate");
        let input = AuthorizedInput::from_command_receipt(command, event);
        let refused = |error| ReplayFailure::InputRefused { gate, error };
        session.enqueue(input).map_err(refused)?;
    }
    let (_, late) = traced(gate, session, rows, case.end(), &Pace::Unbounded)?;
    same_or_diverged(gate, &reference.rows, &reference.last, &late)?;
    Ok(GateEvidence {
        gate,
        run,
        runs: vec![reference.last, late.last],
        clock: None,
    })
}

/// The rewind keeps the parent's core object on purpose: a state it kept while the parent ran ahead has
/// nowhere to hide. It runs from two neighbouring revisions, so a hidden period cannot land on both.
fn rewind_gate(case: &TwoReservoirs, cores: &dyn CoreFactory) -> Result<GateEvidence, ReplayFailure> {
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
    })
}

pub fn replay_gate(
    gate: ReplayGate,
    case: &TwoReservoirs,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<GateEvidence, ReplayFailure> {
    match gate {
        ReplayGate::Pace => pace_gate(case, cores, plan),
        ReplayGate::Resume => resume_gate(case, cores, plan),
        ReplayGate::LateAdmission => late_admission_gate(case, cores),
        ReplayGate::Rewind => rewind_gate(case, cores),
    }
}

pub fn replay_gates(
    case: &TwoReservoirs,
    cores: &dyn CoreFactory,
    plan: &ReplayPlan<'_>,
) -> Result<ReplayEvidence, ReplayFailure> {
    let pace = replay_gate(ReplayGate::Pace, case, cores, plan)?;
    let run = pace.run;
    let of_this_run = |gate| {
        let evidence = replay_gate(gate, case, cores, plan)?;
        if evidence.run == run {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Delivery, ReferenceIntegrator};
    use ng_contract::Measure;

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
        };
        let (reference, run) = (trace(&session), trace(&withheld));
        let compare = |run: &Trace| first_divergence(&reference.rows, &reference.last, &run.rows, &run.last);
        assert_eq!(compare(&reference), None);
        assert_eq!(compare(&run), Some(run.last.time));
    }
}
