use crate::reference::solve_dense;
use crate::ring::solve_ring;
use ng_contract::*;
use ng_record::{Running, Session};

const NEWTON_LIMIT: u32 = 32;
const SWEEP_LIMIT: u32 = 32;
const FIT_REACH: f64 = 0.25;
const STALLED: u32 = 3;
const FLOOR: f64 = 0.1;
const HALVINGS: u32 = 20;
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
    for beat in 0..MARCH_BEATS {
        #[allow(clippy::print_stderr)]
        {
            let rows = body.rows();
            let find = |name: &str| rows.iter().position(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == name));
            let pressure = rows.iter().position(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == "blood pressure" || slot.law.local == "pressure"));
            let unknowns = workspace.unknowns_mut();
            let points = system.points();
            let mean = |row: Option<usize>| row.map_or(f64::NAN, |row| (0..points).map(|p| unknowns[p * size + row]).sum::<f64>() / points as f64);
            eprintln!("DEBUG beat {beat} rate {:.4} pressure-row {:?}={:.3} power {:.3}", mean(find("rate")), pressure, mean(pressure), mean(find("dissipated power")));
        }
        for point in 0..system.points() {
            let before = (point + system.points() - 1) % system.points();
            let (from, to) = (before * size, point * size);
            workspace.unknowns_mut().copy_within(from..from + size, to);
            for _ in 0..NEWTON_LIMIT {
                let (residuals, block) = match system.stage(workspace, point) {
                    Ok(pair) => pair,
                    Err(failure) => {
                        #[allow(clippy::print_stderr)]
                        { eprintln!("DEBUG march stage failed beat {beat} point {point}: {failure:?}"); }
                        return;
                    }
                };
                let Ok(update) = solve_block(body, &block, &residuals) else {
                    #[allow(clippy::print_stderr)]
                    { eprintln!("DEBUG march singular beat {beat} point {point}"); }
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

/// Takes as much of a Newton update as the laws allow. A fit moves by at most a quarter of itself, and a trial outside a
/// law's domain halves, so a guess far from the orbit cannot throw a body past a place its laws have a value.
fn step(system: &InitializationSystem<'_>, body: &CompiledBody, workspace: &mut SolveWorkspace, update: &[f64]) -> bool {
    let size = system.rows();
    let fits = system.points() * size;
    let amounts: Vec<bool> = (body.rows().iter())
        .map(|row| matches!(row.kind(), RowKind::Amount { .. }))
        .collect();
    let unknowns = workspace.unknowns_mut();
    let mut share = (update[fits..].iter().zip(&unknowns[fits..]))
        .filter(|(change, _)| change.abs() > 0.0)
        .fold(1.0_f64, |share, (change, fit)| share.min(FIT_REACH * fit.abs() / change.abs()));
    let start = unknowns.to_vec();
    for _ in 0..HALVINGS {
        let unknowns = workspace.unknowns_mut();
        for (at, ((unknown, was), change)) in unknowns.iter_mut().zip(&start).zip(update).enumerate() {
            let next = was - share * change;
            // An amount keeps a tenth of what it held, so one long update cannot empty a pool a law divides by.
            *unknown = if at < fits && amounts[at % size] && next < FLOOR * was { FLOOR * was } else { next };
        }
        match system.evaluate(workspace) {
            Ok(_) => return true,
            Err(failure) => {
                #[allow(clippy::print_stderr)]
                { eprintln!("DEBUG halve {share:.3e}: {failure:?}"); }
            }
        }
        share /= 2.0;
    }
    workspace.unknowns_mut().copy_from_slice(&start);
    false
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
    let mut dumped = 0_u32;
    for sweep in 0..SWEEP_LIMIT {
        let (mut least, mut stalled) = (f64::INFINITY, 0);
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
                #[allow(clippy::print_stderr)]
                {
                    eprintln!("DEBUG singular at sweep {sweep}");
                }
                return workspace;
            };
            #[allow(clippy::print_stderr)]
            {
                let worst = residuals.iter().fold(0.0_f64, |w, r| w.max(r.abs()));
                let last = update.len() - 1;
                eprintln!(
                    "DEBUG sweep {sweep} worst {worst:.3e} moved {:.3e} fit {:?}",
                    update.iter().fold(0.0_f64, |w, r| w.max(r.abs())),
                    &workspace.unknowns_mut()[last]
                );
                if std::env::var("DUMP").is_ok_and(|v| v.parse::<u32>() == Ok(dumped)) {
                    let held = workspace.unknowns_mut().to_vec();
                    for (at, row) in body.rows().iter().enumerate() {
                        let mean = (0..system.points()).map(|p| held[p * body.rows().len() + at]).sum::<f64>() / system.points() as f64;
                        eprintln!("ROW {:?} = {:.5} (p0 {:.5})", row.kind(), mean, held[at]);
                    }
                }
                dumped += 1;
                let mut order: Vec<usize> = (0..update.len()).collect();
                let size = body.rows().len();
                let share = |slot: usize, w: &[f64]| update[slot].abs() / (1.0 + w[slot].abs());
                let held = workspace.unknowns_mut().to_vec();
                order.sort_by(|a, b| share(*b, &held).total_cmp(&share(*a, &held)));
                let mut seen = Vec::new();
                for &slot in order.iter().take(3000) {
                    if slot >= size * system.points() {
                        eprintln!("DEBUG    fit update {:.3e} value {:.3e}", update[slot], held[slot]);
                        continue;
                    }
                    let row = slot % size;
                    if !seen.contains(&row) && seen.len() < 5 {
                        seen.push(row);
                        eprintln!("DEBUG    {:?}@{} update {:.3e} value {:.3e}", body.rows()[row].kind(), slot / size, update[slot], held[slot]);
                    }
                }
            }
            let settled = (update.iter().zip(workspace.unknowns_mut().iter()))
                .all(|(change, unknown)| change.abs() <= NEWTON_STOP * unknown.abs());
            if !step(system, body, &mut workspace, &update) {
                return workspace;
            }
            if settled {
                break;
            }
            // Rounding keeps a large row moving past where any row can improve.
            let worst = residuals.iter().fold(0.0_f64, |most, residual| most.max(residual.abs()));
            stalled = if worst < 0.5 * least { 0 } else { stalled + 1 };
            least = least.min(worst);
            if stalled >= STALLED {
                break;
            }
        }
        let every = 0..system.points();
        let moved = system.sweep(&workspace, every);
        #[allow(clippy::print_stderr)]
        { eprintln!("DEBUG sweep {sweep} changed {moved:?}"); }
        system.debug_modes();
        if !matches!(moved, Ok(1..)) {
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
