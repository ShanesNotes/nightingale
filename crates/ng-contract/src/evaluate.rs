use crate::body::{Bound, Instance, RowKind, Target, UnknownId};
use crate::boundary::DriveFrame;
use crate::law::{Op, Validity};
use crate::pinned;
use crate::{Compensated, CompiledBody, ExcursionSide, LawName, SimTime, ValidityExcursion};

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

#[derive(Clone, Copy)]
struct Dual {
    value: f64,
    slope: f64,
}

/// A smooth limit, with `sign` on its root. It is defined where the root is zero, and its derivative term
/// there is zero, the mean of the two one-sided derivatives, so a Newton step only loses speed.
fn smooth(a: Dual, b: Dual, width: Dual, sign: f64) -> Dual {
    let difference = a.value - b.value;
    let root = (difference * difference + width.value * width.value).sqrt();
    let term = if root == 0.0 {
        0.0
    } else {
        (difference * (a.slope - b.slope) + width.value * width.slope) / root
    };
    Dual {
        value: 0.5 * (a.value + b.value + sign * root),
        slope: 0.5 * (a.slope + b.slope + sign * term),
    }
}

fn run(ops: &[Op], inputs: &[Dual], partial_volume: f64) -> Result<Dual, OperatorDomain> {
    let mut stack: Vec<Dual> = Vec::with_capacity(ops.len());
    let mut next_input = 0;
    for op in ops {
        let at = |index: usize| stack[index];
        let result = match op {
            Op::Input(_) => {
                next_input += 1;
                inputs[next_input - 1]
            }
            Op::Zero(_) => Dual {
                value: 0.0,
                slope: 0.0,
            },
            Op::Add([a, b]) => Dual {
                value: at(*a).value + at(*b).value,
                slope: at(*a).slope + at(*b).slope,
            },
            Op::Subtract([a, b]) | Op::Deviation([a, b]) => Dual {
                value: at(*a).value - at(*b).value,
                slope: at(*a).slope - at(*b).slope,
            },
            Op::Multiply([a, b]) | Op::Scale([a, b]) | Op::ConductanceFlow([a, b]) => {
                let (a, b) = (at(*a), at(*b));
                Dual {
                    value: a.value * b.value,
                    slope: a.slope * b.value + a.value * b.slope,
                }
            }
            Op::Divide([a, b]) => {
                let (a, b) = (at(*a), at(*b));
                if b.value <= 0.0 {
                    return Err(OperatorDomain::Divisor);
                }
                Dual {
                    value: a.value / b.value,
                    slope: (a.slope * b.value - a.value * b.slope) / (b.value * b.value),
                }
            }
            Op::AsDimension([x]) => at(*x),
            Op::SmoothMax([a, b, width]) => smooth(at(*a), at(*b), at(*width), 1.0),
            Op::SmoothMin([a, b, width]) => smooth(at(*a), at(*b), at(*width), -1.0),
            Op::Exp([x]) => {
                let x = at(*x);
                let value = pinned::exp(x.value);
                Dual {
                    value,
                    slope: value * x.slope,
                }
            }
            Op::Logistic([x]) => {
                let x = at(*x);
                let value = 1.0 / (1.0 + pinned::exp(-x.value));
                Dual {
                    value,
                    slope: value * (1.0 - value) * x.slope,
                }
            }
            Op::Ln([x]) => {
                let x = at(*x);
                let value = pinned::ln(x.value).ok_or(OperatorDomain::LogarithmArgument)?;
                Dual {
                    value,
                    slope: x.slope / x.value,
                }
            }
            Op::Relax([target, current, speed]) => {
                let (target, current, speed) = (at(*target), at(*current), at(*speed));
                let gap = target.value - current.value;
                Dual {
                    value: speed.value * gap,
                    slope: speed.slope * gap + speed.value * (target.slope - current.slope),
                }
            }
            Op::Volume([amount]) => Dual {
                value: at(*amount).value * partial_volume,
                slope: at(*amount).slope * partial_volume,
            },
        };
        if !result.value.is_finite() {
            return Err(OperatorDomain::NonFinite);
        }
        stack.push(result);
    }
    Ok(stack[stack.len() - 1])
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
    fn read(&self, instance: &Instance, inputs: &Inputs<'_>, direction: Option<UnknownId>) -> Vec<Dual> {
        let read = |bound: &Bound| match *bound {
            Bound::Parameter(parameter) => Dual {
                value: inputs.parameters[parameter.index()],
                slope: 0.0,
            },
            Bound::Unknown(unknown) => Dual {
                value: inputs.unknowns[unknown.index()],
                slope: if direction == Some(unknown) { 1.0 } else { 0.0 },
            },
            Bound::Boundary(name) => Dual {
                value: inputs.frame.rate(name),
                slope: 0.0,
            },
            Bound::Draw(stream) => Dual {
                value: inputs.draws[stream],
                slope: 0.0,
            },
            Bound::Forcing(forcing) => Dual {
                value: inputs.factors[forcing],
                slope: 0.0,
            },
        };
        instance.inputs.iter().map(read).collect()
    }
    fn walk(&self, instance: &Instance, duals: &[Dual]) -> Result<Dual, Undefined> {
        let program = &self.layout.programs[instance.program.index()];
        run(&program.code.ops, duals, self.partial_volume()).map_err(|domain| Undefined {
            law: program.law,
            domain,
        })
    }
    pub(crate) fn instance_values(&self, inputs: &Inputs<'_>) -> Result<Vec<Option<f64>>, Undefined> {
        (self.layout.instances.iter())
            .map(|instance| {
                if !instance.holds(inputs.modes) {
                    return Ok(None);
                }
                let duals = self.read(instance, inputs, None);
                Ok(Some(self.walk(instance, &duals)?.value))
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
                Target::Row(row) => rows[row.index()] = positive_part(instance, value),
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
    pub(crate) fn jacobian(&self, inputs: &Inputs<'_>, shift: f64) -> Result<Vec<f64>, Undefined> {
        let sparsity = &self.layout.sparsity;
        let mut values = vec![0.0; sparsity.unknowns.len()];
        let entry = |row: usize, unknown: UnknownId| {
            let range = sparsity.offsets[row]..sparsity.offsets[row + 1];
            let found = sparsity.unknowns[range.clone()].binary_search(&unknown);
            range.start + found.expect("the compiled sparsity holds every unknown a row's laws read")
        };
        for instance in self
            .layout
            .instances
            .iter()
            .filter(|instance| instance.holds(inputs.modes))
        {
            let mut read: Vec<UnknownId> = (instance.inputs.iter()).filter_map(Bound::unknown).collect();
            read.sort_unstable();
            read.dedup();
            for direction in read {
                let duals = self.read(instance, inputs, Some(direction));
                let walked = self.walk(instance, &duals)?;
                let flat = positive_part(instance, walked.value) != walked.value;
                let slope = if flat { 0.0 } else { walked.slope };
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
                            values[entry(row.index(), direction)] -= coefficient as f64 * slope;
                        }
                    }
                    Target::Row(row) => {
                        let sign = match self.layout.rows[row.index()].kind {
                            RowKind::Algebraic(_) => 1.0,
                            RowKind::Amount { .. } | RowKind::Memory(_) => -1.0,
                        };
                        values[entry(row.index(), direction)] += sign * slope;
                    }
                }
            }
        }
        for (at, row) in self.layout.rows.iter().enumerate() {
            if !matches!(row.kind, RowKind::Algebraic(_)) {
                values[entry(at, row.unknown)] += shift;
            }
        }
        Ok(values)
    }
    pub(crate) fn excursions(&self, values: &[Option<f64>], at: SimTime) -> Vec<ValidityExcursion> {
        let mut excursions = Vec::new();
        for (instance, value) in self.layout.instances.iter().zip(values) {
            let Some(value) = *value else { continue };
            let program = &self.layout.programs[instance.program.index()];
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
