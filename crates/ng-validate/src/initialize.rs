use crate::reference::solve_dense;
use ng_contract::*;
use ng_record::{Running, Session};

const NEWTON_LIMIT: u32 = 32;
/// A row is solved at this much rounding on its own terms.
const NEWTON_STOP: f64 = 8.0 * f64::EPSILON;

/// A model default reported with every result, for rows near unit size. A packet of far smaller amounts states its
/// own, because one scale cannot read both until rows have their own.
pub fn admission_tolerance() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1e-9), scale(1e-12)).expect("positive constants")
}

/// Whether every row stands at rounding on its own terms, each a slope times its unknown. Each row is read at its
/// own size and no tolerance is read, so a row or a fit too small for the tolerance to see is solved all the same.
fn at_rounding(residuals: &[f64], jacobian: SparseView<'_>, unknowns: &[f64]) -> bool {
    (residuals.iter().enumerate()).all(|(row, residual)| {
        let entries = jacobian.row_offsets()[row]..jacobian.row_offsets()[row + 1];
        let terms = jacobian.columns()[entries.clone()]
            .iter()
            .zip(&jacobian.values()[entries]);
        let size: f64 = terms
            .map(|(column, slope)| (slope * unknowns[column.index()]).abs())
            .sum();
        residual.abs() <= NEWTON_STOP * size
    })
}

/// The one admission solve, a function of the packet and the window alone, so every core starts from one start
/// state. Newton over `solve_dense`, whose cost is the cube of `points * rows`, so a few thousand unknowns is its
/// reach. A body past that reduces the block ring `InitializationSystem` lays out, behind this one function.
///
/// It judges nothing. It stops where it stands on a singular update or a trial outside a law's domain, and `admit`
/// is the one judge of a start.
fn initialize(system: &InitializationSystem<'_>) -> SolveWorkspace {
    let mut workspace = system.workspace();
    for _ in 0..NEWTON_LIMIT {
        let (Ok(residuals), Ok(jacobian)) = (system.evaluate(&workspace), system.linearize(&workspace))
        else {
            break;
        };
        if at_rounding(&residuals, jacobian.view(), workspace.unknowns_mut()) {
            break;
        }
        let Ok(update) = solve_dense(jacobian.view(), &residuals) else {
            break;
        };
        for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
            *unknown -= change;
        }
    }
    workspace
}

/// A solved admission: the run and where each of its values came from.
pub struct Admitted<'m> {
    pub session: Session<'m, Running>,
    pub provenance: InitializationProvenance,
}

/// Binds a fixture packet, solves its start state once and admits a session. `admit_fixture` returns the guess
/// instead, and that is what a must-fail initializer is. Only a synthetic fixture enters, and `initialize` is
/// private, so no other packet reaches this crate's solve.
pub fn admit_solved<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
    lens: &ObservationLens,
) -> Result<Admitted<'m>, InitializationFailure> {
    let origin = SimTime::from_ns(0);
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::at(origin).reading(lens);
    let system = body.initialization(parameters, boundary, request)?;
    let solved = initialize(&system);
    let resolution = core.resolution();
    let start = MethodHistory::new(resolution.policy(), origin, Vec::new());
    let revision = system.admit(solved, start)?;
    let provenance = revision.provenance().clone();
    let seed = Seed::new([1, 2, 3, 4]);
    let identity = RunIdentity::new(body.model_hash(), revision.packet_hash(), seed, resolution);
    let session = Session::admit(body, core, revision, identity)?;
    Ok(Admitted { session, provenance })
}
