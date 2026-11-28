use crate::{
    AcceptedStateView, AdmissibleSet, AdmissionError, AppliedInputs, Attempt, BoundaryEvent, Candidate,
    CompiledBody, CoupledProblem, DenseSegment, Digest, EngineFault, EngineFaultKind, ImageParts,
    InitialRevision, MethodHistory, NumericalRefusal, RecordHash, RhythmId, RunIdentity, SimTime,
    StepExtents, StepProposal, StepTicket, TrialFailure, ValidityExcursion, VerifiedParts, VerifiedStep,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeadDigest {
    pub(crate) parameters: Digest,
    pub(crate) image: Digest,
    pub(crate) from_admission: bool,
}

/// Only `install` moves its parts, so its digest stays theirs.
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
    /// A head from parts verified elsewhere, such as checkpoint bytes. No split trace reads it or a head after it,
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
            from_admission: admission,
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
        let problem = CoupledProblem::for_head(body, parameters, identity, self.view(), base, self.digest);
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
            check_admissible(body, &step)?;
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
            rhythm_edges,
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
            rhythm_edges,
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
    /// since its extents name where it started. An input admitted since the attempt puts this head out of
    /// every split trace, since the step could not take it.
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
            rhythm_edges: _,
            extents,
        } = next;
        let from_admission = digest.from_admission
            && digest.parameters == self.digest.parameters
            && known == self.parts.pending.known();
        let parts = &mut self.parts;
        parts.time = time;
        parts.state = state;
        parts.continuation = continuation;
        parts.dense = dense;
        parts.pending.advance(applied);
        self.digest = HeadDigest {
            parameters: self.digest.parameters,
            image: digest.image,
            from_admission,
        };
        self.admission = false;
        extents
    }
}

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
    rhythm_edges: Vec<RhythmId>,
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
    /// Each rhythm whose phase this step's firing reset, in `RhythmId` order: the step ends on an edge of each.
    pub fn rhythm_edges(&self) -> &[RhythmId] {
        &self.rhythm_edges
    }
}

fn holds_float(set: AdmissibleSet, value: f64) -> bool {
    value.is_finite()
        && match set {
            AdmissibleSet::Finite => true,
            AdmissibleSet::Nonnegative => value >= 0.0,
            AdmissibleSet::Positive => value > 0.0,
            AdmissibleSet::UnitInterval => (0.0..=1.0).contains(&value),
        }
}

fn check_admissible(body: &CompiledBody, step: &VerifiedStep) -> Result<(), TrialFailure> {
    let state = step.state();
    let admissible = body.tape.admissible.as_slice();
    let amounts_end = state.amounts.len();
    let memory_end = amounts_end + state.memory.len();
    for (at, (&set, &amount)) in admissible[..amounts_end].iter().zip(&state.amounts).enumerate() {
        if !set.holds(amount) {
            return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                source: body.row_source(body.rows()[at].id()),
            }));
        }
    }
    for (offset, (&set, &value)) in admissible[amounts_end..memory_end]
        .iter()
        .zip(&state.memory)
        .enumerate()
    {
        if !holds_float(set, value) {
            return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                source: body.row_source(body.rows()[amounts_end + offset].id()),
            }));
        }
    }
    for (offset, (&set, &value)) in admissible[memory_end..].iter().zip(&state.algebraic).enumerate() {
        if !holds_float(set, value) {
            return Err(step.ticket().refuse(NumericalRefusal::TrialDomain {
                source: body.row_source(body.rows()[memory_end + offset].id()),
            }));
        }
    }
    Ok(())
}
