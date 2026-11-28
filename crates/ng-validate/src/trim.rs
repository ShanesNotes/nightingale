use crate::certificate::{
    Certificate, CertificateCheck, CertificateFailure, CertificateFold, CertificatePolicy, DriftTally,
};
use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::num::NonZeroU64;
use core::ops::RangeInclusive;
use ng_contract::{
    AdmissionError, CompiledBody, CoupledSolver, Digest, EngineFault, RecordHash, RunIdentity, SimTime,
};
use ng_record::{
    AuthorizedInput, Checkpoint, CheckpointError, FaultReport, HistoryStore, InputReceipt, RetainedSpan,
    Running, Session, Step, Witness, retained_span,
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

/// Pluggable storage for complete links held by a `RollingStore` during a run (in memory, compressed frames
/// or disk-backed storage outside the physiology crates).
pub trait LinkBackend {
    fn get(&self, revision: u64, seal: &Digest) -> Option<Vec<u8>>;
    fn put(&mut self, revision: u64, seal: Digest, link: &[u8]) -> Result<(), CheckpointError>;
    /// Drops every link strictly below `min_revision` and returns the seals removed.
    fn remove_below(&mut self, min_revision: u64) -> Vec<Digest>;
    /// Keeps only links whose revisions lie in `revisions` (or drops all when `None`) and returns the seals
    /// removed.
    fn retain_range(&mut self, revisions: Option<RangeInclusive<u64>>) -> Vec<Digest>;
    fn stored_bytes(&self) -> usize;
    fn revisions(&self) -> Vec<u64>;
    fn latest(&self) -> Option<(u64, Digest)>;
}

/// Default in-memory `LinkBackend` keyed by `(revision, seal)`.
#[derive(Default)]
pub struct MemoryLinks {
    links: BTreeMap<(u64, Digest), Vec<u8>>,
}
impl LinkBackend for MemoryLinks {
    fn get(&self, revision: u64, seal: &Digest) -> Option<Vec<u8>> {
        self.links.get(&(revision, *seal)).cloned()
    }
    fn put(&mut self, revision: u64, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.links.insert((revision, seal), link.to_vec());
        Ok(())
    }
    fn remove_below(&mut self, min_revision: u64) -> Vec<Digest> {
        let kept = self.links.split_off(&first_at(min_revision));
        let dropped = core::mem::replace(&mut self.links, kept);
        dropped.into_keys().map(|(_, seal)| seal).collect()
    }
    fn retain_range(&mut self, revisions: Option<RangeInclusive<u64>>) -> Vec<Digest> {
        let mut removed = Vec::new();
        match revisions {
            Some(range) => {
                self.links.retain(|&(rev, seal), _| {
                    let keep = range.contains(&rev);
                    if !keep {
                        removed.push(seal);
                    }
                    keep
                });
            }
            None => {
                let dropped = core::mem::take(&mut self.links);
                removed.extend(dropped.into_keys().map(|(_, seal)| seal));
            }
        }
        removed
    }
    fn stored_bytes(&self) -> usize {
        self.links.values().map(Vec::len).sum()
    }
    fn revisions(&self) -> Vec<u64> {
        self.links.keys().map(|&(rev, _)| rev).collect()
    }
    fn latest(&self) -> Option<(u64, Digest)> {
        self.links.keys().next_back().copied()
    }
}

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
    pub build_id: Option<String>,
    pub witness_count: usize,
    pub witness_bytes: usize,
    pub checkpoint_revisions: Vec<u64>,
    pub checkpoint_bytes: usize,
    pub complete_link_revisions: Vec<u64>,
    pub complete_link_bytes: usize,
    pub flight_recorder: Option<FlightRecorderSpan>,
}

struct RollingState {
    hash: RecordHash,
    build_id: Option<String>,
    origin: Option<Digest>,
    witnesses: Vec<WitnessEntry>,
    witness_index: BTreeMap<Digest, usize>,
    fork_previous: BTreeMap<usize, Digest>,
    seal_to_rev: BTreeMap<Digest, u64>,
    backend: Box<dyn LinkBackend>,
    link_by_seal: BTreeMap<Digest, u64>,
    pinned_links: BTreeMap<Digest, Vec<u8>>,
    checkpoints: Vec<SavedCheckpoint>,
    flight_recorder: Option<FlightRecorderSpan>,
    missing_base_checkpoint: Option<(FlightRecorderTrigger, Option<SimTime>, Digest)>,
    open_from: Option<SimTime>,
    fail_next_keep: bool,
    resolve_calls: usize,
}
impl RollingState {
    fn new(hash: RecordHash, backend: Box<dyn LinkBackend>) -> Self {
        Self {
            hash,
            build_id: None,
            origin: None,
            witnesses: Vec::new(),
            witness_index: BTreeMap::new(),
            fork_previous: BTreeMap::new(),
            seal_to_rev: BTreeMap::new(),
            backend,
            link_by_seal: BTreeMap::new(),
            pinned_links: BTreeMap::new(),
            checkpoints: Vec::new(),
            flight_recorder: None,
            missing_base_checkpoint: None,
            open_from: None,
            fail_next_keep: false,
            resolve_calls: 0,
        }
    }
    fn previous_of(&self, seal: Digest) -> Option<Digest> {
        let &idx = self.witness_index.get(&seal)?;
        if let Some(&prev) = self.fork_previous.get(&idx) {
            Some(prev)
        } else if idx == 0 {
            self.origin
        } else {
            Some(self.witnesses[idx - 1].seal)
        }
    }
    fn on_chain(&self, head_rev: u64, head_seal: Digest, target_rev: u64, target_seal: Digest) -> bool {
        if target_rev > head_rev {
            return false;
        }
        let mut at = head_seal;
        let mut rev = head_rev;
        while rev > target_rev {
            let Some(prev) = self.previous_of(at) else {
                return false;
            };
            at = prev;
            rev -= 1;
        }
        at == target_seal
    }
    fn checkpoint_for(&self, instant: Option<SimTime>) -> Option<&SavedCheckpoint> {
        match instant {
            Some(at) => (self.checkpoints.iter().rev()).find(|saved| saved.time <= at),
            None => self.checkpoints.last(),
        }
    }
    fn checkpoint_for_chain(
        &self,
        instant: Option<SimTime>,
        head_rev: u64,
        head_seal: Digest,
    ) -> Option<&SavedCheckpoint> {
        (self.checkpoints.iter().rev()).find(|saved| {
            instant.is_none_or(|at| saved.time <= at)
                && self.on_chain(head_rev, head_seal, saved.revision, saved.seal)
        })
    }
    fn latch(
        &mut self,
        trigger: FlightRecorderTrigger,
        window_start: Option<SimTime>,
        last_head_revision: u64,
        last_head_seal: Digest,
    ) -> Option<FlightRecorderSpan> {
        if let Some(span) = self.flight_recorder.clone() {
            if last_head_revision > span.last_head_revision
                && self.on_chain(
                    last_head_revision,
                    last_head_seal,
                    span.last_head_revision,
                    span.last_head_seal,
                )
                && let Some(held) = &mut self.flight_recorder
            {
                held.last_head_revision = last_head_revision;
                held.last_head_seal = last_head_seal;
            }
            return self.flight_recorder.clone();
        }
        let target = window_start.or(self.open_from);
        let Some(base) = self.checkpoint_for_chain(target, last_head_revision, last_head_seal) else {
            self.missing_base_checkpoint
                .get_or_insert((trigger, target, last_head_seal));
            return None;
        };
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
    fn kept_head_for_previous(&self, previous: Digest) -> (u64, Digest) {
        if let Some(&prev_rev) = self.seal_to_rev.get(&previous) {
            return (prev_rev, previous);
        }
        if let Some(ckpt) = (self.checkpoints.iter().rev()).find(|saved| saved.seal == previous) {
            return (ckpt.revision, previous);
        }
        (0, previous)
    }
}

fn first_at(revision: u64) -> (u64, Digest) {
    (revision, Digest::from_bytes([0; 32]))
}

fn next_checkpoint_after(current_deadline: SimTime, reached: SimTime, every: NonZeroU64) -> Option<SimTime> {
    let deadline = current_deadline.nanoseconds();
    let now = reached.nanoseconds();
    if now < deadline {
        return Some(current_deadline);
    }
    let step = every.get();
    let elapsed = now - deadline;
    let skips = (elapsed / step).checked_add(1)?;
    let advance = skips.checked_mul(step)?;
    deadline.checked_add(advance).map(SimTime::from_ns)
}

/// A trimming history store that keeps 96-byte witnesses for every step, periodic checkpoints with their
/// head links, a rolling window of complete links through a pluggable `LinkBackend` while the run goes, and
/// a flight recorder span on failure.
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
        Self::with_hash_and_backend(RecordHash::Blake2b, Box::new(MemoryLinks::default()))
    }
    pub fn with_hash(hash: RecordHash) -> Self {
        Self::with_hash_and_backend(hash, Box::new(MemoryLinks::default()))
    }
    pub fn with_backend(backend: Box<dyn LinkBackend>) -> Self {
        Self::with_hash_and_backend(RecordHash::Blake2b, backend)
    }
    pub fn with_hash_and_backend(hash: RecordHash, backend: Box<dyn LinkBackend>) -> Self {
        Self {
            state: RefCell::new(RollingState::new(hash, backend)),
        }
    }
    pub fn hash(&self) -> RecordHash {
        self.state.borrow().hash
    }
    pub fn build_id(&self) -> Option<String> {
        self.state.borrow().build_id.clone()
    }
    pub fn record_build_id(&self, build_id: &str) -> Result<(), RollingError> {
        if build_id.is_empty() {
            return Err(RollingError::EmptyBuildId);
        }
        let mut state = self.state.borrow_mut();
        match &state.build_id {
            None => {
                state.build_id = Some(build_id.to_owned());
                Ok(())
            }
            Some(existing) if existing == build_id => Ok(()),
            Some(_) => Err(RollingError::BuildMismatch),
        }
    }
    /// Forces the next `keep` or `keep_with_witness` call to fail with `StoreFailed` and latch the flight
    /// recorder.
    pub fn fail_next_keep(&self) {
        self.state.borrow_mut().fail_next_keep = true;
    }
    pub fn record_checkpoint(&self, session: &Session<'_, Running>) -> Result<(), CheckpointError> {
        let record = session.record();
        if record.record_hash() != self.hash() {
            return Err(CheckpointError::IdentityMismatch);
        }
        let checkpoint = session.checkpoint()?;
        let rev = record.revision();
        let seal = record.seal().digest();
        let saved = SavedCheckpoint {
            revision: rev,
            time: record.accepted().time(),
            seal,
            checkpoint,
        };
        let mut state = self.state.borrow_mut();
        if state.hash == RecordHash::Sha256
            && rev > 0
            && !state.pinned_links.contains_key(&seal)
            && let Some(link) = state.backend.get(rev, &seal)
        {
            state.pinned_links.insert(seal, link);
        }
        state.checkpoints.push(saved);
        Ok(())
    }
    pub fn record_exported_checkpoint(
        &self,
        revision: u64,
        time: SimTime,
        seal: Digest,
        checkpoint: Checkpoint,
    ) {
        let mut state = self.state.borrow_mut();
        if state.hash == RecordHash::Sha256
            && revision > 0
            && !state.pinned_links.contains_key(&seal)
            && let Some(link) = state.backend.get(revision, &seal)
        {
            state.pinned_links.insert(seal, link);
        }
        state.checkpoints.push(SavedCheckpoint {
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
    pub fn checkpoint_at_revision_and_seal(&self, revision: u64, seal: Digest) -> Option<SavedCheckpoint> {
        let state = self.state.borrow();
        (state.checkpoints.iter())
            .find(|saved| saved.revision == revision && saved.seal == seal)
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
        state.open_from = oldest_open_window_start;
        if state.missing_base_checkpoint.is_some() {
            return;
        }
        let window_floor = match oldest_open_window_start {
            Some(start) => Some(
                state
                    .checkpoint_for(Some(start))
                    .map_or(1, |saved| saved.revision + 1),
            ),
            None => state.checkpoints.last().map(|saved| saved.revision + 1),
        };
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
        let removed = state.backend.remove_below(floor);
        for seal in removed {
            state.link_by_seal.remove(&seal);
        }
    }
    /// Drops all complete links outside any latched flight recorder span, leaving witnesses and checkpoints.
    pub fn finish(&self) {
        let mut state = self.state.borrow_mut();
        if state.missing_base_checkpoint.is_some() && state.flight_recorder.is_none() {
            return;
        }
        let keep = state
            .flight_recorder
            .as_ref()
            .map(|span| span.from_checkpoint_revision + 1..=span.last_head_revision);
        let removed = state.backend.retain_range(keep);
        for seal in removed {
            state.link_by_seal.remove(&seal);
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
            .sum::<usize>()
            + state.pinned_links.values().map(Vec::len).sum::<usize>();
        let complete_link_revisions = state.backend.revisions();
        let complete_link_bytes = state.backend.stored_bytes();
        StoreListing {
            build_id: state.build_id.clone(),
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
    pub fn retained_flight_recorder<'m, 's>(
        &'s self,
        body: &'m CompiledBody,
        expected: &RunIdentity,
    ) -> Result<Option<RetainedSpan<'m, 's>>, CheckpointError> {
        let Some(span) = self.flight_recorder() else {
            let state = self.state.borrow();
            if let Some((_, _, seal)) = state.missing_base_checkpoint {
                return Err(CheckpointError::HistoryMissing { seal });
            }
            return Ok(None);
        };
        let saved = self
            .checkpoint_at_revision_and_seal(span.from_checkpoint_revision, span.from_checkpoint_seal)
            .ok_or(CheckpointError::HistoryMissing {
                seal: span.from_checkpoint_seal,
            })?;
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
    fn note_open_from(&self, open_from: Option<SimTime>) {
        self.state.borrow_mut().open_from = open_from;
    }
}
impl HistoryStore for RollingStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let mut state = self.state.borrow_mut();
        state.resolve_calls += 1;
        if let Some(&rev) = state.link_by_seal.get(seal)
            && let Some(link) = state.backend.get(rev, seal)
        {
            return Some(link);
        }
        state.pinned_links.get(seal).cloned()
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        let hash = self.state.borrow().hash;
        let witness = Witness::from_link(hash, link).ok_or(CheckpointError::HistoryAltered { seal })?;
        self.keep_with_witness(seal, link, witness)
    }
    fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
        let state = self.state.borrow();
        let idx = *state.witness_index.get(seal)?;
        let entry = state.witnesses[idx];
        let previous = state.previous_of(*seal)?;
        Some(Witness::new(previous, entry.image, entry.ledger))
    }
    fn keep_with_witness(&self, seal: Digest, link: &[u8], witness: Witness) -> Result<(), CheckpointError> {
        let mut state = self.state.borrow_mut();
        if state.fail_next_keep {
            state.fail_next_keep = false;
            let (last_rev, last_seal) = state.kept_head_for_previous(witness.previous());
            state.latch(
                FlightRecorderTrigger::StoreFailure { seal },
                None,
                last_rev,
                last_seal,
            );
            return Err(CheckpointError::StoreFailed { seal });
        }
        if let Some(&rev) = state.seal_to_rev.get(&seal) {
            if state.backend.get(rev, &seal).is_none() {
                if let Err(err) = state.backend.put(rev, seal, link) {
                    let (last_rev, last_seal) = state.kept_head_for_previous(witness.previous());
                    state.latch(
                        FlightRecorderTrigger::StoreFailure { seal },
                        None,
                        last_rev,
                        last_seal,
                    );
                    return Err(err);
                }
                state.link_by_seal.insert(seal, rev);
            }
            return Ok(());
        }
        let origin_was_none = state.origin.is_none();
        let rev = if state.origin == Some(witness.previous()) {
            1
        } else if let Some(&prev_rev) = state.seal_to_rev.get(&witness.previous()) {
            prev_rev + 1
        } else if let Some(ckpt) = (state.checkpoints.iter().rev()).find(|c| c.seal == witness.previous()) {
            ckpt.revision + 1
        } else if origin_was_none {
            1
        } else {
            state.witnesses.len() as u64 + 1
        };
        if let Err(err) = state.backend.put(rev, seal, link) {
            let (last_rev, last_seal) = state.kept_head_for_previous(witness.previous());
            state.latch(
                FlightRecorderTrigger::StoreFailure { seal },
                None,
                last_rev,
                last_seal,
            );
            return Err(err);
        }
        if origin_was_none {
            state.origin = Some(witness.previous());
        }
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
        state.link_by_seal.insert(seal, rev);
        if state.hash == RecordHash::Sha256 && rev == 1 {
            state.pinned_links.entry(seal).or_insert_with(|| link.to_vec());
        }
        if let Some(span) = &mut state.flight_recorder
            && rev == span.last_head_revision + 1
            && witness.previous() == span.last_head_seal
        {
            span.last_head_revision = rev;
            span.last_head_seal = seal;
        }
        Ok(())
    }
}

/// One authorized boundary input scheduled for admission at a specific revision of a rolling run.
#[derive(Clone)]
pub struct ScheduledInput {
    pub at_revision: u64,
    pub input: AuthorizedInput,
}

#[derive(Clone)]
pub struct RollingConfig {
    pub build_id: String,
    pub checkpoint_every_ns: NonZeroU64,
    pub late_inputs: Vec<ScheduledInput>,
}
impl Default for RollingConfig {
    fn default() -> Self {
        Self {
            build_id: String::from("nightingale"),
            checkpoint_every_ns: NonZeroU64::new(MINUTE_NS).expect("positive minute"),
            late_inputs: Vec::new(),
        }
    }
}

/// Per-step telemetry recorded during a rolling run (distinct from the replayable `EventLog`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub revision: u64,
    pub time: SimTime,
    pub seal: Digest,
    pub after_breakpoint: bool,
    pub excursions: usize,
}

/// One authorized boundary event admitted during a run, kept with the revision at which it was enqueued and
/// its admission receipt so regeneration can replay it at the exact same point.
#[derive(Clone)]
pub struct LoggedEvent {
    pub revision: u64,
    pub input: AuthorizedInput,
    pub receipt: InputReceipt,
}

/// The append-only event log a run is rebuilt from: the run's identity and every boundary event admitted
/// during the run (SRS I17).
#[derive(Clone)]
pub struct EventLog {
    identity: RunIdentity,
    events: Vec<LoggedEvent>,
}
impl EventLog {
    pub fn new(identity: RunIdentity) -> Self {
        Self {
            identity,
            events: Vec::new(),
        }
    }
    pub fn identity(&self) -> &RunIdentity {
        &self.identity
    }
    pub fn events(&self) -> &[LoggedEvent] {
        &self.events
    }
}

pub enum RollingStatus<'m> {
    Completed(Session<'m, Running>),
    Faulted(FaultReport),
}

pub struct RollingOutcome<'m> {
    pub build_id: String,
    pub status: RollingStatus<'m>,
    pub certificate: Result<Certificate, CertificateFailure>,
    pub drift_tally: Option<DriftTally>,
    pub listing: StoreListing,
    pub event_log: EventLog,
    pub step_trace: Vec<StepTrace>,
}

#[derive(Clone, Debug)]
pub enum RollingError {
    Certificate(CertificateFailure),
    Checkpoint(CheckpointError),
    Input(AdmissionError),
    PastScheduledInput { at_revision: u64, current_revision: u64 },
    ForeignStore,
    HashMismatch,
    EmptyBuildId,
    BuildMismatch,
    MissingCheckpoint { needed_at: SimTime },
}

/// Runs `session` until `end` while folding `policy`, saving periodic checkpoints with their head links,
/// trimming links outside the open window, latching the flight recorder on the first failure, flushing any
/// pending links, and finishing the store at the end of the run.
pub fn run_rolling<'m>(
    session: Session<'m, Running>,
    end: SimTime,
    policy: &CertificatePolicy,
    store: &'m RollingStore,
    config: RollingConfig,
) -> Result<RollingOutcome<'m>, RollingError> {
    run_rolling_inner(session, None, end, policy, store, config)
}

/// Runs `session` until `end` exactly as `run_rolling` does while stepping a fresh-core `shadow` session in
/// lockstep with the exact same inputs and latching `FlightRecorderTrigger::ReplayDivergence` before
/// trimming or finishing on the first seal or digest divergence or shadow fault.
pub fn run_rolling_with_replay<'m>(
    session: Session<'m, Running>,
    shadow: Session<'m, Running>,
    end: SimTime,
    policy: &CertificatePolicy,
    store: &'m RollingStore,
    config: RollingConfig,
) -> Result<RollingOutcome<'m>, RollingError> {
    run_rolling_inner(session, Some(shadow), end, policy, store, config)
}

fn step_shadow<'m>(shadow: Session<'m, Running>) -> Option<Session<'m, Running>> {
    let mut step = shadow.step();
    loop {
        match step {
            Step::Accepted(next, _) => return Some(next),
            Step::Refused(retrying) => step = retrying.retry(),
            Step::Faulted(_) => return None,
        }
    }
}

fn run_rolling_inner<'m>(
    mut session: Session<'m, Running>,
    mut shadow: Option<Session<'m, Running>>,
    end: SimTime,
    policy: &CertificatePolicy,
    store: &'m RollingStore,
    config: RollingConfig,
) -> Result<RollingOutcome<'m>, RollingError> {
    if config.build_id.is_empty() {
        return Err(RollingError::EmptyBuildId);
    }
    if !session.uses_store(store) {
        return Err(RollingError::ForeignStore);
    }
    if session.record().record_hash() != store.hash() {
        return Err(RollingError::HashMismatch);
    }
    if let Some(other) = &shadow {
        if other.uses_store(store) {
            return Err(RollingError::ForeignStore);
        }
        if other.record().identity().digest() != session.record().identity().digest()
            || other.record().revision() != session.record().revision()
            || other.record().seal() != session.record().seal()
        {
            return Err(RollingError::Checkpoint(CheckpointError::IdentityMismatch));
        }
    }
    let mut scheduled = config.late_inputs;
    scheduled.sort_by_key(|item| item.at_revision);
    if let Some(first) = scheduled.first()
        && first.at_revision < session.record().revision()
    {
        return Err(RollingError::PastScheduledInput {
            at_revision: first.at_revision,
            current_revision: session.record().revision(),
        });
    }
    store.record_build_id(&config.build_id)?;
    store
        .record_checkpoint(&session)
        .map_err(RollingError::Checkpoint)?;
    let mut last_kept = (session.record().revision(), session.record().seal().digest());
    let mut fold = CertificateFold::open(&session, policy).map_err(RollingError::Certificate)?;
    store.note_open_from(fold.oldest_open_window_start());
    let every = config.checkpoint_every_ns;
    let mut next_checkpoint = session
        .record()
        .accepted()
        .time()
        .nanoseconds()
        .checked_add(every.get())
        .map(SimTime::from_ns);
    let mut event_log = EventLog::new(session.record().identity().clone());
    let mut step_trace = Vec::new();
    let mut next_input = 0usize;
    while session.record().accepted().time() < end {
        let cur_rev = session.record().revision();
        let mut shadow_input_diverged = false;
        while let Some(item) = scheduled.get(next_input)
            && item.at_revision == cur_rev
        {
            let receipt = session.enqueue(item.input.clone()).map_err(RollingError::Input)?;
            if let Some(other) = shadow.as_mut()
                && other.enqueue(item.input.clone()).is_err()
            {
                shadow_input_diverged = true;
            }
            event_log.events.push(LoggedEvent {
                revision: cur_rev,
                input: item.input.clone(),
                receipt,
            });
            next_input += 1;
        }
        let mut step = session.step();
        let (next, receipt) = loop {
            match step {
                Step::Accepted(next, receipt) => break (next, receipt),
                Step::Refused(retrying) => step = retrying.retry(),
                Step::Faulted(mut faulted) => {
                    let fault = *faulted.fault();
                    let rev = faulted.record().revision();
                    let time = faulted.record().accepted().time();
                    let seal = faulted.record().seal().digest();
                    let report = faulted.export().map_err(RollingError::Checkpoint)?;
                    let needed = fold.oldest_open_window_start();
                    if store
                        .latch_flight_recorder(FlightRecorderTrigger::EngineFault(fault), needed, rev, seal)
                        .is_none()
                    {
                        return Err(RollingError::MissingCheckpoint {
                            needed_at: needed.unwrap_or(time),
                        });
                    }
                    store.finish();
                    return Ok(RollingOutcome {
                        build_id: config.build_id,
                        status: RollingStatus::Faulted(report),
                        certificate: fold.certificate(),
                        drift_tally: fold.drift_tally(),
                        listing: store.listing(),
                        event_log,
                        step_trace,
                    });
                }
            }
        };
        session = next;
        let rev = session.record().revision();
        let time = session.record().accepted().time();
        let seal = session.record().seal().digest();
        step_trace.push(StepTrace {
            revision: rev,
            time,
            seal,
            after_breakpoint: receipt.after_breakpoint(),
            excursions: receipt.excursions().len(),
        });
        fold.advance(&session).expect("same session");
        if let Some(failed) = session.record().oldest_unkept() {
            let (kept_rev, kept_seal) = last_kept;
            let needed = fold.oldest_open_window_start();
            if store
                .latch_flight_recorder(
                    FlightRecorderTrigger::StoreFailure { seal: failed },
                    needed,
                    kept_rev,
                    kept_seal,
                )
                .is_none()
            {
                return Err(RollingError::MissingCheckpoint {
                    needed_at: needed.unwrap_or(time),
                });
            }
        } else {
            last_kept = (rev, seal);
        }
        if let Some(other) = shadow.take() {
            let matched = if shadow_input_diverged {
                None
            } else {
                step_shadow(other).filter(|stepped| {
                    stepped.record().seal() == session.record().seal()
                        && stepped.record().digests() == session.record().digests()
                })
            };
            if let Some(stepped) = matched {
                shadow = Some(stepped);
            } else {
                let needed = fold.oldest_open_window_start().or(Some(time));
                if store
                    .latch_flight_recorder(FlightRecorderTrigger::ReplayDivergence, needed, rev, seal)
                    .is_none()
                {
                    return Err(RollingError::MissingCheckpoint {
                        needed_at: needed.unwrap_or(time),
                    });
                }
            }
        }
        if let Some(failure) = fold.failure()
            && !matches!(failure.check(), CertificateCheck::History(_))
        {
            let needed = failure.window().map(|window| window.start());
            if store
                .latch_flight_recorder(
                    FlightRecorderTrigger::CertificateFailure(failure.check()),
                    needed,
                    rev,
                    seal,
                )
                .is_none()
            {
                return Err(RollingError::MissingCheckpoint {
                    needed_at: needed.or_else(|| fold.oldest_open_window_start()).unwrap_or(time),
                });
            }
        }
        if let Some(deadline) = next_checkpoint
            && time >= deadline
        {
            match store.record_checkpoint(&session) {
                Ok(()) => {
                    next_checkpoint = next_checkpoint_after(deadline, time, every);
                }
                Err(CheckpointError::StoreFailed { seal: failed }) => {
                    let (kept_rev, kept_seal) = last_kept;
                    let needed = fold.oldest_open_window_start();
                    if store
                        .latch_flight_recorder(
                            FlightRecorderTrigger::StoreFailure { seal: failed },
                            needed,
                            kept_rev,
                            kept_seal,
                        )
                        .is_none()
                    {
                        return Err(RollingError::MissingCheckpoint {
                            needed_at: needed.unwrap_or(time),
                        });
                    }
                }
                Err(other) => return Err(RollingError::Checkpoint(other)),
            }
        }
        store.trim_before(fold.oldest_open_window_start(), fold.oldest_retained_revision());
    }
    if let Err(CheckpointError::StoreFailed { seal: failed }) = session.flush_store() {
        let (kept_rev, kept_seal) = last_kept;
        let _ = store.latch_flight_recorder(
            FlightRecorderTrigger::StoreFailure { seal: failed },
            fold.oldest_open_window_start(),
            kept_rev,
            kept_seal,
        );
        store.finish();
        return Err(RollingError::Checkpoint(CheckpointError::StoreFailed {
            seal: failed,
        }));
    }
    let certificate = fold.certificate();
    let drift_tally = fold.drift_tally();
    if let Err(failure) = &certificate
        && !matches!(
            failure.check(),
            CertificateCheck::Coverage | CertificateCheck::UndeclaredRhythm
        )
    {
        let needed = failure.window().map(|window| window.start());
        let rev = session.record().revision();
        let seal = session.record().seal().digest();
        if store
            .latch_flight_recorder(
                FlightRecorderTrigger::CertificateFailure(failure.check()),
                needed,
                rev,
                seal,
            )
            .is_none()
        {
            return Err(RollingError::MissingCheckpoint {
                needed_at: needed
                    .or_else(|| fold.oldest_open_window_start())
                    .unwrap_or_else(|| session.record().accepted().time()),
            });
        }
    }
    store.finish();
    Ok(RollingOutcome {
        build_id: config.build_id,
        status: RollingStatus::Completed(session),
        certificate,
        drift_tally,
        listing: store.listing(),
        event_log,
        step_trace,
    })
}

/// First step at which a regeneration failed to reproduce the stored witnesses (record 0008).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegenerationFinding {
    pub build: String,
    pub revision: u64,
    pub expected_seal: Digest,
    pub actual_seal: Digest,
    pub expected_witness: Witness,
    pub actual_witness: Witness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegenerationError {
    Checkpoint(CheckpointError),
    Input { revision: u64, error: AdmissionError },
    Fault { revision: u64, fault: EngineFault },
    WitnessMissing { revision: u64 },
    BuildMissing,
    Diverged(Box<RegenerationFinding>),
}

pub struct Regenerated<'m> {
    pub build: String,
    pub session: Session<'m, Running>,
    pub steps_verified: u64,
}

/// The store a regeneration steps on: it reads ancestors' witnesses, links and recorded originating build id
/// from `source`, the run's own store, and keeps every regenerated link in `sink` only, so the chain it is
/// checked against never moves.
pub struct ReplayStore<'a> {
    source: &'a dyn HistoryStore,
    sink: &'a dyn HistoryStore,
    originating_build: Option<String>,
}
impl<'a> ReplayStore<'a> {
    pub fn from_rolling(source: &'a RollingStore, sink: &'a dyn HistoryStore) -> Self {
        Self {
            source,
            sink,
            originating_build: source.build_id(),
        }
    }
    pub fn new(source: &'a dyn HistoryStore, sink: &'a dyn HistoryStore) -> Self {
        Self {
            source,
            sink,
            originating_build: None,
        }
    }
    pub fn with_originating_build(mut self, build_id: Option<String>) -> Self {
        self.originating_build = build_id;
        self
    }
    pub fn source(&self) -> &'a dyn HistoryStore {
        self.source
    }
    pub fn sink(&self) -> &'a dyn HistoryStore {
        self.sink
    }
    pub fn originating_build(&self) -> Option<&str> {
        self.originating_build.as_deref()
    }
}
impl HistoryStore for ReplayStore<'_> {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        self.sink.resolve(seal).or_else(|| self.source.resolve(seal))
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.sink.keep(seal, link)
    }
    fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
        (self.source.resolve_witness(seal)).or_else(|| self.sink.resolve_witness(seal))
    }
    fn keep_with_witness(&self, seal: Digest, link: &[u8], witness: Witness) -> Result<(), CheckpointError> {
        self.sink.keep_with_witness(seal, link, witness)
    }
}

pub struct RegenerationSource<'a> {
    pub from_checkpoint: &'a Checkpoint,
    pub expected: &'a RunIdentity,
    pub expected_seals: &'a [Digest],
    pub event_log: Option<&'a EventLog>,
}

/// Resumes `source.from_checkpoint`, at admission or at any later checkpoint, on the originating build
/// recorded in `store` and a fresh `core`, enqueues any post-checkpoint events from `source.event_log` at
/// their recorded revisions, and steps once for each of `source.expected_seals`. Each step's seal and
/// witness are checked against the witness `store.source()` holds for that seal; regenerated links go to
/// `store.sink()`. Reports the first diverging step as `RegenerationError::Diverged`.
pub fn regenerate<'m>(
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    store: &'m ReplayStore<'m>,
    source: RegenerationSource<'_>,
) -> Result<Regenerated<'m>, RegenerationError> {
    let RegenerationSource {
        from_checkpoint,
        expected,
        expected_seals,
        event_log,
    } = source;
    let build = store
        .originating_build()
        .filter(|id| !id.is_empty())
        .ok_or(RegenerationError::BuildMissing)?
        .to_owned();
    if let Some(log) = event_log
        && log.identity().digest() != expected.digest()
    {
        return Err(RegenerationError::Checkpoint(CheckpointError::IdentityMismatch));
    }
    let mut session = Session::resume(from_checkpoint, expected, body, core, store)
        .map_err(RegenerationError::Checkpoint)?;
    let hash = session.record().record_hash();
    for &expected_seal in expected_seals {
        let cur_rev = session.record().revision();
        if let Some(log) = event_log {
            for logged in log.events().iter().filter(|logged| logged.revision == cur_rev) {
                session
                    .enqueue(logged.input.clone())
                    .map_err(|error| RegenerationError::Input {
                        revision: cur_rev,
                        error,
                    })?;
            }
        }
        let next_rev = cur_rev + 1;
        let expected_witness = (store.source())
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
        let link = (store.sink())
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

#[cfg(test)]
mod tests {
    use super::next_checkpoint_after;
    use core::num::NonZeroU64;
    use ng_contract::SimTime;

    #[test]
    fn checkpoint_deadline_advances_in_constant_time_and_returns_none_on_overflow() {
        let every = NonZeroU64::new(10).expect("pos");
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(10), SimTime::from_ns(9), every),
            Some(SimTime::from_ns(10))
        );
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(10), SimTime::from_ns(10), every),
            Some(SimTime::from_ns(20))
        );
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(10), SimTime::from_ns(19), every),
            Some(SimTime::from_ns(20))
        );
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(10), SimTime::from_ns(20), every),
            Some(SimTime::from_ns(30))
        );
        let one_ns = NonZeroU64::new(1).expect("pos");
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(10), SimTime::from_ns(1_000_000_000), one_ns),
            Some(SimTime::from_ns(1_000_000_001))
        );
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(u64::MAX - 5), SimTime::from_ns(u64::MAX), every),
            None
        );
        assert_eq!(
            next_checkpoint_after(SimTime::from_ns(0), SimTime::from_ns(u64::MAX), one_ns),
            None
        );
    }
}
