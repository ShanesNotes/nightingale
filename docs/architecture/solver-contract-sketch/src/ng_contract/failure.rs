//! Failure kinds. Numerical refusal, engine fault and physiological demise have no conversion
//! between them. `TrialFailure` is only the union a trial step can return.
use super::{AnchorId, LawOrigin, RetryTicket, RowId, SimTime};

/// A retryable numerical attempt; the accepted record is untouched.
pub struct Refused {
    _reason: NumericalRefusal,
    _retry: RetryTicket,
    _diagnosis: Vec<RowDiagnosis>,
}
impl Refused {
    /// The same base and inputs over a strictly smaller interval, with the remaining budget.
    pub fn retry(&self) -> &RetryTicket {
        todo!()
    }
}
pub enum NumericalRefusal {
    Accuracy,
    IterationLimit,
    /// A trial value left its `Admissible` set or an operator's precondition. A `ValidityRange`
    /// never produces a refusal.
    TrialDomain,
    /// The witnessed stages do not satisfy the declared scheme's stage equations within the scales.
    StageResidual,
    SplitNotConverged,
    /// The proposal no longer matches the record's accepted base.
    StaleBase,
}
/// The solver or model cannot produce a possible body under the declared policy. The session
/// latches on it (`Session<Faulted>`); it is never a death.
pub struct EngineFault {
    _kind: EngineFaultKind,
    _at: SimTime,
    _attempted_ns: u64,
    _diagnosis: Vec<RowDiagnosis>,
}
impl EngineFault {
    /// `attempted_ns` is the interval that could not be solved; `diagnosis` names each failing row,
    /// the law it belongs to and its residual, so a structural defect is not retried blindly.
    pub fn new(_kind: EngineFaultKind, _at: SimTime, _attempted_ns: u64, _diagnosis: Vec<RowDiagnosis>) -> Self {
        todo!()
    }
}
/// One failing row mapped back to its law; made by `CoupledProblem::diagnose`.
pub struct RowDiagnosis {
    _row: RowId,
    _origin: LawOrigin,
    _residual: f64,
}
pub enum EngineFaultKind {
    ImpossibleAcceptedState,
    /// The retry budget is spent, or a retry would go below the declared minimum interval.
    NoAdmissibleStep,
    NonFiniteModel,
    InvalidMode,
    /// A value bound to one compiled layout or ticket was used with another: a solver bug.
    BindingMismatch,
    /// A witnessed stage's recorded rates differ from the rates the compiled body gives at the
    /// recorded coordinates, or the increment is not the scheme's weighted sum of them: a solver bug.
    MethodMismatch,
    UnqualifiedSplit,
}
pub enum TrialFailure {
    Refused(Refused),
    EngineFault(EngineFault),
}
impl From<Refused> for TrialFailure {
    fn from(refused: Refused) -> Self {
        TrialFailure::Refused(refused)
    }
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
