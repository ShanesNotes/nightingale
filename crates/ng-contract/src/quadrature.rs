use crate::body::{HazardLayout, RowKind};
use crate::problem::{SolveWorkspace, StagePoint};
use crate::scheme::Stage;
use crate::{
    Candidate, Compensated, CompiledBody, CoupledProblem, DenseSegment, Digest, EngineFault, EngineFaultKind,
    InputId, LawName, MethodHistory, ModeValue, NumericalRefusal, Ratio, RowId, SimTime, StepTicket,
    TrialFailure, ValidityExcursion,
};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// One stage's evaluation: bound to its ticket and layout and carrying a copy of its point, so it outlives
/// the workspace. Only `CoupledProblem::evaluate` makes one.
pub struct StageEvaluation {
    pub(crate) layout: Digest,
    pub(crate) ticket: Digest,
    pub(crate) offset_ns: u64,
    pub(crate) unknowns: Vec<f64>,
    pub(crate) derivative: Vec<f64>,
    pub(crate) columns: Vec<f64>,
    pub(crate) rows: Vec<f64>,
    pub(crate) values: Vec<Option<f64>>,
    pub(crate) residuals: Vec<f64>,
}
impl StageEvaluation {
    /// `F` per row: what a Newton core drives to zero. With the tangent at zero, a differential row's
    /// value is minus its rate, so an explicit core reads its rates here too.
    pub fn residuals(&self) -> &[f64] {
        &self.residuals
    }
}

/// Accumulates weighted stage evaluations over one ticket: the only producer of `Extents`. Forward and
/// reverse parts of every column stay apart, so the ledger keeps gross in and out.
pub struct Quadrature {
    layout: Digest,
    ticket: Digest,
    mismatch: EngineFault,
    length_ns: u64,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
    first_memory: usize,
    memory: Vec<f64>,
    total: Ratio,
    stages: Vec<WitnessStage>,
}
impl Quadrature {
    /// Adds the weighted rates and keeps the point and weight as a witness stage. `weight` is an exact
    /// fraction of the ticketed interval. An evaluation from another ticket or layout, as from an earlier
    /// attempt, and weights whose sum does not fit in lowest terms are `BindingMismatch`.
    pub fn add(&mut self, evaluation: &StageEvaluation, weight: Ratio) -> Result<(), EngineFault> {
        if evaluation.ticket != self.ticket || evaluation.layout != self.layout {
            return Err(self.mismatch);
        }
        self.total = self.total.plus(weight).ok_or(self.mismatch)?;
        let seconds = weight.seconds_of(self.length_ns);
        for (column, &rate) in evaluation.columns.iter().enumerate() {
            let side = if rate < 0.0 {
                &mut self.reverse[column]
            } else {
                &mut self.forward[column]
            };
            *side = side.plus(Compensated::exact(rate.abs() * seconds));
        }
        let rates = &evaluation.rows[self.first_memory..];
        for (sum, &rate) in self.memory.iter_mut().zip(rates) {
            *sum += rate * seconds;
        }
        self.stages.push(WitnessStage {
            offset_ns: evaluation.offset_ns,
            weight,
            unknowns: evaluation.unknowns.clone(),
            derivative: evaluation.derivative.clone(),
        });
        Ok(())
    }
    /// Weights that do not sum to one exactly are `BindingMismatch`.
    pub fn finish(self) -> Result<Increment, EngineFault> {
        if self.total != Ratio::ONE {
            return Err(self.mismatch);
        }
        let Self {
            layout,
            ticket,
            forward,
            reverse,
            memory,
            stages,
            ..
        } = self;
        Ok(Increment {
            ticket,
            extents: Extents {
                layout,
                forward,
                reverse,
            },
            memory,
            witness: StageWitness { stages },
        })
    }
}

pub struct Increment {
    ticket: Digest,
    extents: Extents,
    memory: Vec<f64>,
    witness: StageWitness,
}
/// Integrated gross extents per compiled column.
pub struct Extents {
    layout: Digest,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
}
struct StageWitness {
    stages: Vec<WitnessStage>,
}
struct WitnessStage {
    offset_ns: u64,
    weight: Ratio,
    unknowns: Vec<f64>,
    derivative: Vec<f64>,
}

impl CoupledProblem<'_> {
    pub fn quadrature(&self, ticket: &StepTicket) -> Quadrature {
        let columns = self.body.columns().len();
        let first_memory = (self.body.rows().iter())
            .take_while(|row| matches!(row.kind(), RowKind::Amount { .. }))
            .count();
        let memory = (self.body.rows().iter())
            .filter(|row| matches!(row.kind(), RowKind::Memory(_)))
            .count();
        Quadrature {
            layout: self.layout(),
            ticket: ticket.digest(),
            mismatch: ticket.mismatch(),
            length_ns: ticket.length_ns(),
            forward: vec![Compensated::ZERO; columns],
            reverse: vec![Compensated::ZERO; columns],
            first_memory,
            memory: vec![0.0; memory],
            total: Ratio::ZERO,
            stages: Vec::new(),
        }
    }
    /// Assembles a proposal. Amounts and memory will come from the start plus the increment, and the dense
    /// segment from the published ends, so `end` supplies only algebraic values.
    pub fn propose(
        &self,
        ticket: &StepTicket,
        increment: Increment,
        end: &SolveWorkspace,
        history: MethodHistory,
    ) -> StepProposal {
        let algebraic = (self.body.rows().iter().zip(end.coordinates()))
            .filter(|(row, _)| matches!(row.kind(), RowKind::Algebraic(_)))
            .map(|(_, &value)| value)
            .collect();
        StepProposal {
            ticket: ticket.clone(),
            layout: end.layout(),
            increment,
            algebraic,
            history,
        }
    }
}

fn seconds(ns: u64) -> f64 {
    ns as f64 / NANOSECONDS_PER_SECOND
}

#[derive(Clone, Copy)]
struct Worst {
    normalized: f64,
    row: Option<RowId>,
}
impl Worst {
    const NONE: Self = Self {
        normalized: 0.0,
        row: None,
    };
    fn note(&mut self, normalized: f64, row: RowId) {
        if normalized > self.normalized {
            *self = Self {
                normalized,
                row: Some(row),
            };
        }
    }
    fn above_one(self) -> Option<RowId> {
        self.row.filter(|_| self.normalized > 1.0)
    }
}

/// A candidate step. It holds no problem and proves nothing until `verify`.
pub struct StepProposal {
    ticket: StepTicket,
    layout: Digest,
    increment: Increment,
    algebraic: Vec<f64>,
    history: MethodHistory,
}
impl StepProposal {
    pub fn ticket(&self) -> &StepTicket {
        &self.ticket
    }
    /// Whether the core followed its declared method on this body. `problem` must be the record's own:
    /// every value re-read here comes from it, none from the core. A refusal is an honest attempt a smaller
    /// step can fix; a fault is a lie about the method or a binding that no smaller step can.
    pub fn verify(self, problem: &CoupledProblem<'_>) -> Result<VerifiedStep, TrialFailure> {
        let Self {
            ticket,
            layout,
            increment:
                Increment {
                    ticket: finished_for,
                    extents,
                    memory,
                    witness,
                },
            algebraic,
            history,
        } = self;
        let algebraic_rows = (problem.body.rows().iter())
            .filter(|row| matches!(row.kind(), RowKind::Algebraic(_)))
            .count();
        if layout != problem.layout()
            || extents.layout != problem.layout()
            || finished_for != ticket.digest()
            || algebraic.len() != algebraic_rows
        {
            return Err(ticket.mismatch().into());
        }
        let mismatch = |ticket: &StepTicket| EngineFault {
            kind: EngineFaultKind::MethodMismatch,
            ..ticket.mismatch()
        };
        let resolution = problem.identity.resolution();
        let scheme = resolution.scheme();
        let stages = scheme.stages(ticket.length_ns());
        if !same_structure(&stages, &witness) {
            return Err(mismatch(&ticket).into());
        }
        let mut replay = problem.quadrature(&ticket);
        let mut evaluations = Vec::with_capacity(stages.len());
        for stage in &witness.stages {
            let point = StagePoint {
                layout: problem.layout(),
                time: ticket.at(stage.offset_ns)?,
                unknowns: &stage.unknowns,
                derivative: &stage.derivative,
            };
            let evaluation = problem.evaluate(&point)?;
            replay.add(&evaluation, stage.weight)?;
            evaluations.push(evaluation);
        }
        let replayed = replay.finish()?;
        let (forward, reverse) = (&extents.forward, &extents.reverse);
        if !same_bits(forward, &replayed.extents.forward)
            || !same_bits(reverse, &replayed.extents.reverse)
            || !same_floats(&memory, &replayed.memory)
        {
            return Err(mismatch(&ticket).into());
        }
        let body = problem.body;
        let scales = resolution.scales();
        let sizes = &problem.parameters.sizes;
        let start = problem.accepted.state().coordinates();
        let mut stage_defect = Worst::NONE;
        for ((stage, seen), evaluation) in stages.iter().zip(&witness.stages).zip(&evaluations) {
            let from = stage
                .from
                .map_or(&start, |earlier| &witness.stages[earlier].unknowns);
            let slope = &witness.stages[stage.slope].derivative;
            for (at, row) in body.rows().iter().enumerate() {
                let value = seen.unknowns[at];
                let residual = evaluation.residuals[at].abs();
                let defect = match row.kind() {
                    RowKind::Algebraic(_) => residual,
                    RowKind::Amount { .. } | RowKind::Memory(_) => {
                        let reached = from[at] + seconds(stage.span_ns) * slope[at];
                        let position = (value - reached).abs();
                        let rate = seconds(ticket.length_ns()) * residual;
                        if position > rate { position } else { rate }
                    }
                };
                stage_defect.note(defect / scales.at(value, sizes[at]), row.id());
            }
        }
        if let Some(row) = stage_defect.above_one() {
            return Err(ticket.refuse(NumericalRefusal::StageResidual { row }));
        }
        if let Some((fine, coarse)) = scheme.estimate(ticket.length_ns()) {
            let (fine, coarse) = (&witness.stages[fine].unknowns, &witness.stages[coarse].unknowns);
            let mut error = Worst::NONE;
            for (at, row) in body.rows().iter().enumerate() {
                error.note(
                    (fine[at] - coarse[at]).abs() / scales.at(fine[at], sizes[at]),
                    row.id(),
                );
            }
            if let Some(row) = error.above_one() {
                return Err(ticket.refuse(NumericalRefusal::Accuracy { row }));
            }
        }
        if ticket.after_breakpoint() && history.started_at != ticket.interval().start() {
            return Err(mismatch(&ticket).into());
        }
        if history.schema != problem.accepted.method_history().schema {
            return Err(ticket.mismatch().into());
        }
        let base = problem.accepted.state();
        let state = next_state(body, base, &extents, &memory, algebraic);
        let coordinates = state.coordinates();
        let tangent = vec![0.0; coordinates.len()];
        let point = StagePoint {
            layout: problem.layout(),
            time: ticket.at(ticket.length_ns())?,
            unknowns: &coordinates,
            derivative: &tangent,
        };
        let published = problem.evaluate(&point)?;
        let mut algebraic_defect = Worst::NONE;
        for (at, row) in body.rows().iter().enumerate() {
            if matches!(row.kind(), RowKind::Algebraic(_)) {
                let normalized = published.residuals[at].abs() / scales.at(coordinates[at], sizes[at]);
                algebraic_defect.note(normalized, row.id());
            }
        }
        if let Some(row) = algebraic_defect.above_one() {
            return Err(ticket.refuse(NumericalRefusal::StageResidual { row }));
        }
        let end = ticket.interval().end();
        Ok(VerifiedStep {
            excursions: body.excursions(&published.values, &coordinates, end),
            dense: DenseSegment {
                interval: Some(ticket.interval()),
                start: base.values(),
                end: state.values(),
            },
            fired: Vec::new(),
            state,
            continuation: history,
            forward: extents.forward,
            reverse: extents.reverse,
            ticket,
        })
    }
}

fn same_structure(stages: &[Stage], witness: &StageWitness) -> bool {
    stages.len() == witness.stages.len()
        && (stages.iter().zip(&witness.stages))
            .all(|(stage, seen)| stage.offset_ns == seen.offset_ns && stage.weight == seen.weight)
}

fn same_bits(a: &[Compensated], b: &[Compensated]) -> bool {
    a.iter()
        .map(|value| value.bits())
        .eq(b.iter().map(|value| value.bits()))
}

fn same_floats(a: &[f64], b: &[f64]) -> bool {
    a.iter()
        .map(|value| value.to_bits())
        .eq(b.iter().map(|value| value.to_bits()))
}

fn next_state(
    body: &CompiledBody,
    base: &Candidate,
    extents: &Extents,
    memory: &[f64],
    algebraic: Vec<f64>,
) -> Candidate {
    let mut amounts = base.amounts.clone();
    let gross = extents.forward.iter().zip(&extents.reverse);
    for (column, (&forward, &reverse)) in body.columns().iter().zip(gross) {
        let net = forward.minus(reverse);
        for &(row, coefficient) in column.terms() {
            amounts[row.index()] = amounts[row.index()].plus(net.scaled(coefficient));
        }
    }
    let memory = (base.memory.iter().zip(memory))
        .map(|(start, increment)| start + increment)
        .collect();
    Candidate {
        amounts,
        memory,
        algebraic,
        modes: base.modes.clone(),
        ordinals: base.ordinals.clone(),
        fired: false,
    }
}

/// A step that followed its declared method. Everything in it derives from the record's own problem; the
/// record still decides whether it may publish.
pub struct VerifiedStep {
    ticket: StepTicket,
    state: Candidate,
    continuation: MethodHistory,
    dense: DenseSegment,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
    excursions: Vec<ValidityExcursion>,
    fired: Vec<Fired>,
}
pub struct VerifiedParts {
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub excursions: Vec<ValidityExcursion>,
    pub fired: Vec<Fired>,
}
impl VerifiedStep {
    pub fn ticket(&self) -> &StepTicket {
        &self.ticket
    }
    pub fn value(&self, row: RowId) -> Compensated {
        self.state.value(row)
    }
    /// Each column's gross forward and reverse extent, in `CompiledBody::columns` order.
    pub fn gross(&self) -> impl Iterator<Item = (Compensated, Compensated)> + '_ {
        self.forward.iter().copied().zip(self.reverse.iter().copied())
    }
    /// The inputs this step applies, each once, in `(time, id)` order.
    pub fn applied(&self) -> &[InputId] {
        &self.ticket.due
    }
    pub(crate) fn crossed<'b>(&self, problem: &CoupledProblem<'b>) -> Option<&'b HazardLayout> {
        problem.chance.crossed(problem.body, &self.state)
    }
    pub(crate) fn gap(&self, problem: &CoupledProblem<'_>, guard: &HazardLayout) -> f64 {
        problem.chance.gap(&self.state, guard)
    }
    /// The dense segment keeps the left limit, so a chord never runs through a reset and the next segment
    /// starts at the reset. A firing moves a mode, a draw and an integral, so what it leaves is read again: a law
    /// with no value there refuses the trial, and a range is judged on both sides of the instant.
    pub(crate) fn fire(self, problem: &CoupledProblem<'_>) -> Result<Self, TrialFailure> {
        let at = self.ticket.interval().end();
        let (state, fired) = problem.chance.fired(problem.body, self.state, at);
        let values = problem.values_after_firing(&self.ticket, &state)?;
        let mut excursions = self.excursions;
        excursions.extend(problem.body.excursions(&values, &state.coordinates(), at));
        Ok(Self {
            state,
            fired,
            excursions,
            ..self
        })
    }
    pub fn into_parts(self) -> VerifiedParts {
        let Self {
            ticket: _,
            state,
            continuation,
            dense,
            forward: _,
            reverse: _,
            excursions,
            fired,
        } = self;
        VerifiedParts {
            state,
            continuation,
            dense,
            excursions,
            fired,
        }
    }
}

/// A chance event is no input: replay recreates it from the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fired {
    pub(crate) stream: LawName,
    pub(crate) at: SimTime,
    pub(crate) ordinal: u64,
    pub(crate) threshold_bits: u64,
    pub(crate) entered: ModeValue,
}
impl Fired {
    pub fn stream(&self) -> LawName {
        self.stream
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }
    pub fn threshold(&self) -> Compensated {
        Compensated::exact(f64::from_bits(self.threshold_bits))
    }
    pub fn entered(&self) -> ModeValue {
        self.entered
    }
}
