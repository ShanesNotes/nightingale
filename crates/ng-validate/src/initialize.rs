use crate::reference::solve_dense;
use ng_contract::*;
use ng_record::{Running, Session};

const NEWTON_LIMIT: u32 = 32;
const SWEEP_LIMIT: u32 = 32;
const FIT_REACH: f64 = 0.25;
const NOISE: f64 = 1e-11;
const FLOOR: f64 = 0.1;
const HALVINGS: u32 = 12;
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
    let (size, points) = (system.rows(), system.points());
    for _ in 0..MARCH_BEATS {
        for point in 0..points {
            let before = (point + points - 1) % points;
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
        system.restart();
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

/// How far the rows stand from solved, each at its own size.
fn merit(system: &InitializationSystem<'_>, unknowns: &[f64], residuals: &[f64]) -> f64 {
    let read = |(slot, (residual, unknown)): (usize, (&f64, &f64))| {
        let share = residual / (system.size(slot) + unknown.abs());
        share * share
    };
    (residuals.iter().zip(unknowns).enumerate()).map(read).sum()
}

/// Takes as much of a Newton update as the laws allow. A fit moves by at most a quarter of itself and an amount keeps a
/// tenth of what it held, so a long update cannot throw a body past a place its laws have a value, and a trial that
/// has no value there or stands further from solved halves. The nearest trial stands when none is nearer than the start.
fn step(
    system: &InitializationSystem<'_>,
    body: &CompiledBody,
    workspace: &mut SolveWorkspace,
    residuals: &[f64],
    update: &[f64],
) -> bool {
    let size = system.rows();
    let fits = system.points() * size;
    let amounts: Vec<bool> = (body.rows().iter())
        .map(|row| matches!(row.kind(), RowKind::Amount { .. }))
        .collect();
    let start = workspace.unknowns_mut().to_vec();
    let mut share = (update[fits..].iter().zip(&start[fits..]))
        .filter(|(change, _)| change.abs() > 0.0)
        .map(|(change, fit)| FIT_REACH * fit.abs() / change.abs())
        .fold(1.0, |least, reach| if reach < least { reach } else { least });
    let was = merit(system, &start, residuals);
    let mut best: Option<(f64, Vec<f64>)> = None;
    for _ in 0..HALVINGS {
        let unknowns = workspace.unknowns_mut();
        for (at, ((unknown, was), change)) in unknowns.iter_mut().zip(&start).zip(update).enumerate() {
            let next = was - share * change;
            *unknown = if at < fits && amounts[at % size] && next < FLOOR * was {
                FLOOR * was
            } else {
                next
            };
        }
        if let Ok(trial) = system.evaluate(workspace) {
            let unknowns = workspace.unknowns_mut().to_vec();
            let now = merit(system, &unknowns, &trial);
            if now < was {
                return true;
            }
            if best.as_ref().is_none_or(|(known, _)| now < *known) {
                best = Some((now, unknowns));
            }
        }
        share /= 2.0;
    }
    match best {
        Some((_, unknowns)) => {
            workspace.unknowns_mut().copy_from_slice(&unknowns);
            true
        }
        None => {
            workspace.unknowns_mut().copy_from_slice(&start);
            false
        }
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
        let mut last = f64::INFINITY;
        for _ in 0..NEWTON_LIMIT {
            let (Ok(residuals), Ok(jacobian)) = (system.evaluate(&workspace), system.linearize(&workspace))
            else {
                return workspace;
            };
            let update = if system.points() > 1 {
                system.solve(&jacobian, &residuals)
            } else {
                solve_dense(jacobian.view(), &residuals)
            };
            let Ok(update) = update else {
                return workspace;
            };
            let moved = (update.iter().zip(workspace.unknowns_mut().iter()).enumerate())
                .map(|(slot, (change, unknown))| change.abs() / (system.size(slot) + unknown.abs()))
                .fold(0.0, |most, share| if share > most { share } else { most });
            if !step(system, body, &mut workspace, &residuals, &update) {
                return workspace;
            }
            // Rounding keeps a store of 1e7 moving past where any row can improve, so an update that has stopped
            // shrinking this close to rounding has nothing left to give.
            if moved <= NEWTON_STOP || (moved >= 0.5 * last && moved <= NOISE) {
                break;
            }
            last = moved;
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
