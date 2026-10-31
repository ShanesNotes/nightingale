use core::cell::{Cell, RefCell};
use ng_contract::*;
use ng_validate::{ClockSource, ReferenceIntegrator, WallClock};

const MILLISECOND: u64 = 1_000_000;

fn explicit_resolution(tag: &[u8]) -> DeclaredResolution {
    let measure = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let scales = ErrorScales::uniform(measure(1.0), measure(1e-5)).expect("positive");
    DeclaredResolution::new(
        Digest::of_canonical(tag),
        MILLISECOND,
        12,
        ArithmeticProfile::contract_v1(),
        StageScheme::forward_euler(),
        scales,
    )
}

/// One honest forward Euler stage from `workspace`, which may hold a tilted row: the declared scales allow a
/// stage to sit off the start by their sum, and a dimensionless row near 1 is held to about 1.
fn euler(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
    mut workspace: SolveWorkspace,
) -> Result<StepProposal, TrialFailure> {
    let time = ticket.at(0)?;
    let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
        .map(|residual| -residual)
        .collect();
    workspace.derivative_mut().copy_from_slice(&rates);
    let stage = problem.evaluate(&workspace.point(time))?;
    let mut quadrature = problem.quadrature(ticket);
    quadrature.add(&stage, Ratio::ONE)?;
    let schema = problem.accepted().method_history().schema();
    let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
    Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
}

/// A number in [-1/2, 1/2] from a digest.
fn dice_of(digest: Digest) -> f64 {
    f64::from(digest.as_bytes()[0]) / 255.0 - 0.5
}

/// Draws per step where the seam leaves room: a hash of each ticket, its private dice, tilts a row the hazard
/// rate reads, inside the declared scales. `verify` re-adds the rates at the tilted point and passes. It is a
/// pure function of what the seam hands it, so it replays bit for bit and every bit gate passes it. Only
/// another step policy shows the event moved.
pub struct DrawsPerStep {
    pub interval_ns: u64,
    pub tone: usize,
    pub reach: f64,
}
impl CoupledSolver for DrawsPerStep {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"draws per step")
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let mut workspace = problem.workspace();
        workspace.unknowns_mut()[self.tone] += self.reach * dice_of(ticket.digest());
        euler(problem, ticket, workspace)
    }
}

/// The legal twin: its step length comes from private dice, and only a span. An event's instant is a property
/// of the rate, so no gate may reject this.
pub struct JittersItsSpan {
    pub inner: ReferenceIntegrator,
    pub spread_ns: u64,
}
impl CoupledSolver for JittersItsSpan {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let head = accepted.time().nanoseconds().to_be_bytes();
        let digest = Digest::of_canonical(&head);
        let bytes = digest.as_bytes().first_chunk().expect("thirty-two bytes");
        self.inner.next_step(accepted) + u64::from_be_bytes(*bytes) % self.spread_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

/// Draws per attempt from a count in a `Cell`, so its dice move with every solve: a forced extra evaluation
/// shows at the first step.
pub struct DrawsPerEvaluation {
    pub interval_ns: u64,
    pub tone: usize,
    pub reach: f64,
    pub solves: Cell<u64>,
}
impl CoupledSolver for DrawsPerEvaluation {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"draws per evaluation")
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.solves.set(self.solves.get() + 1);
        let digest = Digest::of_canonical(&self.solves.get().to_be_bytes());
        let mut workspace = problem.workspace();
        workspace.unknowns_mut()[self.tone] += self.reach * dice_of(digest);
        euler(problem, ticket, workspace)
    }
}

/// Holds the same row low on every step, so its own two step policies agree and an event simply comes late.
/// Only the reference integrator shows it, which is why the gate compares against that and not each other.
pub struct DelaysEveryEvent {
    pub interval_ns: u64,
    pub tone: usize,
    pub by: f64,
    pub from: SimTime,
}
impl CoupledSolver for DelaysEveryEvent {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"delays every event")
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let mut workspace = problem.workspace();
        if ticket.interval().start() >= self.from {
            workspace.unknowns_mut()[self.tone] -= self.by;
        }
        euler(problem, ticket, workspace)
    }
}

/// Refuses every ticket shorter than its span, each trial of a landing among them, hoping to skip the event.
/// No span crosses a guard unseen, so it spends its budget at the crossing and faults, and nothing publishes
/// past it.
pub struct RefusesEveryLanding {
    pub inner: ReferenceIntegrator,
    pub interval_ns: u64,
}
impl CoupledSolver for RefusesEveryLanding {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.length_ns() < self.interval_ns {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// A pure core that refuses a seventh of all tickets past twice its floor, by a hash of the interval, so a span, a
/// trial and a cut each get refused: the worst an honest function of its arguments can do to the causal claim
/// with a guard in play. Its dice stop at twice the floor, where a retry would have nowhere left to go.
pub struct RefusesByHash {
    pub inner: ReferenceIntegrator,
    pub salt: u64,
}
impl CoupledSolver for RefusesByHash {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let mut bytes = self.salt.to_be_bytes().to_vec();
        bytes.extend_from_slice(&ticket.interval().start().nanoseconds().to_be_bytes());
        bytes.extend_from_slice(&ticket.interval().end().nanoseconds().to_be_bytes());
        let long = ticket.length_ns() >= 2 * MILLISECOND;
        if long && Digest::of_canonical(&bytes).as_bytes()[0].is_multiple_of(7) {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// Short spans early, so an input can arrive, then spans that hold a crossing and an input together, with a hash
/// refusing a seventh of its tickets. It traces every ticket, so two runs compare attempt by attempt.
pub struct TracesItsTickets {
    pub inner: RefusesByHash,
    pub trace: RefCell<Vec<(u64, u64, u32)>>,
}
impl CoupledSolver for TracesItsTickets {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        if accepted.time().nanoseconds() < 10_000_000_000 {
            1_000_000_000
        } else {
            64_000_000_000
        }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let start = ticket.interval().start().nanoseconds();
        self.trace
            .borrow_mut()
            .push((start, ticket.length_ns(), ticket.attempt()));
        self.inner.solve(problem, ticket)
    }
}

/// Refuses its first landing trial once, so a test can watch the retry. The `Cell` makes it impure, which only
/// this test's single run can afford.
pub struct RefusesItsFirstTrial {
    pub inner: ReferenceIntegrator,
    pub interval_ns: u64,
    pub seen: Cell<Option<u64>>,
}
impl CoupledSolver for RefusesItsFirstTrial {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.attempt() == 0 && ticket.length_ns() < self.interval_ns && self.seen.get().is_none() {
            self.seen.set(Some(ticket.length_ns()));
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// Keeps its continuation's start across a firing, as if a mode change were not a breakpoint.
pub struct KeepsItsHistoryAcrossAFiring {
    pub interval_ns: u64,
}
impl CoupledSolver for KeepsItsHistoryAcrossAFiring {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"keeps its history across a firing")
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.interval_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let mut workspace = problem.workspace();
        let time = ticket.at(0)?;
        let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
            .map(|residual| -residual)
            .collect();
        workspace.derivative_mut().copy_from_slice(&rates);
        let stage = problem.evaluate(&workspace.point(time))?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let kept = problem.accepted().method_history().clone();
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, kept))
    }
}

/// Honest, and it evaluates once more before each solve, so an extra evaluation is in play even in the gate's
/// reference run.
pub struct CountsEvaluations {
    pub inner: ReferenceIntegrator,
    pub evaluations: Cell<u64>,
}
impl CoupledSolver for CountsEvaluations {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let workspace = problem.workspace();
        problem.evaluate(&workspace.point(ticket.at(0)?))?;
        self.evaluations.set(self.evaluations.get() + 1);
        self.inner.solve(problem, ticket)
    }
}

/// Honest, and it runs the public `verify` on its own proposal before it hands in another of the same.
pub struct VerifiesDry {
    pub inner: ReferenceIntegrator,
    pub checked: Cell<u64>,
}
impl CoupledSolver for VerifiesDry {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let parts = self.inner.solve(problem, ticket)?.verify(problem)?.into_parts();
        assert!(parts.fired.is_empty(), "a dry verify fired");
        self.checked.set(self.checked.get() + 1);
        self.inner.solve(problem, ticket)
    }
}

/// Answers from a list, so the live pace stops a run short of its end at known instants.
pub struct ReadingClock {
    pub readings: Vec<u64>,
    pub next: Cell<usize>,
}
impl WallClock for ReadingClock {
    fn elapsed_ns(&self) -> Option<u64> {
        let reading = *self.readings.get(self.next.get())?;
        self.next.set(self.next.get() + 1);
        Some(reading)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}
