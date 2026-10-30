use core::cell::{Cell, RefCell};
use ng_contract::*;
use ng_validate::{ClockSource, ReferenceIntegrator, WallClock};
use std::rc::Rc;

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

/// An honest forward Euler step with these `counters` as its continuation, since a core that keeps
/// anything there has to propose for itself.
fn explicit(
    problem: &CoupledProblem<'_>,
    ticket: &StepTicket,
    counters: Vec<u64>,
) -> Result<StepProposal, TrialFailure> {
    let mut workspace = problem.workspace();
    let time = ticket.at(0)?;
    let rates: Vec<f64> = (problem.evaluate(&workspace.point(time))?.residuals().iter())
        .map(|residual| -residual)
        .collect();
    workspace.derivative_mut().copy_from_slice(&rates);
    let stage = problem.evaluate(&workspace.point(time))?;
    let mut quadrature = problem.quadrature(ticket);
    quadrature.add(&stage, Ratio::ONE)?;
    let schema = problem.accepted().method_history().schema();
    let history = MethodHistory::new(schema, ticket.interval().start(), counters);
    Ok(problem.propose(ticket, quadrature.finish()?, &workspace, history))
}

/// A count in a `Cell`, outside its continuation, shapes its span.
pub struct KeepsACache {
    pub interval_ns: u64,
    pub steps: Cell<u64>,
}
impl CoupledSolver for KeepsACache {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"keeps a count")
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        let steps = self.steps.get();
        self.steps.set(steps + 1);
        self.interval_ns / (1 + steps % 3)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        explicit(problem, ticket, Vec::new())
    }
}

/// The legal twin of `KeepsACache`: the same count, in its continuation.
pub struct KeepsACount {
    pub interval_ns: u64,
}
impl CoupledSolver for KeepsACount {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"keeps a count")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let steps = accepted.method_history().counters().first().copied().unwrap_or(0);
        self.interval_ns / (1 + steps % 3)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let history = problem.accepted().method_history();
        let steps = history.counters().first().copied().unwrap_or(0);
        explicit(problem, ticket, vec![steps + 1])
    }
}

/// Cuts its span at the output times a caller told it, as a core that clips at them does. It never hears
/// them through the seam, so a test's factory hands them over.
pub struct ClipsAt {
    pub inner: ReferenceIntegrator,
    pub outputs: Vec<SimTime>,
}
impl CoupledSolver for ClipsAt {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let own = self.inner.next_step(accepted);
        let head = accepted.time();
        let next = self.outputs.iter().find(|output| **output > head);
        next.map_or(own, |output| own.min(output.nanoseconds() - head.nanoseconds()))
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

/// A list of readings, each read once, and what the last read returned, which `ReadsTheHostClock` shapes
/// its span from.
pub struct ScriptedClock {
    pub readings: RefCell<std::vec::IntoIter<u64>>,
    pub last: Cell<u64>,
}
impl ScriptedClock {
    pub fn new(readings: Vec<u64>) -> Self {
        Self {
            readings: RefCell::new(readings.into_iter()),
            last: Cell::new(0),
        }
    }
}
impl WallClock for ScriptedClock {
    fn elapsed_ns(&self) -> Option<u64> {
        let reading = self.readings.borrow_mut().next()?;
        self.last.set(reading);
        Some(reading)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

pub struct ReadsTheHostClock {
    pub inner: ReferenceIntegrator,
    pub clock: Rc<ScriptedClock>,
}
impl CoupledSolver for ReadsTheHostClock {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted) + self.clock.last.get() % 3
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

/// One solve as a core sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub interval: Interval,
    pub attempt: u32,
    pub digest: Digest,
}

/// The attempt to read past the breakpoint: commits to any span and writes down every ticket it is shown.
/// It refuses a ticket longer than `refuses_over_ns`, as a core's own error control would.
pub struct ReadsPastTheBreakpoint {
    pub inner: ReferenceIntegrator,
    pub span_ns: u64,
    pub refuses_over_ns: u64,
    pub seen: Rc<RefCell<Vec<Seen>>>,
}
impl CoupledSolver for ReadsPastTheBreakpoint {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, _accepted: AcceptedStateView<'_>) -> u64 {
        self.span_ns
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.seen.borrow_mut().push(Seen {
            interval: ticket.interval(),
            attempt: ticket.attempt(),
            digest: ticket.digest(),
        });
        if ticket.length_ns() > self.refuses_over_ns {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// Reads a cut, refuses it, and spreads its steps toward the instant it read. The instant crosses from the
/// refused attempt to the retry in a `Cell`, because a retry's length no longer tells it.
pub struct ActsOnACut {
    pub span_ns: u64,
    pub read: Cell<Option<u64>>,
}
impl CoupledSolver for ActsOnACut {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"acts on a cut")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let read = accepted.method_history().counters().first().copied();
        let left = read.map(|read| read.saturating_sub(accepted.time().nanoseconds()));
        left.filter(|left| *left > 1)
            .map_or(self.span_ns, |left| left / 2)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.attempt() == 0 && ticket.length_ns() < self.span_ns && self.read.get().is_none() {
            self.read.set(Some(ticket.interval().end().nanoseconds()));
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        let kept = problem.accepted().method_history().counters().to_vec();
        let counters = self.read.get().map_or(kept, |read| vec![read]);
        explicit(problem, ticket, counters)
    }
}

/// Refuses exactly one ticket, by its ends: a pure core, so what it does a run with the same tickets does.
pub struct RefusesOne {
    pub inner: ReferenceIntegrator,
    pub interval: Interval,
    /// Its span from every head past the origin, so a test can step short of a fence.
    pub later_ns: u64,
    pub seen: Rc<RefCell<Vec<Seen>>>,
}
impl CoupledSolver for RefusesOne {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        if accepted.time() == SimTime::from_ns(0) {
            self.inner.next_step(accepted)
        } else {
            self.later_ns
        }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.seen.borrow_mut().push(Seen {
            interval: ticket.interval(),
            attempt: ticket.attempt(),
            digest: ticket.digest(),
        });
        if ticket.interval() == self.interval {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

/// Keeps the bytes of each ticket's digest in its continuation and shapes its span from them. Honest in
/// every way, so a run that differs by a waiting input differs only if a digest carried that input.
pub struct KeepsTheDigest {
    pub interval_ns: u64,
}
impl CoupledSolver for KeepsTheDigest {
    fn resolution(&self) -> DeclaredResolution {
        explicit_resolution(b"keeps the digest")
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let kept = accepted.method_history().counters().first().copied().unwrap_or(0);
        self.interval_ns + kept % 1_000
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let digest = ticket.digest();
        let bytes = digest.as_bytes().first_chunk().expect("thirty-two bytes");
        explicit(problem, ticket, vec![u64::from_be_bytes(*bytes)])
    }
}
