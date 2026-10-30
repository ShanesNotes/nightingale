//! One step's problem and what a core reads and writes while it solves. No time step enters a kernel: a
//! stage time is an exact integer offset inside a ticketed interval.
use crate::body::UnknownId;
use crate::boundary::DriveFrame;
use crate::evaluate::{Inputs, Undefined};
use crate::identity::{Canonical, Seal};
use crate::quadrature::StageEvaluation;
use crate::{
    AcceptedStateView, BoundParameters, CompiledBody, Digest, EngineFault, EngineFaultKind, InputId,
    Interval, NumericalRefusal, Ratio, Refused, RunIdentity, TrialFailure,
};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// What a solver core sees of one attempt: the compiled body, the bound values, the identity and the
/// accepted image. The record builds one per attempt from its own fields. Building one on other values is
/// allowed and reaches nothing, because `StepProposal::verify` re-evaluates on the record's.
pub struct CoupledProblem<'r> {
    pub(crate) body: &'r CompiledBody,
    pub(crate) parameters: &'r BoundParameters,
    pub(crate) identity: &'r RunIdentity,
    pub(crate) accepted: AcceptedStateView<'r>,
    pub(crate) base: Digest,
}
impl<'r> CoupledProblem<'r> {
    pub fn new(
        body: &'r CompiledBody,
        parameters: &'r BoundParameters,
        identity: &'r RunIdentity,
        accepted: AcceptedStateView<'r>,
        base: Digest,
    ) -> Self {
        Self {
            body,
            parameters,
            identity,
            accepted,
            base,
        }
    }
    pub fn accepted(&self) -> AcceptedStateView<'r> {
        self.accepted
    }
    /// The core's own span, cut short only at the next breakpoint, which the core never sees. A zero span,
    /// or one past the end of time, is `BindingMismatch`.
    pub fn ticket(&self, intrinsic_span_ns: u64) -> Result<StepTicket, EngineFault> {
        self.issue(intrinsic_span_ns, 0)
    }
    /// The same base over the smaller span a refusal left, through the same clipping as any ticket. A
    /// refusal of another base is `BindingMismatch`.
    pub fn retry(&self, refused: &Refused) -> Result<StepTicket, EngineFault> {
        if refused.base != self.base {
            return Err(EngineFault {
                kind: EngineFaultKind::BindingMismatch,
                at: refused.at,
                attempted_ns: refused.attempted_ns,
                last_refusal: None,
            });
        }
        self.issue(refused.retry_ns, refused.attempt + 1)
    }
    fn issue(&self, span_ns: u64, attempt: u32) -> Result<StepTicket, EngineFault> {
        let start = self.accepted.time();
        let mismatch = EngineFault {
            kind: EngineFaultKind::BindingMismatch,
            at: start,
            attempted_ns: span_ns,
            last_refusal: None,
        };
        let plan = self.accepted.pending.plan(start);
        let intrinsic_end = start.plus_ns(span_ns).ok_or(mismatch)?;
        let end = plan
            .next_breakpoint
            .map_or(intrinsic_end, |next| next.min(intrinsic_end));
        let interval = Interval::new(start, end).ok_or(mismatch)?;
        let identity = self.identity.digest();
        let mut out = Canonical::new("ng-contract ticket 1");
        self.base.seal(&mut out);
        identity.seal(&mut out);
        start.seal(&mut out);
        end.seal(&mut out);
        plan.due.seal(&mut out);
        plan.frame.seal(&mut out);
        out.number(u64::from(plan.after_breakpoint));
        out.number(u64::from(attempt));
        let resolution = self.identity.resolution();
        Ok(StepTicket {
            base: self.base,
            interval,
            due: plan.due,
            frame: plan.frame,
            after_breakpoint: plan.after_breakpoint,
            attempt,
            floor_ns: resolution.minimum_interval_ns(),
            budget: resolution.retry_budget(),
            digest: out.digest(),
        })
    }
    /// Numerical scratch seeded from the accepted state: each amount rounded once, the tangent at zero.
    pub fn workspace(&self) -> SolveWorkspace {
        SolveWorkspace::seeded(self.body, self.accepted.state().coordinates())
    }
    pub(crate) fn layout(&self) -> Digest {
        self.body.model_hash().digest()
    }
    /// Refuses a point from another body's workspace, and a non-finite coordinate, by its row.
    fn check(&self, point: &StagePoint<'_, '_>) -> Result<(), TrialFailure> {
        let ticket = point.time.ticket;
        if point.layout != self.layout() {
            return Err(ticket.mismatch().into());
        }
        let bad = |values: &[f64]| values.iter().position(|value| !value.is_finite());
        match bad(point.unknowns).or_else(|| bad(point.derivative)) {
            None => Ok(()),
            Some(at) => Err(ticket.refuse(NumericalRefusal::TrialDomain {
                source: self.body.row_source(self.body.rows()[at].id()),
            })),
        }
    }
    fn undefined(&self, ticket: &StepTicket, undefined: Undefined) -> TrialFailure {
        ticket.refuse(NumericalRefusal::TrialDomain {
            source: self.body.undefined_source(undefined),
        })
    }
    /// `F(t, y, y')` on every row with the bound values. A drive reads the ticket's frame, never the
    /// stage's instant.
    pub fn evaluate(&self, point: &StagePoint<'_, '_>) -> Result<StageEvaluation, TrialFailure> {
        self.check(point)?;
        let ticket = point.time.ticket;
        let inputs = Inputs {
            parameters: &self.parameters.values,
            frame: &ticket.frame,
            unknowns: point.unknowns,
        };
        let values =
            (self.body.instance_values(&inputs)).map_err(|undefined| self.undefined(ticket, undefined))?;
        let (columns, rows) = self.body.targets(&values);
        let residuals = self.body.residuals(&columns, &rows, point.derivative);
        Ok(StageEvaluation {
            layout: point.layout,
            ticket: ticket.digest,
            offset_ns: point.time.offset_ns,
            unknowns: point.unknowns.to_vec(),
            derivative: point.derivative.to_vec(),
            columns,
            rows,
            values,
            residuals,
        })
    }
    /// `dF/dy + shift * dF/dy'` by fixed-order forward differentiation over the compiled sparsity.
    pub fn linearize(&self, point: &StagePoint<'_, '_>, shift: Shift) -> Result<Jacobian, TrialFailure> {
        self.check(point)?;
        let ticket = point.time.ticket;
        let inputs = Inputs {
            parameters: &self.parameters.values,
            frame: &ticket.frame,
            unknowns: point.unknowns,
        };
        let values = (self.body.jacobian(&inputs, shift.per_second))
            .map_err(|undefined| self.undefined(ticket, undefined))?;
        let sparsity = self.body.sparsity();
        Ok(Jacobian {
            row_offsets: sparsity.row_offsets().to_vec(),
            columns: sparsity.unknowns().to_vec(),
            values,
        })
    }
}

/// Binds one attempt: its base, the identity, the exact interval, the due inputs, the frozen drives and the
/// attempt number, all in its digest.
#[derive(Clone, Debug)]
pub struct StepTicket {
    pub(crate) base: Digest,
    pub(crate) interval: Interval,
    pub(crate) due: Vec<InputId>,
    pub(crate) frame: DriveFrame,
    pub(crate) after_breakpoint: bool,
    pub(crate) attempt: u32,
    pub(crate) floor_ns: u64,
    pub(crate) budget: u32,
    pub(crate) digest: Digest,
}
impl StepTicket {
    pub fn interval(&self) -> Interval {
        self.interval
    }
    pub fn length_ns(&self) -> u64 {
        self.interval.length_ns()
    }
    /// How many refusals of this base came before this ticket.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    /// True when a delivery starts or ends at this interval's start. A core restarts its numerical caches
    /// here, and `verify` checks that the proposal's `MethodHistory` starts at this instant.
    pub fn after_breakpoint(&self) -> bool {
        self.after_breakpoint
    }
    pub fn digest(&self) -> Digest {
        self.digest
    }
    /// A stage time inside this interval. An offset past the end is `BindingMismatch`.
    pub fn at(&self, offset_ns: u64) -> Result<StageTime<'_>, EngineFault> {
        if offset_ns > self.length_ns() {
            return Err(self.mismatch());
        }
        Ok(StageTime {
            ticket: self,
            offset_ns,
        })
    }
    /// `span_ns` as a fraction of this interval: an exact stage weight.
    pub fn fraction(&self, span_ns: u64) -> Ratio {
        Ratio::new(span_ns, self.length_ns()).expect("a ticket's length is not zero")
    }
    /// `1 / span` for a stage that spans `span_ns` of this interval. Zero is `BindingMismatch`.
    pub fn shift(&self, span_ns: u64) -> Result<Shift, EngineFault> {
        if span_ns == 0 {
            return Err(self.mismatch());
        }
        Ok(Shift {
            per_second: NANOSECONDS_PER_SECOND / span_ns as f64,
        })
    }
    /// A refusal of this attempt, or the fault `NoAdmissibleStep` when its retry would go below the floor
    /// or past the budget. One path, so nothing can accept excess error at the floor.
    pub fn refuse(&self, reason: NumericalRefusal) -> TrialFailure {
        let retry_ns = self.length_ns() / 2;
        let at = self.interval.start();
        if self.attempt >= self.budget || retry_ns < self.floor_ns {
            return TrialFailure::EngineFault(EngineFault {
                kind: EngineFaultKind::NoAdmissibleStep,
                at,
                attempted_ns: self.length_ns(),
                last_refusal: Some(reason),
            });
        }
        TrialFailure::Refused(Refused {
            reason,
            ticket: self.digest,
            base: self.base,
            at,
            attempted_ns: self.length_ns(),
            retry_ns,
            attempt: self.attempt,
        })
    }
    /// A `BindingMismatch` naming this attempt.
    pub fn mismatch(&self) -> EngineFault {
        EngineFault {
            kind: EngineFaultKind::BindingMismatch,
            at: self.interval.start(),
            attempted_ns: self.length_ns(),
            last_refusal: None,
        }
    }
    /// Whether this ticket is the one `problem` issues for its length and attempt: the same base, identity,
    /// interval, due inputs and drives.
    pub fn binds(&self, problem: &CoupledProblem<'_>) -> bool {
        (problem.issue(self.length_ns(), self.attempt)).is_ok_and(|issued| issued.digest == self.digest)
    }
}

/// An offset inside one ticket. It borrows the ticket, so `evaluate` can refuse with only a point in hand.
#[derive(Clone, Copy)]
pub struct StageTime<'t> {
    pub(crate) ticket: &'t StepTicket,
    pub(crate) offset_ns: u64,
}

/// Solver-writable trial coordinates, row `i` at slot `i`.
pub struct SolveWorkspace {
    layout: Digest,
    unknowns: Vec<f64>,
    derivative: Vec<f64>,
}
impl SolveWorkspace {
    pub(crate) fn seeded(body: &CompiledBody, unknowns: Vec<f64>) -> Self {
        Self {
            layout: body.model_hash().digest(),
            derivative: vec![0.0; unknowns.len()],
            unknowns,
        }
    }
    pub(crate) fn layout(&self) -> Digest {
        self.layout
    }
    pub(crate) fn coordinates(&self) -> &[f64] {
        &self.unknowns
    }
    pub fn unknowns_mut(&mut self) -> &mut [f64] {
        &mut self.unknowns
    }
    /// The tangent `y'` a stage evaluates against.
    pub fn derivative_mut(&mut self) -> &mut [f64] {
        &mut self.derivative
    }
    /// The current `(y, y')` at a stage time.
    pub fn point<'w, 't>(&'w self, time: StageTime<'t>) -> StagePoint<'w, 't> {
        StagePoint {
            layout: self.layout,
            time,
            unknowns: &self.unknowns,
            derivative: &self.derivative,
        }
    }
}
pub struct StagePoint<'w, 't> {
    pub(crate) layout: Digest,
    pub(crate) time: StageTime<'t>,
    pub(crate) unknowns: &'w [f64],
    pub(crate) derivative: &'w [f64],
}

/// The derivative coefficient in `dF/dy + shift * dF/dy'`.
#[derive(Clone, Copy)]
pub struct Shift {
    per_second: f64,
}

pub struct Jacobian {
    row_offsets: Vec<usize>,
    columns: Vec<UnknownId>,
    values: Vec<f64>,
}
impl Jacobian {
    pub fn view(&self) -> SparseView<'_> {
        SparseView {
            row_offsets: &self.row_offsets,
            columns: &self.columns,
            values: &self.values,
        }
    }
}
/// Compressed rows on the compiled sparsity. View row `r` is `residuals()[r]`, its entries are
/// `row_offsets()[r]..row_offsets()[r + 1]`, and a column's `UnknownId::index` is its workspace slot.
pub struct SparseView<'a> {
    row_offsets: &'a [usize],
    columns: &'a [UnknownId],
    values: &'a [f64],
}
impl<'a> SparseView<'a> {
    pub fn row_offsets(&self) -> &'a [usize] {
        self.row_offsets
    }
    pub fn columns(&self) -> &'a [UnknownId] {
        self.columns
    }
    pub fn values(&self) -> &'a [f64] {
        self.values
    }
}
