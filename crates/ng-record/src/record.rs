use crate::checkpoint::{
    BranchAncestry, BranchFork, Checkpoint, CheckpointError, HistoryStore, Restored, Sections, write,
};
use crate::history::{ConservationView, History, HistorySeal, LedgerChunk, Link};
use ng_contract::{
    AcceptedStateView, AdmissionError, AppliedInputs, Attempt, BoundaryEvent, Candidate, CompiledBody,
    CoupledProblem, CoupledSolver, DenseSegment, Digest, Fired, ImageParts, InitialRevision, InputId,
    MethodHistory, NumericalRefusal, RunIdentity, SimTime, StepProposal, StepTicket, TrialFailure,
    ValidityExcursion, VerifiedParts, VerifiedStep,
};

/// The chained seal of one accepted revision: the previous seal, the image's digests and the ledger chunk.
/// It covers the past and nothing that waits, so a ticket's digest built on it reads no later input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seal {
    digest: Digest,
}
impl Seal {
    pub fn digest(&self) -> Digest {
        self.digest
    }
}

/// One patient's accepted condition and ledger. No `Clone`, no public constructor and no setter.
pub struct PatientRecord<'m> {
    body: &'m CompiledBody,
    identity: RunIdentity,
    branch: BranchAncestry,
    revision: u64,
    image: ImageParts,
    history: History,
}

struct Image<'a> {
    time: SimTime,
    state: &'a Candidate,
    continuation: &'a MethodHistory,
    dense: &'a DenseSegment,
    applied: &'a AppliedInputs,
}
fn image_digest(image: Image<'_>) -> Digest {
    let Image {
        time,
        state,
        continuation,
        dense,
        applied,
    } = image;
    let mut bytes = time.nanoseconds().to_be_bytes().to_vec();
    for digest in [
        state.digest(),
        continuation.digest(),
        dense.digest(),
        applied.digest(),
    ] {
        bytes.extend_from_slice(digest.as_bytes());
    }
    Digest::of_canonical(&bytes)
}
fn digest_of(image: &ImageParts) -> Digest {
    image_digest(Image {
        time: image.time,
        state: &image.state,
        continuation: &image.continuation,
        dense: &image.dense,
        applied: image.pending.applied(),
    })
}

impl<'m> PatientRecord<'m> {
    pub(crate) fn admit(body: &'m CompiledBody, identity: RunIdentity, origin: InitialRevision) -> Self {
        let image = origin.into_parts();
        let run = identity.digest();
        let accepted = AcceptedStateView::new(image.time, &image.state, &image.continuation, &image.dense);
        let opened = digest_of(&image);
        let history = History::opening(body, accepted, run, opened, image.parameters().digest());
        Self {
            body,
            identity,
            branch: BranchAncestry::of_run(run),
            revision: 0,
            image,
            history,
        }
    }
    pub(crate) fn restored(
        body: &'m CompiledBody,
        identity: RunIdentity,
        parts: Restored,
    ) -> Result<Self, CheckpointError> {
        let Restored {
            branch,
            revision,
            image,
            history,
        } = parts;
        if digest_of(&image) != history.head_image() {
            return Err(CheckpointError::SealMismatch);
        }
        Ok(Self {
            body,
            identity,
            branch,
            revision,
            image,
            history,
        })
    }
    pub(crate) fn body(&self) -> &'m CompiledBody {
        self.body
    }
    pub(crate) fn branch(&self) -> &BranchAncestry {
        &self.branch
    }
    pub(crate) fn history(&self) -> &History {
        &self.history
    }
    pub fn identity(&self) -> &RunIdentity {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn seal(&self) -> Seal {
        Seal {
            digest: self.history.seal(),
        }
    }
    pub fn accepted(&self) -> AcceptedStateView<'_> {
        let image = &self.image;
        AcceptedStateView::new(image.time, &image.state, &image.continuation, &image.dense)
    }
    /// The earliest stamp `Session::enqueue` takes now.
    pub fn earliest_stamp(&self) -> SimTime {
        self.image.pending.earliest_stamp(self.image.time)
    }
    /// Oldest first. Empty on the run's own branch.
    pub fn ancestry(&self) -> &[BranchFork] {
        self.branch.forks()
    }
    pub fn problem(&self) -> CoupledProblem<'_> {
        CoupledProblem::new(
            self.body,
            self.image.parameters(),
            &self.identity,
            self.accepted(),
            self.history.seal(),
        )
    }
    /// Immutable, so a host can drive a core by hand. Only a session publishes.
    pub fn attempt(
        &self,
        solver: &dyn CoupledSolver,
        attempt: Attempt<'_>,
    ) -> Result<NextImage, TrialFailure> {
        let problem = self.problem();
        let (step, chunk) = self.image.pending.attempt(&problem, attempt, |ticket| {
            let proposal = solver.solve(&problem, ticket).map_err(|failure| match failure {
                TrialFailure::Refused(refused) if refused.ticket() != ticket.digest() => {
                    ticket.mismatch().into()
                }
                other => other,
            })?;
            self.validate(&problem, ticket, proposal)
        })?;
        Ok(self.next_image(step, chunk))
    }
    /// The contract answers whether the core followed its declared method; this answers whether the result
    /// may stand. A firing moves no amount and resets to zero, which compile holds in the slot's set, so these
    /// checks hold after it, and it reads the laws on what it leaves itself.
    fn validate(
        &self,
        problem: &CoupledProblem<'_>,
        expected: &StepTicket,
        proposal: StepProposal,
    ) -> Result<(VerifiedStep, LedgerChunk), TrialFailure> {
        if proposal.ticket().digest() != expected.digest() {
            return Err(expected.mismatch().into());
        }
        let step = proposal.verify(problem)?;
        for row in self.body.rows() {
            if !row.admissible().holds(step.value(row.id())) {
                return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                    source: self.body.row_source(row.id()),
                }));
            }
        }
        let chunk = LedgerChunk::after(&self.history, self.body, &step);
        if !chunk.is_finite() {
            return Err(step.ticket().refuse(NumericalRefusal::LedgerOverflow));
        }
        Ok((step, chunk))
    }
    fn next_image(&self, step: VerifiedStep, chunk: LedgerChunk) -> NextImage {
        let pending = self.image.pending.after(step.ticket());
        let time = step.ticket().interval().end();
        let after_breakpoint = step.ticket().after_breakpoint();
        let applied = step.applied().to_vec();
        let VerifiedParts {
            state,
            continuation,
            dense,
            excursions,
            fired,
        } = step.into_parts();
        let revision = self.revision + 1;
        let image = image_digest(Image {
            time,
            state: &state,
            continuation: &continuation,
            dense: &dense,
            applied: &pending,
        });
        let link = self.history.linked(chunk, revision, image);
        NextImage {
            revision,
            time,
            state,
            continuation,
            dense,
            pending,
            link,
            applied,
            after_breakpoint,
            excursions,
            fired,
        }
    }
    /// The one path that changes accepted state. Installs every part of the image together and cannot fail.
    pub fn publish(&mut self, next: NextImage) -> StepReceipt {
        let NextImage {
            revision,
            time,
            state,
            continuation,
            dense,
            pending,
            link,
            applied,
            after_breakpoint,
            excursions,
            fired,
        } = next;
        self.revision = revision;
        self.image.time = time;
        self.image.state = state;
        self.image.continuation = continuation;
        self.image.dense = dense;
        self.image.pending.advance(pending);
        let digest = link.seal();
        self.history.push(link);
        StepReceipt {
            revision,
            seal: Seal { digest },
            applied,
            after_breakpoint,
            excursions,
            fired,
        }
    }
    pub(crate) fn admit_input(&mut self, input: AuthorizedInput) -> Result<InputReceipt, AdmissionError> {
        let AuthorizedInput { command: _, event } = input;
        let (id, at) = (event.id(), event.at());
        let payload = self.image.pending.admit(self.body, self.image.time, event)?;
        Ok(InputReceipt { id, payload, at })
    }
    pub(crate) fn checkpoint(
        &self,
        store: &mut dyn HistoryStore,
        latched: bool,
    ) -> Result<Checkpoint, CheckpointError> {
        for (seal, link) in self.history.unkept(&|seal| store.resolve(seal).is_some()) {
            store.keep(seal, link)?;
        }
        let sections = Sections {
            image: &self.image,
            branch: &self.branch,
            history: &self.history,
        };
        Ok(write(&self.identity, self.revision, latched, sections))
    }
    pub fn conservation(&self) -> ConservationView<'_> {
        ConservationView {
            history: &self.history,
            basis: self.body.conserved_basis(),
            seal: HistorySeal {
                run: self.identity.digest(),
                branch: self.branch.id(),
                base: self.history.seal(),
            },
        }
    }
    /// One digest per part, recomputed from the live fields on every call.
    pub fn digests(&self) -> RecordDigests {
        let Self {
            body: _,
            identity: _,
            branch: _,
            revision,
            image,
            history,
        } = self;
        RecordDigests {
            revision: *revision,
            time: image.time,
            state: image.state.digest(),
            continuation: image.continuation.digest(),
            segment: image.dense.digest(),
            modes: image.state.modes_digest(),
            streams: image.state.streams_digest(),
            applied: image.pending.applied().digest(),
            waiting: image.pending.waiting_digest(),
            ledger: history.digest(),
        }
    }
}

/// What must stay byte-identical across a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordDigests {
    pub revision: u64,
    pub time: SimTime,
    pub state: Digest,
    pub continuation: Digest,
    pub segment: Digest,
    pub modes: Digest,
    pub streams: Digest,
    pub applied: Digest,
    pub waiting: Digest,
    pub ledger: Digest,
}

/// The entire next accepted image, validated and not yet visible.
pub struct NextImage {
    revision: u64,
    time: SimTime,
    state: Candidate,
    continuation: MethodHistory,
    dense: DenseSegment,
    pending: AppliedInputs,
    link: Link,
    applied: Vec<InputId>,
    after_breakpoint: bool,
    excursions: Vec<ValidityExcursion>,
    fired: Vec<Fired>,
}

#[derive(Debug)]
pub struct StepReceipt {
    revision: u64,
    seal: Seal,
    applied: Vec<InputId>,
    after_breakpoint: bool,
    excursions: Vec<ValidityExcursion>,
    fired: Vec<Fired>,
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
    pub fn after_breakpoint(&self) -> bool {
        self.after_breakpoint
    }
    pub fn excursions(&self) -> &[ValidityExcursion] {
        &self.excursions
    }
    pub fn fired(&self) -> &[Fired] {
        &self.fired
    }
}

/// A durable input reference from an authorized command's receipt, never a speculative request.
pub struct AuthorizedInput {
    command: Digest,
    event: BoundaryEvent,
}
impl AuthorizedInput {
    pub fn from_command_receipt(receipt: Digest, event: BoundaryEvent) -> Self {
        Self {
            command: receipt,
            event,
        }
    }
    /// No seal covers it, so an input with a command and the same input known at admission give the same
    /// bits. A host's own log pairs the two.
    pub fn command(&self) -> Digest {
        self.command
    }
}

/// What a host keeps of an admitted input, derived from the input alone, so one request has one receipt
/// whichever command repeats it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputReceipt {
    id: InputId,
    payload: Digest,
    at: SimTime,
}
impl InputReceipt {
    pub fn id(&self) -> InputId {
        self.id
    }
    pub fn payload(&self) -> Digest {
        self.payload
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
}
