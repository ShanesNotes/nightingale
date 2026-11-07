use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::Step;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const NANOSECONDS_PER_SECOND: f64 = 1e9;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn number(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn loose() -> ErrorScales {
    ErrorScales::uniform(measure(1e9), measure(1.0)).expect("positive")
}

fn fixed(interval_ns: u64, floor_ns: u64) -> ReferenceIntegrator {
    let floor = NonZeroU64::new(floor_ns).expect("a floor");
    ReferenceIntegrator::new(interval_ns)
        .with_scales(loose())
        .with_floor(floor)
}

/// One resolution for every core here, so the slice admits one start for them all.
fn declared(interval_ns: u64, scales: ErrorScales) -> DeclaredResolution {
    let mut policy = b"doubled backward Euler".to_vec();
    policy.extend_from_slice(&interval_ns.to_be_bytes());
    DeclaredResolution::new(
        Digest::of_canonical(&policy),
        NonZeroU64::new(100_000).expect("a floor"),
        12,
        ArithmeticProfile::contract_v1(),
        StageScheme::backward_euler_doubled(),
        scales,
    )
}

fn doubled(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
    stage: impl Fn(&mut SolveWorkspace, StageTime<'_>, u64) -> Result<StageEvaluation, TrialFailure>,
) -> Result<StepProposal, TrialFailure> {
    let length = ticket.length_ns();
    let half = length / 2;
    let mut quadrature = problem.quadrature(ticket);
    let whole = stage(&mut problem.workspace(), ticket.at(length)?, length)?;
    quadrature.add(&whole, Ratio::ZERO)?;
    let mut workspace = problem.workspace();
    let first = stage(&mut workspace, ticket.at(half)?, half)?;
    quadrature.add(&first, ticket.fraction(half))?;
    let second = stage(&mut workspace, ticket.at(length)?, length - half)?;
    quadrature.add(&second, ticket.fraction(length - half))?;
    let schema = problem.accepted().method_history().schema();
    let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
    Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
}

fn backward_tangent(workspace: &mut SolveWorkspace, start: &[f64], span_ns: u64) {
    let per_second = NANOSECONDS_PER_SECOND / span_ns as f64;
    let unknowns = workspace.unknowns_mut().to_vec();
    for ((tangent, now), then) in workspace.derivative_mut().iter_mut().zip(&unknowns).zip(start) {
        *tangent = (now - then) * per_second;
    }
}

/// Doubled backward Euler that stops each stage after `updates` Newton updates, then books it with the rows in
/// `behind` back at the coordinates the stage started from.
struct Sweeps {
    interval_ns: u64,
    scales: ErrorScales,
    updates: usize,
    behind: Vec<usize>,
}
impl CoupledSolver for Sweeps {
    fn resolution(&self) -> DeclaredResolution {
        declared(self.interval_ns, self.scales)
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        doubled(problem, ticket, |workspace, time, span_ns| {
            let shift = ticket.shift(span_ns)?;
            let start = workspace.unknowns_mut().to_vec();
            for _ in 0..self.updates {
                backward_tangent(workspace, &start, span_ns);
                let point = workspace.point(time);
                let stage = problem.evaluate(&point)?;
                let jacobian = problem.linearize(&point, shift)?;
                let factors = (problem.body().factor(&jacobian))
                    .map_err(|_| ticket.refuse(NumericalRefusal::IterationLimit))?;
                let mut update = stage.residuals().to_vec();
                factors.solve(&mut update);
                for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
                    *unknown -= change;
                }
            }
            for &slot in &self.behind {
                let unknown = problem.body().rows()[slot].unknown().index();
                workspace.unknowns_mut()[unknown] = start[unknown];
            }
            backward_tangent(workspace, &start, span_ns);
            problem.evaluate(&workspace.point(time))
        })
    }
}

/// Solves each stage with the wall power held where the stage started: the walls' volumes pair, and the store follows
/// a stale power, so its energy no longer closes on the walls' sum.
struct HoldsTheWallPower {
    interval_ns: u64,
    scales: ErrorScales,
    wall: usize,
}
impl CoupledSolver for HoldsTheWallPower {
    fn resolution(&self) -> DeclaredResolution {
        declared(self.interval_ns, self.scales)
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        doubled(problem, ticket, |workspace, time, span_ns| {
            let shift = ticket.shift(span_ns)?;
            let start = workspace.unknowns_mut().to_vec();
            let held = problem.body().rows()[self.wall].unknown().index();
            for _ in 0..8 {
                backward_tangent(workspace, &start, span_ns);
                let point = workspace.point(time);
                let stage = problem.evaluate(&point)?;
                let jacobian = problem.linearize(&point, shift)?;
                let view = jacobian.view();
                let unknowns = workspace.unknowns_mut().to_vec();
                let size = unknowns.len();
                let mut matrix = vec![vec![0.0; size + 1]; size];
                for (slot, cells) in matrix.iter_mut().enumerate() {
                    if slot == self.wall {
                        cells[held] = 1.0;
                        cells[size] = unknowns[held] - start[held];
                        continue;
                    }
                    for entry in view.row_offsets()[slot]..view.row_offsets()[slot + 1] {
                        cells[view.columns()[entry].index()] += view.values()[entry];
                    }
                    cells[size] = stage.residuals()[slot];
                }
                let update =
                    eliminate(matrix).map_err(|_| ticket.refuse(NumericalRefusal::IterationLimit))?;
                for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
                    *unknown -= change;
                }
            }
            backward_tangent(workspace, &start, span_ns);
            problem.evaluate(&workspace.point(time))
        })
    }
}

fn rows_outside_the_largest_block(body: &CompiledBody) -> Vec<usize> {
    let blocks = body.blocks();
    let largest = (0..blocks.len())
        .max_by_key(|&at| blocks[at].rows().len())
        .expect("a block");
    let slot = |id: &RowId| (body.rows().iter().position(|row| row.id() == *id)).expect("a row of the body");
    (blocks.iter().enumerate())
        .filter(|&(at, _)| at != largest)
        .flat_map(|(_, block)| block.rows().iter().map(slot))
        .collect()
}

fn wall_power(body: &CompiledBody) -> usize {
    let law = LawName {
        owner: "CV",
        local: "wall power",
    };
    let wall = RowKind::Algebraic(Slot {
        law,
        at: Context::Body,
    });
    (body.rows().iter().position(|row| row.kind() == wall)).expect("a wall power row")
}

#[test]
fn the_first_window_holds_each_core_to_the_references_own_distances() {
    let window = SliceWindow::first().expect("the reference runs the window at each refinement");
    let cases = vec![AccuracyCase::SliceWindow(Box::new(window))];
    let policy = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy");

    let arena_scales = ErrorScales::uniform(measure(1.0), measure(1e-3)).expect("positive");
    let adapting = ReferenceIntegrator::new(5 * MILLISECOND)
        .with_scales(arena_scales)
        .with_floor(NonZeroU64::new(100_000).expect("a floor"));
    for honest in [fixed(5 * MILLISECOND, 100_000), adapting] {
        let evidence = accuracy(&honest, &policy).expect("within every distance");
        let share = number(evidence.worst()[0]);
        assert!(share > 0.0 && share < 1.0, "{share}");
    }

    let to_a_millisecond = accuracy(&fixed(5 * MILLISECOND, MILLISECOND), &policy);
    assert!(
        matches!(to_a_millisecond, Err(AccuracyFailure::Outside { case: 0, .. })),
        "{to_a_millisecond:?}"
    );

    let manufactured = AccuracyPolicy::manufactured().expect("the model default");
    let sixteen = Sweeps {
        interval_ns: 16 * SECOND,
        scales: loose(),
        updates: 1,
        behind: Vec::new(),
    };
    accuracy(&sixteen, &manufactured).expect("one update solves a linear law");
    let after_one_update = Sweeps {
        interval_ns: 10 * MILLISECOND,
        scales: loose(),
        updates: 1,
        behind: Vec::new(),
    };
    let failure = accuracy(&after_one_update, &policy);
    assert!(
        matches!(failure, Err(AccuracyFailure::Outside { case: 0, .. })),
        "{failure:?}"
    );

    let body = arena_body().expect("compiled");
    let stale = Sweeps {
        interval_ns: 10 * MILLISECOND,
        scales: loose(),
        updates: 8,
        behind: rows_outside_the_largest_block(&body),
    };
    let failure = accuracy(&stale, &policy);
    assert!(
        matches!(failure, Err(AccuracyFailure::ModesApart { case: 0, .. })),
        "{failure:?}"
    );

    let to_its_step = accuracy(&fixed(10 * MILLISECOND, 10 * MILLISECOND), &policy);
    assert!(
        matches!(to_its_step, Err(AccuracyFailure::ModesApart { case: 0, .. })),
        "{to_its_step:?}"
    );
}

#[test]
fn validate_refuses_a_slice_stage_its_core_left_unsolved() {
    let case = SliceCase::new().expect("compiled");
    let body = case.body();
    let scales = ErrorScales::uniform(measure(1.0), measure(1e-2)).expect("positive");
    let sweeps = |updates, behind| Sweeps {
        interval_ns: 5 * MILLISECOND,
        scales,
        updates,
        behind,
    };
    let store = MemoryStore::default();
    let first = |core: &dyn CoupledSolver| -> Option<NumericalRefusal> {
        match case.admit(core, &store).expect("admitted").step() {
            Step::Accepted(..) => None,
            Step::Refused(refused) => Some(refused.refused().reason()),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        }
    };
    assert_eq!(
        first(&sweeps(8, Vec::new())),
        None,
        "the honest core's step is accepted"
    );
    let after_one_update = first(&sweeps(1, Vec::new()));
    assert!(
        matches!(after_one_update, Some(NumericalRefusal::StageResidual { .. })),
        "{after_one_update:?}"
    );
    let stale = first(&sweeps(8, rows_outside_the_largest_block(body)));
    assert!(
        matches!(stale, Some(NumericalRefusal::StageResidual { .. })),
        "{stale:?}"
    );
    let wall = wall_power(body);
    let unclosed = HoldsTheWallPower {
        interval_ns: 5 * MILLISECOND,
        scales,
        wall,
    };
    assert_eq!(
        first(&unclosed),
        Some(NumericalRefusal::StageResidual {
            row: body.rows()[wall].id()
        })
    );
}
