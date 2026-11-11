use crate::body::RowKind;
use crate::chance::Trigger;
use crate::problem::{SolveWorkspace, StagePoint};
use crate::scheme::{Estimate, Stage};
use crate::{
    Candidate, Compensated, CompiledBody, CoupledProblem, DenseSegment, Digest, EngineFault, EngineFaultKind,
    ErrorScales, InputId, LawName, MethodHistory, ModeValue, NumericalRefusal, Ratio, RowId, SimTime,
    StageSet, StepTicket, TrialFailure, ValidityExcursion,
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
        let start = self.accepted.state().coordinates();
        let (scales, sizes) = (self.identity.resolution().scales(), &self.parameters.sizes);
        let worst = increment
            .witness
            .error(estimate, &start, scales, sizes, self.body);
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
        let resolution = problem.identity.resolution();
        let set = problem.stage_set(&ticket);
        let stages = set.stages(ticket.length_ns());
        if !same_structure(&stages, &witness) {
            return Err(method_mismatch(&ticket).into());
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
            return Err(method_mismatch(&ticket).into());
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
                let judge = body.judged_at(at);
                let scale = if judge == at {
                    scales.at(value, sizes[at])
                } else {
                    scales.at(seen.unknowns[judge], sizes[judge])
                };
                stage_defect.note(defect / scale, row.id());
            }
        }
        if let Some(row) = stage_defect.above_one() {
            return Err(ticket.refuse(NumericalRefusal::StageResidual { row }));
        }
        if let Some(estimate) = set.estimate()
            && let Some(row) = witness.error(estimate, &start, scales, sizes, body).above_one()
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
        // A crossing and a crossing back inside one step leave no mark at its end, so every stage before the end is read.
        // At the floor no shorter step can look again, so the declared resolution is the limit.
        let before_the_end = (witness.stages.iter())
            .filter(|stage| stage.offset_ns < ticket.length_ns())
            .map(|stage| (stage.unknowns.as_slice(), stage.offset_ns));
        if ticket.length_ns() > ticket.floor_ns.get()
            && problem.crossed_inside(&ticket, before_the_end, &state)?
        {
            return Err(ticket.refuse(NumericalRefusal::GuardInside));
        }
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
                let judge = body.judged_at(at);
                let normalized = published.residuals[at].abs() / scales.at(coordinates[judge], sizes[judge]);
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
    /// The first guard in stable order this step's end has crossed, hazards first. The search reads only that guard's
    /// gap, and any crossing moves the bracket's top, so a landing closes on the earliest crossing of any guard.
    pub(crate) fn crossed<'b>(
        &self,
        problem: &CoupledProblem<'b>,
    ) -> Result<Option<Trigger<'b>>, TrialFailure> {
        problem.crossed(&self.ticket, &self.state)
    }
    pub(crate) fn gap(&self, problem: &CoupledProblem<'_>, guard: Trigger<'_>) -> Result<f64, TrialFailure> {
        problem.gap(&self.ticket, &self.state, guard)
    }
    /// The dense segment keeps the left limit, so a chord never runs through a reset and the next segment
    /// starts at the reset. A firing moves a mode, a draw and an integral, so what it leaves is read again: a law
    /// with no value there refuses the trial, and a range is judged on both sides of the instant.
    pub(crate) fn fire(self, problem: &CoupledProblem<'_>) -> Result<Self, TrialFailure> {
        let at = self.ticket.interval().end();
        let (state, fired) = problem.fired(&self.ticket, self.state, at)?;
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
        };
        StageWitness {
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
}
