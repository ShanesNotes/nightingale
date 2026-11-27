use crate::certificate::{
    Certificate, CertificateCheck, CertificateFailure, CertificateFold, CertificatePolicy, DriftTally,
};
use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::num::NonZeroU64;
use ng_contract::{CompiledBody, CoupledSolver, Digest, EngineFault, RecordHash, RunIdentity, SimTime};
use ng_record::{
    Checkpoint, CheckpointError, FaultReport, HistoryStore, RetainedSpan, Running, Session, Step, Witness,
    retained_span,
};

const MINUTE_NS: u64 = 60_000_000_000;

/// One step's 96-byte persisted witness record: its seal, its image digest and its ledger digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessEntry {
    pub seal: Digest,
    pub image: Digest,
    pub ledger: Digest,
}

const _: () = assert!(size_of::<WitnessEntry>() == 96);

/// One checkpoint saved during a run together with its revision, time and head seal.
pub struct SavedCheckpoint {
    pub revision: u64,
    pub time: SimTime,
    pub seal: Digest,
    pub checkpoint: Checkpoint,
}
impl Clone for SavedCheckpoint {
    fn clone(&self) -> Self {
        Self {
            revision: self.revision,
            time: self.time,
            seal: self.seal,
            checkpoint: Checkpoint::from_bytes(self.checkpoint.as_bytes().to_vec()),
        }
    }
}

/// The first failure condition that latched the flight recorder (record 0008).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightRecorderTrigger {
    CertificateFailure(CertificateCheck),
    ReplayDivergence,
    EngineFault(EngineFault),
    StoreFailure { seal: Digest },
}

/// The contiguous span of original links preserved from the checkpoint before a failure to the last head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlightRecorderSpan {
    pub trigger: FlightRecorderTrigger,
    pub from_checkpoint_revision: u64,
    pub from_checkpoint_time: SimTime,
    pub from_checkpoint_seal: Digest,
    pub last_head_revision: u64,
    pub last_head_seal: Digest,
}

/// Inventory of everything a `RollingStore` currently keeps and its exact byte counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreListing {
    pub witness_count: usize,
    pub witness_bytes: usize,
    pub checkpoint_revisions: Vec<u64>,
    pub checkpoint_bytes: usize,
    pub complete_link_revisions: Vec<u64>,
    pub complete_link_bytes: usize,
    pub flight_recorder: Option<FlightRecorderSpan>,
}

struct RollingState {
    origin: Option<Digest>,
    witnesses: Vec<WitnessEntry>,
    witness_index: BTreeMap<Digest, usize>,
    fork_previous: BTreeMap<usize, Digest>,
    seal_to_rev: BTreeMap<Digest, u64>,
    links: BTreeMap<u64, (Digest, Vec<u8>)>,
    link_by_seal: BTreeMap<Digest, u64>,
    checkpoints: Vec<SavedCheckpoint>,
    flight_recorder: Option<FlightRecorderSpan>,
    fail_next_keep: bool,
    resolve_calls: usize,
}
impl RollingState {
    fn new() -> Self {
        Self {
            origin: None,
            witnesses: Vec::new(),
            witness_index: BTreeMap::new(),
            fork_previous: BTreeMap::new(),
            seal_to_rev: BTreeMap::new(),
            links: BTreeMap::new(),
            link_by_seal: BTreeMap::new(),
            checkpoints: Vec::new(),
            flight_recorder: None,
            fail_next_keep: false,
            resolve_calls: 0,
        }
    }
    fn checkpoint_for(&self, instant: Option<SimTime>) -> Option<&SavedCheckpoint> {
        match instant {
            Some(at) => (self.checkpoints.iter().rev())
                .find(|saved| saved.time <= at)
                .or_else(|| self.checkpoints.first()),
            None => self.checkpoints.last(),
        }
    }
    fn latch(
        &mut self,
        trigger: FlightRecorderTrigger,
        window_start: Option<SimTime>,
        last_head_revision: u64,
        last_head_seal: Digest,
    ) -> Option<FlightRecorderSpan> {
        if let Some(span) = &mut self.flight_recorder {
            if last_head_revision >= span.last_head_revision {
                span.last_head_revision = last_head_revision;
                span.last_head_seal = last_head_seal;
            }
            return Some(span.clone());
        }
        let base = self.checkpoint_for(window_start)?;
        let span = FlightRecorderSpan {
            trigger,
            from_checkpoint_revision: base.revision,
            from_checkpoint_time: base.time,
            from_checkpoint_seal: base.seal,
            last_head_revision,
            last_head_seal,
        };
        self.flight_recorder = Some(span.clone());
        Some(span)
    }
    fn latest_kept(&self) -> (u64, Digest) {
        if let Some((&rev, &(seal, _))) = self.links.iter().next_back() {
            return (rev, seal);
        }
        if let Some(ckpt) = self.checkpoints.last() {
            return (ckpt.revision, ckpt.seal);
        }
        (0, self.origin.unwrap_or_else(|| Digest::of_canonical(b"")))
    }
}

/// A trimming history store that keeps 96-byte witnesses for every step, periodic checkpoints with their
/// head links, a rolling window of complete links while the run goes, and a flight recorder span on failure.
pub struct RollingStore {
    state: RefCell<RollingState>,
}
impl Default for RollingStore {
    fn default() -> Self {
        Self::new()
    }
}
impl RollingStore {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(RollingState::new()),
        }
    }
    /// Forces the next `keep` or `keep_with_witness` call to fail with `StoreFailed` and latch the flight
    /// recorder.
    pub fn fail_next_keep(&self) {
        self.state.borrow_mut().fail_next_keep = true;
    }
    pub fn record_checkpoint(&self, session: &Session<'_, Running>) -> Result<(), CheckpointError> {
        let checkpoint = session.checkpoint()?;
        let record = session.record();
        let saved = SavedCheckpoint {
            revision: record.revision(),
            time: record.accepted().time(),
            seal: record.seal().digest(),
            checkpoint,
        };
        self.state.borrow_mut().checkpoints.push(saved);
        Ok(())
    }
    pub fn record_exported_checkpoint(
        &self,
        revision: u64,
        time: SimTime,
        seal: Digest,
        checkpoint: Checkpoint,
    ) {
        self.state.borrow_mut().checkpoints.push(SavedCheckpoint {
            revision,
            time,
            seal,
            checkpoint,
        });
    }
    pub fn checkpoint_before(&self, instant: SimTime) -> Option<SavedCheckpoint> {
        self.state.borrow().checkpoint_for(Some(instant)).cloned()
    }
    pub fn checkpoint_at_or_before_revision(&self, revision: u64) -> Option<SavedCheckpoint> {
        let state = self.state.borrow();
        (state.checkpoints.iter().rev())
            .find(|saved| saved.revision <= revision)
            .cloned()
    }
    pub fn checkpoint_at_revision(&self, revision: u64) -> Option<SavedCheckpoint> {
        let state = self.state.borrow();
        (state.checkpoints.iter())
            .find(|saved| saved.revision == revision)
            .cloned()
    }
    pub fn latch_flight_recorder(
        &self,
        trigger: FlightRecorderTrigger,
        window_start: Option<SimTime>,
        last_head_revision: u64,
        last_head_seal: Digest,
    ) -> Option<FlightRecorderSpan> {
        self.state
            .borrow_mut()
            .latch(trigger, window_start, last_head_revision, last_head_seal)
    }
    /// Trims complete links strictly older than the checkpoint before `oldest_open_window_start`, while
    /// keeping links still required by `oldest_retained_revision` and any latched flight recorder span.
    pub fn trim_before(
        &self,
        oldest_open_window_start: Option<SimTime>,
        oldest_retained_revision: Option<u64>,
    ) {
        let mut state = self.state.borrow_mut();
        let window_floor = oldest_open_window_start.map(|start| {
            state
                .checkpoint_for(Some(start))
                .map_or(1, |saved| saved.revision.max(1))
        });
        let mut floor = match (window_floor, oldest_retained_revision) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if let Some(span) = &state.flight_recorder {
            let fr_floor = span.from_checkpoint_revision + 1;
            floor = Some(floor.map_or(fr_floor, |current| current.min(fr_floor)));
        }
        let Some(floor) = floor else {
            return;
        };
        let drop_revs: Vec<u64> = state.links.range(..floor).map(|(&rev, _)| rev).collect();
        for rev in drop_revs {
            if let Some((seal, _)) = state.links.remove(&rev) {
                state.link_by_seal.remove(&seal);
            }
        }
    }
    /// Drops all complete links outside any latched flight recorder span, leaving witnesses and checkpoints.
    pub fn finish(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(span) = state.flight_recorder.clone() {
            let keep_from = span.from_checkpoint_revision + 1;
            let keep_to = span.last_head_revision;
            let drop_revs: Vec<u64> = (state.links.keys())
                .copied()
                .filter(|&rev| rev < keep_from || rev > keep_to)
                .collect();
            for rev in drop_revs {
                if let Some((seal, _)) = state.links.remove(&rev) {
                    state.link_by_seal.remove(&seal);
                }
            }
        } else {
            state.links.clear();
            state.link_by_seal.clear();
        }
    }
    pub fn listing(&self) -> StoreListing {
        let state = self.state.borrow();
        let witness_count = state.witnesses.len();
        let witness_bytes = witness_count * size_of::<WitnessEntry>();
        let checkpoint_revisions = state.checkpoints.iter().map(|saved| saved.revision).collect();
        let checkpoint_bytes = state
            .checkpoints
            .iter()
            .map(|saved| saved.checkpoint.as_bytes().len())
            .sum();
        let complete_link_revisions = state.links.keys().copied().collect();
        let complete_link_bytes = state.links.values().map(|(_, bytes)| bytes.len()).sum();
        StoreListing {
            witness_count,
            witness_bytes,
            checkpoint_revisions,
            checkpoint_bytes,
            complete_link_revisions,
            complete_link_bytes,
            flight_recorder: state.flight_recorder.clone(),
        }
    }
    pub fn witnesses(&self) -> Vec<WitnessEntry> {
        self.state.borrow().witnesses.clone()
    }
    pub fn witness_seals(&self) -> Vec<Digest> {
        self.state
            .borrow()
            .witnesses
            .iter()
            .map(|entry| entry.seal)
            .collect()
    }
    pub fn flight_recorder(&self) -> Option<FlightRecorderSpan> {
        self.state.borrow().flight_recorder.clone()
    }
    pub fn retained_flight_recorder<'m>(
        &self,
        body: &'m CompiledBody,
        expected: &RunIdentity,
    ) -> Result<Option<RetainedSpan<'m>>, CheckpointError> {
        let Some(span) = self.flight_recorder() else {
            return Ok(None);
        };
        let saved = self.checkpoint_at_revision(span.from_checkpoint_revision).ok_or(
            CheckpointError::HistoryMissing {
                seal: span.from_checkpoint_seal,
            },
        )?;
        retained_span(
            body,
            expected,
            &saved.checkpoint,
            span.last_head_seal,
            span.last_head_revision,
            self,
        )
        .map(Some)
    }
    pub fn resolve_calls(&self) -> usize {
        self.state.borrow().resolve_calls
    }
    pub fn reset_resolve_calls(&self) {
        self.state.borrow_mut().resolve_calls = 0;
    }
}
impl HistoryStore for RollingStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let mut state = self.state.borrow_mut();
        state.resolve_calls += 1;
        let rev = *state.link_by_seal.get(seal)?;
        state.links.get(&rev).map(|(_, bytes)| bytes.clone())
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        let witness =
            Witness::from_link(RecordHash::Blake2b, link).ok_or(CheckpointError::HistoryAltered { seal })?;
        self.keep_with_witness(seal, link, witness)
    }
    fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
        let state = self.state.borrow();
        let idx = *state.witness_index.get(seal)?;
        let entry = state.witnesses[idx];
        let previous = if let Some(&prev) = state.fork_previous.get(&idx) {
            prev
        } else if idx == 0 {
            state.origin?
        } else {
            state.witnesses[idx - 1].seal
        };
        Some(Witness::new(previous, entry.image, entry.ledger))
    }
    fn keep_with_witness(&self, seal: Digest, link: &[u8], witness: Witness) -> Result<(), CheckpointError> {
        let mut state = self.state.borrow_mut();
        if state.fail_next_keep {
            state.fail_next_keep = false;
            let (last_rev, last_seal) = state.latest_kept();
            state.latch(
                FlightRecorderTrigger::StoreFailure { seal },
                None,
                last_rev,
                last_seal,
            );
            return Err(CheckpointError::StoreFailed { seal });
        }
        if state.witness_index.contains_key(&seal) {
            return Ok(());
        }
        if state.origin.is_none() {
            state.origin = Some(witness.previous());
        }
        let rev = if state.origin == Some(witness.previous()) {
            1
        } else if let Some(&prev_rev) = state.seal_to_rev.get(&witness.previous()) {
            prev_rev + 1
        } else {
            state.witnesses.len() as u64 + 1
        };
        let idx = state.witnesses.len();
        let linear = if idx == 0 {
            state.origin == Some(witness.previous())
        } else {
            state.witnesses[idx - 1].seal == witness.previous()
        };
        if !linear {
            state.fork_previous.insert(idx, witness.previous());
        }
        state.witnesses.push(WitnessEntry {
            seal,
            image: witness.image(),
            ledger: witness.ledger(),
        });
        state.witness_index.insert(seal, idx);
        state.seal_to_rev.insert(seal, rev);
        state.links.insert(rev, (seal, link.to_vec()));
        state.link_by_seal.insert(seal, rev);
        if let Some(span) = &mut state.flight_recorder
            && rev >= span.last_head_revision
        {
            span.last_head_revision = rev;
            span.last_head_seal = seal;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RollingConfig {
    pub checkpoint_every_ns: NonZeroU64,
}
impl Default for RollingConfig {
    fn default() -> Self {
        Self {
            checkpoint_every_ns: NonZeroU64::new(MINUTE_NS).expect("positive minute"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepEvent {
    pub revision: u64,
    pub time: SimTime,
    pub seal: Digest,
    pub after_breakpoint: bool,
    pub excursions: usize,
}

pub enum RollingStatus<'m> {
    Completed(Session<'m, Running>),
    Faulted(FaultReport),
}

pub struct RollingOutcome<'m> {
    pub status: RollingStatus<'m>,
    pub certificate: Result<Certificate, CertificateFailure>,
    pub drift_tally: Option<DriftTally>,
    pub listing: StoreListing,
    pub event_log: Vec<StepEvent>,
}

#[derive(Clone, Copy, Debug)]
pub enum RollingError {
    Certificate(CertificateFailure),
    Checkpoint(CheckpointError),
}

/// Runs `session` until `end` while folding `policy`, saving periodic checkpoints with their head links,
/// trimming links outside the open window, latching the flight recorder on the first failure, and finishing
/// the store at the end of the run.
pub fn run_rolling<'m>(
    mut session: Session<'m, Running>,
    end: SimTime,
    policy: &CertificatePolicy,
    store: &'m RollingStore,
    config: RollingConfig,
) -> Result<RollingOutcome<'m>, RollingError> {
    store
        .record_checkpoint(&session)
        .map_err(RollingError::Checkpoint)?;
    let mut fold = CertificateFold::open(&session, policy).map_err(RollingError::Certificate)?;
    let every = config.checkpoint_every_ns.get();
    let mut next_checkpoint_ns = session
        .record()
        .accepted()
        .time()
        .nanoseconds()
        .saturating_add(every);
    let mut event_log = Vec::new();
    while session.record().accepted().time() < end {
        let mut step = session.step();
        let (next, receipt) = loop {
            match step {
                Step::Accepted(next, receipt) => break (next, receipt),
                Step::Refused(retrying) => step = retrying.retry(),
                Step::Faulted(mut faulted) => {
                    let fault = *faulted.fault();
                    let rev = faulted.record().revision();
                    let seal = faulted.record().seal().digest();
                    let report = faulted.export().map_err(RollingError::Checkpoint)?;
                    store.latch_flight_recorder(FlightRecorderTrigger::EngineFault(fault), None, rev, seal);
                    store.finish();
                    return Ok(RollingOutcome {
                        status: RollingStatus::Faulted(report),
                        certificate: fold.certificate(),
                        drift_tally: fold.drift_tally(),
                        listing: store.listing(),
                        event_log,
                    });
                }
            }
        };
        session = next;
        let rev = session.record().revision();
        let time = session.record().accepted().time();
        let seal = session.record().seal().digest();
        event_log.push(StepEvent {
            revision: rev,
            time,
            seal,
            after_breakpoint: receipt.after_breakpoint(),
            excursions: receipt.excursions().len(),
        });
        fold.advance(&session).expect("same session");
        if let Some(failure) = fold.failure()
            && !matches!(failure.check(), CertificateCheck::History(_))
        {
            store.latch_flight_recorder(
                FlightRecorderTrigger::CertificateFailure(failure.check()),
                failure.window().map(|window| window.start()),
                rev,
                seal,
            );
        }
        if time.nanoseconds() >= next_checkpoint_ns {
            match store.record_checkpoint(&session) {
                Ok(()) => {
                    while next_checkpoint_ns <= time.nanoseconds() {
                        next_checkpoint_ns = next_checkpoint_ns.saturating_add(every);
                    }
                }
                Err(CheckpointError::StoreFailed { seal: failed }) => {
                    store.latch_flight_recorder(
                        FlightRecorderTrigger::StoreFailure { seal: failed },
                        None,
                        rev,
                        seal,
                    );
                }
                Err(other) => return Err(RollingError::Checkpoint(other)),
            }
        }
        store.trim_before(fold.oldest_open_window_start(), fold.oldest_retained_revision());
    }
    let certificate = fold.certificate();
    let drift_tally = fold.drift_tally();
    if let Err(failure) = &certificate
        && !matches!(
            failure.check(),
            CertificateCheck::Coverage | CertificateCheck::UndeclaredRhythm
        )
    {
        store.latch_flight_recorder(
            FlightRecorderTrigger::CertificateFailure(failure.check()),
            failure.window().map(|window| window.start()),
            session.record().revision(),
            session.record().seal().digest(),
        );
    }
    store.finish();
    Ok(RollingOutcome {
        status: RollingStatus::Completed(session),
        certificate,
        drift_tally,
        listing: store.listing(),
        event_log,
    })
}

/// First step at which a regeneration failed to reproduce the stored witnesses (record 0008).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegenerationFinding {
    pub build: &'static str,
    pub revision: u64,
    pub expected_seal: Digest,
    pub actual_seal: Digest,
    pub expected_witness: Witness,
    pub actual_witness: Witness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegenerationError {
    Checkpoint(CheckpointError),
    Fault { revision: u64, fault: EngineFault },
    WitnessMissing { revision: u64 },
    Diverged(Box<RegenerationFinding>),
}

pub struct Regenerated<'m> {
    pub build: &'static str,
    pub session: Session<'m, Running>,
    pub steps_verified: u64,
}

/// Replays forward from a checkpoint at or before `target_revision` on a fresh core, checking each stepped
/// revision against any stored witness.
pub fn replay_to_revision<'m>(
    from_checkpoint: &Checkpoint,
    target_revision: u64,
    expected: &RunIdentity,
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    store: &'m dyn HistoryStore,
) -> Result<Session<'m, Running>, RegenerationError> {
    let mut session = Session::resume(from_checkpoint, expected, body, core, store)
        .map_err(RegenerationError::Checkpoint)?;
    if session.record().revision() > target_revision {
        return Err(RegenerationError::Checkpoint(CheckpointError::NotOnThisBranch));
    }
    let hash = session.record().record_hash();
    while session.record().revision() < target_revision {
        let next_rev = session.record().revision() + 1;
        let mut step = session.step();
        session = loop {
            match step {
                Step::Accepted(next, _) => break next,
                Step::Refused(retrying) => step = retrying.retry(),
                Step::Faulted(faulted) => {
                    return Err(RegenerationError::Fault {
                        revision: next_rev,
                        fault: *faulted.fault(),
                    });
                }
            }
        };
        let actual_seal = session.record().seal().digest();
        if let Some(witness) = store.resolve_witness(&actual_seal)
            && witness.seal(hash, next_rev) != actual_seal
        {
            return Err(RegenerationError::Checkpoint(CheckpointError::HistoryAltered {
                seal: actual_seal,
            }));
        }
    }
    Ok(session)
}

pub struct RegenerationSource<'a> {
    pub build: &'static str,
    pub from_checkpoint: &'a Checkpoint,
    pub expected: &'a RunIdentity,
    pub witnesses: &'a dyn HistoryStore,
    pub expected_seals: &'a [Digest],
}

/// Replays `source.expected_seals` step by step from `source.from_checkpoint` on the pinned build and
/// `core`, writing complete regenerated links to `target_store` and verifying every step's seal and
/// witness against `source.witnesses`. Reports the first diverging step as `RegenerationError::Diverged`.
pub fn regenerate<'m>(
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    target_store: &'m dyn HistoryStore,
    source: RegenerationSource<'_>,
) -> Result<Regenerated<'m>, RegenerationError> {
    let RegenerationSource {
        build,
        from_checkpoint,
        expected,
        witnesses,
        expected_seals,
    } = source;
    let mut session = Session::resume(from_checkpoint, expected, body, core, target_store)
        .map_err(RegenerationError::Checkpoint)?;
    let hash = session.record().record_hash();
    for &expected_seal in expected_seals {
        let next_rev = session.record().revision() + 1;
        let expected_witness = witnesses
            .resolve_witness(&expected_seal)
            .ok_or(RegenerationError::WitnessMissing { revision: next_rev })?;
        if expected_witness.seal(hash, next_rev) != expected_seal {
            return Err(RegenerationError::Checkpoint(CheckpointError::HistoryAltered {
                seal: expected_seal,
            }));
        }
        let mut step = session.step();
        session = loop {
            match step {
                Step::Accepted(next, _) => break next,
                Step::Refused(retrying) => step = retrying.retry(),
                Step::Faulted(faulted) => {
                    return Err(RegenerationError::Fault {
                        revision: next_rev,
                        fault: *faulted.fault(),
                    });
                }
            }
        };
        let actual_seal = session.record().seal().digest();
        let link = target_store
            .resolve(&actual_seal)
            .ok_or(RegenerationError::Checkpoint(CheckpointError::HistoryMissing {
                seal: actual_seal,
            }))?;
        let actual_witness = Witness::from_link(hash, &link).ok_or(RegenerationError::Checkpoint(
            CheckpointError::HistoryAltered { seal: actual_seal },
        ))?;
        if actual_seal != expected_seal || actual_witness != expected_witness {
            return Err(RegenerationError::Diverged(Box::new(RegenerationFinding {
                build,
                revision: next_rev,
                expected_seal,
                actual_seal,
                expected_witness,
                actual_witness,
            })));
        }
    }
    Ok(Regenerated {
        build,
        session,
        steps_verified: expected_seals.len() as u64,
    })
}
