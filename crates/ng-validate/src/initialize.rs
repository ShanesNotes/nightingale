use crate::reference::solve_dense;
use ng_contract::*;
use ng_record::{HistoryStore, Running, Session};

const NEWTON_LIMIT: u32 = 32;
const SWEEP_LIMIT: u32 = 32;
const FIT_REACH: f64 = 0.25;
const NOISE: f64 = 1e-11;
const FLOOR: f64 = 0.1;
const HALVINGS: u32 = 12;
const MARCH_BEATS: u32 = 40;
/// Newton stops when no unknown moved by more than this share of itself, or in `initialize` of its slot's
/// natural size plus itself. Neither reads a tolerance. In `march` a row too small for the tolerance to see
/// is solved all the same, and an unknown that rounding keeps near zero never gets there, which costs the
/// updates until the point's state repeats, at most the limit's, and nothing else.
const NEWTON_STOP: f64 = 8.0 * f64::EPSILON;

/// A model default reported with every result. A packet of far smaller amounts states its rows' natural sizes.
pub fn admission_tolerance() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1e-9), scale(1e-12)).expect("positive constants")
}

/// Walks the ring's own stages from the packet's guess: each point's Newton on its own block, each firing into the next
/// point, a pass per beat. A pulsatile orbit is too far from a constant guess for Newton on the ring, and a walk needs
/// nothing but the stages, so it reaches a guess with the modes in their places. `admit` still judges all of it.
fn march(system: &mut InitializationSystem<'_>, body: &CompiledBody, workspace: &mut SolveWorkspace) {
    let (size, points) = (system.rows(), system.points());
    let mut held = Vec::with_capacity(NEWTON_LIMIT as usize * size);
    for _ in 0..MARCH_BEATS {
        for point in 0..points {
            let before = (point + points - 1) % points;
            let (from, to) = (before * size, point * size);
            workspace.unknowns_mut().copy_within(from..from + size, to);
            held.clear();
            for _ in 0..NEWTON_LIMIT {
                held.extend_from_slice(&workspace.unknowns_mut()[to..to + size]);
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
                if let Some(state) = state_at_limit(&held, unknowns) {
                    unknowns.copy_from_slice(state);
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

fn state_at_limit<'h>(held: &'h [f64], now: &[f64]) -> Option<&'h [f64]> {
    let size = now.len();
    let same = |state: &[f64]| {
        state
            .iter()
            .zip(now)
            .all(|(was, is)| was.to_bits() == is.to_bits())
    };
    let first = held.chunks(size).position(same)?;
    let period = held.len() / size - first;
    let last = first + (NEWTON_LIMIT as usize - first) % period;
    Some(&held[last * size..][..size])
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

fn merit(system: &InitializationSystem<'_>, unknowns: &[f64], residuals: &[f64]) -> f64 {
    let read = |(slot, (residual, unknown)): (usize, (&f64, &f64))| {
        let share = residual / (system.size(slot) + unknown.abs());
        share * share
    };
    (residuals.iter().zip(unknowns).enumerate()).map(read).sum()
}

/// Takes as much of a Newton update as the laws allow. A fit moves by at most a quarter of its size and an amount keeps a
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
        .map(|(change, fit)| FIT_REACH * (1.0 + fit.abs()) / change.abs())
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
            let now = merit(system, &start, &trial);
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

/// The one admission solve. No core reaches it, so every core starts from one start state.
///
/// It judges nothing. It stops where it stands at a point outside a law's domain, on a singular update or when no
/// trial of an update has a value, and `admit` is the one judge of a start.
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
            // Rounding keeps a store moving past where any row can improve, so an update that has stopped
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
    store: &'m dyn HistoryStore,
) -> Result<Admitted<'m>, InitializationFailure> {
    admit_at(body, packet, boundary, core, lens, SimTime::from_ns(0), store)
}

pub fn admit_at<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
    lens: &ObservationLens,
    origin: SimTime,
    store: &'m dyn HistoryStore,
) -> Result<Admitted<'m>, InitializationFailure> {
    let start = solved_start(body, packet, boundary, core, lens, origin)?;
    let provenance = start.provenance().clone();
    let session = admit_start(body, core, start, Seed::new([1, 2, 3, 4]), store)?;
    Ok(Admitted { session, provenance })
}

fn system<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    lens: &ObservationLens,
    origin: SimTime,
) -> Result<InitializationSystem<'m>, InitializationFailure> {
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::at(origin).reading(lens);
    Ok(body.initialization(parameters, boundary, request)?)
}

fn started(
    system: InitializationSystem<'_>,
    solved: SolveWorkspace,
    core: &dyn CoupledSolver,
    origin: SimTime,
) -> Result<InitialRevision, InitializationFailure> {
    let start = MethodHistory::new(core.resolution().policy(), origin, Vec::new());
    system.admit(solved, start)
}

pub(crate) fn solved_start(
    body: &CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &dyn CoupledSolver,
    lens: &ObservationLens,
    origin: SimTime,
) -> Result<InitialRevision, InitializationFailure> {
    let mut system = system(body, packet, boundary, lens, origin)?;
    let solved = initialize(&mut system, body);
    started(system, solved, core, origin)
}

pub(crate) fn solution(
    body: &CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    lens: &ObservationLens,
    origin: SimTime,
) -> Result<Vec<f64>, InitializationFailure> {
    let mut system = system(body, packet, boundary, lens, origin)?;
    Ok(initialize(&mut system, body).unknowns_mut().to_vec())
}

/// A fresh system holds the packet's modes at every point, so sweeps carry the solution's own modes round the ring
/// first.
pub(crate) fn start_from(
    body: &CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &dyn CoupledSolver,
    lens: &ObservationLens,
    origin: SimTime,
    solution: &[f64],
) -> Result<InitialRevision, InitializationFailure> {
    let mut system = system(body, packet, boundary, lens, origin)?;
    let mut solved = system.workspace();
    solved.unknowns_mut().copy_from_slice(solution);
    for _ in 0..SWEEP_LIMIT {
        if !matches!(system.sweep(&solved, 0..system.points()), Ok(1..)) {
            break;
        }
    }
    started(system, solved, core, origin)
}

pub(crate) fn admit_start<'m>(
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    start: InitialRevision,
    seed: Seed,
    store: &'m dyn HistoryStore,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let identity = RunIdentity::new(body.model_hash(), start.packet_hash(), seed, core.resolution());
    Ok(Session::admit(body, core, start, identity, store)?)
}
