use crate::{
    AnatomyDefect, AnchorId, Binding, Digest, InputId, LawName, ModeValue, ParameterKey, Predicate,
    QuantityId, RowId, RowKind, SimTime, SiteId, SourceId, Space,
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
    /// A mode variable a plain guard fires or no hazard guard moves, any mode variable in a body with an
    /// algebraic row, which a firing could leave false, and any mode variable under a cycle, whose hazard integral
    /// never returns. A guard that silently never fires is worse than a run that never starts.
    ModesNotYetRun {
        variable: LawName,
    },
    /// A nonzero constant built only from literals, or a division of two equal nodes, which no packet entry
    /// backs. This check is partial: it reads the graph, never a value, so a copy built from separate reads
    /// passes.
    UnbackedConstant {
        law: LawName,
    },
    /// A forcing's period that is no whole number of nanoseconds in the clock's range, which a run could only
    /// round into another period.
    ForcingPeriod {
        key: ParameterKey,
    },
    MissingStartMode {
        variable: LawName,
    },
    UnknownStartMode {
        variable: LawName,
        value: &'static str,
    },
    DuplicateStartMode {
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
    UnknownChannel {
        row: RowKind,
    },
    ChannelDimension {
        row: RowKind,
    },
    NegativeChannelError {
        row: RowKind,
    },
    /// Two channels that read one row one way, which would share their draws.
    DuplicateChannel {
        row: RowKind,
    },
    /// A delivery ending at or before its event.
    EmptyDelivery {
        id: InputId,
    },
    NegativeDelivery {
        id: InputId,
    },
    /// An amount so large over its span that the rate left the finite range.
    UnboundedDelivery {
        id: InputId,
    },
    /// An event on a boundary whose law never reads its drive, so the amount would never arrive.
    DriveNotRead {
        name: &'static str,
    },
    /// One id with two different payloads.
    InputConflict {
        id: InputId,
    },
    /// An event before the moment the run starts from, or before the accepted head.
    EventBeforeHead {
        id: InputId,
    },
    /// An input stamped behind the fence a cut left by not publishing at its own end, or inside the admission
    /// window, whose drives initialization has already read.
    EventBeforeFence {
        id: InputId,
        fence: SimTime,
    },
    /// A period the cycle's interval count does not divide.
    UnevenCycle,
    DuplicateAnchor {
        anchor: AnchorId,
    },
    /// An anchor on a row this body does not have, or in another dimension than the row's.
    AnchorRow {
        anchor: AnchorId,
    },
    /// An anchor on a row that reads no amount and no drive, so its value is a function of parameters alone and
    /// the anchor would be a parameter's value.
    AnchorOnParameters {
        anchor: AnchorId,
    },
    /// An instant anchor on a row in the wave or beat band: such a row has no one value to meet.
    InstantOnPulsatile {
        anchor: AnchorId,
    },
    /// A cycle-mean anchor in a packet with no cycle.
    NoCycle {
        anchor: AnchorId,
    },
    /// An anchor no channel of the request's lens reads with its method and window (rule 17).
    AnchorUnread {
        anchor: AnchorId,
    },
    /// A calibrated entry that names an anchor its packet does not hold.
    UnknownAnchor {
        key: ParameterKey,
        binding: Binding,
    },
    /// Two calibrated entries on one anchor, which would leave the system with more unknowns than rows.
    AnchorFittedTwice {
        anchor: AnchorId,
    },
    /// A derived entry whose source the packet does not hold, or one that is itself derived or fitted.
    IdentitySource {
        key: ParameterKey,
        source: ParameterKey,
    },
    /// A calibrated entry no law behind its anchor's row reads, so the fit could only stamp its guess.
    FitUnread {
        key: ParameterKey,
        binding: Binding,
    },
    /// A second population record for a key, a lower bound above the upper bound, or a width past the finite range.
    PopulationRange {
        key: ParameterKey,
    },
    /// A value that needs a population record its packet does not hold yet.
    NoPopulation {
        key: ParameterKey,
    },
    /// An authored value outside its population record's range (I16).
    OutsidePopulation {
        key: ParameterKey,
        binding: Binding,
    },
    /// A delivery that starts or ends between two points of the cycle.
    DeliveryOffGrid {
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
    /// A stream no guard draws on, so its ordinal would never move.
    UndrawnStream { stream: LawName },
    /// A forcing no law reads.
    UnreadForcing { forcing: LawName },
    /// A second guard on one stream: one draw cannot be two events' threshold.
    SecondGuardOnStream { stream: LawName },
    /// A second guard on one integral: one reset cannot serve two thresholds, because a firing would erase what
    /// the other guard crossed.
    SecondGuardOnIntegral { integral: LawName },
    /// A hazard integral not declared nonnegative, the set that holds the reset and every threshold.
    HazardSet { integral: LawName },
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
    pub(crate) fence: SimTime,
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
