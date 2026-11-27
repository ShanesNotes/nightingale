use crate::body::RowKind;
use crate::chance::Trigger;
use crate::problem::{ProblemSeal, SolveWorkspace, StagePoint};
use crate::scheme::{Estimate, Stage};
use crate::{
    Candidate, Compensated, CompiledBody, CoupledProblem, DenseSegment, Digest, EngineFault, EngineFaultKind,
    ErrorScales, InputId, LawName, MethodHistory, ModeValue, NumericalRefusal, Ratio, RhythmId, RowId,
    SimTime, StageSet, StepTicket, TrialFailure, ValidityExcursion,
};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

/// One stage's evaluation: bound to its ticket and layout and carrying a copy of its point, so it outlives
/// the workspace. Only `CoupledProblem::evaluate` makes one.
pub struct StageEvaluation {
    pub(crate) layout: Digest,
    pub(crate) ticket: Digest,
    pub(crate) offset_ns: u64,
    pub(crate) provenance: Option<ProblemSeal>,
    pub(crate) unknowns: Vec<f64>,
    pub(crate) derivative: Vec<f64>,
    pub(crate) columns: Vec<f64>,
    pub(crate) rows: Vec<f64>,
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
    provenance: Option<ProblemSeal>,
    mismatch: EngineFault,
    length_ns: u64,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
    first_memory: usize,
    memory: Vec<f64>,
    total: Ratio,
    accumulated: bool,
    stages: Vec<WitnessStage>,
}
impl Quadrature {
    pub(crate) fn accumulate(
        &mut self,
        columns: &[f64],
        rows: &[f64],
        weight: Ratio,
    ) -> Result<(), EngineFault> {
        self.total = self.total.plus(weight).ok_or(self.mismatch)?;
        let seconds = weight.seconds_of(self.length_ns);
        let first = !self.accumulated;
        self.accumulated = true;
        if first {
            for (column, &rate) in columns.iter().enumerate() {
                if rate == 0.0 {
                    continue;
                }
                let extent = rate.abs() * seconds;
                if extent == 0.0 {
                    continue;
                }
                let scaled = Compensated::exact(extent);
                if rate < 0.0 {
                    self.reverse[column] = scaled;
                } else {
                    self.forward[column] = scaled;
                }
            }
        } else {
            for (column, &rate) in columns.iter().enumerate() {
                if rate == 0.0 {
                    continue;
                }
                let side = if rate < 0.0 {
                    &mut self.reverse[column]
                } else {
                    &mut self.forward[column]
                };
                *side = side.plus(Compensated::exact(rate.abs() * seconds));
            }
        }
        let rates = &rows[self.first_memory..];
        for (sum, &rate) in self.memory.iter_mut().zip(rates) {
            *sum += rate * seconds;
        }
        Ok(())
    }
    /// Adds the weighted rates and keeps the point and weight as a witness stage. `weight` is an exact
    /// fraction of the ticketed interval. An evaluation from another ticket or layout, as from an earlier
    /// attempt, and weights whose sum does not fit in lowest terms are `BindingMismatch`.
    pub fn add(&mut self, evaluation: &StageEvaluation, weight: Ratio) -> Result<(), EngineFault> {
        if evaluation.ticket != self.ticket || evaluation.layout != self.layout {
            return Err(self.mismatch);
        }
        self.accumulate(&evaluation.columns, &evaluation.rows, weight)?;
        if evaluation.provenance != self.provenance {
            self.provenance = None;
        }
        let residuals = self.provenance.is_some().then(|| evaluation.residuals.clone());
        self.stages.push(WitnessStage {
            offset_ns: evaluation.offset_ns,
            weight,
            unknowns: evaluation.unknowns.clone(),
            derivative: evaluation.derivative.clone(),
            residuals,
        });
        Ok(())
    }
    /// Same as `add`, moving the evaluation's witness point buffers instead of cloning them.
    pub fn add_owned(&mut self, evaluation: StageEvaluation, weight: Ratio) -> Result<(), EngineFault> {
        if evaluation.ticket != self.ticket || evaluation.layout != self.layout {
            return Err(self.mismatch);
        }
        self.accumulate(&evaluation.columns, &evaluation.rows, weight)?;
        if evaluation.provenance != self.provenance {
            self.provenance = None;
        }
        let residuals = self.provenance.is_some().then_some(evaluation.residuals);
        self.stages.push(WitnessStage {
            offset_ns: evaluation.offset_ns,
            weight,
            unknowns: evaluation.unknowns,
            derivative: evaluation.derivative,
            residuals,
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
            provenance,
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
            witness: StageWitness { provenance, stages },
        })
    }
}

pub struct Increment {
    ticket: Digest,
    extents: Extents,
    memory: Vec<f64>,
    witness: StageWitness,
}
pub struct Extents {
    layout: Digest,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
}
struct StageWitness {
    provenance: Option<ProblemSeal>,
    stages: Vec<WitnessStage>,
}
struct WitnessStage {
    offset_ns: u64,
    weight: Ratio,
    unknowns: Vec<f64>,
    derivative: Vec<f64>,
    residuals: Option<Vec<f64>>,
}
impl StageWitness {
    fn error(
        &self,
        estimate: Estimate,
        start: &[f64],
        scales: &ErrorScales,
        sizes: &[f64],
        body: &CompiledBody,
    ) -> Worst {
        let mut error = Worst::NONE;
        match estimate {
            Estimate::Difference { fine, coarse } => {
                let (fine, coarse) = (&self.stages[fine].unknowns, &self.stages[coarse].unknowns);
                for (at, row) in body.rows().iter().enumerate() {
                    error.note(
                        (fine[at] - coarse[at]).abs() / scales.at(fine[at], sizes[at]),
                        row.id(),
                    );
                }
            }
            Estimate::SecondDifference { half, end } => {
                let (early, late) = (&self.stages[half], &self.stages[end]);
                let first = early.offset_ns as f64;
                let rest = (late.offset_ns - early.offset_ns) as f64;
                let reach = late.offset_ns as f64 / 2.0;
                for (at, row) in body.rows().iter().enumerate() {
                    let (from, middle, to) = (start[at], early.unknowns[at], late.unknowns[at]);
                    let curvature = reach * ((to - middle) / rest - (middle - from) / first);
                    error.note(curvature.abs() / scales.at(to, sizes[at]), row.id());
                }
            }
        }
        error
    }
}

impl CoupledProblem<'_> {
    pub fn quadrature(&self, ticket: &StepTicket) -> Quadrature {
        let columns = self.body.columns().len();
        Quadrature {
            layout: self.layout(),
            ticket: ticket.digest(),
            provenance: self.provenance,
            mismatch: ticket.mismatch(),
            length_ns: ticket.length_ns(),
            forward: vec![Compensated::ZERO; columns],
            reverse: vec![Compensated::ZERO; columns],
            first_memory: self.first_memory,
            memory: vec![0.0; self.first_algebraic - self.first_memory],
            total: Ratio::ZERO,
            accumulated: false,
            stages: Vec::new(),
        }
    }
    /// The stages `ticket` must show under this run's scheme, which `StepProposal::verify` checks.
    pub fn stage_set(&self, ticket: &StepTicket) -> StageSet {
        self.identity.resolution().scheme().set(ticket)
    }
    /// The record's error estimate of a core's own increment, by `verify`'s arithmetic: the worst row's error over
    /// its declared tolerance, and zero for a set that reads none. Above one it is the refusal, or the fault at the
    /// floor, that `verify` gives. Another ticket's or layout's increment is `BindingMismatch`, and stages off the
    /// ticket's set are `MethodMismatch`.
    pub fn estimate(&self, ticket: &StepTicket, increment: &Increment) -> Result<f64, TrialFailure> {
        if increment.ticket != ticket.digest() || increment.extents.layout != self.layout() {
            return Err(ticket.mismatch().into());
        }
        let set = self.stage_set(ticket);
        if !same_structure(&set.stages(ticket.length_ns()), &increment.witness) {
            return Err(method_mismatch(ticket).into());
        }
        let Some(estimate) = set.estimate() else {
            return Ok(0.0);
        };
        let start = &self.coordinates;
        let (scales, sizes) = (self.identity.resolution().scales(), &self.parameters.sizes);
        let worst = increment.witness.error(estimate, start, scales, sizes, self.body);
        if let Some(row) = worst.above_one() {
            return Err(ticket.refuse(NumericalRefusal::Accuracy { row }));
        }
        Ok(worst.normalized)
    }
    /// `verify` takes amounts and memory from the start plus the increment, and the dense segment from the
    /// ends, so `end` supplies only algebraic values.
    pub fn propose(
        &self,
        ticket: &StepTicket,
        increment: Increment,
        end: &SolveWorkspace,
        history: MethodHistory,
    ) -> StepProposal {
        let coordinates = end.coordinates();
        let algebraic = if coordinates.len() == self.body.rows().len() {
            coordinates[self.first_algebraic..].to_vec()
        } else {
            (self.body.rows().iter().zip(coordinates))
                .filter(|(row, _)| matches!(row.kind(), RowKind::Algebraic(_)))
                .map(|(_, &value)| value)
                .collect()
        };
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
    /// every value read here comes from it, directly when attested to that head or re-evaluated otherwise,
    /// none from the core. A refusal is an honest attempt a smaller step can fix; a fault is a lie about the
    /// method or a binding that no smaller step can.
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
        let algebraic_rows = problem.body.rows().len() - problem.first_algebraic;
        if layout != problem.layout()
            || extents.layout != problem.layout()
            || finished_for != ticket.digest()
            || algebraic.len() != algebraic_rows
        {
            return Err(ticket.mismatch().into());
        }
        let resolution = problem.identity.resolution();
        let set = problem.stage_set(&ticket);
        let stages = set.stages(ticket.length_ns());
        if !same_structure(&stages, &witness) {
            return Err(method_mismatch(&ticket).into());
        }
        let body = problem.body;
        let scales = resolution.scales();
        let sizes = &problem.parameters.sizes;
        let start = &problem.coordinates;
        let mut stage_defect = Worst::NONE;
        let attested = problem.provenance.is_some() && witness.provenance == problem.provenance;
        if attested {
            for (stage, seen) in stages.iter().zip(&witness.stages) {
                let residuals = seen.residuals.as_deref().expect("attested stage holds residuals");
                note_stage_defect(
                    problem,
                    ticket.length_ns(),
                    stage,
                    seen,
                    &witness.stages,
                    residuals,
                    &mut stage_defect,
                );
            }
        } else {
            let mut replay = problem.quadrature(&ticket);
            for (stage, seen) in stages.iter().zip(&witness.stages) {
                let point = StagePoint {
                    layout: problem.layout(),
                    time: ticket.at(seen.offset_ns)?,
                    unknowns: &seen.unknowns,
                    derivative: &seen.derivative,
                };
                let raw = problem.evaluate_raw(&point)?;
                replay.accumulate(&raw.columns, &raw.rows, seen.weight)?;
                note_stage_defect(
                    problem,
                    ticket.length_ns(),
                    stage,
                    seen,
                    &witness.stages,
                    &raw.residuals,
                    &mut stage_defect,
                );
            }
            let replayed = replay.finish()?;
            let (forward, reverse) = (&extents.forward, &extents.reverse);
            if !same_bits(forward, &replayed.extents.forward)
                || !same_bits(reverse, &replayed.extents.reverse)
                || !same_floats(&memory, &replayed.memory)
            {
                return Err(method_mismatch(&ticket).into());
            }
        }
        if let Some(row) = stage_defect.above_one() {
            return Err(ticket.refuse(NumericalRefusal::StageResidual { row }));
        }
        if let Some(estimate) = set.estimate()
            && let Some(row) = witness.error(estimate, start, scales, sizes, body).above_one()
        {
            return Err(ticket.refuse(NumericalRefusal::Accuracy { row }));
        }
        if ticket.after_breakpoint() && history.started_at != ticket.interval().start() {
            return Err(method_mismatch(&ticket).into());
        }
        if history.schema != problem.accepted.method_history().schema {
            return Err(ticket.mismatch().into());
        }
        let base = problem.accepted.state();
        let state = next_state(body, base, &extents, &memory, algebraic);
        let coordinates = state.coordinates();
        // A crossing and a crossing back inside one step leave no mark at its end, so every stage before the end is read.
        // At the floor no shorter step can look again, so the declared resolution is the limit.
        let before_the_end = (witness.stages.iter())
            .filter(|stage| stage.offset_ns < ticket.length_ns())
            .map(|stage| (stage.unknowns.as_slice(), stage.offset_ns));
        if ticket.length_ns() > ticket.floor_ns.get()
            && problem.crossed_inside(&ticket, before_the_end, &state, &coordinates)?
        {
            return Err(ticket.refuse(NumericalRefusal::GuardInside));
        }
        let published = problem.evaluate_published(&ticket, &coordinates)?;
        if algebraic_rows > 0 {
            let first_algebraic = problem.first_algebraic;
            let mut algebraic_defect = Worst::NONE;
            for (slot, (at, row)) in body.rows().iter().enumerate().skip(first_algebraic).enumerate() {
                let judge = body.judged_at(at);
                let normalized =
                    published.algebraic[slot].abs() / scales.at(coordinates[judge], sizes[judge]);
                algebraic_defect.note(normalized, row.id());
            }
            if let Some(row) = algebraic_defect.above_one() {
                return Err(ticket.refuse(NumericalRefusal::StageResidual { row }));
            }
        }
        Ok(VerifiedStep {
            excursions: published.excursions,
            dense: DenseSegment {
                interval: Some(ticket.interval()),
                start: base.values(),
                end: state.values(),
            },
            fired: Vec::new(),
            rhythm_edges: Vec::new(),
            state,
            coordinates,
            continuation: history,
            forward: extents.forward,
            reverse: extents.reverse,
            ticket,
        })
    }
}

fn note_stage_defect(
    problem: &CoupledProblem<'_>,
    ticket_ns: u64,
    stage: &Stage,
    seen: &WitnessStage,
    stages: &[WitnessStage],
    residuals: &[f64],
    stage_defect: &mut Worst,
) {
    let body = problem.body;
    let scales = problem.identity.resolution().scales();
    let sizes = &problem.parameters.sizes;
    let start = problem.coordinates.as_slice();
    let from = stage
        .from
        .map_or(start, |earlier| stages[earlier].unknowns.as_slice());
    let slope = &stages[stage.slope].derivative;
    let step_seconds = seconds(ticket_ns);
    let span_seconds = seconds(stage.span_ns);
    for (at, row) in body.rows().iter().enumerate() {
        let value = seen.unknowns[at];
        let residual = residuals[at].abs();
        let defect = match row.kind() {
            RowKind::Algebraic(_) => residual,
            RowKind::Amount { .. } | RowKind::Memory(_) => {
                let reached = from[at] + span_seconds * slope[at];
                let position = (value - reached).abs();
                let rate = step_seconds * residual;
                if position > rate { position } else { rate }
            }
        };
        let judge = body.judged_at(at);
        let scale = if judge == at {
            scales.at(value, sizes[at])
        } else {
            scales.at(seen.unknowns[judge], sizes[judge])
        };
        stage_defect.note(defect / scale, row.id());
    }
}

fn method_mismatch(ticket: &StepTicket) -> EngineFault {
    EngineFault {
        kind: EngineFaultKind::MethodMismatch,
        ..ticket.mismatch()
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
    const NEG_ZERO: u64 = (-0.0f64).to_bits();
    let mut amounts = base.amounts.clone();
    let gross = extents.forward.iter().zip(&extents.reverse);
    for (column, (&forward, &reverse)) in body.columns().iter().zip(gross) {
        if forward.bits() == [0, 0] && reverse.bits() == [0, 0] {
            for &(row, _) in column.terms() {
                let amount = amounts[row.index()];
                let bits = amount.bits();
                if bits[0] == NEG_ZERO || bits[1] == NEG_ZERO || !amount.is_finite() {
                    amounts[row.index()] = amount.plus(Compensated::ZERO);
                }
            }
            continue;
        }
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
    coordinates: Vec<f64>,
    continuation: MethodHistory,
    dense: DenseSegment,
    forward: Vec<Compensated>,
    reverse: Vec<Compensated>,
    excursions: Vec<ValidityExcursion>,
    fired: Vec<Fired>,
    rhythm_edges: Vec<RhythmId>,
}
pub struct VerifiedParts {
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub excursions: Vec<ValidityExcursion>,
    pub fired: Vec<Fired>,
    pub rhythm_edges: Vec<RhythmId>,
}
impl VerifiedStep {
    pub fn ticket(&self) -> &StepTicket {
        &self.ticket
    }
    pub(crate) fn state(&self) -> &Candidate {
        &self.state
    }
    pub(crate) fn coordinates(&self) -> &[f64] {
        &self.coordinates
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
    pub(crate) fn gap(&self, problem: &CoupledProblem<'_>, guard: Trigger<'_>) -> Result<f64, TrialFailure> {
        problem.gap(&self.ticket, &self.state, &self.coordinates, guard)
    }
    /// The dense segment keeps the left limit, so a chord never runs through a reset and the next segment
    /// starts at the reset. A firing moves a mode, a draw and an integral, so what it leaves is read again: a law
    /// with no value there refuses the trial, and a range is judged on both sides of the instant.
    pub(crate) fn fire(self, problem: &CoupledProblem<'_>) -> Result<Self, TrialFailure> {
        let at = self.ticket.interval().end();
        let (state, fired, rhythm_edges) = problem.fired(&self.ticket, self.state, &self.coordinates, at)?;
        let coordinates = state.coordinates();
        let values = problem.values_after_firing(&self.ticket, &state, &coordinates)?;
        let mut excursions = self.excursions;
        excursions.extend(problem.body.excursions(&values, &coordinates, at));
        Ok(Self {
            state,
            coordinates,
            fired,
            rhythm_edges,
            excursions,
            ..self
        })
    }
    pub fn into_parts(self) -> VerifiedParts {
        let Self {
            ticket: _,
            state,
            coordinates: _,
            continuation,
            dense,
            forward: _,
            reverse: _,
            excursions,
            fired,
            rhythm_edges,
        } = self;
        VerifiedParts {
            state,
            continuation,
            dense,
            excursions,
            fired,
            rhythm_edges,
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

#[cfg(test)]
mod tests {
    use super::{StageWitness, WitnessStage};
    use crate::scheme::Estimate;
    use crate::*;

    fn measure<U>(value: f64) -> Measure<U> {
        Measure::new(value).expect("finite")
    }

    fn one_row() -> CompiledBody {
        let mut map = AnatomyBuilder::new();
        map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
        let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        assemble(schema, |mut body| {
            let (mut laws, _) = body.nervous()?.parts();
            let x = laws.state::<Dimensionless>("x", Admissible::finite());
            let speed =
                laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
            let class = RateClass::new(RateBand::Minutes, 1_000_000_000)?;
            let rate = laws.kernel(|mut k| {
                let speed = k.parameter(&speed);
                let x = k.state(&x);
                let rate = k.scale(speed, x);
                k.finish(rate, laws.every_mode(), class)
            });
            laws.memory_rate(&x, rate);
            body.add(laws.finish());
            body.compile()
        })
        .expect("compiled")
    }

    fn witnessed(stages: &[(u64, f64)]) -> StageWitness {
        let stage = |&(offset_ns, value): &(u64, f64)| WitnessStage {
            offset_ns,
            weight: Ratio::ZERO,
            unknowns: vec![value],
            derivative: vec![0.0],
            residuals: None,
        };
        StageWitness {
            provenance: None,
            stages: stages.iter().map(stage).collect(),
        }
    }

    #[test]
    fn each_estimate_reads_its_own_stages_and_scales_at_its_end() {
        let body = one_row();
        let row = Some(body.rows()[0].id());
        let scales = ErrorScales::uniform(measure(1.0), measure(1.0)).expect("positive");
        let error = |stages: &[(u64, f64)], estimate| {
            let worst = witnessed(stages).error(estimate, &[1.0], &scales, &[1.0], &body);
            (worst.normalized, worst.row)
        };
        let second = Estimate::SecondDifference { half: 0, end: 1 };
        assert_eq!(
            error(&[(1, 3.0), (2, 7.0)], second),
            (0.25, row),
            "|7 - 2 * 3 + 1| over 1 + 7"
        );
        assert_eq!(
            error(&[(1, 7.0), (2, 3.0)], second),
            (2.5, row),
            "|3 - 2 * 7 + 1| over 1 + 3"
        );
        let difference = Estimate::Difference { fine: 2, coarse: 0 };
        assert_eq!(
            error(&[(2, 2.0), (1, 3.0), (2, 7.0)], difference),
            (0.625, row),
            "|7 - 2| over 1 + 7"
        );
    }

    #[test]
    fn the_second_difference_weighs_an_odd_tickets_unequal_halves() {
        let body = one_row();
        let scales = ErrorScales::uniform(measure(1.0), measure(1.0)).expect("positive");
        let second = Estimate::SecondDifference { half: 0, end: 1 };
        let error = |start: f64, stages: &[(u64, f64)]| {
            let worst = witnessed(stages).error(second, &[start], &scales, &[1.0], &body);
            (worst.normalized, worst.row)
        };
        let affine = error(-100_001.0, &[(50_000, -50_001.0), (100_001, 0.0)]);
        assert_eq!(
            affine,
            (0.0, None),
            "one a nanosecond over 100,001 ns reads no error"
        );
        let row = Some(body.rows()[0].id());
        let parabola = error(0.0, &[(3, 9.0), (7, 49.0)]);
        assert_eq!(
            parabola,
            (0.49, row),
            "t^2 over 7 ns reads h^2 y'' / 4, 24.5, over 1 + 49"
        );
    }

    fn admitted_one_row(
        body: &CompiledBody,
        speed: f64,
        start_x: f64,
        scheme: StageScheme,
    ) -> (AcceptedHead, RunIdentity) {
        use core::num::NonZeroU64;
        let mut packet = ParameterPacket::new();
        packet
            .synthetic(
                ParameterKey::new("ns.speed"),
                Binding::Body,
                measure::<Rate<Dimensionless>>(speed),
                "synthetic",
            )
            .expect("speed");
        packet.start_rest(body, |_| start_x, "synthetic").expect("start");
        let bound = body.bind(packet).expect("bound");
        let system = body
            .initialization(
                bound,
                BoundaryPort::new(Vec::new()).expect("port"),
                InitializationRequest::at(SimTime::from_ns(0)),
            )
            .expect("system");
        let solved = system.workspace();
        let history = MethodHistory::new(Digest::of_canonical(b"test"), SimTime::from_ns(0), Vec::new());
        let origin = system.admit(solved, history).expect("admitted");
        let scales = ErrorScales::uniform(measure(1.0), measure(1e-3)).expect("scales");
        let resolution = DeclaredResolution::new(
            Digest::of_canonical(b"test core"),
            NonZeroU64::new(100_000).expect("nonzero"),
            4,
            ArithmeticProfile::contract_v1(),
            scheme,
            scales,
        );
        let identity = RunIdentity::new(
            body.model_hash(),
            origin.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution,
        );
        (AcceptedHead::admitted(origin, RecordHash::Blake2b), identity)
    }

    #[test]
    fn stage_jacobian_reevaluate_and_add_owned_match_their_owning_twins() {
        let body = one_row();
        let (head, identity) = admitted_one_row(&body, 2.0, 1.0, StageScheme::forward_euler());
        let base = Digest::of_canonical(b"base");
        let span_ns = 10_000_000;
        let _ = head
            .attempt(
                &body,
                &identity,
                base,
                Attempt::First { span_ns },
                |problem, ticket| {
                    let time = ticket.at(0)?;
                    let shift = ticket.shift(span_ns)?;
                    let mut workspace = problem.workspace();
                    workspace.unknowns_mut()[0] = 1.02;
                    workspace.derivative_mut()[0] = 2.0;
                    let point = workspace.point(time);
                    let owned = problem.linearize(&point, shift)?;
                    let borrowed = problem.linearize_stage(&point, shift)?;
                    assert_eq!(owned.view().row_offsets(), borrowed.view().row_offsets());
                    assert_eq!(owned.view().columns(), borrowed.view().columns());
                    assert_eq!(
                        owned
                            .view()
                            .values()
                            .iter()
                            .map(|v| v.to_bits())
                            .collect::<Vec<_>>(),
                        borrowed
                            .view()
                            .values()
                            .iter()
                            .map(|v| v.to_bits())
                            .collect::<Vec<_>>()
                    );
                    let mut a = vec![3.0];
                    let mut b = vec![3.0];
                    body.factor(&owned).expect("factor").solve(&mut a);
                    borrowed.factor().expect("factor").solve(&mut b);
                    assert_eq!(a[0].to_bits(), b[0].to_bits());

                    let mut eval = problem.evaluate(&point)?;
                    let before_bits = eval.residuals()[0].to_bits();
                    workspace.unknowns_mut()[0] = f64::NAN;
                    assert!(problem.reevaluate(&workspace.point(time), &mut eval).is_err());
                    assert_eq!(eval.residuals()[0].to_bits(), before_bits);

                    workspace.unknowns_mut()[0] = 1.0;
                    workspace.derivative_mut()[0] = 2.0;
                    let point = workspace.point(time);
                    let fresh = problem.evaluate(&point)?;
                    problem.reevaluate(&point, &mut eval)?;
                    assert_eq!(fresh.residuals()[0].to_bits(), eval.residuals()[0].to_bits());

                    let mut by_ref = problem.quadrature(ticket);
                    by_ref.add(&fresh, Ratio::ONE)?;
                    let inc_ref = by_ref.finish()?;
                    let mut by_val = problem.quadrature(ticket);
                    by_val.add_owned(eval, Ratio::ONE)?;
                    let inc_val = by_val.finish()?;
                    assert_eq!(inc_ref.memory[0].to_bits(), inc_val.memory[0].to_bits());
                    assert_eq!(
                        inc_ref.witness.stages[0].unknowns[0].to_bits(),
                        inc_val.witness.stages[0].unknowns[0].to_bits()
                    );
                    let history = MethodHistory::new(
                        problem.accepted().method_history().schema(),
                        SimTime::from_ns(0),
                        Vec::new(),
                    );
                    Ok(problem.propose(ticket, inc_val, &workspace, history))
                },
                |_| Ok(()),
            )
            .expect("step passes");
    }

    #[test]
    fn mixing_an_unattested_stage_replays_and_catches_other_values() {
        let body = one_row();
        let (head, identity) = admitted_one_row(&body, 2.0, 1.0, StageScheme::backward_euler_halved());
        let (other_head, other_identity) =
            admitted_one_row(&body, 5.0, 1.0, StageScheme::backward_euler_halved());
        let base = Digest::of_canonical(b"base");
        let span_ns = 10_000_000;
        let outcome = head.attempt(
            &body,
            &identity,
            base,
            Attempt::First { span_ns },
            |problem, ticket| {
                let half = span_ns / 2;
                let rest = span_ns - half;
                let other_problem = CoupledProblem::new(
                    &body,
                    other_head.parts().parameters(),
                    &other_identity,
                    other_head.view(),
                    base,
                );
                let mut ws = problem.workspace();
                let x1 = 1.0 / (1.0 - 2.0 * (half as f64 / 1e9));
                ws.unknowns_mut()[0] = x1;
                ws.derivative_mut()[0] = (x1 - 1.0) / (half as f64 / 1e9);
                let early = problem.evaluate(&ws.point(ticket.at(half)?))?;
                let x2 = x1 / (1.0 - 5.0 * (rest as f64 / 1e9));
                ws.unknowns_mut()[0] = x2;
                ws.derivative_mut()[0] = (x2 - x1) / (rest as f64 / 1e9);
                let forged_late = other_problem.evaluate(&ws.point(ticket.at(span_ns)?))?;
                let mut q = problem.quadrature(ticket);
                q.add_owned(early, ticket.fraction(half))?;
                q.add_owned(forged_late, ticket.fraction(rest))?;
                let inc = q.finish()?;
                assert!(inc.witness.provenance.is_none());
                let history = MethodHistory::new(
                    problem.accepted().method_history().schema(),
                    SimTime::from_ns(0),
                    Vec::new(),
                );
                Ok(problem.propose(ticket, inc, &ws, history))
            },
            |_| Ok(()),
        );
        match outcome {
            Err(TrialFailure::EngineFault(fault)) => {
                assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch)
            }
            other => panic!("expected MethodMismatch, got {:?}", other.err()),
        }
    }

    #[test]
    fn cross_head_install_keeps_receiving_heads_parameter_digest() {
        let body = one_row();
        let (mut head, identity) = admitted_one_row(&body, 2.0, 1.0, StageScheme::forward_euler());
        let (mut other_head, other_identity) =
            admitted_one_row(&body, 5.0, 1.0, StageScheme::forward_euler());
        let base = Digest::of_canonical(b"base");
        let span_ns = 10_000_000;
        let step_other = |h: &AcceptedHead| {
            h.attempt(
                &body,
                &other_identity,
                base,
                Attempt::First { span_ns },
                |problem, ticket| {
                    let time = ticket.at(0)?;
                    let mut ws = problem.workspace();
                    ws.unknowns_mut()[0] = 1.0;
                    ws.derivative_mut()[0] = 5.0;
                    let eval = problem.evaluate(&ws.point(time))?;
                    let mut q = problem.quadrature(ticket);
                    q.add_owned(eval, Ratio::ONE)?;
                    let inc = q.finish()?;
                    let history = MethodHistory::new(
                        problem.accepted().method_history().schema(),
                        SimTime::from_ns(0),
                        Vec::new(),
                    );
                    Ok(problem.propose(ticket, inc, &ws, history))
                },
                |_| Ok(()),
            )
            .expect("other step passes")
            .0
        };
        let next_for_head = step_other(&other_head);
        let next_for_other = step_other(&other_head);
        let _ = head.install(next_for_head);
        let _ = other_head.install(next_for_other);
        assert_eq!(head.digest().parameters, head.parts().parameters().digest());
        assert_ne!(head.digest().parameters, other_head.parts().parameters().digest());
        assert!(!head.digest().from_admission);
        assert_eq!(head.image_digest(), other_head.image_digest());
        let outcome = head.attempt(
            &body,
            &identity,
            base,
            Attempt::First { span_ns },
            |problem, ticket| {
                let time = ticket.at(0)?;
                let other_problem = CoupledProblem::for_head(
                    &body,
                    other_head.parts().parameters(),
                    &identity,
                    other_head.view(),
                    base,
                    other_head.digest(),
                );
                let mut ws = problem.workspace();
                ws.unknowns_mut()[0] = problem.accepted().state().memory[0];
                ws.derivative_mut()[0] = 5.0;
                let forged = other_problem.evaluate(&ws.point(time))?;
                let mut q = problem.quadrature(ticket);
                q.add_owned(forged, Ratio::ONE)?;
                let inc = q.finish()?;
                assert!(inc.witness.provenance.is_none());
                let history = MethodHistory::new(
                    problem.accepted().method_history().schema(),
                    SimTime::from_ns(0),
                    Vec::new(),
                );
                Ok(problem.propose(ticket, inc, &ws, history))
            },
            |_| Ok(()),
        );
        match outcome {
            Err(TrialFailure::EngineFault(fault)) => {
                assert_eq!(fault.kind(), EngineFaultKind::MethodMismatch)
            }
            other => panic!("expected MethodMismatch, got {:?}", other.err()),
        }
    }

    #[test]
    fn unattested_multi_stage_transfer_proposal_verifies_both_stages() {
        use core::num::NonZeroU64;
        let at = || Position::new(measure(0.0), measure(0.0), measure(0.0));
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at());
        let a = map.site::<VesselSegment>("artery", at());
        let b = map.site::<VesselSegment>("vein", at());
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.unsupplied(&[a.id(), b.id()]);
        map.connect(Network::Vascular, a.id(), b.id());
        map.connect(Network::Vascular, b.id(), a.id());
        let mut schema = Schema::new(map.finish().expect("anatomy")).expect("schema");
        schema.partial_volume(measure(0.018)).expect("partial volume");
        for space in [Space::Cells, Space::Interstitium] {
            schema.leave_out(space, "bench");
        }
        let body = assemble(schema, |mut body| {
            let class = RateClass::new(RateBand::Minutes, 1_000_000_000)?;
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let flow = cv.edge_parameter::<VascularEdge, Rate<Water>>(
                ParameterKey::new("cv.flow"),
                Admissible::finite(),
            );
            let circulation = ports.circulation(&mut cv);
            let law = cv.edge_kernel(|mut k| {
                let rate = k.edge_parameter(&flow);
                k.finish(rate, cv.every_mode(), class)
            });
            cv.transfer_along(&circulation, law);
            body.add(cv.finish());
            body.compile()
        })
        .expect("compiled");
        let mut packet = ParameterPacket::new();
        for (from, to, rate) in [(a.id(), b.id(), 2.0), (b.id(), a.id(), -0.5)] {
            packet
                .synthetic(
                    ParameterKey::new("cv.flow"),
                    Binding::Edge { from, to },
                    measure::<Rate<Water>>(rate),
                    "synthetic",
                )
                .expect("flow");
        }
        packet.start_rest(&body, |_| 10.0, "synthetic").expect("start");
        let bound = body.bind(packet).expect("bound");
        let system = body
            .initialization(
                bound,
                BoundaryPort::new(Vec::new()).expect("port"),
                InitializationRequest::at(SimTime::from_ns(0)),
            )
            .expect("system");
        let solved = system.workspace();
        let history = MethodHistory::new(Digest::of_canonical(b"test"), SimTime::from_ns(0), Vec::new());
        let origin = system.admit(solved, history).expect("admitted");
        let scales = ErrorScales::uniform(measure(1.0), measure(1e-3)).expect("scales");
        let resolution = DeclaredResolution::new(
            Digest::of_canonical(b"test core"),
            NonZeroU64::new(100_000).expect("nonzero"),
            4,
            ArithmeticProfile::contract_v1(),
            StageScheme::backward_euler_halved(),
            scales,
        );
        let identity = RunIdentity::new(
            body.model_hash(),
            origin.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution,
        );
        let mut head = AcceptedHead::admitted(origin, RecordHash::Blake2b);
        let base = Digest::of_canonical(b"base");
        let span_ns = 10_000_000;
        let half = span_ns / 2;
        let rest = span_ns - half;
        let a_row = body
            .rows()
            .iter()
            .position(|row| matches!(row.kind(), RowKind::Amount { site, .. } if site == a.id()))
            .expect("a row");
        let b_row = body
            .rows()
            .iter()
            .position(|row| matches!(row.kind(), RowKind::Amount { site, .. } if site == b.id()))
            .expect("b row");
        let (next_head, (forward, reverse)) = head
            .attempt(
                &body,
                &identity,
                base,
                Attempt::First { span_ns },
                |_problem, ticket| {
                    let unattested =
                        CoupledProblem::new(&body, head.parts().parameters(), &identity, head.view(), base);
                    let mut ws = unattested.workspace();
                    let dt = half as f64 / 1e9;
                    ws.unknowns_mut()[a_row] = 10.0 - 2.5 * dt;
                    ws.unknowns_mut()[b_row] = 10.0 + 2.5 * dt;
                    ws.derivative_mut()[a_row] = -2.5;
                    ws.derivative_mut()[b_row] = 2.5;
                    let s1 = unattested.evaluate(&ws.point(ticket.at(half)?))?;
                    ws.unknowns_mut()[a_row] = 10.0 - 2.5 * (2.0 * dt);
                    ws.unknowns_mut()[b_row] = 10.0 + 2.5 * (2.0 * dt);
                    let s2 = unattested.evaluate(&ws.point(ticket.at(span_ns)?))?;
                    let mut q = unattested.quadrature(ticket);
                    q.add_owned(s1, ticket.fraction(half))?;
                    q.add_owned(s2, ticket.fraction(rest))?;
                    let inc = q.finish()?;
                    assert!(inc.witness.provenance.is_none());
                    let history = MethodHistory::new(
                        unattested.accepted().method_history().schema(),
                        SimTime::from_ns(0),
                        Vec::new(),
                    );
                    Ok(unattested.propose(ticket, inc, &ws, history))
                },
                |step| Ok(step.gross().unzip::<_, _, Vec<_>, Vec<_>>()),
            )
            .expect("unattested 2-stage transfer proposal must verify");
        let _ = head.install(next_head);
        let amounts = &head.view().state().amounts;
        assert!((amounts[a_row].rounded() - (10.0 - 0.025)).abs() < 1e-15);
        assert!((amounts[b_row].rounded() - (10.0 + 0.025)).abs() < 1e-15);
        let total_forward: f64 = forward.iter().map(|v| v.rounded()).sum();
        let total_reverse: f64 = reverse.iter().map(|v| v.rounded()).sum();
        assert!((total_forward - 0.02).abs() < 1e-15);
        assert!((total_reverse - 0.005).abs() < 1e-15);
    }
}
