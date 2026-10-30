//! Failure kinds. Numerical refusal, engine fault and physiological demise have no conversion
//! between them. `TrialFailure` is only the union a trial step can return.
use super::{AnchorId, Digest, LawOrigin, RowId, SimTime, SourceId};

/// A retryable numerical attempt; the accepted record is untouched. Only `StepTicket::refuse`
/// makes one.
pub struct Refused {
    _reason: NumericalRefusal,
    _ticket: Digest,
    _base: Digest,
    _at: SimTime,
    _attempted_ns: u64,
    _retry_ns: u64,
    _attempt: u32,
}
impl Refused {
    pub fn reason(&self) -> NumericalRefusal {
        todo!()
    }
    /// The ticket this refuses. A refusal of any ticket but the one the session issued is
    /// `BindingMismatch`, so no core resets its budget through a fresh ticket.
    pub fn ticket(&self) -> Digest {
        todo!()
    }
    pub fn at(&self) -> SimTime {
        todo!()
    }
    pub fn attempted_ns(&self) -> u64 {
        todo!()
    }
    /// The strictly smaller span a retry solves, over the same base and inputs.
    pub fn retry_ns(&self) -> u64 {
        todo!()
    }
}
/// A row or source names its law through the compiled body.
#[derive(Clone, Copy)]
pub enum NumericalRefusal {
    /// The scheme's error estimate exceeds one on this row.
    Accuracy { row: RowId },
    /// The core's iteration stopped short of its own convergence.
    IterationLimit,
    /// A trial left an `Admissible` set or an operator's domain, or came back non-finite. A
    /// `ValidityRange` never produces a refusal.
    TrialDomain { source: SourceId },
    /// The witnessed stages do not satisfy the declared scheme's stage equations within the scales.
    StageResidual { row: RowId },
    /// A conserved sum or a cumulative exchange left the finite range, so the ledger cannot hold the step.
    LedgerOverflow,
    SplitNotConverged,
}
/// The solver or model cannot produce a possible body under the declared policy. The session
/// latches on it (`Session<Faulted>`); it is never a death. Only the contract makes one.
pub struct EngineFault {
    _kind: EngineFaultKind,
    _at: SimTime,
    _attempted_ns: u64,
    _last_refusal: Option<NumericalRefusal>,
}
impl EngineFault {
    pub fn kind(&self) -> &EngineFaultKind {
        todo!()
    }
    /// The instant the attempt started from.
    pub fn at(&self) -> SimTime {
        todo!()
    }
    pub fn attempted_ns(&self) -> u64 {
        todo!()
    }
    /// The refusal that ended a `NoAdmissibleStep`, whose row or source names the failing law.
    pub fn last_refusal(&self) -> Option<NumericalRefusal> {
        todo!()
    }
}
pub enum EngineFaultKind {
    /// The retry budget is spent, or a retry would go below the declared minimum interval.
    NoAdmissibleStep,
    InvalidMode,
    /// A value bound to one compiled layout, base or attempt reached another: a solver bug.
    BindingMismatch,
    /// The stages are not the declared scheme's on this body: a witnessed offset or weight differs,
    /// a replayed rate or the weighted sum differs, or the method history outlived a breakpoint.
    MethodMismatch,
    UnqualifiedSplit,
}
pub enum TrialFailure {
    Refused(Refused),
    EngineFault(EngineFault),
}
impl From<EngineFault> for TrialFailure {
    fn from(fault: EngineFault) -> Self {
        TrialFailure::EngineFault(fault)
    }
}
pub struct AdmissionError {
    _reason: String,
}
pub enum InitializationFailure {
    Admission(AdmissionError),
    NotConverged { iterations: u32 },
    AnchorUnreachable { anchor: AnchorId },
    Singular { rows: Vec<RowId> },
    /// A row the solved workspace leaves inconsistent with the packet, its set or its residual.
    Inconsistent { row: RowId },
    /// A law the solved workspace leaves without a value, such as a divisor at zero.
    Undefined { law: LawOrigin },
}
pub struct CheckpointError {
    _reason: String,
}

/// Types reserved now; no death-producing operation exists in this sketch.
pub struct Demise {
    _rhythm: ArrestRhythm,
    _injury: IrreversibleInjury,
    _evidence: PhysiologicalEvidence,
}
pub enum ArrestRhythm {
    VentricularFibrillation,
    PulselessVentricularTachycardia,
    Pea,
    Asystole,
}
pub struct IrreversibleInjury {
    _integral: f64,
}
pub struct PhysiologicalEvidence {
    _accepted_revision: u64,
}
