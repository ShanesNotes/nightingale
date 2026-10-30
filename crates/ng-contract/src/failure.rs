use crate::{
    AnatomyDefect, Binding, Digest, InputId, LawName, ModeValue, ParameterKey, Predicate, QuantityId, RowId,
    RowKind, SimTime, SiteId, SourceId, Space,
};

/// A refusal at a construction call, at `AnatomyBuilder::finish`, at `Assembly::compile`, or at admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    NonFinite,
    Anatomy(AnatomyDefect),
    Compile(CompileDefect),
    ScopeIssuedTwice {
        owner: &'static str,
    },
    /// `Mode::to` with the same value, or with a value of another mode variable.
    InvalidTransition {
        from: ModeValue,
        to: ModeValue,
    },
    ZeroRateInterval,
    ReversedValidityRange,
    NonPositivePartialVolume,
    /// A second partial-volume coefficient that differs from the first.
    PartialVolumeTwice,
    BoundaryNamedTwice {
        name: &'static str,
    },
    UnknownBoundary {
        name: &'static str,
    },
    ZeroDenominator,
    /// An absolute scale that is not positive, or a relative scale below zero.
    NonPositiveScale,
    /// A body with a mode variable.
    ModesNotYetRun {
        variable: LawName,
    },
    UnboundParameter {
        key: ParameterKey,
        binding: Binding,
    },
    UnknownParameter {
        key: ParameterKey,
        binding: Binding,
    },
    DuplicateParameter {
        key: ParameterKey,
        binding: Binding,
    },
    ParameterDimension {
        key: ParameterKey,
        binding: Binding,
    },
    ParameterOutsideSet {
        key: ParameterKey,
        binding: Binding,
    },
    MissingStart {
        row: RowKind,
    },
    UnknownStart {
        row: RowKind,
    },
    DuplicateStart {
        row: RowKind,
    },
    StartDimension {
        row: RowKind,
    },
    StartOutsideSet {
        row: RowKind,
    },
    /// A delivery ending at or before its event.
    EmptyDelivery {
        id: InputId,
    },
    NegativeDelivery {
        id: InputId,
    },
    /// One id with two different payloads.
    InputConflict {
        id: InputId,
    },
    /// An event before the moment the run starts from.
    EventBeforeHead {
        id: InputId,
    },
    /// Parameters bound on another compiled body.
    ForeignParameters,
    /// A workspace from another compiled body.
    ForeignWorkspace,
    /// A run identity that is not this body's, this origin's and this solver's.
    IdentityMismatch,
    ZeroMinimumInterval,
    /// A policy limit in a dimension the combination does not balance.
    QuantityDimension {
        quantity: QuantityId,
    },
    /// A certificate policy that does not hold one entry per combination, in basis order.
    PolicyCoverage,
    /// A sliding window coverage that does not exceed one window.
    ShortCoverage,
    /// An accuracy policy with no case, a case that never steps or a zero grid.
    EmptyAccuracyPolicy,
}
impl From<AnatomyDefect> for AdmissionError {
    fn from(defect: AnatomyDefect) -> Self {
        Self::Anatomy(defect)
    }
}
impl From<CompileDefect> for AdmissionError {
    fn from(defect: CompileDefect) -> Self {
        Self::Compile(defect)
    }
}

/// Each names the first offender in canonical order: owners by card, laws by name, pools by site key
/// and then space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileDefect {
    /// A declared slot, a requested route family or a declared boundary that no law defines. An
    /// omitted law is not zero.
    OmittedLaw { law: LawName },
    /// Two laws on one row that hold in one mode.
    SecondOwner {
        law: LawName,
        first: Predicate,
        second: Predicate,
    },
    /// A value of the row's mode variable that no law on the row holds in.
    UncoveredMode { law: LawName, mode: ModeValue },
    /// A guard whose zero kernel does not hold in the value its transition leaves.
    GuardOutsideMode { variable: LawName, leaves: &'static str },
    /// A scope this assembly issued whose fragment was never added.
    MissingFragment { owner: &'static str },
    /// Two slots, mode variables, route families or schema boundaries under one name in one owner.
    NamedTwice { law: LawName },
    /// One parameter key declared with two dimensions, contexts or admissible sets.
    ParameterConflict { key: ParameterKey },
    /// A mode variable with no values, or with one value twice.
    BadModeVariable { variable: LawName },
    /// An algebraic row that no structural matching pairs with an algebraic unknown it reads.
    StructuralSingularity { law: LawName },
    /// A pool this body models that no transfer moves. A read never moves a pool, and a transfer whose
    /// two ends are one pool moves nothing.
    OrphanPool { site: SiteId, space: Space },
    /// A law that moves or reads a pool in a space the schema leaves out.
    LeftOutTouched {
        law: LawName,
        site: SiteId,
        space: Space,
    },
    /// A law or a guard that reads a volume when the schema declares no partial-volume coefficient. A
    /// guard is named by its mode variable.
    MissingPartialVolume { law: LawName },
    /// A water boundary at a space its site does not have, or at a site the anatomy does not declare.
    BoundaryEndpoint { law: LawName },
}

/// Why one attempt cannot stand. A smaller retry of the same base can still succeed. A row or source id
/// names the law through `CompiledBody`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericalRefusal {
    /// The scheme's error estimate exceeds one on this row.
    Accuracy { row: RowId },
    /// The core's iteration stopped short of its own convergence.
    IterationLimit,
    /// A trial left a set, or an operator's domain, that the source names.
    TrialDomain { source: SourceId },
    /// A stage equation is unmet on this row.
    StageResidual { row: RowId },
    /// A conserved sum or a cumulative exchange left the finite range, so the ledger cannot hold the step.
    LedgerOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineFaultKind {
    /// A refusal whose retry would pass the floor or the retry budget.
    NoAdmissibleStep,
    /// Something bound to one attempt, layout or base reached another: a core bug no smaller step fixes.
    BindingMismatch,
    /// The stages are not the declared scheme's on this body.
    MethodMismatch,
}

/// The run stops here for diagnosis. It never writes death (rule 7), and no conversion to `Demise` exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineFault {
    pub(crate) kind: EngineFaultKind,
    pub(crate) at: SimTime,
    pub(crate) attempted_ns: u64,
    pub(crate) last_refusal: Option<NumericalRefusal>,
}
impl EngineFault {
    pub fn kind(&self) -> EngineFaultKind {
        self.kind
    }
    /// The instant the attempt started from.
    pub fn at(&self) -> SimTime {
        self.at
    }
    pub fn attempted_ns(&self) -> u64 {
        self.attempted_ns
    }
    /// The refusal that ended a `NoAdmissibleStep`.
    pub fn last_refusal(&self) -> Option<NumericalRefusal> {
        self.last_refusal
    }
}

/// A retryable attempt with the record untouched. Only `StepTicket::refuse` makes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused {
    pub(crate) reason: NumericalRefusal,
    pub(crate) ticket: Digest,
    pub(crate) base: Digest,
    pub(crate) at: SimTime,
    pub(crate) attempted_ns: u64,
    pub(crate) retry_ns: u64,
    pub(crate) attempt: u32,
}
impl Refused {
    pub fn reason(&self) -> NumericalRefusal {
        self.reason
    }
    pub fn ticket(&self) -> Digest {
        self.ticket
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
    pub fn attempted_ns(&self) -> u64 {
        self.attempted_ns
    }
    pub fn retry_ns(&self) -> u64 {
        self.retry_ns
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialFailure {
    Refused(Refused),
    EngineFault(EngineFault),
}
impl From<EngineFault> for TrialFailure {
    fn from(fault: EngineFault) -> Self {
        Self::EngineFault(fault)
    }
}

pub struct Demise {
    _rhythm: ArrestRhythm,
    _injury: IrreversibleInjury,
}
pub enum ArrestRhythm {
    VentricularFibrillation,
    PulselessVentricularTachycardia,
    PulselessElectricalActivity,
    Asystole,
}
pub struct IrreversibleInjury {
    _accepted_revision: u64,
}
