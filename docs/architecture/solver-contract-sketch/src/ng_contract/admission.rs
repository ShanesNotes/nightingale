//! The admission solve: once, yielding the only `InitialRevision` a session can be admitted from. The goal, the
//! tolerance and the anchors come from the packet; provenance is computed by the contract from the packet, never
//! supplied by a solver.
use super::*;

/// When the start stands and what reads the anchors.
pub struct InitializationRequest<'l> {
    _anchor_moment: SimTime,
    _lens: Option<&'l ObservationLens>,
}
impl InitializationRequest<'static> {
    /// A packet with an anchor needs `reading` too.
    pub fn at(_anchor_moment: SimTime) -> Self {
        todo!()
    }
}
impl InitializationRequest<'_> {
    /// Every anchor must be read by a channel of `lens` with its method and window (rule 17).
    pub fn reading(self, _lens: &ObservationLens) -> InitializationRequest<'_> {
        todo!()
    }
}
impl CompiledBody {
    /// The square admission system: `N` points of the rows plus one unknown per fit. Each point is one backward
    /// Euler stage around the ring of the packet's `Cycle`, one conserved total replaces one row of each
    /// combination, and each fit adds its anchor's row. An instant is one point with no ring.
    pub fn initialization<'m>(
        &'m self,
        _parameters: BoundParameters,
        _boundary: BoundaryPort,
        _request: InitializationRequest<'_>,
    ) -> Result<InitializationSystem<'m>, AdmissionError> {
        todo!()
    }
}
pub struct InitializationSystem<'m> {
    _body: &'m CompiledBody,
    _parameters: BoundParameters,
    _pending: PendingInputs,
    _anchor_moment: SimTime,
}
impl InitializationSystem<'_> {
    pub fn workspace(&self) -> SolveWorkspace {
        todo!()
    }
    /// Whether `admit` would pass this workspace, so a solve stops where the judge would.
    pub fn holds(&self, _workspace: &SolveWorkspace) -> bool {
        todo!()
    }
    /// One row per unknown, in the workspace's order.
    pub fn evaluate(&self, _workspace: &SolveWorkspace) -> Result<Vec<f64>, InitializationFailure> {
        todo!()
    }
    pub fn linearize(&self, _workspace: &SolveWorkspace) -> Result<Jacobian, InitializationFailure> {
        todo!()
    }
    /// The only constructor of `InitialRevision`: rechecks every row, total and anchor on the workspace alone,
    /// with no core and no solver, writes fitted values into the bound parameters and reports provenance.
    pub fn admit(self, _solved: SolveWorkspace, _history: MethodHistory) -> Result<InitialRevision, InitializationFailure> {
        todo!()
    }
}
/// What a refusal names: a body row, a conserved total or an anchor.
pub enum AdmissionRow {
    Body(RowId),
    Total(QuantityId),
    Anchor(AnchorId),
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
/// What the contract owns of one accepted image. Parts enter a record only through `InitialRevision`,
/// which only `InitializationSystem::admit` makes, or through checkpoint bytes the record has verified,
/// so `parse` mints nothing a record takes.
pub struct ImageParts {
    pub time: SimTime,
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub pending: PendingInputs,
    parameters: BoundParameters,
}
impl ImageParts {
    /// Lent, never moved, so bound values in a parsed image cannot seed a new admission.
    pub fn parameters(&self) -> &BoundParameters {
        &self.parameters
    }
    pub fn canonical(&self) -> Vec<u8> {
        todo!()
    }
    /// Any bytes give parts or `Malformed`, never a panic.
    pub fn parse(_body: &CompiledBody, _bytes: &[u8]) -> Result<Self, Malformed> {
        todo!()
    }
}
pub struct Malformed;
/// Where every value of an admitted start came from, the tolerance it was held to and the row nearest it.
pub struct InitializationProvenance {
    _values: Vec<ValueRecord>,
    _tolerance: ErrorScales,
    _worst: Option<(AdmissionRow, Compensated)>,
}
