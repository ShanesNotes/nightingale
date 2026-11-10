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
const CONTRACTION: f64 = 0.5;

/// A model default reported with every result. A packet of far smaller amounts states its rows' natural sizes.
pub fn admission_tolerance() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1e-9), scale(1e-12)).expect("positive constants")
}

/// Newton updates in a row that hold a crossing at its interval's edge before a sweep moves it across: one, since the
/// next linearization would read a root its leg can no longer reach.
const CLAMPED: u32 = 1;
/// Passes a march takes over every leg after a sweep moves the topology, solving each leg's stages on the fits Newton
/// reached and moving no crossing: a leg split or merged off the old curves is no stage solution, and a stiff row's
/// Newton would chase it. Two, so the modes the wrap opens follow the legs a pass changed.
const RESEED_BEATS: u32 = 2;
const RELOCATIONS: u32 = 6;

/// Walks the ring's own legs from the packet's guess, in time order from the wrap: each leg's Newton on its own
/// block from the state its start leaves, its crossings relocated off its own curve, a pass per beat. A pulsatile orbit
/// is too far from a constant guess for Newton on the ring, and a walk needs nothing but the stages, so it reaches a
/// guess with the modes and crossings near their places. `admit` still judges all of it.
fn march(
    system: &InitializationSystem<'_>,
    workspace: &mut AdmissionWorkspace,
    beats: u32,
    relocating: bool,
) {
    let points = system.points();
    for _ in 0..beats {
        for step in 1..=points {
            let point = step % points;
            let (mut at, mut moves) = (0, 0);
            while let Some(&leg) = workspace.legs_of(point).get(at) {
                if system.settle(workspace, leg).is_err() {
                    return;
                }
                system.seed(workspace, leg);
                if !solve_leg(system, workspace, leg) {
                    return;
                }
                if !relocating {
                    at += 1;
                    continue;
                }
                match system.relocate(workspace, leg) {
                    // A guess far from the orbit can hold a guard at its zero; a later pass or the sweep settles it.
                    Err(InitializationFailure::Unresolved { .. }) => (at, moves) = (at + 1, 0),
                    Err(_) => return,
                    Ok(Relocated::Settled) => (at, moves) = (at + 1, 0),
                    Ok(_) if moves < RELOCATIONS => moves += 1,
                    Ok(_) => (at, moves) = (at + 1, 0),
                }
            }
        }
    }
}

/// Newton on one leg's block alone, holding the block it factored while its updates shrink by `CONTRACTION` or more:
/// a held block solves for the same rows, and building and factoring one costs many residuals. An update that grew is
/// not taken, and the block is built again where it stands. `false` where a law has no value or the block no pivot.
fn solve_leg(system: &InitializationSystem<'_>, workspace: &mut AdmissionWorkspace, leg: LegHandle) -> bool {
    let slots = workspace.leg_slots(leg);
    let mut held = Vec::with_capacity(NEWTON_LIMIT as usize * slots.len());
    let mut factored: Option<Factors<'_>> = None;
    let mut last = f64::INFINITY;
    let mut taken = 0;
    while taken < NEWTON_LIMIT {
        let reused = factored.is_some();
        let update = match &factored {
            Some(factors) => {
                let Ok(mut update) = system.stage_residuals(workspace, leg) else {
                    return false;
                };
                factors.solve(&mut update);
                update
            }
            None => {
                let Ok((mut residuals, block)) = system.stage(workspace, leg) else {
                    return false;
                };
                match system.factor(&block) {
                    Ok(factors) => {
                        factors.solve(&mut residuals);
                        factored = Some(factors);
                        residuals
                    }
                    Err(_) => match system.solve_stage(&block, &residuals) {
                        Ok(update) => update,
                        Err(_) => return false,
                    },
                }
            }
        };
        let now = workspace.unknowns_mut()[slots.clone()].to_vec();
        let moved = (update.iter().zip(&now).zip(slots.clone()))
            .map(|((change, unknown), slot)| change.abs() / (system.size(workspace, slot) + unknown.abs()))
            .fold(0.0, |most, share| if share > most { share } else { most });
        if reused && moved > NEWTON_STOP && moved > CONTRACTION * last {
            factored = None;
            if moved >= last || moved.is_nan() {
                continue;
            }
        }
        last = moved;
        taken += 1;
        held.extend_from_slice(&now);
        let unknowns = &mut workspace.unknowns_mut()[slots.clone()];
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
    true
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

struct Stepped {
    stood: bool,
    leaving: bool,
}

/// Takes as much of a Newton update as the laws allow. A fit moves by at most a quarter of its size and an amount keeps a
/// tenth of what it held, so a long update cannot throw a body past a place its laws have a value, and a trial that
/// has no value there, moves a crossing out of its interval or stands further from solved halves. The nearest trial
/// stands when none is nearer than the start.
fn step(
    system: &InitializationSystem<'_>,
    body: &CompiledBody,
    workspace: &mut AdmissionWorkspace,
    residuals: &[f64],
    correction: &Correction,
) -> Stepped {
    let size = system.rows();
    let fits = workspace.fitted_slots().start;
    let amounts: Vec<bool> = (body.rows().iter())
        .map(|row| matches!(row.kind(), RowKind::Amount { .. }))
        .collect();
    let start = workspace.clone();
    let update = correction.slots();
    let was_unknowns = start.clone().unknowns_mut().to_vec();
    let mut share = (update[fits..].iter().zip(&was_unknowns[fits..]))
        .filter(|(change, _)| change.abs() > 0.0)
        .map(|(change, fit)| FIT_REACH * (1.0 + fit.abs()) / change.abs())
        .fold(1.0, |least, reach| if reach < least { reach } else { least });
    let was = system.merit(&start, residuals);
    let mut best: Option<(f64, AdmissionWorkspace, bool)> = None;
    for _ in 0..HALVINGS {
        let mut trial = start.clone();
        let clamped = system.apply(&mut trial, correction, share) != Applied::Inside;
        for (at, (unknown, was)) in trial.unknowns_mut().iter_mut().zip(&was_unknowns).enumerate() {
            if at < fits && amounts[at % size] && *unknown < FLOOR * was {
                *unknown = FLOOR * was;
            }
        }
        if let Ok(residuals) = system.evaluate(&trial) {
            let now = system.merit(&start, &residuals);
            if now < was {
                *workspace = trial;
                return Stepped {
                    stood: true,
                    leaving: clamped,
                };
            }
            if best.as_ref().is_none_or(|(known, _, _)| now < *known) {
                best = Some((now, trial, clamped));
            }
        }
        share /= 2.0;
    }
    match best {
        Some((_, trial, clamped)) => {
            *workspace = trial;
            Stepped {
                stood: true,
                leaving: clamped,
            }
        }
        None => Stepped {
            stood: false,
            leaving: false,
        },
    }
}

/// The one admission solve. No core reaches it, so every core starts from one start state.
///
/// It judges nothing. It stops where it stands at a point outside a law's domain, on a singular update, when no
/// trial of an update has a value or when a sweep cannot settle a crossing, and `admit` is the one judge of a start.
fn initialize(system: &InitializationSystem<'_>, body: &CompiledBody) -> AdmissionWorkspace {
    let mut workspace = system.workspace();
    if system.cycles() {
        march(system, &mut workspace, MARCH_BEATS, true);
    }
    for _ in 0..SWEEP_LIMIT {
        let mut last = f64::INFINITY;
        let mut held_back = 0;
        for _ in 0..NEWTON_LIMIT {
            let (Ok(residuals), Ok(jacobian)) = (system.evaluate(&workspace), system.linearize(&workspace))
            else {
                return workspace;
            };
            let Ok(correction) = system.solve(&workspace, &jacobian, &residuals) else {
                return workspace;
            };
            let unknowns = workspace.unknowns_mut().to_vec();
            let moved = (correction.slots().iter().zip(&unknowns).enumerate())
                .map(|(slot, (change, unknown))| {
                    change.abs() / (system.size(&workspace, slot) + unknown.abs())
                })
                .fold(correction.time_share(&workspace), |most, share| {
                    if share > most { share } else { most }
                });
            let stepped = step(system, body, &mut workspace, &residuals, &correction);
            if !stepped.stood {
                return workspace;
            }
            // Rounding keeps a store moving past where any row can improve, so an update that has stopped
            // shrinking this close to rounding has nothing left to give.
            held_back = if stepped.leaving { held_back + 1 } else { 0 };
            if held_back >= CLAMPED || moved <= NEWTON_STOP || (moved >= 0.5 * last && moved <= NOISE) {
                break;
            }
            last = moved;
        }
        match system.sweep(&mut workspace) {
            Ok(true) => march(system, &mut workspace, RESEED_BEATS, false),
            Ok(false) if held_back > 0 => march(system, &mut workspace, RESEED_BEATS, false),
            _ => break,
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
    let session = admit_start(body, core, start, crate::arena_seed(), store)?;
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
    solved: AdmissionWorkspace,
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
    let system = system(body, packet, boundary, lens, origin)?;
    let solved = initialize(&system, body);
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
