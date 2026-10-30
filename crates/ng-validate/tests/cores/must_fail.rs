use super::stages::{ForwardEuler, Heun, advance, euler_step, explicit_stage, heun_stages};
use super::{fresh_history, resolution};
use core::cell::RefCell;
use ng_contract::*;
use ng_validate::ReferenceIntegrator;

/// Declares Heun but books the one sample at the start at weight one, while its endpoint came from the
/// stage it never booked. `Extents` cannot be written by hand, so this is the shortest path to the lie.
pub struct OneSample {
    pub interval_ns: u64,
}
impl CoupledSolver for OneSample {
    fn resolution(&self) -> DeclaredResolution {
        Heun {
            interval_ns: self.interval_ns,
        }
        .resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (first, _, workspace) = heun_stages(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&first, Ratio::ONE)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Declares Heun and books its second stage away from the interval's end.
pub struct MisplacedStage {
    pub interval_ns: u64,
}
impl CoupledSolver for MisplacedStage {
    fn resolution(&self) -> DeclaredResolution {
        Heun {
            interval_ns: self.interval_ns,
        }
        .resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (mut workspace, first) = euler_step(problem, ticket)?;
        advance(&mut workspace, ticket);
        let second = explicit_stage(problem, &mut workspace, ticket.at(ticket.length_ns() / 2)?)?;
        let half = Ratio::new(1, 2).expect("a fraction");
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&first, half)?;
        quadrature.add(&second, half)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Declares doubled backward Euler and books each stage at the start state with its tangent at the rates
/// there. Every residual is zero and no stage reaches the value its tangent says it should.
pub struct UnreachedStage {
    pub inner: ReferenceIntegrator,
}
impl CoupledSolver for UnreachedStage {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns();
        let half = length / 2;
        let mut workspace = problem.workspace();
        let mut stage = |offset| explicit_stage(problem, &mut workspace, ticket.at(offset)?);
        let (whole, first, second) = (stage(length)?, stage(half)?, stage(length)?);
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&whole, Ratio::ZERO)?;
        quadrature.add(&first, ticket.fraction(half))?;
        quadrature.add(&second, ticket.fraction(length - half))?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &problem.workspace(), history))
    }
}

/// Declares Heun and weighs its stages unevenly. The weights still sum to one.
pub struct WrongWeight {
    pub interval_ns: u64,
}
impl CoupledSolver for WrongWeight {
    fn resolution(&self) -> DeclaredResolution {
        Heun {
            interval_ns: self.interval_ns,
        }
        .resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (first, second, workspace) = heun_stages(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&first, Ratio::new(3, 4).expect("a fraction"))?;
        quadrature.add(&second, Ratio::new(1, 4).expect("a fraction"))?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Books weights that do not sum to one.
pub struct UnevenWeights {
    pub interval_ns: u64,
}
impl CoupledSolver for UnevenWeights {
    fn resolution(&self) -> DeclaredResolution {
        resolution(StageScheme::forward_euler(), b"uneven weights", 12)
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::new(1, 2).expect("a fraction"))?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Books weights whose exact sum does not fit in lowest terms.
pub struct UnfitWeights {
    pub interval_ns: u64,
}
impl CoupledSolver for UnfitWeights {
    fn resolution(&self) -> DeclaredResolution {
        resolution(StageScheme::forward_euler(), b"unfit weights", 12)
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::new(1, u64::MAX).expect("a fraction"))?;
        quadrature.add(&stage, Ratio::new(1, u64::MAX - 1).expect("a fraction"))?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Solves on other parameter values under the real ticket. Each number is honest for those values, so only
/// the record's replay on its own values sees it: this is what makes a recorded rate differ while
/// `StageEvaluation` stays unforgeable.
pub struct OtherValues<'c> {
    pub body: &'c CompiledBody,
    pub values: &'c BoundParameters,
    pub identity: &'c RunIdentity,
    pub inner: ReferenceIntegrator,
}
impl CoupledSolver for OtherValues<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let base = Digest::of_canonical(b"not the record's base");
        let shadow = CoupledProblem::new(self.body, self.values, self.identity, problem.accepted(), base);
        self.inner.solve(&shadow, ticket)
    }
}

/// Restarts honestly through `keeps_after`, then keeps its history across the next breakpoint.
pub struct KeptHistory {
    pub inner: ForwardEuler,
    pub keeps_after: SimTime,
}
impl CoupledSolver for KeptHistory {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let accepted = problem.accepted().method_history();
        let history = if ticket.interval().start() > self.keeps_after {
            MethodHistory::new(accepted.schema(), accepted.started_at(), Vec::new())
        } else {
            fresh_history(problem, ticket)
        };
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Publishes a continuation tagged with another core's schema.
pub struct ForeignHistory {
    pub inner: ForwardEuler,
}
impl CoupledSolver for ForeignHistory {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let schema = Digest::of_canonical(b"another core's continuation");
        let history = MethodHistory::new(schema, ticket.interval().start(), Vec::new());
        Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Solves the first step twice and answers the second step with the spare: a proposal of a stale base.
pub struct StaleProposal {
    pub inner: ForwardEuler,
    pub kept: RefCell<Option<StepProposal>>,
}
impl CoupledSolver for StaleProposal {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if let Some(stale) = self.kept.borrow_mut().take() {
            return Ok(stale);
        }
        let spare = self.inner.solve(problem, ticket)?;
        self.kept.replace(Some(spare));
        self.inner.solve(problem, ticket)
    }
}

/// Solves its first step twice and books the spare increment for the next step, with a fresh ticket and
/// a fresh history, so only the increment is stale. The state barely moved, so each witnessed stage still
/// lands within the scales.
pub struct StaleIncrement {
    pub inner: ForwardEuler,
    pub kept: RefCell<Option<Increment>>,
}
impl CoupledSolver for StaleIncrement {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let history = fresh_history(problem, ticket);
        if let Some(stale) = self.kept.borrow_mut().take() {
            return Ok(problem.propose(ticket, stale, &workspace, history));
        }
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        self.kept.replace(Some(quadrature.finish()?));
        self.inner.solve(problem, ticket)
    }
}

/// Refuses its first attempt, then books the evaluation it kept from it.
pub struct EarlierAttempt {
    pub inner: ForwardEuler,
    pub kept: RefCell<Option<StageEvaluation>>,
}
impl CoupledSolver for EarlierAttempt {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let (workspace, stage) = euler_step(problem, ticket)?;
        let kept = self.kept.borrow_mut().take();
        match kept {
            None => {
                self.kept.replace(Some(stage));
                Err(ticket.refuse(NumericalRefusal::IterationLimit))
            }
            Some(earlier) => {
                let mut quadrature = problem.quadrature(ticket);
                quadrature.add(&earlier, Ratio::ONE)?;
                let history = fresh_history(problem, ticket);
                Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
            }
        }
    }
}

/// Solves a fresh ticket of its own span and hands back that proposal for the one it was issued.
pub struct SwappedTicket {
    pub inner: ForwardEuler,
    pub own_span_ns: u64,
}
impl CoupledSolver for SwappedTicket {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(
        &self,
        problem: &CoupledProblem<'_>,
        _ticket: &StepTicket,
    ) -> Result<StepProposal, TrialFailure> {
        let fresh = problem.ticket(self.own_span_ns)?;
        self.inner.solve(problem, &fresh)
    }
}

/// Refuses every attempt through a fresh ticket, whose budget has never been spent.
pub struct ForeignRefusal {
    pub inner: ForwardEuler,
}
impl CoupledSolver for ForeignRefusal {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let fresh = problem.ticket(ticket.length_ns() / 2)?;
        Err(fresh.refuse(NumericalRefusal::IterationLimit))
    }
}

/// Reads its ticket from a problem over another record's pending inputs under `base`, so with the real
/// base the ticket still misses a delivery. With `refuse` it refuses that ticket instead of solving it.
pub struct ShadowTicket<'c> {
    pub inner: ForwardEuler,
    pub body: &'c CompiledBody,
    pub values: &'c BoundParameters,
    pub identity: &'c RunIdentity,
    pub shadow: AcceptedStateView<'c>,
    pub base: Digest,
    pub refuse: bool,
}
impl CoupledSolver for ShadowTicket<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, _problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        let shadow = CoupledProblem::new(self.body, self.values, self.identity, self.shadow, self.base);
        shadow.ticket(self.inner.interval_ns)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if self.refuse {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// Evaluates on another body under the real ticket.
pub struct ForeignBody<'c> {
    pub inner: ForwardEuler,
    pub body: &'c CompiledBody,
    pub values: &'c BoundParameters,
    pub identity: &'c RunIdentity,
    pub accepted: AcceptedStateView<'c>,
}
impl CoupledSolver for ForeignBody<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let base = Digest::of_canonical(b"another body");
        let foreign = CoupledProblem::new(self.body, self.values, self.identity, self.accepted, base);
        let (_, stage) = euler_step(&foreign, ticket)?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &problem.workspace(), history))
    }
}

/// Evaluates the real problem at another body's workspace.
pub struct ForeignWorkspace<'c> {
    pub inner: ForwardEuler,
    pub body: &'c CompiledBody,
    pub values: &'c BoundParameters,
    pub identity: &'c RunIdentity,
    pub accepted: AcceptedStateView<'c>,
}
impl CoupledSolver for ForeignWorkspace<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let base = Digest::of_canonical(b"another body");
        let foreign = CoupledProblem::new(self.body, self.values, self.identity, self.accepted, base);
        let workspace = foreign.workspace();
        let stage = problem.evaluate(&workspace.point(ticket.at(0)?))?;
        let mut quadrature = problem.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let history = fresh_history(problem, ticket);
        Ok(problem.propose(ticket, quadrature.finish()?, &problem.workspace(), history))
    }
}

/// Builds its whole proposal on another body.
pub struct ForeignProposal<'c> {
    pub inner: ForwardEuler,
    pub body: &'c CompiledBody,
    pub values: &'c BoundParameters,
    pub identity: &'c RunIdentity,
    pub accepted: AcceptedStateView<'c>,
}
impl CoupledSolver for ForeignProposal<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(
        &self,
        _problem: &CoupledProblem<'_>,
        ticket: &StepTicket,
    ) -> Result<StepProposal, TrialFailure> {
        let base = Digest::of_canonical(b"another body");
        let foreign = CoupledProblem::new(self.body, self.values, self.identity, self.accepted, base);
        let (workspace, stage) = euler_step(&foreign, ticket)?;
        let mut quadrature = foreign.quadrature(ticket);
        quadrature.add(&stage, Ratio::ONE)?;
        let history = fresh_history(&foreign, ticket);
        Ok(foreign.propose(ticket, quadrature.finish()?, &workspace, history))
    }
}

/// Refuses every attempt, so the budget or the floor must end it.
pub struct NeverConverges {
    pub interval_ns: u64,
    pub budget: u32,
}
impl CoupledSolver for NeverConverges {
    fn resolution(&self) -> DeclaredResolution {
        resolution(StageScheme::forward_euler(), b"never converges", self.budget)
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        problem.ticket(self.interval_ns)
    }
    fn solve(
        &self,
        _problem: &CoupledProblem<'_>,
        ticket: &StepTicket,
    ) -> Result<StepProposal, TrialFailure> {
        Err(ticket.refuse(NumericalRefusal::IterationLimit))
    }
}

/// Writes a non-finite coordinate before it evaluates.
pub struct PoisonedPoint {
    pub inner: ForwardEuler,
}
impl CoupledSolver for PoisonedPoint {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, problem: &CoupledProblem<'_>) -> Result<StepTicket, EngineFault> {
        self.inner.next_step(problem)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let mut workspace = problem.workspace();
        workspace.unknowns_mut()[1] = f64::INFINITY;
        problem.evaluate(&workspace.point(ticket.at(0)?))?;
        self.inner.solve(problem, ticket)
    }
}
