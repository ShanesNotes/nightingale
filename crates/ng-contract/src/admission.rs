use crate::body::{Bound, RowKind};
use crate::boundary::{Delivery, PendingInputs};
use crate::chance::{HELD_DRAW, HELD_FACTOR};
use crate::evaluate::Inputs;
use crate::problem::SolveWorkspace;
use crate::{
    AdmissionError, BoundParameters, BoundaryPort, Candidate, CompiledBody, DenseSegment, ImageParts,
    LawName, MethodHistory, ModelHash, PacketHash, RowId, SimTime,
};

pub struct InitializationRequest {
    anchor_moment: SimTime,
}
impl InitializationRequest {
    pub fn consistent_instant(anchor_moment: SimTime) -> Self {
        Self { anchor_moment }
    }
}

impl CompiledBody {
    fn reads_drive(&self, boundary: &str) -> bool {
        (self.layout.instances.iter().flat_map(|instance| &instance.inputs))
            .any(|input| matches!(input, Bound::Boundary(name) if *name == boundary))
    }
    pub(crate) fn check_delivery(
        &self,
        delivery: &Delivery,
        earliest: SimTime,
    ) -> Result<(), AdmissionError> {
        let boundaries = &self.layout.schema.boundaries;
        if !boundaries.iter().any(|known| known.name == delivery.boundary) {
            return Err(AdmissionError::UnknownBoundary {
                name: delivery.boundary,
            });
        }
        if delivery.at < earliest {
            return Err(AdmissionError::EventBeforeHead { id: delivery.id });
        }
        if !self.reads_drive(delivery.boundary) {
            return Err(AdmissionError::DriveNotRead {
                name: delivery.boundary,
            });
        }
        Ok(())
    }
    pub(crate) fn boundary_name(&self, name: &str) -> Option<&'static str> {
        let boundaries = &self.layout.schema.boundaries;
        boundaries
            .iter()
            .find(|known| known.name == name)
            .map(|known| known.name)
    }
    /// Refuses parameters bound on another body and an event `check_delivery` refuses at the anchor moment.
    pub fn initialization<'m>(
        &'m self,
        parameters: BoundParameters,
        boundary: BoundaryPort,
        request: InitializationRequest,
    ) -> Result<InitializationSystem<'m>, AdmissionError> {
        if parameters.model != self.model_hash() {
            return Err(AdmissionError::ForeignParameters);
        }
        for delivery in &boundary.deliveries {
            self.check_delivery(delivery, request.anchor_moment)?;
        }
        Ok(InitializationSystem {
            body: self,
            parameters,
            pending: PendingInputs::opening(boundary, request.anchor_moment),
            request,
        })
    }
}

pub struct InitializationSystem<'m> {
    body: &'m CompiledBody,
    parameters: BoundParameters,
    pending: PendingInputs,
    request: InitializationRequest,
}
impl InitializationSystem<'_> {
    /// Seeded with the packet's start values.
    pub fn workspace(&self) -> SolveWorkspace {
        SolveWorkspace::seeded(self.body, self.parameters.starts.clone())
    }
    /// The only constructor of `InitialRevision`. Refuses a workspace from another body. Refuses by row a
    /// moved amount or memory value, a value outside its admissible set and an algebraic row that is not
    /// exactly zero, and by law a law with no value at the start.
    pub fn admit(
        self,
        solved: SolveWorkspace,
        history: MethodHistory,
    ) -> Result<InitialRevision, InitializationFailure> {
        let Self {
            body,
            parameters,
            pending,
            request,
        } = self;
        if solved.layout() != body.model_hash().digest() {
            return Err(AdmissionError::ForeignWorkspace.into());
        }
        let unknowns = solved.coordinates();
        for (row, (&value, &start)) in body.rows().iter().zip(unknowns.iter().zip(&parameters.starts)) {
            let moved = value.to_bits() != start.to_bits();
            let algebraic = matches!(row.kind(), RowKind::Algebraic(_));
            let outside = !row.admissible().holds(crate::Compensated::exact(value));
            if (moved && !algebraic) || outside {
                return Err(InitializationFailure::Inconsistent { row: row.id() });
            }
        }
        let time = request.anchor_moment;
        let frame = pending.plan(time, false).frame;
        let draws = vec![HELD_DRAW; body.layout.streams.len()];
        let factors = vec![HELD_FACTOR; body.layout.forcings.len()];
        let inputs = Inputs {
            parameters: &parameters.values,
            frame: &frame,
            unknowns,
            modes: &parameters.modes,
            draws: &draws,
            factors: &factors,
        };
        let values = (body.instance_values(&inputs))
            .map_err(|undefined| InitializationFailure::Undefined { law: undefined.law })?;
        let (columns, rows) = body.targets(&values);
        let residuals = body.residuals(&columns, &rows, &vec![0.0; rows.len()]);
        for row in body.rows() {
            let solved = residuals[row.id().index()] == 0.0;
            if matches!(row.kind(), RowKind::Algebraic(_)) && !solved {
                return Err(InitializationFailure::Inconsistent { row: row.id() });
            }
        }
        Ok(InitialRevision {
            parts: ImageParts {
                time,
                state: Candidate::of_coordinates(body, unknowns, parameters.modes.clone()),
                continuation: history,
                dense: DenseSegment::empty(),
                pending,
                parameters,
            },
        })
    }
}

pub struct InitialRevision {
    parts: ImageParts,
}
impl InitialRevision {
    pub fn packet_hash(&self) -> PacketHash {
        self.parts.parameters.packet
    }
    pub fn model_hash(&self) -> ModelHash {
        self.parts.parameters.model
    }
    pub fn into_parts(self) -> ImageParts {
        self.parts
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitializationFailure {
    Admission(AdmissionError),
    /// A row the solved workspace leaves inconsistent with the packet, its set or its residual.
    Inconsistent {
        row: RowId,
    },
    /// A law the solved workspace leaves without a value, such as a divisor at zero.
    Undefined {
        law: LawName,
    },
}
impl From<AdmissionError> for InitializationFailure {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}
