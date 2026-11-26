use core::num::NonZeroU64;
use ng_contract::{
    AcceptedStateView, ArithmeticProfile, CoupledProblem, CoupledSolver, DeclaredResolution, Digest,
    EngineFaultKind, ErrorScales, Factors, Increment, Measure, MethodHistory, NumericalRefusal, Ratio,
    RowKind, SolveWorkspace, SparseView, StageEvaluation, StageJacobian, StageScheme, StageSet, StageTime,
    StepProposal, StepTicket, TrialFailure, eliminate,
};

const ABSOLUTE: f64 = 0.1;
const RELATIVE: f64 = 1e-3;
const FLOOR_NS: u64 = 100_000;
const FLOOR: NonZeroU64 = NonZeroU64::MIN.saturating_add(FLOOR_NS - 1);
/// A refusal halves the committed span and a retry below the floor is a fault, so a span committed this far above
/// the floor keeps two refusals before it.
const SHORTEST_NS: u64 = 4 * FLOOR_NS;
const RETRY_BUDGET: u32 = 12;
const NANOSECONDS_PER_SECOND: f64 = 1e9;
/// Short of the estimate the record refuses above, so a span that grows into a faster phase is seldom refused.
const TARGET: f64 = 0.7;
const GROWTH: f64 = 2.0;
const SHRINK: f64 = 0.25;
const NEWTON_LIMIT: usize = 8;
const REBUILDS: usize = 2;
/// Newton stops once its next correction is a tenth of the last step's estimate, so what it leaves stays a
/// tenth of the step's own error. A step at its cap has an estimate far below the scales, and a stop at the
/// scales there would swamp the method's own error.
const NEWTON_SHARE: f64 = 0.1;
const NEWTON_LOOSEST: f64 = 0.05;
const NEWTON_TIGHTEST: f64 = 1e-6;
const NEWTON_FRESH: f64 = 1e-4;
/// The coarse stage carries no weight, so its Newton stop only has to keep the estimate honest.
const COARSE_SHARE: f64 = 0.2;
/// The record refuses a stage defect above one.
const DEFECT_LIMIT: f64 = 0.5;
/// A Jacobian within this share of a stage's shift converges as well as its own: an odd ticket's halves
/// differ by a nanosecond.
const SHIFT_MATCH: f64 = 1e-2;
const KRYLOV_LIMIT: usize = 6;
/// The coarse stage's stop is a share of its own estimate, so a correction this close to the exact one serves it.
const KRYLOV_REDUCTION: f64 = 1e-2;

fn scale(value: f64, size: f64) -> f64 {
    ABSOLUTE * size + RELATIVE * value.abs()
}

/// The accepted step's first half rides along because, with the segment's ends, it puts the next guess on
/// three points.
struct Carried {
    span_ns: u64,
    estimate: f64,
    midpoint: Vec<f64>,
}
impl Carried {
    fn read_parts(history: &MethodHistory) -> Option<(u64, f64, &[u64])> {
        let (&[span_ns, estimate], midpoint) = history.counters().split_first_chunk()?;
        Some((span_ns, f64::from_bits(estimate), midpoint))
    }
    fn counters(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(2 + self.midpoint.len());
        out.push(self.span_ns);
        out.push(self.estimate.to_bits());
        out.extend(self.midpoint.iter().map(|value| value.to_bits()));
        out
    }
}

pub struct Core {
    longest_step_ns: u64,
    scheme: StageScheme,
}
impl Core {
    /// A core of `StageScheme::backward_euler_halved` whose committed span never exceeds `longest_step_ns`.
    pub fn new(longest_step_ns: u64) -> Self {
        Self {
            longest_step_ns,
            scheme: StageScheme::backward_euler_halved(),
        }
    }
    /// The same core on `StageScheme::backward_euler_doubled`: the identity and course `new` had before record
    /// 0009, so a run it wrote resumes and replays.
    pub fn doubled(longest_step_ns: u64) -> Self {
        Self {
            longest_step_ns,
            scheme: StageScheme::backward_euler_doubled(),
        }
    }
    fn bounded(&self, span_ns: u64) -> u64 {
        let longest = self.longest_step_ns;
        span_ns.clamp(SHORTEST_NS.min(longest), longest)
    }
    fn next_span(&self, ticket: &StepTicket, committed_ns: u64, estimate: f64) -> u64 {
        let length = ticket.length_ns();
        let basis = if ticket.after_breakpoint() || ticket.attempt() > 0 {
            length
        } else {
            committed_ns.max(length)
        } as f64;
        let aimed = if estimate > 0.0 {
            length as f64 * (TARGET / estimate).sqrt()
        } else {
            f64::INFINITY
        };
        let chosen = aimed.clamp(SHRINK * basis, GROWTH * basis);
        self.bounded(chosen as u64)
    }
}

impl CoupledSolver for Core {
    fn resolution(&self) -> DeclaredResolution {
        let Self {
            longest_step_ns,
            scheme,
        } = self;
        let mut policy = b"ng-solver core 1".to_vec();
        policy.extend_from_slice(&longest_step_ns.to_be_bytes());
        let scale = |value| Measure::new(value).expect("a finite constant");
        let scales = ErrorScales::uniform(scale(ABSOLUTE), scale(RELATIVE)).expect("positive constants");
        DeclaredResolution::new(
            Digest::of_canonical(&policy),
            FLOOR,
            RETRY_BUDGET,
            ArithmeticProfile::contract_v1(),
            *scheme,
            scales,
        )
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        let span_ns = Carried::read_parts(accepted.method_history()).map(|(span_ns, _, _)| span_ns);
        self.bounded(span_ns.unwrap_or(self.longest_step_ns))
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let history = problem.accepted().method_history();
        let carried = Carried::read_parts(history).filter(|_| !ticket.after_breakpoint());
        let tolerance = carried.map_or(NEWTON_FRESH, |(_, estimate, _)| {
            (NEWTON_SHARE * estimate).clamp(NEWTON_TIGHTEST, NEWTON_LOOSEST)
        });
        let midpoint = carried.map_or(&[][..], |(_, _, midpoint)| midpoint);
        let stages = Stages::new(problem, ticket, tolerance);
        let solved = stages.solve(problem.stage_set(ticket), midpoint)?;
        let committed_ns = self.next_step(problem.accepted());
        let carried = Carried {
            span_ns: self.next_span(ticket, committed_ns, solved.estimate),
            estimate: solved.estimate,
            midpoint: solved.midpoint,
        };
        let started_at = if ticket.after_breakpoint() {
            ticket.interval().start()
        } else {
            history.started_at()
        };
        let history = MethodHistory::new(history.schema(), started_at, carried.counters());
        Ok(problem.propose(ticket, solved.increment, &solved.end, history))
    }
}

struct Linear<'b> {
    per_second: f64,
    jacobian: StageJacobian<'b>,
    factors: Option<Factors<'b>>,
}
impl Linear<'_> {
    fn solve_into(&self, residuals: &[f64], out: &mut Vec<f64>) -> bool {
        if let Some(factors) = &self.factors {
            out.clear();
            out.extend_from_slice(residuals);
            factors.solve(out);
            if out.iter().all(|change| change.is_finite()) {
                return true;
            }
        }
        if let Some(solved) = dense(self.jacobian.view(), residuals) {
            *out = solved;
            return true;
        }
        false
    }
    fn solve(&self, residuals: &[f64]) -> Option<Vec<f64>> {
        let mut out = Vec::with_capacity(residuals.len());
        self.solve_into(residuals, &mut out).then_some(out)
    }
    fn fits(&self, per_second: f64) -> bool {
        (self.per_second - per_second).abs() <= SHIFT_MATCH * per_second
    }
}

fn dense(jacobian: SparseView<'_>, residuals: &[f64]) -> Option<Vec<f64>> {
    let size = residuals.len();
    let mut matrix = vec![vec![0.0; size + 1]; size];
    for (row, cells) in matrix.iter_mut().enumerate() {
        for entry in jacobian.row_offsets()[row]..jacobian.row_offsets()[row + 1] {
            cells[jacobian.columns()[entry].index()] += jacobian.values()[entry];
        }
        cells[size] = residuals[row];
    }
    eliminate(matrix).ok()
}

fn in_domain(failure: &TrialFailure) -> bool {
    let domain = |reason: NumericalRefusal| matches!(reason, NumericalRefusal::TrialDomain { .. });
    match failure {
        TrialFailure::Refused(refused) => domain(refused.reason()),
        TrialFailure::EngineFault(fault) => {
            fault.kind() == EngineFaultKind::NoAdmissibleStep && fault.last_refusal().is_some_and(domain)
        }
    }
}

#[derive(Clone, Copy)]
enum PointValues<'a> {
    Floats(&'a [f64]),
    Bits(&'a [u64]),
}

struct Trend<'p> {
    earlier: Option<(f64, Vec<f64>)>,
    midpoint: Option<(f64, &'p [u64])>,
}
impl<'p> Trend<'p> {
    fn of(problem: &CoupledProblem<'p>, ticket: &StepTicket, midpoint: &'p [u64]) -> Self {
        let segment = problem.accepted().last_segment();
        let Some(interval) = segment.interval().filter(|_| !ticket.after_breakpoint()) else {
            return Self {
                earlier: None,
                midpoint: None,
            };
        };
        let length = interval.length_ns();
        let rows = problem.body().rows();
        let mut earlier = Vec::with_capacity(rows.len());
        let mut complete = true;
        for row in rows {
            let Some(held) = segment.sample(row.id(), interval.start()) else {
                complete = false;
                break;
            };
            earlier.push(f64::from_bits(held.bits()[0]));
        }
        let earlier = complete.then_some((-(length as f64), earlier));
        let midpoint = (midpoint.len() == rows.len() && length >= 2)
            .then_some((-((length - length / 2) as f64), midpoint));
        Self { earlier, midpoint }
    }
    fn reach_into(&self, known: &[(f64, &[f64])], count: usize, at: f64, out: &mut [f64]) {
        let mut stored = [(0.0, PointValues::Floats(&[])); 2];
        let mut stored_len = 0;
        if let Some((time, values)) = &self.earlier {
            stored[stored_len] = (*time, PointValues::Floats(values));
            stored_len += 1;
        }
        if let Some((time, bits)) = self.midpoint {
            stored[stored_len] = (time, PointValues::Bits(bits));
            stored_len += 1;
        }
        let skip = stored_len.saturating_sub(count);
        let mut points = [(0.0, PointValues::Floats(&[])); 4];
        let mut n = 0;
        for &point in &stored[skip..stored_len] {
            points[n] = point;
            n += 1;
        }
        for &(time, values) in known {
            points[n] = (time, PointValues::Floats(values));
            n += 1;
        }
        let mut weights = [0.0; 4];
        for (k, weight_out) in weights[..n].iter_mut().enumerate() {
            let time = points[k].0;
            let mut weight = 1.0;
            for (j, &(other, _)) in points[..n].iter().enumerate() {
                if j != k {
                    weight = weight * (at - other) / (time - other);
                }
            }
            *weight_out = weight;
        }
        out.fill(0.0);
        for (&weight, &(_, source)) in weights[..n].iter().zip(&points[..n]) {
            match source {
                PointValues::Floats(values) => {
                    for (value, &point) in out.iter_mut().zip(values) {
                        *value += weight * point;
                    }
                }
                PointValues::Bits(bits) => {
                    for (value, &word) in out.iter_mut().zip(bits) {
                        *value += weight * f64::from_bits(word);
                    }
                }
            }
        }
    }
}

/// A stage the increment reads takes at least one update from its guess, so every linear row it publishes holds to
/// rounding and a store its rates fill closes on the row that sums it.
struct Leg<'f> {
    from: &'f [f64],
    offset_ns: u64,
    span_ns: u64,
    tolerance: f64,
    weighted: bool,
}
impl<'f> Leg<'f> {
    fn weighted(from: &'f [f64], offset_ns: u64, span_ns: u64, tolerance: f64) -> Self {
        Self {
            from,
            offset_ns,
            span_ns,
            tolerance,
            weighted: true,
        }
    }
    fn coarse(from: &'f [f64], length_ns: u64, tolerance: f64) -> Self {
        Self {
            weighted: false,
            ..Self::weighted(from, length_ns, length_ns, tolerance)
        }
    }
}

struct Solved {
    increment: Increment,
    end: SolveWorkspace,
    estimate: f64,
    midpoint: Vec<f64>,
}

struct Scratch {
    guess: Vec<f64>,
    before: Vec<f64>,
    correction: Vec<f64>,
}
impl Scratch {
    fn new(rows: usize) -> Self {
        Self {
            guess: vec![0.0; rows],
            before: vec![0.0; rows],
            correction: vec![0.0; rows],
        }
    }
}

struct Stages<'a, 'p> {
    problem: &'a CoupledProblem<'p>,
    ticket: &'a StepTicket,
    differential_end: usize,
    tolerance: f64,
}
impl<'a, 'p> Stages<'a, 'p> {
    fn new(problem: &'a CoupledProblem<'p>, ticket: &'a StepTicket, tolerance: f64) -> Self {
        let rows = problem.body().rows();
        Self {
            problem,
            ticket,
            differential_end: rows.partition_point(|row| !matches!(row.kind(), RowKind::Algebraic(_))),
            tolerance,
        }
    }

    fn solve(&self, set: StageSet, midpoint: &'p [u64]) -> Result<Solved, TrialFailure> {
        let (problem, ticket) = (self.problem, self.ticket);
        let length = ticket.length_ns();
        let mut workspace = problem.workspace();
        let start = workspace.unknowns_mut().to_vec();
        let trend = Trend::of(problem, ticket, midpoint);
        let mut scratch = Scratch::new(start.len());
        let mut quadrature = problem.quadrature(ticket);
        let mut linear = None;
        let midpoint = match set {
            StageSet::Whole => {
                trend.reach_into(&[(0.0, &start)], 2, length as f64, &mut scratch.guess);
                let leg = Leg::weighted(&start, length, length, self.tolerance);
                let stage = self.settle(&mut workspace, leg, &mut linear, &mut scratch)?;
                quadrature.add_owned(stage, Ratio::ONE)?;
                Vec::new()
            }
            StageSet::Doubled | StageSet::Halved => {
                let (half, rest) = (length / 2, length - length / 2);
                trend.reach_into(&[(0.0, &start)], 2, half as f64, &mut scratch.guess);
                let early = self.settle(
                    &mut workspace,
                    Leg::weighted(&start, half, half, self.tolerance),
                    &mut linear,
                    &mut scratch,
                )?;
                let reached = workspace.unknowns_mut().to_vec();
                trend.reach_into(
                    &[(0.0, &start), (half as f64, &reached)],
                    1,
                    length as f64,
                    &mut scratch.guess,
                );
                let leg = Leg::weighted(&reached, length, rest, self.tolerance);
                let late = self.settle(&mut workspace, leg, &mut linear, &mut scratch)?;
                if set == StageSet::Doubled {
                    let fine = workspace.unknowns_mut().to_vec();
                    let coarse =
                        self.coarse(&mut workspace, &start, &reached, &fine, &mut linear, &mut scratch)?;
                    quadrature.add_owned(coarse, Ratio::ZERO)?;
                }
                quadrature.add_owned(early, ticket.fraction(half))?;
                quadrature.add_owned(late, ticket.fraction(rest))?;
                reached
            }
            StageSet::ForwardEuler | StageSet::Heun => return Err(ticket.mismatch().into()),
        };
        let increment = quadrature.finish()?;
        let estimate = problem.estimate(ticket, &increment)?;
        Ok(Solved {
            increment,
            end: workspace,
            estimate,
            midpoint,
        })
    }

    fn coarse(
        &self,
        workspace: &mut SolveWorkspace,
        start: &[f64],
        reached: &[f64],
        fine: &[f64],
        linear: &mut Option<Linear<'p>>,
        scratch: &mut Scratch,
    ) -> Result<StageEvaluation, TrialFailure> {
        let mut apart = 0.0;
        for (at, ((&fine_val, &half), &from)) in fine.iter().zip(reached).zip(start).enumerate() {
            let guess = from + 2.0 * (fine_val - half);
            scratch.guess[at] = guess;
            let normalized = (fine_val - guess).abs() / scale(fine_val, self.problem.row_size(at));
            if normalized > apart {
                apart = normalized;
            }
        }
        let tolerance = COARSE_SHARE * apart;
        let tolerance = if tolerance > self.tolerance {
            tolerance
        } else {
            self.tolerance
        };
        let leg = Leg::coarse(start, self.ticket.length_ns(), tolerance);
        let stage = self.settle(workspace, leg, linear, scratch)?;
        workspace.unknowns_mut().copy_from_slice(fine);
        Ok(stage)
    }

    /// The record's stage defect, each row judged at its own value: the record judges a row that closes a store
    /// at that store, which a core cannot name.
    fn defect(&self, residuals: &[f64], unknowns: &[f64]) -> f64 {
        let seconds = self.ticket.length_ns() as f64 / NANOSECONDS_PER_SECOND;
        let mut worst = 0.0;
        for at in 0..self.differential_end {
            let normalized = (seconds * residuals[at].abs()) / scale(unknowns[at], self.problem.row_size(at));
            if normalized > worst {
                worst = normalized;
            }
        }
        for at in self.differential_end..residuals.len() {
            let normalized = residuals[at].abs() / scale(unknowns[at], self.problem.row_size(at));
            if normalized > worst {
                worst = normalized;
            }
        }
        worst
    }

    fn norm(&self, correction: &[f64], unknowns: &[f64]) -> f64 {
        let mut worst = 0.0;
        for (at, (change, value)) in correction.iter().zip(unknowns).enumerate() {
            let normalized = change.abs() / scale(*value, self.problem.row_size(at));
            if normalized > worst {
                worst = normalized;
            }
        }
        worst
    }

    fn evaluate_with(
        &self,
        workspace: &mut SolveWorkspace,
        unknowns: &[f64],
        from: &[f64],
        per_second: f64,
        time: StageTime<'_>,
    ) -> Result<StageEvaluation, TrialFailure> {
        workspace.unknowns_mut().copy_from_slice(unknowns);
        for ((tangent, &now), &then) in workspace.derivative_mut().iter_mut().zip(unknowns).zip(from) {
            *tangent = (now - then) * per_second;
        }
        self.problem.evaluate(&workspace.point(time))
    }

    fn linearized(
        &self,
        workspace: &SolveWorkspace,
        time: StageTime<'_>,
        span_ns: u64,
    ) -> Result<Linear<'p>, TrialFailure> {
        let jacobian = self
            .problem
            .linearize_stage(&workspace.point(time), self.ticket.shift(span_ns)?)?;
        let factors = jacobian.factor().ok();
        Ok(Linear {
            per_second: NANOSECONDS_PER_SECOND / span_ns as f64,
            jacobian,
            factors,
        })
    }

    /// The correction at a shift the factored Jacobian was not built at, by GMRES with that factorization as the
    /// right preconditioner, on residuals weighted as the record's defect weighs them. The two matrices differ only
    /// on each differential row's own unknown, so the preconditioned spectrum sits near one and near the shifts'
    /// ratio, and a few iterations reach the stop.
    fn krylov(
        &self,
        held: &Linear<'_>,
        per_second: f64,
        residuals: &[f64],
        unknowns: &[f64],
    ) -> Option<Vec<f64>> {
        let body_rows = self.problem.body().rows();
        let rows = residuals.len();
        let seconds = self.ticket.length_ns() as f64 / NANOSECONDS_PER_SECOND;
        let weights: Vec<f64> = (0..rows)
            .map(|at| {
                let weight = 1.0 / scale(unknowns[at], self.problem.row_size(at));
                if at < self.differential_end {
                    seconds * weight
                } else {
                    weight
                }
            })
            .collect();
        let view = held.jacobian.view();
        let shift = per_second - held.per_second;
        let operate = |direction: &[f64]| -> Vec<f64> {
            (0..rows)
                .map(|row| {
                    let entries = view.row_offsets()[row]..view.row_offsets()[row + 1];
                    let mut sum = 0.0;
                    for entry in entries {
                        sum += view.values()[entry] * direction[view.columns()[entry].index()];
                    }
                    if row < self.differential_end {
                        sum += shift * direction[body_rows[row].unknown().index()];
                    }
                    sum * weights[row]
                })
                .collect()
        };
        let dot = |a: &[f64], b: &[f64]| -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
        let start: Vec<f64> = residuals
            .iter()
            .zip(&weights)
            .map(|(value, weight)| value * weight)
            .collect();
        let beta = dot(&start, &start).sqrt();
        if beta == 0.0 {
            return Some(vec![0.0; rows]);
        }
        let mut basis = vec![start.iter().map(|value| value / beta).collect::<Vec<f64>>()];
        let (mut lifted, mut columns, mut rotations) = (Vec::new(), Vec::new(), Vec::new());
        let mut projected = vec![beta];
        for _ in 0..KRYLOV_LIMIT {
            let latest = basis.last().expect("one direction at least");
            let unweighted: Vec<f64> = latest
                .iter()
                .zip(&weights)
                .map(|(value, weight)| value / weight)
                .collect();
            let lift = held.solve(&unweighted)?;
            let mut next = operate(&lift);
            let mut column = Vec::with_capacity(basis.len() + 1);
            for direction in &basis {
                let along = dot(&next, direction);
                for (value, part) in next.iter_mut().zip(direction) {
                    *value -= along * part;
                }
                column.push(along);
            }
            let length = dot(&next, &next).sqrt();
            column.push(length);
            for (at, &(cosine, sine)) in rotations.iter().enumerate() {
                let (a, b) = (column[at], column[at + 1]);
                column[at] = cosine * a + sine * b;
                column[at + 1] = cosine * b - sine * a;
            }
            let last = column.len() - 2;
            let (a, b) = (column[last], column[last + 1]);
            let radius = (a * a + b * b).sqrt();
            let (cosine, sine) = if radius == 0.0 {
                (1.0, 0.0)
            } else {
                (a / radius, b / radius)
            };
            column[last] = radius;
            column.truncate(last + 1);
            rotations.push((cosine, sine));
            let reached = projected[last];
            projected[last] = cosine * reached;
            projected.push(-sine * reached);
            columns.push(column);
            lifted.push(lift);
            if projected[last + 1].abs() <= KRYLOV_REDUCTION * beta || length == 0.0 {
                break;
            }
            basis.push(next.iter().map(|value| value / length).collect());
        }
        let mut along = vec![0.0; columns.len()];
        for at in (0..columns.len()).rev() {
            let mut sum = projected[at];
            for later in at + 1..columns.len() {
                sum -= columns[later][at] * along[later];
            }
            along[at] = sum / columns[at][at];
        }
        let mut correction = vec![0.0; rows];
        for (share, lift) in along.iter().zip(&lifted) {
            for (value, part) in correction.iter_mut().zip(lift) {
                *value += share * part;
            }
        }
        correction
            .iter()
            .all(|value| value.is_finite())
            .then_some(correction)
    }

    fn update(
        &self,
        workspace: &mut SolveWorkspace,
        from: &[f64],
        per_second: f64,
        time: StageTime<'_>,
        scratch: &mut Scratch,
        evaluation: &mut StageEvaluation,
    ) -> Result<(), TrialFailure> {
        scratch.before.copy_from_slice(workspace.unknowns_mut());
        let mut share = 1.0;
        loop {
            for ((unknown, &was), &change) in workspace
                .unknowns_mut()
                .iter_mut()
                .zip(&scratch.before)
                .zip(&scratch.correction)
            {
                *unknown = was - share * change;
            }
            for (((tangent, &was), &change), &then) in workspace
                .derivative_mut()
                .iter_mut()
                .zip(&scratch.before)
                .zip(&scratch.correction)
                .zip(from)
            {
                *tangent = ((was - share * change) - then) * per_second;
            }
            match self.problem.reevaluate(&workspace.point(time), evaluation) {
                Err(failure) if share > 0.2 && in_domain(&failure) => share *= 0.5,
                outcome => return outcome,
            }
        }
    }

    fn settle(
        &self,
        workspace: &mut SolveWorkspace,
        leg: Leg<'_>,
        linear: &mut Option<Linear<'p>>,
        scratch: &mut Scratch,
    ) -> Result<StageEvaluation, TrialFailure> {
        let Leg {
            from,
            offset_ns,
            span_ns,
            tolerance,
            weighted,
        } = leg;
        let time = self.ticket.at(offset_ns)?;
        let per_second = NANOSECONDS_PER_SECOND / span_ns as f64;
        let mut evaluation = match self.evaluate_with(workspace, &scratch.guess, from, per_second, time) {
            Err(failure) if in_domain(&failure) && scratch.guess.as_slice() != from => {
                self.evaluate_with(workspace, from, from, per_second, time)?
            }
            outcome => outcome?,
        };
        let (mut updates, mut rebuilds, mut last) = (0, 0, f64::INFINITY);
        let mut built_here = false;
        loop {
            let residuals = evaluation.residuals();
            if residuals.iter().all(|residual| *residual == 0.0) {
                return Ok(evaluation);
            }
            if linear.is_none() {
                *linear = Some(self.linearized(workspace, time, span_ns)?);
                (rebuilds, built_here) = (rebuilds + 1, true);
            }
            let held = linear.as_ref().expect("built above");
            let refused = || self.ticket.refuse(NumericalRefusal::IterationLimit);
            let solved = if held.fits(per_second) {
                held.solve_into(residuals, &mut scratch.correction)
            } else if held.factors.is_some() {
                if let Some(correction) = self.krylov(held, per_second, residuals, workspace.unknowns_mut()) {
                    scratch.correction = correction;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let rebuild = !built_here && rebuilds < REBUILDS;
            if !solved {
                if rebuild {
                    *linear = Some(self.linearized(workspace, time, span_ns)?);
                    (rebuilds, built_here) = (rebuilds + 1, true);
                    continue;
                }
                return Err(refused());
            }
            let size = self.norm(&scratch.correction, workspace.unknowns_mut());
            let stalled = size >= 0.5 * last;
            let updated = updates > 0 || !weighted;
            if updated
                && size <= tolerance
                && (stalled || self.defect(residuals, workspace.unknowns_mut()) <= DEFECT_LIMIT)
            {
                return Ok(evaluation);
            }
            if rebuild && stalled {
                *linear = Some(self.linearized(workspace, time, span_ns)?);
                (rebuilds, built_here) = (rebuilds + 1, true);
                continue;
            }
            if updates == NEWTON_LIMIT || size >= last {
                return Err(refused());
            }
            (updates, last, built_here) = (updates + 1, size, false);
            self.update(workspace, from, per_second, time, scratch, &mut evaluation)?;
        }
    }
}
