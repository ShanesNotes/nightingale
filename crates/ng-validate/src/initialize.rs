use crate::reference::solve_dense;
use crate::ring::solve_ring;
use ng_contract::*;
use ng_record::{Running, Session};

const NEWTON_LIMIT: u32 = 32;
const SWEEP_LIMIT: u32 = 32;
/// How many passes of the ring's own stages a guess marches before Newton. P2 reached a settled beat in 40.
const MARCH_BEATS: u32 = 40;
/// Newton stops when no unknown moved by more than this share of itself. Each is read at its own size and no
/// tolerance is read, so a row or a fit too small for the tolerance to see is solved all the same. An unknown that
/// rounding keeps near zero never gets there, which costs the limit's updates and nothing else.
const NEWTON_STOP: f64 = 8.0 * f64::EPSILON;

/// A model default reported with every result, for rows near unit size. A packet of far smaller amounts states its
/// own, because one scale cannot read both until rows have their own.
pub fn admission_tolerance() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1e-9), scale(1e-12)).expect("positive constants")
}

/// Walks the ring's own stages from the packet's guess: each point's Newton on its own block, each firing into the next
/// point, a pass per beat. A pulsatile orbit is too far from a constant guess for Newton on the ring, and a walk needs
/// nothing but the stages, so it reaches a guess with the modes in their places. `admit` still judges all of it.
fn march(system: &mut InitializationSystem<'_>, body: &CompiledBody, workspace: &mut SolveWorkspace) {
    let size = system.rows();
    for _ in 0..MARCH_BEATS {
        for point in 0..system.points() {
            let before = (point + system.points() - 1) % system.points();
            let (from, to) = (before * size, point * size);
            workspace.unknowns_mut().copy_within(from..from + size, to);
            for _ in 0..NEWTON_LIMIT {
                let Ok((residuals, block)) = system.stage(workspace, point) else {
                    return;
                };
                let Ok(update) = solve_block(body, &block, &residuals) else {
                    return;
                };
                let unknowns = &mut workspace.unknowns_mut()[to..to + size];
                let settled = (update.iter().zip(unknowns.iter()))
                    .all(|(change, unknown)| change.abs() <= NEWTON_STOP * unknown.abs());
                for (unknown, change) in unknowns.iter_mut().zip(update) {
                    *unknown -= change;
                }
                if settled {
                    break;
                }
            }
            if system.sweep(workspace, point..point + 1).is_err() {
                return;
            }
        }
    }
}

fn solve_block(body: &CompiledBody, block: &Jacobian, residuals: &[f64]) -> Result<Vec<f64>, usize> {
    match body.factor(block) {
        Ok(factors) => {
            let mut update = residuals.to_vec();
            factors.solve(&mut update);
            Ok(update)
        }
        Err(_) => solve_dense(block.view(), residuals),
    }
}

/// The one admission solve, a function of the packet and the window alone, so every core starts from one start
/// state. A march to a guess, then Newton over the block ring at fixed modes, then a sweep of the guards, until a
/// sweep moves nothing.
///
/// It judges nothing. It stops where it stands on a singular update or a trial outside a law's domain, and `admit`
/// is the one judge of a start.
fn initialize(system: &mut InitializationSystem<'_>, body: &CompiledBody) -> SolveWorkspace {
    let mut workspace = system.workspace();
    if system.points() > 1 {
        march(system, body, &mut workspace);
    }
    for _ in 0..SWEEP_LIMIT {
        for _ in 0..NEWTON_LIMIT {
            let (Ok(residuals), Ok(jacobian)) = (system.evaluate(&workspace), system.linearize(&workspace))
            else {
                return workspace;
            };
            let update = if system.points() > 1 {
                solve_ring(system, body, &jacobian, &residuals)
            } else {
                solve_dense(jacobian.view(), &residuals)
            };
            let Ok(update) = update else {
                return workspace;
            };
            let unknowns = workspace.unknowns_mut();
            let settled = (update.iter().zip(unknowns.iter()))
                .all(|(change, unknown)| change.abs() <= NEWTON_STOP * unknown.abs());
            for (unknown, change) in unknowns.iter_mut().zip(update) {
                *unknown -= change;
            }
            if settled {
                break;
            }
        }
        let every = 0..system.points();
        if !matches!(system.sweep(&workspace, every), Ok(1..)) {
            break;
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
    admit_at(body, packet, boundary, core, lens, SimTime::from_ns(0))
}

/// `admit_solved` at a moment of the day, as the slice admits at 06:55.
pub fn admit_at<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
    lens: &ObservationLens,
    origin: SimTime,
) -> Result<Admitted<'m>, InitializationFailure> {
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::at(origin).reading(lens);
    let mut system = body.initialization(parameters, boundary, request)?;
    let solved = initialize(&mut system, body);
    let resolution = core.resolution();
    let start = MethodHistory::new(resolution.policy(), origin, Vec::new());
    let revision = system.admit(solved, start)?;
    let provenance = revision.provenance().clone();
    let seed = Seed::new([1, 2, 3, 4]);
    let identity = RunIdentity::new(body.model_hash(), revision.packet_hash(), seed, resolution);
    let session = Session::admit(body, core, revision, identity)?;
    Ok(Admitted { session, provenance })
}
