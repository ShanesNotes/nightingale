use crate::reference::solve_dense;
use ng_contract::*;
use ng_record::{Running, Session};

const NEWTON_LIMIT: u32 = 32;
/// A row within this share of the largest unknown is rounding. The stop reads no tolerance, so what a tolerance
/// passes shapes no bit of a start state.
const ROWS_AT_ROUNDING: f64 = 64.0 * f64::EPSILON;

/// A model default reported with every result. Newton leaves the pulsatile case's rows near 1e-14 on amounts near
/// 100, about 1e-5 of this, so a guess passes only if it was solved.
pub fn admission_tolerance() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1e-9), scale(1e-12)).expect("positive constants")
}

fn largest(values: &[f64]) -> f64 {
    values.iter().fold(
        0.0,
        |most, value| if value.abs() > most { value.abs() } else { most },
    )
}

/// The one admission solve, a function of the packet and the window alone, so every core starts from one start
/// state. Newton over `solve_dense`, whose cost is the cube of `points * rows`, so a few thousand unknowns is its
/// reach. A body past that reduces the block ring `InitializationSystem` lays out, behind this one function.
///
/// It judges nothing. A singular update stops it where it stands, and `admit` is the one judge of a start.
fn initialize(system: &InitializationSystem<'_>) -> Result<SolveWorkspace, InitializationFailure> {
    let mut workspace = system.workspace();
    for _ in 0..NEWTON_LIMIT {
        let residuals = system.evaluate(&workspace)?;
        if largest(&residuals) <= ROWS_AT_ROUNDING * (1.0 + largest(workspace.unknowns_mut())) {
            break;
        }
        let jacobian = system.linearize(&workspace)?;
        let Ok(update) = solve_dense(jacobian.view(), &residuals) else {
            break;
        };
        for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
            *unknown -= change;
        }
    }
    Ok(workspace)
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
    let request = InitializationRequest::at(origin, admission_tolerance()).reading(lens);
    let system = body.initialization(parameters, boundary, request)?;
    let solved = initialize(&system)?;
    let resolution = core.resolution();
    let start = MethodHistory::new(resolution.policy(), origin, Vec::new());
    let revision = system.admit(solved, start)?;
    let provenance = revision.provenance().clone();
    let seed = Seed::new([1, 2, 3, 4]);
    let identity = RunIdentity::new(body.model_hash(), revision.packet_hash(), seed, resolution);
    let session = Session::admit(body, core, revision, identity)?;
    Ok(Admitted { session, provenance })
}
