use crate::body::{Bound, GuardLayout, Instance, ParameterId, RowId, RowKind, Target, UnknownId};
use crate::boundary::DriveFrame;
use crate::law::{Op, Validity};
use crate::pinned;
use crate::{
    Compensated, CompiledBody, Crossing, ErrorScales, ExcursionSide, LawName, SimTime, ValidityExcursion,
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum OperatorDomain {
    Divisor,
    LogarithmArgument,
    NonFinite,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Undefined {
    pub(crate) law: LawName,
    pub(crate) domain: OperatorDomain,
}

/// A smooth limit, with `sign` on its root, and the root's slopes in `a - b` and in `width`. Both slopes are zero
/// where the root is zero, the mean of the two one-sided derivatives, so a Newton step only loses speed.
fn smooth(a: f64, b: f64, width: f64, sign: f64) -> (f64, f64, f64) {
    let difference = a - b;
    let root = (difference * difference + width * width).sqrt();
    let (along, across) = if root == 0.0 {
        (0.0, 0.0)
    } else {
        (difference / root, width / root)
    };
    (0.5 * (a + b + sign * root), along, across)
}

fn run(
    ops: &[Op],
    mut next_input: impl FnMut() -> f64,
    partial_volume: f64,
    stack: &mut Vec<f64>,
) -> Result<f64, OperatorDomain> {
    stack.clear();
    for op in ops {
        let at = |index: usize| stack[index];
        let result = match op {
            Op::Input(_) => next_input(),
            Op::Zero(_) => 0.0,
            Op::Add([a, b]) => at(*a) + at(*b),
            Op::Subtract([a, b]) | Op::Deviation([a, b]) => at(*a) - at(*b),
            Op::Multiply([a, b]) | Op::Scale([a, b]) | Op::ConductanceFlow([a, b]) => at(*a) * at(*b),
            Op::Divide([a, b]) => {
                if at(*b) <= 0.0 {
                    return Err(OperatorDomain::Divisor);
                }
                at(*a) / at(*b)
            }
            Op::AsDimension([x]) => at(*x),
            Op::SmoothMax([a, b, width]) => smooth(at(*a), at(*b), at(*width), 1.0).0,
            Op::SmoothMin([a, b, width]) => smooth(at(*a), at(*b), at(*width), -1.0).0,
            Op::Exp([x]) => pinned::exp(at(*x)),
            Op::Logistic([x]) => 1.0 / (1.0 + pinned::exp(-at(*x))),
            Op::Ln([x]) => pinned::ln(at(*x)).ok_or(OperatorDomain::LogarithmArgument)?,
            Op::Relax([target, current, speed]) => at(*speed) * (at(*target) - at(*current)),
            Op::Volume([amount]) => at(*amount) * partial_volume,
            Op::Upwind([flow, from, from_water, to, to_water]) => {
                let (amount, water) = if at(*flow) >= 0.0 {
                    (at(*from), at(*from_water))
                } else {
                    (at(*to), at(*to_water))
                };
                if at(*flow) == 0.0 {
                    0.0
                } else if water <= 0.0 {
                    return Err(OperatorDomain::Divisor);
                } else {
                    at(*flow) * (amount / water)
                }
            }
        };
        if !result.is_finite() {
            return Err(OperatorDomain::NonFinite);
        }
        stack.push(result);
    }
    Ok(stack[stack.len() - 1])
}

/// One backward sweep over a finished `run`: the slope of the kernel's result in every operation's value, so one
/// sweep gives every read's slope at about the cost of the forward run.
fn sweep(ops: &[Op], values: &[f64], partial_volume: f64, slopes: &mut Vec<f64>) {
    slopes.clear();
    slopes.resize(ops.len(), 0.0);
    slopes[ops.len() - 1] = 1.0;
    for (at, op) in ops.iter().enumerate().rev() {
        let slope = slopes[at];
        if slope == 0.0 {
            continue;
        }
        match op {
            Op::Input(_) | Op::Zero(_) => {}
            Op::Add([a, b]) => {
                slopes[*a] += slope;
                slopes[*b] += slope;
            }
            Op::Subtract([a, b]) | Op::Deviation([a, b]) => {
                slopes[*a] += slope;
                slopes[*b] -= slope;
            }
            Op::Multiply([a, b]) | Op::Scale([a, b]) | Op::ConductanceFlow([a, b]) => {
                slopes[*a] += slope * values[*b];
                slopes[*b] += slope * values[*a];
            }
            Op::Divide([a, b]) => {
                let divisor = values[*b];
                slopes[*a] += slope / divisor;
                slopes[*b] -= slope * values[*a] / (divisor * divisor);
            }
            Op::AsDimension([x]) => slopes[*x] += slope,
            Op::SmoothMax([a, b, width]) | Op::SmoothMin([a, b, width]) => {
                let sign = if matches!(op, Op::SmoothMax(_)) { 1.0 } else { -1.0 };
                let (_, along, across) = smooth(values[*a], values[*b], values[*width], sign);
                slopes[*a] += slope * 0.5 * (1.0 + sign * along);
                slopes[*b] += slope * 0.5 * (1.0 - sign * along);
                slopes[*width] += slope * 0.5 * sign * across;
            }
            Op::Exp([x]) => slopes[*x] += slope * values[at],
            Op::Logistic([x]) => slopes[*x] += slope * values[at] * (1.0 - values[at]),
            Op::Ln([x]) => slopes[*x] += slope / values[*x],
            Op::Relax([target, current, speed]) => {
                let gap = values[*target] - values[*current];
                slopes[*target] += slope * values[*speed];
                slopes[*current] -= slope * values[*speed];
                slopes[*speed] += slope * gap;
            }
            Op::Volume([amount]) => slopes[*amount] += slope * partial_volume,
            Op::Upwind([flow, from, from_water, to, to_water]) => {
                let (amount, water) = if values[*flow] >= 0.0 {
                    (*from, *from_water)
                } else {
                    (*to, *to_water)
                };
                if values[water] > 0.0 {
                    let share = values[amount] / values[water];
                    slopes[*flow] += slope * share;
                    slopes[amount] += slope * values[*flow] / values[water];
                    slopes[water] -= slope * values[*flow] * share / values[water];
                }
            }
        }
    }
}

pub(crate) struct Inputs<'a> {
    pub(crate) parameters: &'a [f64],
    pub(crate) frame: &'a DriveFrame,
    pub(crate) unknowns: &'a [f64],
    pub(crate) modes: &'a [usize],
    pub(crate) draws: &'a [f64],
    pub(crate) factors: &'a [f64],
}

/// A law's own value stays what a validity range judges.
fn positive_part(instance: &Instance, value: f64) -> f64 {
    if instance.rectified && value <= 0.0 {
        0.0
    } else {
        value
    }
}

impl CompiledBody {
    fn partial_volume(&self) -> f64 {
        (self.layout.schema.partial_volume.as_ref()).map_or(0.0, |coefficient| coefficient.value())
    }
    pub(crate) fn guard_gap(&self, guard: &GuardLayout, inputs: &Inputs<'_>) -> Result<f64, Undefined> {
        let mut bounds = guard.inputs.iter();
        let next = || Self::bound(bounds.next().expect("one bound per input"), inputs);
        let mut stack = Vec::new();
        let value = run(&guard.code.ops, next, self.partial_volume(), &mut stack).map_err(|domain| {
            let law = self.layout.modes[guard.variable].variable.law;
            Undefined { law, domain }
        })?;
        Ok(match guard.crossing {
            Crossing::Rising => value,
            Crossing::Falling => -value,
        })
    }
    /// A gap within `tolerance` of zero counts as crossed, so a point that lands on its guard, as the ring's last point
    /// lands on the wrap, reads the same on every sweep.
    pub(crate) fn fired_at(
        &self,
        inputs: &Inputs<'_>,
        tolerance: &ErrorScales,
    ) -> Result<(Vec<usize>, Vec<RowId>), Undefined> {
        let mut crossed = Vec::with_capacity(self.layout.guards.len());
        for guard in &self.layout.guards {
            crossed.push(
                guard.armed(inputs.modes) && {
                    let gap = self.guard_gap(guard, inputs)?;
                    gap >= -tolerance.at(gap, 1.0)
                },
            );
        }
        let mut modes = inputs.modes.to_vec();
        let fired = self.layout.fire(&mut modes, &crossed);
        let resets = fired.iter().filter_map(|guard| guard.resets).collect();
        Ok((modes, resets))
    }
    fn bound(bound: &Bound, inputs: &Inputs<'_>) -> f64 {
        match *bound {
            Bound::Parameter(parameter) => inputs.parameters[parameter.index()],
            Bound::Unknown(unknown) => inputs.unknowns[unknown.index()],
            Bound::Boundary(name) => inputs.frame.rate(name),
            Bound::Draw(stream) => inputs.draws[stream],
            Bound::Forcing(forcing) => inputs.factors[forcing],
            Bound::Gauge => 0.0,
            Bound::Height(axial) => inputs.frame.height(f64::from_bits(axial)),
        }
    }
    fn walk(&self, instance: &Instance, inputs: &Inputs<'_>, stack: &mut Vec<f64>) -> Result<f64, Undefined> {
        let program = &self.layout.programs[instance.program.index()];
        let mut bounds = instance.inputs.iter();
        let next = || Self::bound(bounds.next().expect("one bound per input"), inputs);
        run(&program.code.ops, next, self.partial_volume(), stack).map_err(|domain| Undefined {
            law: program.law,
            domain,
        })
    }
    pub(crate) fn instance_values(&self, inputs: &Inputs<'_>) -> Result<Vec<Option<f64>>, Undefined> {
        let mut stack = Vec::new();
        (self.layout.instances.iter())
            .map(|instance| {
                if !instance.holds(inputs.modes) {
                    return Ok(None);
                }
                Ok(Some(self.walk(instance, inputs, &mut stack)?))
            })
            .collect()
    }
    pub(crate) fn targets(&self, values: &[Option<f64>]) -> (Vec<f64>, Vec<f64>) {
        let mut columns = vec![0.0; self.layout.columns.len()];
        let mut rows = vec![0.0; self.layout.rows.len()];
        for (instance, value) in self.layout.instances.iter().zip(values) {
            let Some(value) = *value else { continue };
            match instance.target {
                Target::Column(column) => columns[column.index()] = value,
                Target::Row(row) => rows[row.index()] += positive_part(instance, value),
            }
        }
        (columns, rows)
    }
    pub(crate) fn residuals(&self, columns: &[f64], rows: &[f64], derivative: &[f64]) -> Vec<f64> {
        let mut residuals = Vec::with_capacity(rows.len());
        for (row, (&value, &tangent)) in self.layout.rows.iter().zip(rows.iter().zip(derivative)) {
            residuals.push(match row.kind {
                RowKind::Amount { .. } => tangent,
                RowKind::Memory(_) => tangent - value,
                RowKind::Algebraic(_) => value,
            });
        }
        for (column, &rate) in self.layout.columns.iter().zip(columns) {
            for &(row, coefficient) in &column.terms {
                residuals[row.index()] -= coefficient as f64 * rate;
            }
        }
        residuals
    }
    fn slopes(
        &self,
        inputs: &Inputs<'_>,
        mut each: impl FnMut(&Instance, &mut dyn Iterator<Item = (&Bound, f64)>) -> Result<(), Undefined>,
    ) -> Result<(), Undefined> {
        let (mut stack, mut slopes) = (Vec::new(), Vec::new());
        let holds = |instance: &&Instance| instance.holds(inputs.modes);
        for instance in self.layout.instances.iter().filter(holds) {
            let value = self.walk(instance, inputs, &mut stack)?;
            let program = &self.layout.programs[instance.program.index()];
            let ops = &program.code.ops;
            sweep(ops, &stack, self.partial_volume(), &mut slopes);
            let flat = positive_part(instance, value) != value;
            let reads = (ops.iter().zip(&slopes))
                .filter(|(op, _)| matches!(op, Op::Input(_)))
                .map(|(_, &slope)| if flat { 0.0 } else { slope });
            let mut read = instance.inputs.iter().zip(reads);
            each(instance, &mut read)?;
        }
        Ok(())
    }
    pub(crate) fn jacobian(&self, inputs: &Inputs<'_>, shift: f64) -> Result<Vec<f64>, Undefined> {
        let sparsity = &self.layout.sparsity;
        let mut values = vec![0.0; sparsity.unknowns.len()];
        let entry = |row: usize, unknown: UnknownId| {
            let range = sparsity.offsets[row]..sparsity.offsets[row + 1];
            let found = sparsity.unknowns[range.clone()].binary_search(&unknown);
            range.start + found.expect("the compiled sparsity holds every unknown a row's laws read")
        };
        self.slopes(inputs, |instance, reads| {
            for (bound, slope) in reads {
                let Some(unknown) = bound.unknown() else { continue };
                if !slope.is_finite() {
                    let program = &self.layout.programs[instance.program.index()];
                    return Err(Undefined {
                        law: program.law,
                        domain: OperatorDomain::NonFinite,
                    });
                }
                match instance.target {
                    Target::Column(column) => {
                        for &(row, coefficient) in &self.layout.columns[column.index()].terms {
                            values[entry(row.index(), unknown)] -= coefficient as f64 * slope;
                        }
                    }
                    Target::Row(row) => {
                        let sign = match self.layout.rows[row.index()].kind {
                            RowKind::Algebraic(_) => 1.0,
                            RowKind::Amount { .. } | RowKind::Memory(_) => -1.0,
                        };
                        values[entry(row.index(), unknown)] += sign * slope;
                    }
                }
            }
            Ok(())
        })?;
        for (at, row) in self.layout.rows.iter().enumerate() {
            if !matches!(row.kind, RowKind::Algebraic(_)) {
                values[entry(at, row.unknown)] += shift;
            }
        }
        Ok(values)
    }
    /// `dF/dp` per row, for one fit: every slot it fills moves together.
    pub(crate) fn parameter_slopes(
        &self,
        inputs: &Inputs<'_>,
        slots: &[ParameterId],
    ) -> Result<Vec<f64>, Undefined> {
        let mut slopes = vec![0.0; self.layout.rows.len()];
        self.slopes(inputs, |instance, reads| {
            let fitted = reads.filter(
                |(bound, _)| matches!(bound, Bound::Parameter(parameter) if slots.contains(parameter)),
            );
            let slope: f64 = fitted.map(|(_, slope)| slope).sum();
            match instance.target {
                Target::Column(column) => {
                    for &(row, coefficient) in &self.layout.columns[column.index()].terms {
                        slopes[row.index()] -= coefficient as f64 * slope;
                    }
                }
                Target::Row(row) => {
                    let sign = match self.layout.rows[row.index()].kind {
                        RowKind::Algebraic(_) => 1.0,
                        RowKind::Amount { .. } | RowKind::Memory(_) => -1.0,
                    };
                    slopes[row.index()] += sign * slope;
                }
            }
            Ok(())
        })?;
        Ok(slopes)
    }
    pub(crate) fn excursions(
        &self,
        values: &[Option<f64>],
        unknowns: &[f64],
        at: SimTime,
    ) -> Vec<ValidityExcursion> {
        let mut excursions = Vec::new();
        for (instance, value) in self.layout.instances.iter().zip(values) {
            let Some(value) = *value else { continue };
            let program = &self.layout.programs[instance.program.index()];
            let value = match instance.target {
                Target::Row(row) if program.code.defines => unknowns[row.index()],
                _ => value,
            };
            for &Validity { lower, upper, .. } in &program.code.validity {
                let side = if value < f64::from_bits(lower) {
                    ExcursionSide::Below
                } else if value > f64::from_bits(upper) {
                    ExcursionSide::Above
                } else {
                    continue;
                };
                excursions.push(ValidityExcursion {
                    law: program.law,
                    context: instance.context,
                    at,
                    side,
                    value: Compensated::exact(value),
                });
            }
        }
        excursions
    }
}

#[cfg(test)]
mod tests {
    use super::Inputs;
    use crate::boundary::DriveFrame;
    use crate::*;
    use core::fmt::Write;

    const NOT_FINITE_OR_EDGE: [f64; 7] = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -1.0,
        1e-310,
    ];

    fn measure<U: Dimension>(value: f64) -> Measure<U> {
        Measure::new(value).expect("finite")
    }

    fn memory(local: &'static str) -> RowKind {
        RowKind::Memory(LawName { owner: "NS", local }.into())
    }

    /// `x`'s rate reads `x` before it divides by `y`, `y`'s takes the log of `y`, and `q`'s residual is a bare read, so a
    /// read that is not finite is told apart from the first op that consumes it. `z`'s rate reads `y` and `x` once each
    /// and uses each read twice, so the sweep's two updates of one slot keep their order.
    fn reads_before_a_divide() -> (CompiledBody, BoundParameters) {
        let mut map = AnatomyBuilder::new();
        map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
        let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        let body = assemble(schema, |mut body| {
            let (mut laws, _ports) = body.nervous()?.parts();
            let x = laws.state::<Dimensionless>("x", Admissible::finite());
            let y = laws.state::<Dimensionless>("y", Admissible::finite());
            let z = laws.state::<Dimensionless>("z", Admissible::finite());
            let q = laws.algebraic::<Dimensionless>("q", Admissible::finite());
            let p = laws.parameter::<Dimensionless>(ParameterKey::new("ns.p"), Admissible::positive());
            let speed =
                laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
            let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
            let x_rate = laws.kernel(|mut k| {
                let rate = k.parameter(&speed);
                let read = k.state(&x);
                let numerator = k.parameter(&p);
                let divisor = k.state(&y);
                let quotient = k.divide(numerator, divisor);
                let quotient = k.as_dimension::<_, Dimensionless>(quotient);
                let sum = k.add(read, quotient);
                let scaled = k.scale(rate, sum);
                k.finish(scaled, laws.every_mode(), wave)
            });
            laws.memory_rate(&x, x_rate);
            let y_rate = laws.kernel(|mut k| {
                let rate = k.parameter(&speed);
                let read = k.state(&y);
                let log = k.ln(read);
                let scaled = k.scale(rate, log);
                k.finish(scaled, laws.every_mode(), wave)
            });
            laws.memory_rate(&y, y_rate);
            let z_rate = laws.kernel(|mut k| {
                let rate = k.parameter(&speed);
                let held = k.state(&y);
                let relaxed = k.relax(held, held, rate);
                let read = k.state(&x);
                let square = k.scale(read, read);
                let rate = k.parameter(&speed);
                let scaled = k.scale(rate, square);
                let sum = k.add(relaxed, scaled);
                k.finish(sum, laws.every_mode(), wave)
            });
            laws.memory_rate(&z, z_rate);
            let q_residual = laws.kernel(|mut k| {
                let read = k.algebraic(&q);
                k.finish(read, laws.every_mode(), wave)
            });
            laws.residual(&q, q_residual);
            body.add(laws.finish());
            body.compile()
        })
        .expect("compiled");
        let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
        let synthetic = "synthetic";
        (packet.synthetic(
            ParameterKey::new("ns.p"),
            Binding::Body,
            measure::<Dimensionless>(0.5),
            synthetic,
        ))
        .expect("once");
        let speed = measure::<Rate<Dimensionless>>(0.7);
        (packet.synthetic(ParameterKey::new("ns.speed"), Binding::Body, speed, synthetic)).expect("once");
        (packet.start(memory("x"), measure::<Dimensionless>(0.4), synthetic)).expect("once");
        (packet.start(memory("y"), measure::<Dimensionless>(1.5), synthetic)).expect("once");
        (packet.start(memory("z"), measure::<Dimensionless>(-0.75), synthetic)).expect("once");
        let q = RowKind::Algebraic(
            LawName {
                owner: "NS",
                local: "q",
            }
            .into(),
        );
        (packet.start(q, measure::<Dimensionless>(0.25), synthetic)).expect("once");
        let bound = body.bind(packet).expect("bound");
        (body, bound)
    }

    fn outcome(body: &CompiledBody, modes: &[usize], parameters: &[f64], unknowns: &[f64]) -> String {
        let frame = DriveFrame::default();
        let inputs = Inputs {
            parameters,
            frame: &frame,
            unknowns,
            modes,
            draws: &[],
            factors: &[],
        };
        let hex = |value: f64| format!("{:016x}", value.to_bits());
        let values = match body.instance_values(&inputs) {
            Ok(values) => (values.iter())
                .map(|value| value.map_or_else(|| "none".to_string(), hex))
                .collect::<Vec<_>>()
                .join(","),
            Err(undefined) => format!("{undefined:?}"),
        };
        let jacobian = match body.jacobian(&inputs, 2.0) {
            Ok(values) => values
                .iter()
                .map(|value| hex(*value))
                .collect::<Vec<_>>()
                .join(","),
            Err(undefined) => format!("{undefined:?}"),
        };
        let fits = (0..parameters.len()).map(|slot| {
            match body.parameter_slopes(&inputs, &[ParameterId::at(slot)]) {
                Ok(values) => values
                    .iter()
                    .map(|value| hex(*value))
                    .collect::<Vec<_>>()
                    .join(","),
                Err(undefined) => format!("{undefined:?}"),
            }
        });
        format!(
            "values {values}; jacobian {jacobian}; fits {}",
            fits.collect::<Vec<_>>().join(" | ")
        )
    }

    #[test]
    fn a_value_that_is_not_finite_fails_at_the_read_that_takes_it() {
        let (body, bound) = reads_before_a_divide();
        let rows = body.rows();
        let slot = |kind: RowKind| rows.iter().position(|row| row.kind() == kind).expect("a row");
        let (x, y) = (slot(memory("x")), slot(memory("y")));
        let start: Vec<f64> = rows.iter().map(|row| bound.starts[row.id().index()]).collect();
        let (modes, parameters) = (&bound.modes, &bound.values);
        let mut trace = String::new();
        writeln!(trace, "start {}", outcome(&body, modes, parameters, &start)).expect("a string takes it");
        for at in 0..start.len() {
            for value in NOT_FINITE_OR_EDGE {
                let mut unknowns = start.clone();
                unknowns[at] = value;
                let seen = outcome(&body, modes, parameters, &unknowns);
                writeln!(trace, "unknown {at} {:016x}: {seen}", value.to_bits()).expect("a string takes it");
            }
            for other in (0..start.len()).filter(|other| *other != at) {
                let mut unknowns = start.clone();
                (unknowns[at], unknowns[other]) = (f64::NAN, -1.0);
                let seen = outcome(&body, modes, parameters, &unknowns);
                writeln!(trace, "unknown {at} nan, {other} -1: {seen}").expect("a string takes it");
            }
        }
        for at in 0..parameters.len() {
            for value in NOT_FINITE_OR_EDGE {
                let mut changed = parameters.clone();
                changed[at] = value;
                let seen = outcome(&body, modes, &changed, &start);
                writeln!(trace, "parameter {at} {:016x}: {seen}", value.to_bits())
                    .expect("a string takes it");
            }
        }
        let mut unknowns = start.clone();
        (unknowns[x], unknowns[y]) = (f64::NAN, -1.0);
        let seen = outcome(&body, modes, parameters, &unknowns);
        assert!(
            seen.starts_with("values Undefined") && seen.contains("domain: NonFinite"),
            "{seen}"
        );
        let q = slot(RowKind::Algebraic(
            LawName {
                owner: "NS",
                local: "q",
            }
            .into(),
        ));
        let mut unknowns = start.clone();
        unknowns[q] = f64::NAN;
        let seen = outcome(&body, modes, parameters, &unknowns);
        assert!(
            seen.starts_with("values Undefined") && seen.contains("local: \"q\""),
            "{seen}"
        );
        let digest = Digest::of_canonical(trace.as_bytes());
        let hex: String = digest
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex, "1fe2a75674a19112a16252a3a6b55dfc50c780124a9b155c00495e2d228cbbe3",
            "{trace}"
        );
    }
}
