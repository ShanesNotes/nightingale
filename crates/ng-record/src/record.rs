//! The accepted image's owner and the check that accepts or refuses each step. `validate` is immutable and
//! returns the whole next image or a failure; `publish` installs that image and cannot fail. `&mut
//! PatientRecord` is never lent outside this crate, so only `Session` publishes.
use crate::history::{ConservationView, History, HistorySeal, LedgerChunk};
use ng_contract::{
    AcceptedStateView, BoundParameters, Candidate, CompiledBody, CoupledProblem, DenseSegment, Digest,
    InitialParts, InitialRevision, InputId, MethodHistory, NumericalRefusal, PendingInputs, RunIdentity,
    SimTime, StepProposal, StepTicket, TrialFailure, ValidityExcursion, VerifiedParts,
};

/// The chained seal of one accepted revision: the previous seal, the image's digests and the ledger chunk.
/// It hashes with `Digest::of_canonical`, so the contract's canonical writer stays private.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seal {
    digest: Digest,
}
impl Seal {
    pub fn digest(&self) -> Digest {
        self.digest
    }
}

/// Each law stream's event ordinal, kept once, here. Empty until ticket 06 declares a stream.
#[derive(Default)]
struct StreamPositions {
    ordinals: Vec<(Digest, u64)>,
}
impl StreamPositions {
    fn digest(&self) -> Digest {
        let Self { ordinals } = self;
        let mut bytes = Vec::new();
        for (stream, ordinal) in ordinals {
            bytes.extend_from_slice(stream.as_bytes());
            bytes.extend_from_slice(&ordinal.to_be_bytes());
        }
        Digest::of_canonical(&bytes)
    }
}

/// One patient's accepted condition and ledger. No `Clone`, no public constructor and no setter.
pub struct PatientRecord<'m> {
    body: &'m CompiledBody,
    identity: RunIdentity,
    parameters: BoundParameters,
    branch: Digest,
    revision: u64,
    seal: Seal,
    time: SimTime,
    state: Candidate,
    continuation: MethodHistory,
    dense: DenseSegment,
    streams: StreamPositions,
    pending: PendingInputs,
    history: History,
}

struct Image<'a> {
    time: SimTime,
    state: &'a Candidate,
    continuation: &'a MethodHistory,
    dense: &'a DenseSegment,
    streams: &'a StreamPositions,
    pending: &'a PendingInputs,
}
fn chain(previous: Digest, revision: u64, image: Image<'_>, ledger: Digest) -> Seal {
    let Image {
        time,
        state,
        continuation,
        dense,
        streams,
        pending,
    } = image;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(previous.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(&time.nanoseconds().to_be_bytes());
    for digest in [
        state.digest(),
        continuation.digest(),
        dense.digest(),
        streams.digest(),
        pending.digest(),
        ledger,
    ] {
        bytes.extend_from_slice(digest.as_bytes());
    }
    Seal {
        digest: Digest::of_canonical(&bytes),
    }
}

impl<'m> PatientRecord<'m> {
    pub(crate) fn admit(body: &'m CompiledBody, identity: RunIdentity, origin: InitialRevision) -> Self {
        let InitialParts {
            time,
            state,
            continuation,
            dense,
            pending,
            parameters,
        } = origin.into_parts();
        let run = identity.digest();
        let streams = StreamPositions::default();
        let history = History::opening(
            body,
            AcceptedStateView::new(time, &state, &continuation, &dense, &pending),
        );
        let image = Image {
            time,
            state: &state,
            continuation: &continuation,
            dense: &dense,
            streams: &streams,
            pending: &pending,
        };
        let seal = chain(run, 0, image, history.digest());
        Self {
            body,
            identity,
            parameters,
            branch: run,
            revision: 0,
            seal,
            time,
            state,
            continuation,
            dense,
            streams,
            pending,
            history,
        }
    }
    pub fn identity(&self) -> &RunIdentity {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn seal(&self) -> Seal {
        self.seal
    }
    pub fn accepted(&self) -> AcceptedStateView<'_> {
        AcceptedStateView::new(
            self.time,
            &self.state,
            &self.continuation,
            &self.dense,
            &self.pending,
        )
    }
    /// Lends the problem for one attempt. Only `Session` publishes.
    pub fn problem(&self) -> CoupledProblem<'_> {
        CoupledProblem::new(
            self.body,
            &self.parameters,
            &self.identity,
            self.accepted(),
            self.seal.digest(),
        )
    }
    /// The contract answers whether the core followed its declared method; this answers whether the result
    /// may become the next accepted image. `expected` is the ticket the session issued, so a proposal for
    /// any other attempt, or an issued ticket this record would not issue, is `BindingMismatch`. Never
    /// mutates. A refusal is retryable; a fault latches.
    pub fn validate(&self, expected: &StepTicket, proposal: StepProposal) -> Result<NextImage, TrialFailure> {
        let problem = self.problem();
        if !expected.binds(&problem) || proposal.ticket().digest() != expected.digest() {
            return Err(expected.mismatch().into());
        }
        let step = proposal.verify(&problem)?;
        for row in self.body.rows() {
            if !row.admissible().holds(step.value(row.id())) {
                return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                    source: self.body.row_source(row.id()),
                }));
            }
        }
        let chunk = LedgerChunk::after(&self.history, self.body, &step);
        let pending = self.pending.after(step.ticket());
        let time = step.ticket().interval().end();
        let applied = step.applied().to_vec();
        let VerifiedParts {
            state,
            continuation,
            dense,
            excursions,
        } = step.into_parts();
        let revision = self.revision + 1;
        let image = Image {
            time,
            state: &state,
            continuation: &continuation,
            dense: &dense,
            streams: &self.streams,
            pending: &pending,
        };
        let seal = chain(self.seal.digest(), revision, image, chunk.digest());
        Ok(NextImage {
            revision,
            seal,
            time,
            state,
            continuation,
            dense,
            pending,
            chunk,
            applied,
            excursions,
        })
    }
    /// The only `&mut` path. Installs every part of the image together and cannot fail.
    pub fn publish(&mut self, next: NextImage) -> StepReceipt {
        let NextImage {
            revision,
            seal,
            time,
            state,
            continuation,
            dense,
            pending,
            chunk,
            applied,
            excursions,
        } = next;
        self.revision = revision;
        self.seal = seal;
        self.time = time;
        self.state = state;
        self.continuation = continuation;
        self.dense = dense;
        self.pending = pending;
        self.history.push(chunk);
        StepReceipt {
            revision,
            seal,
            applied,
            excursions,
        }
    }
    /// The one history the certificate reads.
    pub fn conservation(&self) -> ConservationView<'_> {
        ConservationView {
            history: &self.history,
            basis: self.body.conserved_basis(),
            seal: HistorySeal {
                run: self.identity.digest(),
                branch: self.branch,
                base: self.seal.digest(),
            },
        }
    }
    /// One digest per part, recomputed from the live fields on every call. It reads no seal, because a
    /// seal would still match if a field changed underneath it.
    pub fn digests(&self) -> RecordDigests {
        let Self {
            body: _,
            identity: _,
            parameters: _,
            branch: _,
            revision,
            seal: _,
            time,
            state,
            continuation,
            dense,
            streams,
            pending,
            history,
        } = self;
        RecordDigests {
            revision: *revision,
            time: *time,
            state: state.digest(),
            continuation: continuation.digest(),
            segment: dense.digest(),
            streams: streams.digest(),
            pending: pending.digest(),
            ledger: history.digest(),
        }
    }
}

/// What must stay byte-identical across a refusal. `streams` is over an empty list until ticket 06, so
/// today it proves only that the field is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordDigests {
    pub revision: u64,
    pub time: SimTime,
    pub state: Digest,
    pub continuation: Digest,
    pub segment: Digest,
    pub streams: Digest,
    pub pending: Digest,
    pub ledger: Digest,
}

/// The entire next accepted image, validated and not yet visible.
pub struct NextImage {
    revision: u64,
    seal: Seal,
    time: SimTime,
    state: Candidate,
    continuation: MethodHistory,
    dense: DenseSegment,
    pending: PendingInputs,
    chunk: LedgerChunk,
    applied: Vec<InputId>,
    excursions: Vec<ValidityExcursion>,
}

/// What one published step did: its revision and seal, each input it applied, and any excursion.
#[derive(Debug)]
pub struct StepReceipt {
    revision: u64,
    seal: Seal,
    applied: Vec<InputId>,
    excursions: Vec<ValidityExcursion>,
}
impl StepReceipt {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn seal(&self) -> Seal {
        self.seal
    }
    /// Each input this step applied, once, in `(time, id)` order.
    pub fn applied(&self) -> &[InputId] {
        &self.applied
    }
    pub fn excursions(&self) -> &[ValidityExcursion] {
        &self.excursions
    }
}
