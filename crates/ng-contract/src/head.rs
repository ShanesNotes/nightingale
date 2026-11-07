use crate::{
    AcceptedStateView, AdmissionError, AppliedInputs, Attempt, BoundaryEvent, Candidate, CompiledBody,
    CoupledProblem, DenseSegment, Digest, EngineFault, EngineFaultKind, ImageParts, InitialRevision,
    MethodHistory, NumericalRefusal, RecordHash, RunIdentity, SimTime, StepExtents, StepProposal, StepTicket,
    TrialFailure, ValidityExcursion, VerifiedParts, VerifiedStep,
};

/// A head's bound values, its image under the run's record hash (the image its seal links), and whether it
/// descends from an admission through steps that each took every input admitted before them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeadDigest {
    parameters: Digest,
    image: Digest,
    traced: bool,
}

/// One accepted head. Only `install` moves its parts, so its digest stays theirs.
pub struct AcceptedHead {
    parts: ImageParts,
    hash: RecordHash,
    digest: HeadDigest,
    admission: bool,
}
impl AcceptedHead {
    pub fn admitted(origin: InitialRevision, hash: RecordHash) -> Self {
        Self::sealed(origin.into_parts(), hash, true)
    }
    /// A head from parts verified elsewhere, such as checkpoint bytes. Neither it nor a head after it is traced,
    /// since its waiting inputs came from no admission.
    pub fn restored(parts: ImageParts, hash: RecordHash) -> Self {
        Self::sealed(parts, hash, false)
    }
    fn sealed(parts: ImageParts, hash: RecordHash, admission: bool) -> Self {
        let image = hash.image(
            parts.time,
            &parts.state,
            &parts.continuation,
            &parts.dense,
            parts.pending.applied(),
        );
        let digest = HeadDigest {
            parameters: parts.parameters().digest(),
            image,
            traced: admission,
        };
        Self {
            parts,
            hash,
            digest,
            admission,
        }
    }
    pub fn parts(&self) -> &ImageParts {
        &self.parts
    }
    pub fn view(&self) -> AcceptedStateView<'_> {
        let parts = &self.parts;
        AcceptedStateView::new(parts.time, &parts.state, &parts.continuation, &parts.dense)
    }
    pub fn image_digest(&self) -> Digest {
        self.digest.image
    }
    pub(crate) fn digest(&self) -> HeadDigest {
        self.digest
    }
    pub(crate) fn admission(&self) -> bool {
        self.admission
    }
    /// A late input waits, and no digest covers what waits.
    pub fn admit_input(
        &mut self,
        body: &CompiledBody,
        event: BoundaryEvent,
    ) -> Result<Digest, AdmissionError> {
        let parts = &mut self.parts;
        parts.pending.admit(body, parts.time, event)
    }
    /// One attempt of the next step on this head's own problem. `solve` proposes; the contract verifies, refuses
    /// a row outside its admissible set, then asks `accept`. A body or identity of another model or packet, and a
    /// proposal or refusal of another ticket, are `BindingMismatch`.
    pub fn attempt<T>(
        &self,
        body: &CompiledBody,
        identity: &RunIdentity,
        base: Digest,
        attempt: Attempt<'_>,
        mut solve: impl FnMut(&CoupledProblem<'_>, &StepTicket) -> Result<StepProposal, TrialFailure>,
        mut accept: impl FnMut(&VerifiedStep) -> Result<T, TrialFailure>,
    ) -> Result<(NextHead, T), TrialFailure> {
        let parts = &self.parts;
        let parameters = parts.parameters();
        if body.model_hash() != parameters.model
            || identity.model() != parameters.model
            || identity.packet() != parameters.packet
        {
            return Err(TrialFailure::EngineFault(EngineFault {
                kind: EngineFaultKind::BindingMismatch,
                at: parts.time,
                attempted_ns: 0,
                last_refusal: None,
            }));
        }
        let problem = CoupledProblem::new(body, parameters, identity, self.view(), base);
        let (step, accepted) = parts.pending.attempt(&problem, attempt, |ticket| {
            let proposal = solve(&problem, ticket).map_err(|failure| match failure {
                TrialFailure::Refused(refused) if refused.ticket() != ticket.digest() => {
                    ticket.mismatch().into()
                }
                other => other,
            })?;
            if proposal.ticket().digest() != ticket.digest() {
                return Err(ticket.mismatch().into());
            }
            let step = proposal.verify(&problem)?;
            for row in body.rows() {
                if !row.admissible().holds(step.value(row.id())) {
                    return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                        source: body.row_source(row.id()),
                    }));
                }
            }
            let accepted = accept(&step)?;
            Ok((step, accepted))
        })?;
        Ok((self.next(identity, step), accepted))
    }
    fn next(&self, identity: &RunIdentity, step: VerifiedStep) -> NextHead {
        let ticket = step.ticket();
        let (time, after_breakpoint) = (ticket.interval().end(), ticket.after_breakpoint());
        let applied = self.parts.pending.after(ticket);
        let inputs = step.applied().to_vec();
        let (forward, reverse) = step.gross().unzip();
        let VerifiedParts {
            state,
            continuation,
            dense,
            excursions,
            fired,
        } = step.into_parts();
        let digest = HeadDigest {
            image: self.hash.image(time, &state, &continuation, &dense, &applied),
            ..self.digest
        };
        NextHead {
            time,
            state,
            continuation,
            dense,
            applied,
            known: self.parts.pending.known(),
            digest,
            after_breakpoint,
            excursions,
            extents: StepExtents {
                run: identity.digest(),
                start: self.digest,
                end: digest,
                applied: inputs,
                forward,
                reverse,
                fired,
            },
        }
    }
    /// Becomes the head `next` settled. It cannot fail: a head settled from another head is the draw's to refuse,
    /// since its extents name where it started. An input admitted since the attempt leaves this head untraced,
    /// since the step could not take it.
    pub fn install(&mut self, next: NextHead) -> StepExtents {
        let NextHead {
            time,
            state,
            continuation,
            dense,
            applied,
            known,
            digest,
            after_breakpoint: _,
            excursions: _,
            extents,
        } = next;
        let traced = digest.traced && known == self.parts.pending.known();
        let parts = &mut self.parts;
        parts.time = time;
        parts.state = state;
        parts.continuation = continuation;
        parts.dense = dense;
        parts.pending.advance(applied);
        self.digest = HeadDigest { traced, ..digest };
        self.admission = false;
        extents
    }
}

/// The whole next head of one settled attempt, its image already digested.
pub struct NextHead {
    time: SimTime,
    state: Candidate,
    continuation: MethodHistory,
    dense: DenseSegment,
    applied: AppliedInputs,
    known: usize,
    digest: HeadDigest,
    after_breakpoint: bool,
    excursions: Vec<ValidityExcursion>,
    extents: StepExtents,
}
impl NextHead {
    pub fn image_digest(&self) -> Digest {
        self.digest.image
    }
    pub fn after_breakpoint(&self) -> bool {
        self.after_breakpoint
    }
    pub fn excursions(&self) -> &[ValidityExcursion] {
        &self.excursions
    }
}
