//! The reference integrator: fully coupled backward Euler with Newton on the contract's public seam and
//! nothing else, step doubled so its own error is read and its floor is real.
use ng_contract::*;

const NEWTON_LIMIT: u32 = 8;
const RETRY_BUDGET: u32 = 12;
const MILLISECOND: u64 = 1_000_000;
const NANOSECONDS_PER_SECOND: f64 = 1e9;
/// Eight units in the last place. Newton stops at rounding, not at the declared scales: those say what
/// may be accepted, and an accuracy reference must not stop early merely because it may.
const ROUNDOFF: f64 = 1.0 / 562_949_953_421_312.0;

/// A model default reported with every result. Measured on the two-reservoir case: the estimate reaches
/// 4.0 on 64 s steps of amounts near 5e5, so this passes those steps and refuses much longer ones.
fn default_scales() -> ErrorScales {
    let scale = |value| Measure::new(value).expect("a finite constant");
    ErrorScales::uniform(scale(1.0), scale(1e-5)).expect("positive constants")
}

/// One fixed interval, cut short only at breakpoints and by refusals.
pub struct ReferenceIntegrator {
    interval_ns: u64,
    minimum_interval_ns: u64,
    scales: ErrorScales,
}
impl ReferenceIntegrator {
    pub fn new(interval_ns: u64) -> Self {
        Self {
            interval_ns,
            minimum_interval_ns: MILLISECOND,
            scales: default_scales(),
        }
    }
    /// Tighter or looser scales, which make the estimate refuse sooner or later.
    pub fn with_scales(self, scales: ErrorScales) -> Self {
        Self { scales, ..self }
    }
    pub fn with_floor(self, minimum_interval_ns: u64) -> Self {
        Self {
            minimum_interval_ns,
            ..self
        }
    }
}
impl CoupledSolver for ReferenceIntegrator {
    fn resolution(&self) -> DeclaredResolution {
        let mut policy = b"reference integrator 1".to_vec();
        policy.extend_from_slice(&self.interval_ns.to_be_bytes());
        DeclaredResolution::new(
            Digest::of_canonical(&policy),
            self.minimum_interval_ns,
            RETRY_BUDGET,
            ArithmeticProfile::contract_v1(),
            StageScheme::backward_euler_doubled(),
            self.scales,
        )
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns();
        let mut quadrature = problem.quadrature(ticket);
        let mut workspace = problem.workspace();
        if length < 2 {
            let stage = newton(problem, &mut workspace, ticket, length, length)?;
            quadrature.add(&stage, Ratio::ONE)?;
        } else {
            let half = length / 2;
            let whole = newton(problem, &mut problem.workspace(), ticket, length, length)?;
            quadrature.add(&whole, Ratio::ZERO)?;
            let first = newton(problem, &mut workspace, ticket, half, half)?;
            quadrature.add(&first, ticket.fraction(half))?;
            let second = newton(problem, &mut workspace, ticket, length, length - half)?;
            quadrature.add(&second, ticket.fraction(length - half))?;
        }
        let schema = problem.accepted().method_history().schema();
        let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Solves `F(t, y, (y - y0) / span) = 0` at `offset_ns`, from the workspace's point `y0`.
fn newton(
    problem: &CoupledProblem<'_>,
    workspace: &mut SolveWorkspace,
    ticket: &StepTicket,
    offset_ns: u64,
    span_ns: u64,
) -> Result<StageEvaluation, TrialFailure> {
    let time = ticket.at(offset_ns)?;
    let shift = ticket.shift(span_ns)?;
    let per_second = NANOSECONDS_PER_SECOND / span_ns as f64;
    let weight = if per_second > 1.0 { 1.0 / per_second } else { 1.0 };
    let start = workspace.unknowns_mut().to_vec();
    for _ in 0..NEWTON_LIMIT {
        let unknowns = workspace.unknowns_mut().to_vec();
        for ((tangent, now), then) in workspace.derivative_mut().iter_mut().zip(&unknowns).zip(&start) {
            *tangent = (now - then) * per_second;
        }
        let point = workspace.point(time);
        let stage = problem.evaluate(&point)?;
        let converged = (stage.residuals().iter().zip(&unknowns))
            .all(|(residual, unknown)| weight * residual.abs() <= ROUNDOFF * (1.0 + unknown.abs()));
        if converged {
            return Ok(stage);
        }
        let jacobian = problem.linearize(&point, shift)?;
        let Some(update) = solve_dense(jacobian.view(), stage.residuals()) else {
            break;
        };
        for (unknown, change) in workspace.unknowns_mut().iter_mut().zip(update) {
            *unknown -= change;
        }
    }
    Err(ticket.refuse(NumericalRefusal::IterationLimit))
}

/// Gaussian elimination with partial pivoting in fixed order: the first largest pivot wins, so a tie is
/// settled the same way on every run. `None` on a singular matrix.
fn solve_dense(jacobian: SparseView<'_>, residuals: &[f64]) -> Option<Vec<f64>> {
    let size = residuals.len();
    let mut matrix = vec![vec![0.0; size + 1]; size];
    for (row, cells) in matrix.iter_mut().enumerate() {
        for entry in jacobian.row_offsets()[row]..jacobian.row_offsets()[row + 1] {
            cells[jacobian.columns()[entry].index()] = jacobian.values()[entry];
        }
        cells[size] = residuals[row];
    }
    for pivot in 0..size {
        let mut best = pivot;
        for row in pivot + 1..size {
            if matrix[row][pivot].abs() > matrix[best][pivot].abs() {
                best = row;
            }
        }
        if matrix[best][pivot] == 0.0 {
            return None;
        }
        matrix.swap(pivot, best);
        let (above, below) = matrix.split_at_mut(pivot + 1);
        let leading = &above[pivot];
        for row in below {
            let factor = row[pivot] / leading[pivot];
            for (cell, lead) in row.iter_mut().zip(leading).skip(pivot) {
                *cell -= factor * lead;
            }
        }
    }
    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let known: f64 = (row + 1..size)
            .map(|column| matrix[row][column] * solution[column])
            .sum();
        solution[row] = (matrix[row][size] - known) / matrix[row][row];
    }
    Some(solution)
}
