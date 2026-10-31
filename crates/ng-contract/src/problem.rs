use crate::body::UnknownId;
use crate::boundary::DriveFrame;
use crate::chance::Chance;
use crate::evaluate::{Inputs, Undefined};
use crate::identity::{Canonical, Seal};
use crate::quadrature::StageEvaluation;
use crate::{
    AcceptedStateView, BoundParameters, Candidate, CompiledBody, Digest, EngineFault, EngineFaultKind,
    InputId, Interval, NumericalRefusal, Ratio, Refused, RowId, RunIdentity, SimTime, TrialFailure,
};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// What a solver core sees of one attempt. The record builds one per attempt from its own fields. Building
/// one on other values is allowed and reaches nothing, because `StepProposal::verify` re-evaluates on the
/// record's.
pub struct CoupledProblem<'r> {
    pub(crate) body: &'r CompiledBody,
    pub(crate) parameters: &'r BoundParameters,
    pub(crate) identity: &'r RunIdentity,
    pub(crate) accepted: AcceptedStateView<'r>,
    pub(crate) base: Digest,
    pub(crate) chance: Chance,
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
            chance: Chance::at(body, identity.seed(), accepted.state()),
        }
    }
    pub fn accepted(&self) -> AcceptedStateView<'r> {
        self.accepted
    }
    /// The row at a workspace slot, so a core can name where a stage failed.
    pub fn row(&self, slot: usize) -> Option<RowId> {
        self.body.rows().get(slot).map(|row| row.id())
    }
    /// Numerical scratch seeded from the accepted state: each amount rounded once, the tangent at zero.
    pub fn workspace(&self) -> SolveWorkspace {
        SolveWorkspace::seeded(self.body, self.accepted.state().coordinates())
    }
    pub(crate) fn layout(&self) -> Digest {
        self.body.model_hash().digest()
    }
    fn check(&self, point: &StagePoint<'_, '_>) -> Result<(), TrialFailure> {
        let ticket = point.time.ticket;
        let rows = self.body.rows().len();
        if point.layout != self.layout() || point.unknowns.len() != rows || point.derivative.len() != rows {
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
    /// `F(t, y, y')` on every row with the bound values, in the head's modes. A drive and a draw read the
    /// head's, never the stage's instant; a forcing reads the stage's instant, which the witness carries.
    pub fn evaluate(&self, point: &StagePoint<'_, '_>) -> Result<StageEvaluation, TrialFailure> {
        self.check(point)?;
        let ticket = point.time.ticket;
        let factors = self.factors(point);
        let modes = &self.accepted.state().modes;
        let inputs = Inputs {
            parameters: &self.parameters.values,
            frame: &ticket.frame,
            unknowns: point.unknowns,
            modes,
            draws: self.chance.draws(),
            factors: &factors,
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
        let factors = self.factors(point);
        let inputs = Inputs {
            parameters: &self.parameters.values,
            frame: &ticket.frame,
            unknowns: point.unknowns,
            modes: &self.accepted.state().modes,
            draws: self.chance.draws(),
            factors: &factors,
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
    /// Every law on `state` at the ticket's end, in its modes and on its draws.
    pub(crate) fn values_after_firing(
        &self,
        ticket: &StepTicket,
        state: &Candidate,
    ) -> Result<Vec<Option<f64>>, TrialFailure> {
        let chance = Chance::at(self.body, self.identity.seed(), state);
        let inputs = Inputs {
            parameters: &self.parameters.values,
            frame: &ticket.frame,
            unknowns: &state.coordinates(),
            modes: &state.modes,
            draws: chance.draws(),
            factors: &chance.factors(&self.parameters.values, ticket.interval().end()),
        };
        (self.body.instance_values(&inputs)).map_err(|undefined| self.undefined(ticket, undefined))
    }
    fn factors(&self, point: &StagePoint<'_, '_>) -> Vec<f64> {
        let ticket = point.time.ticket;
        let at = SimTime::from_ns(ticket.interval.start().nanoseconds() + point.time.offset_ns);
        self.chance.factors(&self.parameters.values, at)
    }
}

/// Binds one attempt: its base, the identity, the exact interval, the span it came from, the due inputs, the
/// frozen drives, the attempt number and the fence, all in its digest.
#[derive(Clone, Debug)]
pub struct StepTicket {
    pub(crate) base: Digest,
    pub(crate) interval: Interval,
    pub(crate) span_ns: u64,
    pub(crate) due: Vec<InputId>,
    pub(crate) frame: DriveFrame,
    pub(crate) after_breakpoint: bool,
    pub(crate) attempt: u32,
    pub(crate) fence: SimTime,
    pub(crate) floor_ns: u64,
    pub(crate) budget: u32,
    pub(crate) digest: Digest,
}
impl StepTicket {
    /// The floor and the budget are the identity's, so sealing the identity covers them.
    pub(crate) fn sealed(&self, identity: Digest) -> Digest {
        let Self {
            base,
            interval,
            span_ns,
            due,
            frame,
            after_breakpoint,
            attempt,
            fence,
            floor_ns: _,
            budget: _,
            digest: _,
        } = self;
        let mut out = Canonical::new("ng-contract ticket 1");
        base.seal(&mut out);
        identity.seal(&mut out);
        interval.start().seal(&mut out);
        interval.end().seal(&mut out);
        span_ns.seal(&mut out);
        due.seal(&mut out);
        frame.seal(&mut out);
        out.number(u64::from(*after_breakpoint));
        out.number(u64::from(*attempt));
        fence.seal(&mut out);
        out.digest()
    }
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
    /// True when a delivery starts or ends at this interval's start, or a guard fired there. A core restarts
    /// its numerical caches here, and `verify` checks that the proposal's `MethodHistory` starts at this
    /// instant.
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
    /// or past the budget. The retry halves the committed span, not the interval: a cut's length depends on
    /// a waiting input past the retry's end, and a retry's end must not.
    ///
    /// A landing's trial takes this rule too. Its retry is a new attempt at a shorter span, which lands in turn
    /// or steps short of the crossing and loses nothing, since the integral keeps what it gained. A retried
    /// step is then at least the floor or ends at a breakpoint: a retry at a bracket's verified bottom
    /// skipped that, and crawled an honest core at 1 ns a step.
    pub fn refuse(&self, reason: NumericalRefusal) -> TrialFailure {
        let mut retry_ns = self.span_ns / 2;
        while retry_ns >= self.length_ns() {
            retry_ns /= 2;
        }
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
            fence: self.fence,
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
}

/// An offset inside one ticket.
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
    pub fn derivative_mut(&mut self) -> &mut [f64] {
        &mut self.derivative
    }
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
