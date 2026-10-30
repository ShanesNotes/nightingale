//! The admission solve: once, through `Initializer`, yielding the only `InitialRevision` a session
//! can be admitted from. The request, the guess and the provenance all have public producers;
//! provenance is computed by the contract from the bound packet, never supplied by the solver.
use super::*;

pub enum InitializationGoal {
    ConsistentInstant,
    /// Adds periodicity rows over one period (multiple shooting on the same evaluation surface).
    Periodic { period_ns: u64 },
}
pub struct InitializationBudget {
    _iterations: u32,
    _maximum_span_ns: u64,
}
impl InitializationBudget {
    pub fn new(_iterations: u32, _maximum_span_ns: u64) -> Self {
        todo!()
    }
}
/// Where the admission workspace starts.
pub enum InitialStateGuess {
    /// The bound packet's anchor and design values.
    PacketValues,
}
pub struct InitializationRequest {
    _goal: InitializationGoal,
    _budget: InitializationBudget,
    _guess: InitialStateGuess,
    _anchor_moment: SimTime,
}
impl InitializationRequest {
    pub fn new(
        _goal: InitializationGoal,
        _budget: InitializationBudget,
        _guess: InitialStateGuess,
        _anchor_moment: SimTime,
    ) -> Self {
        todo!()
    }
    /// Consistent at the anchor moment from the packet's start values, with no calibration.
    pub fn consistent_instant(_anchor_moment: SimTime) -> Self {
        todo!()
    }
}
impl CompiledBody {
    /// The square admission system: state unknowns plus calibrated parameters; consistency
    /// residuals plus one row per calibration target anchor (plus periodicity rows if asked).
    pub fn initialization<'m>(
        &'m self,
        _parameters: BoundParameters,
        _boundary: BoundaryPort,
        _request: InitializationRequest,
    ) -> Result<InitializationSystem<'m>, AdmissionError> {
        todo!()
    }
}
pub struct InitializationSystem<'m> {
    _body: &'m CompiledBody,
    _parameters: BoundParameters,
    _pending: PendingInputs,
    _request: InitializationRequest,
}
impl InitializationSystem<'_> {
    pub fn workspace(&self) -> SolveWorkspace {
        todo!()
    }
    pub fn evaluate(&self, _workspace: &SolveWorkspace) -> Result<StageEvaluation, InitializationFailure> {
        todo!()
    }
    pub fn linearize(&self, _workspace: &SolveWorkspace) -> Result<Jacobian, InitializationFailure> {
        todo!()
    }
    /// The only constructor of `InitialRevision`: rechecks every row and anchor, writes fitted
    /// values into the bound parameters, and records Specified/Derived/Calibrated/Fixture
    /// provenance per value from the packet. Specified conserved totals cannot move.
    pub fn admit(self, _solved: SolveWorkspace, _history: MethodHistory) -> Result<InitialRevision, InitializationFailure> {
        todo!()
    }
}
/// The first accepted revision: the continuation point, never re-imposed (no daily reset).
pub struct InitialRevision {
    _state: Candidate,
    _parameters: BoundParameters,
    _continuation: MethodHistory,
    /// No interval yet: the first step's chord starts from the admitted state.
    _dense: DenseSegment,
    _pending: PendingInputs,
    _provenance: InitializationProvenance,
}
impl InitialRevision {
    pub fn packet_hash(&self) -> PacketHash {
        todo!()
    }
    pub fn model_hash(&self) -> ModelHash {
        todo!()
    }
    /// What the record takes on at admission.
    pub fn into_parts(self) -> ImageParts {
        todo!()
    }
    pub fn provenance(&self) -> &InitializationProvenance {
        todo!()
    }
}
/// What the contract owns of one accepted image. Admission makes the first, and a record moves it
/// forward. Parts enter a record only through `InitialRevision`, which only
/// `InitializationSystem::admit` makes, or through checkpoint bytes the record has verified, so `parse`
/// mints nothing a record takes.
pub struct ImageParts {
    pub time: SimTime,
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub pending: PendingInputs,
    pub parameters: BoundParameters,
}
impl ImageParts {
    /// One image has one output, so a checkpoint's bytes do not depend on when it was taken.
    pub fn canonical(&self) -> Vec<u8> {
        todo!()
    }
    /// Reads `canonical` bytes against `body`. Any bytes give parts or `Malformed`, never a panic.
    pub fn parse(_body: &CompiledBody, _bytes: &[u8]) -> Result<Self, Malformed> {
        todo!()
    }
}
/// Bytes that are not the canonical form they were read as.
pub struct Malformed;
pub struct InitializationProvenance {
    _values: Vec<(ParameterKey, ValueProvenance)>,
    _anchors_met: Vec<AnchorId>,
}
pub trait Initializer {
    fn initialize(&self, system: InitializationSystem<'_>) -> Result<InitialRevision, InitializationFailure>;
}
