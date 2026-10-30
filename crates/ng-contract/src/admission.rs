use crate::body::RowKind;
use crate::boundary::PendingInputs;
use crate::evaluate::Inputs;
use crate::problem::SolveWorkspace;
use crate::{
    AdmissionError, BoundParameters, BoundaryPort, Candidate, CompiledBody, DenseSegment, LawName,
    MethodHistory, ModelHash, PacketHash, RowId, SimTime,
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
    /// Refuses parameters bound on another body, an event naming a boundary the schema does not declare and
    /// an event before the anchor moment.
    pub fn initialization<'m>(
        &'m self,
        parameters: BoundParameters,
        boundary: BoundaryPort,
        request: InitializationRequest,
    ) -> Result<InitializationSystem<'m>, AdmissionError> {
        if parameters.model != self.model_hash() {
            return Err(AdmissionError::ForeignParameters);
        }
        let boundaries = &self.layout.schema.boundaries;
        for delivery in &boundary.deliveries {
            if !boundaries.iter().any(|known| known.name == delivery.boundary) {
                return Err(AdmissionError::UnknownBoundary {
                    name: delivery.boundary,
                });
            }
            if delivery.at < request.anchor_moment {
                return Err(AdmissionError::EventBeforeHead { id: delivery.id });
            }
        }
        Ok(InitializationSystem {
            body: self,
            parameters,
            pending: PendingInputs::opening(boundary),
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
    /// exactly zero.
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
        if (body.rows().iter()).any(|row| matches!(row.kind(), RowKind::Algebraic(_))) {
            let frame = pending.plan(time).frame;
            let inputs = Inputs {
                parameters: &parameters.values,
                frame: &frame,
                unknowns,
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
        }
        Ok(InitialRevision {
            parts: InitialParts {
                time,
                state: Candidate::of_coordinates(body, unknowns),
                continuation: history,
                dense: DenseSegment::empty(),
                pending,
                parameters,
            },
        })
    }
}

pub struct InitialRevision {
    parts: InitialParts,
}
pub struct InitialParts {
    pub time: SimTime,
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub pending: PendingInputs,
    pub parameters: BoundParameters,
}
impl InitialRevision {
    pub fn packet_hash(&self) -> PacketHash {
        self.parts.parameters.packet
    }
    pub fn model_hash(&self) -> ModelHash {
        self.parts.parameters.model
    }
    pub fn into_parts(self) -> InitialParts {
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
