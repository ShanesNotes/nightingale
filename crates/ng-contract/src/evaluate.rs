use crate::body::{Bound, Instance, Layout, ParameterId, ProgramId, RowKind, Target, UnknownId};
use crate::boundary::DriveFrame;
use crate::law::{Op, Validity};
use crate::pinned;
use crate::{Compensated, CompiledBody, Crossing, ExcursionSide, LawName, SimTime, ValidityExcursion};
use core::ops::Range;

mod enclosure;
pub(crate) use enclosure::{Bounds, Jet};

pub(crate) struct GuardSlopes {
    pub(crate) unknowns: Vec<(usize, f64)>,
    pub(crate) parameters: Vec<(usize, f64)>,
}

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

#[derive(Clone, Copy)]
enum Instr {
    Zero,
    Add([usize; 2]),
    Subtract([usize; 2]),
    Multiply([usize; 2]),
    Divide([usize; 2]),
    Copy([usize; 1]),
    SmoothMax([usize; 3]),
    SmoothMin([usize; 3]),
    Exp([usize; 1]),
    Logistic([usize; 1]),
    Ln([usize; 1]),
    Relax([usize; 3]),
    Volume([usize; 1]),
    Upwind([usize; 5]),
}

/// A read is split by where its value comes from, so a parameter or unknown is copied from the caller's slice with no
/// dispatch.
pub(crate) struct Tape {
    programs: Vec<Kernel>,
    instances: Vec<ReadRanges>,
    guards: Vec<(Kernel, ReadRanges)>,
    steps: Vec<(usize, Instr)>,
    parameters: Vec<(usize, usize)>,
    unknowns: Vec<(usize, usize)>,
    others: Vec<(usize, Bound)>,
    entries: Vec<usize>,
    entry_ranges: Vec<Range<usize>>,
    diagonals: Vec<usize>,
    longest: usize,
}
struct Kernel {
    length: usize,
    steps: Range<usize>,
}
struct ReadRanges {
    parameters: Range<usize>,
    unknowns: Range<usize>,
    others: Range<usize>,
}
impl Tape {
    pub(crate) fn of(layout: &Layout) -> Self {
        let mut tape = Self {
            programs: Vec::new(),
            instances: Vec::new(),
            guards: Vec::new(),
            steps: Vec::new(),
            parameters: Vec::new(),
            unknowns: Vec::new(),
            others: Vec::new(),
            entries: Vec::new(),
            entry_ranges: Vec::new(),
            diagonals: Vec::new(),
            longest: 0,
        };
        for program in &layout.programs {
            let kernel = tape.lower(&program.code.ops);
            tape.programs.push(kernel);
        }
        let sparsity = &layout.sparsity;
        let entry = |row: usize, unknown: UnknownId| {
            let range = sparsity.offsets[row]..sparsity.offsets[row + 1];
            let found = sparsity.unknowns[range.clone()].binary_search(&unknown);
            range.start + found.expect("the compiled sparsity holds every unknown a row's laws read")
        };
        for instance in &layout.instances {
            let start = tape.entries.len();
            for unknown in instance.inputs.iter().filter_map(Bound::unknown) {
                match instance.target {
                    Target::Column(column) => {
                        for &(row, _) in &layout.columns[column.index()].terms {
                            tape.entries.push(entry(row.index(), unknown));
                        }
                    }
                    Target::Row(row) => tape.entries.push(entry(row.index(), unknown)),
                }
            }
            tape.entry_ranges.push(start..tape.entries.len());
            let ops = &layout.programs[instance.program.index()].code.ops;
            let reads = tape.bind(ops, &instance.inputs);
            tape.instances.push(reads);
        }
        for guard in &layout.guards {
            let kernel = tape.lower(&guard.code.ops);
            let reads = tape.bind(&guard.code.ops, &guard.inputs);
            tape.guards.push((kernel, reads));
        }
        tape.diagonals = (layout.rows.iter().enumerate())
            .filter(|(_, row)| !matches!(row.kind, RowKind::Algebraic(_)))
            .map(|(at, row)| entry(at, row.unknown))
            .collect();
        tape
    }
    fn lower(&mut self, ops: &[Op]) -> Kernel {
        let start = self.steps.len();
        for (at, op) in ops.iter().enumerate() {
            let instr = match op {
                Op::Input(_) => continue,
                Op::Zero(_) => Instr::Zero,
                Op::Add(operands) => Instr::Add(*operands),
                Op::Subtract(operands) | Op::Deviation(operands) => Instr::Subtract(*operands),
                Op::Multiply(operands) | Op::Scale(operands) | Op::ConductanceFlow(operands) => {
                    Instr::Multiply(*operands)
                }
                Op::Divide(operands) => Instr::Divide(*operands),
                Op::AsDimension(operand) => Instr::Copy(*operand),
                Op::SmoothMax(operands) => Instr::SmoothMax(*operands),
                Op::SmoothMin(operands) => Instr::SmoothMin(*operands),
                Op::Exp(operand) => Instr::Exp(*operand),
                Op::Logistic(operand) => Instr::Logistic(*operand),
                Op::Ln(operand) => Instr::Ln(*operand),
                Op::Relax(operands) => Instr::Relax(*operands),
                Op::Volume(operand) => Instr::Volume(*operand),
                Op::Upwind(operands) => Instr::Upwind(*operands),
            };
            self.steps.push((at, instr));
        }
        self.longest = self.longest.max(ops.len());
        Kernel {
            length: ops.len(),
            steps: start..self.steps.len(),
        }
    }
    fn bind(&mut self, ops: &[Op], inputs: &[Bound]) -> ReadRanges {
        let (parameters, unknowns, others) = (self.parameters.len(), self.unknowns.len(), self.others.len());
        let mut bounds = inputs.iter();
        for (at, _) in (ops.iter().enumerate()).filter(|(_, op)| matches!(op, Op::Input(_))) {
            match *bounds.next().expect("one bound per input") {
                Bound::Parameter(parameter) => self.parameters.push((at, parameter.index())),
                Bound::Unknown(unknown) => self.unknowns.push((at, unknown.index())),
                other => self.others.push((at, other)),
            }
        }
        ReadRanges {
            parameters: parameters..self.parameters.len(),
            unknowns: unknowns..self.unknowns.len(),
            others: others..self.others.len(),
        }
    }
    fn arithmetic(&self, kernel: &Kernel) -> Arithmetic<'_> {
        Arithmetic {
            length: kernel.length,
            steps: &self.steps[kernel.steps.clone()],
        }
    }
    fn reads(&self, ranges: &ReadRanges) -> Reads<'_> {
        Reads {
            parameters: &self.parameters[ranges.parameters.clone()],
            unknowns: &self.unknowns[ranges.unknowns.clone()],
            others: &self.others[ranges.others.clone()],
        }
    }
    fn program(&self, program: ProgramId) -> Arithmetic<'_> {
        self.arithmetic(&self.programs[program.index()])
    }
    fn instance(&self, at: usize) -> Reads<'_> {
        self.reads(&self.instances[at])
    }
    fn entries(&self, at: usize) -> &[usize] {
        &self.entries[self.entry_ranges[at].clone()]
    }
    fn guard(&self, guard: usize) -> (Arithmetic<'_>, Reads<'_>) {
        let (kernel, reads) = &self.guards[guard];
        (self.arithmetic(kernel), self.reads(reads))
    }
}

#[derive(Clone, Copy)]
struct Arithmetic<'t> {
    length: usize,
    steps: &'t [(usize, Instr)],
}

#[derive(Clone, Copy)]
struct Reads<'t> {
    parameters: &'t [(usize, usize)],
    unknowns: &'t [(usize, usize)],
    others: &'t [(usize, Bound)],
}
impl Reads<'_> {
    fn fill(&self, inputs: &Inputs<'_>, values: &mut [f64]) -> Option<(usize, Failure)> {
        let mut clean = true;
        for &(slot, index) in self.parameters {
            match inputs.parameters.get(index) {
                Some(&value) => {
                    values[slot] = value;
                    clean &= value.is_finite();
                }
                None => clean = false,
            }
        }
        for &(slot, index) in self.unknowns {
            match inputs.unknowns.get(index) {
                Some(&value) => {
                    values[slot] = value;
                    clean &= value.is_finite();
                }
                None => clean = false,
            }
        }
        for &(slot, bound) in self.others {
            match read(&bound, inputs) {
                Some(value) => {
                    values[slot] = value;
                    clean &= value.is_finite();
                }
                None => clean = false,
            }
        }
        (!clean).then(|| self.first_failure(inputs))
    }
    fn first_failure(&self, inputs: &Inputs<'_>) -> (usize, Failure) {
        let failure = |value: Option<f64>, bound: Bound| match value {
            None => Some(Failure::PastItsSlice(bound)),
            Some(value) if !value.is_finite() => Some(Failure::NotFinite),
            Some(_) => None,
        };
        let parameters = (self.parameters.iter()).find_map(|&(slot, index)| {
            let bound = Bound::Parameter(ParameterId::at(index));
            Some((slot, failure(inputs.parameters.get(index).copied(), bound)?))
        });
        let unknowns = (self.unknowns.iter()).find_map(|&(slot, index)| {
            let bound = Bound::Unknown(UnknownId::at(index));
            Some((slot, failure(inputs.unknowns.get(index).copied(), bound)?))
        });
        let others = (self.others.iter())
            .find_map(|&(slot, bound)| Some((slot, failure(read(&bound, inputs), bound)?)));
        (parameters.into_iter().chain(unknowns).chain(others))
            .min_by_key(|&(slot, _)| slot)
            .expect("a read that is not clean")
    }
}

#[derive(Clone, Copy)]
enum Failure {
    PastItsSlice(Bound),
    NotFinite,
}

fn read(bound: &Bound, inputs: &Inputs<'_>) -> Option<f64> {
    match *bound {
        Bound::Parameter(parameter) => inputs.parameters.get(parameter.index()).copied(),
        Bound::Unknown(unknown) => inputs.unknowns.get(unknown.index()).copied(),
        Bound::Draw(stream) => inputs.draws.get(stream).copied(),
        Bound::Forcing(forcing) => inputs.factors.get(forcing).copied(),
        Bound::Boundary(_) | Bound::Gauge | Bound::Height(_) => Some(CompiledBody::bound(bound, inputs)),
    }
}

fn apply(instr: Instr, values: &[f64], partial_volume: f64) -> Result<f64, OperatorDomain> {
    let at = |slot: usize| values[slot];
    let result = match instr {
        Instr::Zero => 0.0,
        Instr::Add([a, b]) => at(a) + at(b),
        Instr::Subtract([a, b]) => at(a) - at(b),
        Instr::Multiply([a, b]) => at(a) * at(b),
        Instr::Divide([a, b]) => {
            if at(b) <= 0.0 {
                return Err(OperatorDomain::Divisor);
            }
            at(a) / at(b)
        }
        Instr::Copy([x]) => at(x),
        Instr::SmoothMax([a, b, width]) => smooth(at(a), at(b), at(width), 1.0).0,
        Instr::SmoothMin([a, b, width]) => smooth(at(a), at(b), at(width), -1.0).0,
        Instr::Exp([x]) => pinned::exp(at(x)),
        Instr::Logistic([x]) => 1.0 / (1.0 + pinned::exp(-at(x))),
        Instr::Ln([x]) => pinned::ln(at(x)).ok_or(OperatorDomain::LogarithmArgument)?,
        Instr::Relax([target, current, speed]) => at(speed) * (at(target) - at(current)),
        Instr::Volume([amount]) => at(amount) * partial_volume,
        Instr::Upwind([flow, from, from_water, to, to_water]) => {
            let (amount, water) = if at(flow) >= 0.0 {
                (at(from), at(from_water))
            } else {
                (at(to), at(to_water))
            };
            if at(flow) == 0.0 {
                0.0
            } else if water <= 0.0 {
                return Err(OperatorDomain::Divisor);
            } else {
                at(flow) * (amount / water)
            }
        }
    };
    if !result.is_finite() {
        return Err(OperatorDomain::NonFinite);
    }
    Ok(result)
}

fn forward(
    arithmetic: Arithmetic<'_>,
    reads: Reads<'_>,
    inputs: &Inputs<'_>,
    partial_volume: f64,
    values: &mut [f64],
) -> Result<f64, OperatorDomain> {
    let failed = reads.fill(inputs, values);
    let steps = failed.map_or(arithmetic.steps, |(slot, _)| {
        &arithmetic.steps[..arithmetic.steps.partition_point(|&(at, _)| at < slot)]
    });
    for &(at, instr) in steps {
        let value = apply(instr, values, partial_volume)?;
        values[at] = value;
    }
    match failed {
        None => Ok(values[arithmetic.length - 1]),
        Some((_, Failure::NotFinite)) => Err(OperatorDomain::NonFinite),
        Some((_, Failure::PastItsSlice(bound))) => {
            CompiledBody::bound(&bound, inputs);
            unreachable!("a read past its slice panics in bound")
        }
    }
}

fn backward(arithmetic: Arithmetic<'_>, values: &[f64], partial_volume: f64, slopes: &mut [f64]) {
    slopes[..arithmetic.length].fill(0.0);
    slopes[arithmetic.length - 1] = 1.0;
    for &(at, instr) in arithmetic.steps.iter().rev() {
        let slope = slopes[at];
        if slope == 0.0 {
            continue;
        }
        match instr {
            Instr::Zero => {}
            Instr::Add([a, b]) => {
                slopes[a] += slope;
                slopes[b] += slope;
            }
            Instr::Subtract([a, b]) => {
                slopes[a] += slope;
                slopes[b] -= slope;
            }
            Instr::Multiply([a, b]) => {
                slopes[a] += slope * values[b];
                slopes[b] += slope * values[a];
            }
            Instr::Divide([a, b]) => {
                let divisor = values[b];
                slopes[a] += slope / divisor;
                slopes[b] -= slope * values[a] / (divisor * divisor);
            }
            Instr::Copy([x]) => slopes[x] += slope,
            Instr::SmoothMax([a, b, width]) | Instr::SmoothMin([a, b, width]) => {
                let sign = if matches!(instr, Instr::SmoothMax(_)) {
                    1.0
                } else {
                    -1.0
                };
                let (_, along, across) = smooth(values[a], values[b], values[width], sign);
                slopes[a] += slope * 0.5 * (1.0 + sign * along);
                slopes[b] += slope * 0.5 * (1.0 - sign * along);
                slopes[width] += slope * 0.5 * sign * across;
            }
            Instr::Exp([x]) => slopes[x] += slope * values[at],
            Instr::Logistic([x]) => slopes[x] += slope * values[at] * (1.0 - values[at]),
            Instr::Ln([x]) => slopes[x] += slope / values[x],
            Instr::Relax([target, current, speed]) => {
                let gap = values[target] - values[current];
                slopes[target] += slope * values[speed];
                slopes[current] -= slope * values[speed];
                slopes[speed] += slope * gap;
            }
            Instr::Volume([amount]) => slopes[amount] += slope * partial_volume,
            Instr::Upwind([flow, from, from_water, to, to_water]) => {
                let (amount, water) = if values[flow] >= 0.0 {
                    (from, from_water)
                } else {
                    (to, to_water)
                };
                if values[water] > 0.0 {
                    let share = values[amount] / values[water];
                    slopes[flow] += slope * share;
                    slopes[amount] += slope * values[flow] / values[water];
                    slopes[water] -= slope * values[flow] * share / values[water];
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

struct Swept<'a> {
    at: usize,
    instance: &'a Instance,
    reads: Reads<'a>,
    slopes: &'a [f64],
    flat: bool,
}
impl Swept<'_> {
    fn slope(&self, slot: usize) -> f64 {
        if self.flat { 0.0 } else { self.slopes[slot] }
    }
}

impl CompiledBody {
    fn partial_volume(&self) -> f64 {
        (self.layout.schema.partial_volume.as_ref()).map_or(0.0, |coefficient| coefficient.value())
    }
    pub(crate) fn guard_gap(&self, guard: usize, inputs: &Inputs<'_>) -> Result<f64, Undefined> {
        let layout = &self.layout.guards[guard];
        let (arithmetic, reads) = self.tape.guard(guard);
        let mut values = vec![0.0; arithmetic.length];
        let value =
            forward(arithmetic, reads, inputs, self.partial_volume(), &mut values).map_err(|domain| {
                let law = self.layout.modes[layout.variable].variable.law;
                Undefined { law, domain }
            })?;
        Ok(match layout.crossing {
            Crossing::Rising => value,
            Crossing::Falling => -value,
        })
    }
    fn guard_sign(&self, guard: usize) -> f64 {
        match self.layout.guards[guard].crossing {
            Crossing::Rising => 1.0,
            Crossing::Falling => -1.0,
        }
    }
    pub(crate) fn guard_slopes(&self, guard: usize, inputs: &Inputs<'_>) -> Result<GuardSlopes, Undefined> {
        let layout = &self.layout.guards[guard];
        let (arithmetic, reads) = self.tape.guard(guard);
        let (mut values, mut slopes) = (vec![0.0; arithmetic.length], vec![0.0; arithmetic.length]);
        let partial_volume = self.partial_volume();
        let undefined = |domain| Undefined {
            law: self.layout.modes[layout.variable].variable.law,
            domain,
        };
        forward(arithmetic, reads, inputs, partial_volume, &mut values).map_err(undefined)?;
        backward(arithmetic, &values, partial_volume, &mut slopes);
        let sign = self.guard_sign(guard);
        let finite = |(index, slope): (usize, f64)| {
            if slope.is_finite() {
                Ok((index, sign * slope))
            } else {
                Err(undefined(OperatorDomain::NonFinite))
            }
        };
        let unknowns = (reads.unknowns.iter()).map(|&(slot, index)| finite((index, slopes[slot])));
        let parameters = (reads.parameters.iter()).map(|&(slot, index)| finite((index, slopes[slot])));
        Ok(GuardSlopes {
            unknowns: unknowns.collect::<Result<_, _>>()?,
            parameters: parameters.collect::<Result<_, _>>()?,
        })
    }
    pub(crate) fn guard_enclosure(
        &self,
        guard: usize,
        inputs: &Inputs<'_>,
        unknown: &dyn Fn(usize) -> Jet,
    ) -> Option<Jet> {
        let (arithmetic, reads) = self.tape.guard(guard);
        let jet = enclosure::enclose(arithmetic, reads, inputs, unknown, self.partial_volume())?;
        Some(if self.guard_sign(guard) > 0.0 {
            jet
        } else {
            jet.negated()
        })
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
    fn undefined(&self, instance: &Instance, domain: OperatorDomain) -> Undefined {
        let law = self.layout.programs[instance.program.index()].law;
        Undefined { law, domain }
    }
    pub(crate) fn instance_values(&self, inputs: &Inputs<'_>) -> Result<Vec<Option<f64>>, Undefined> {
        let mut values = vec![0.0; self.tape.longest];
        let partial_volume = self.partial_volume();
        let mut out = Vec::with_capacity(self.layout.instances.len());
        for (at, instance) in self.layout.instances.iter().enumerate() {
            if !instance.holds(inputs.modes) {
                out.push(None);
                continue;
            }
            let (arithmetic, reads) = (self.tape.program(instance.program), self.tape.instance(at));
            let value = forward(arithmetic, reads, inputs, partial_volume, &mut values);
            out.push(Some(value.map_err(|domain| self.undefined(instance, domain))?));
        }
        Ok(out)
    }
    fn slopes(
        &self,
        inputs: &Inputs<'_>,
        mut each: impl FnMut(&Swept<'_>) -> Result<(), Undefined>,
    ) -> Result<(), Undefined> {
        let (mut values, mut slopes) = (vec![0.0; self.tape.longest], vec![0.0; self.tape.longest]);
        let partial_volume = self.partial_volume();
        for (at, instance) in self.layout.instances.iter().enumerate() {
            if !instance.holds(inputs.modes) {
                continue;
            }
            let (arithmetic, reads) = (self.tape.program(instance.program), self.tape.instance(at));
            let value = forward(arithmetic, reads, inputs, partial_volume, &mut values)
                .map_err(|domain| self.undefined(instance, domain))?;
            backward(arithmetic, &values, partial_volume, &mut slopes);
            let flat = positive_part(instance, value) != value;
            each(&Swept {
                at,
                instance,
                reads,
                slopes: &slopes,
                flat,
            })?;
        }
        Ok(())
    }
    pub(crate) fn jacobian(&self, inputs: &Inputs<'_>, shift: f64) -> Result<Vec<f64>, Undefined> {
        let mut values = vec![0.0; self.layout.sparsity.unknowns.len()];
        self.slopes(inputs, |swept| {
            let mut entries = self.tape.entries(swept.at).iter();
            let mut next = || *entries.next().expect("an entry per unknown read and term");
            for &(slot, _) in swept.reads.unknowns {
                let slope = swept.slope(slot);
                if !slope.is_finite() {
                    return Err(self.undefined(swept.instance, OperatorDomain::NonFinite));
                }
                match swept.instance.target {
                    Target::Column(column) => {
                        let column = &self.layout.columns[column.index()];
                        let slope = column.turnovers(slope);
                        for &(_, coefficient) in &column.terms {
                            values[next()] -= coefficient as f64 * slope;
                        }
                    }
                    Target::Row(row) => {
                        let sign = match self.layout.rows[row.index()].kind {
                            RowKind::Algebraic(_) => 1.0,
                            RowKind::Amount { .. } | RowKind::Memory(_) => -1.0,
                        };
                        values[next()] += sign * slope;
                    }
                }
            }
            Ok(())
        })?;
        for &entry in &self.tape.diagonals {
            values[entry] += shift;
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
        self.slopes(inputs, |swept| {
            let fitted = (swept.reads.parameters.iter())
                .filter(|&&(_, parameter)| slots.contains(&ParameterId::at(parameter)));
            let slope: f64 = fitted.map(|&(slot, _)| swept.slope(slot)).sum();
            match swept.instance.target {
                Target::Column(column) => {
                    let column = &self.layout.columns[column.index()];
                    let slope = column.turnovers(slope);
                    for &(row, coefficient) in &column.terms {
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
    pub(crate) fn targets(&self, values: &[Option<f64>]) -> (Vec<f64>, Vec<f64>) {
        let mut columns = vec![0.0; self.layout.columns.len()];
        let mut rows = vec![0.0; self.layout.rows.len()];
        for (instance, value) in self.layout.instances.iter().zip(values) {
            let Some(value) = *value else { continue };
            match instance.target {
                Target::Column(column) => {
                    columns[column.index()] = self.layout.columns[column.index()].turnovers(value);
                }
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
    use super::{Inputs, Instr};
    use crate::body::{Bound, Target};
    use crate::boundary::DriveFrame;
    use crate::law::Op;
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

    fn algebraic(local: &'static str) -> RowKind {
        RowKind::Algebraic(LawName { owner: "NS", local }.into())
    }

    fn reads_before_a_divide() -> (CompiledBody, BoundParameters) {
        let mut map = AnatomyBuilder::new();
        map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
        let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        let body = assemble(schema, |mut body| {
            let (mut laws, _ports) = body.nervous()?.parts();
            let x = laws.state::<Dimensionless>("x", Admissible::finite());
            let y = laws.state::<Dimensionless>("y", Admissible::finite());
            let z = laws.state::<Dimensionless>("z", Admissible::finite());
            let u = laws.state::<Dimensionless>("u", Admissible::finite());
            let v = laws.state::<Dimensionless>("v", Admissible::finite());
            let q = laws.algebraic::<Dimensionless>("q", Admissible::finite());
            let p = laws.parameter::<Dimensionless>(ParameterKey::new("ns.p"), Admissible::positive());
            let speed =
                laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::positive());
            let tail = laws.parameter::<Dimensionless>(ParameterKey::new("ns.tail"), Admissible::positive());
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
            let u_rate = laws.kernel(|mut k| {
                let rate = k.parameter(&speed);
                let held = k.state(&u);
                let numerator = k.parameter(&p);
                let divisor = k.state(&y);
                let quotient = k.divide(numerator, divisor);
                let quotient = k.as_dimension::<_, Dimensionless>(quotient);
                let late = k.parameter(&tail);
                let inner = k.add(quotient, late);
                let sum = k.add(held, inner);
                let scaled = k.scale(rate, sum);
                k.finish(scaled, laws.every_mode(), wave)
            });
            laws.memory_rate(&u, u_rate);
            let v_rate = laws.kernel(|mut k| {
                let rate = k.parameter(&speed);
                let numerator = k.parameter(&p);
                let divisor = k.state(&v);
                let quotient = k.divide(numerator, divisor);
                let quotient = k.as_dimension::<_, Dimensionless>(quotient);
                let nothing = k.subtract(quotient, quotient);
                let scaled = k.scale(rate, nothing);
                k.finish(scaled, laws.every_mode(), wave)
            });
            laws.memory_rate(&v, v_rate);
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
        let dimensionless = |value| measure::<Dimensionless>(value);
        (packet.synthetic(
            ParameterKey::new("ns.p"),
            Binding::Body,
            dimensionless(0.5),
            synthetic,
        ))
        .expect("once");
        let speed = measure::<Rate<Dimensionless>>(0.7);
        (packet.synthetic(ParameterKey::new("ns.speed"), Binding::Body, speed, synthetic)).expect("once");
        (packet.synthetic(
            ParameterKey::new("ns.tail"),
            Binding::Body,
            dimensionless(0.3),
            synthetic,
        ))
        .expect("once");
        for (local, start) in [("x", 0.4), ("y", 1.5), ("z", -0.75), ("u", 0.6), ("v", 2.0)] {
            (packet.start(memory(local), dimensionless(start), synthetic)).expect("once");
        }
        (packet.start(algebraic("q"), dimensionless(0.25), synthetic)).expect("once");
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
        let listed = |values: &[f64]| {
            values
                .iter()
                .map(|value| hex(*value))
                .collect::<Vec<_>>()
                .join(",")
        };
        let values = match body.instance_values(&inputs) {
            Ok(values) => (values.iter())
                .map(|value| value.map_or_else(|| "none".to_string(), hex))
                .collect::<Vec<_>>()
                .join(","),
            Err(undefined) => format!("{undefined:?}"),
        };
        let jacobian = match body.jacobian(&inputs, 2.0) {
            Ok(values) => listed(&values),
            Err(undefined) => format!("{undefined:?}"),
        };
        let fits = (0..parameters.len()).map(|slot| {
            match body.parameter_slopes(&inputs, &[ParameterId::at(slot)]) {
                Ok(values) => listed(&values),
                Err(undefined) => format!("{undefined:?}"),
            }
        });
        format!(
            "values {values}; jacobian {jacobian}; fits {}",
            fits.collect::<Vec<_>>().join(" | ")
        )
    }

    fn failed(law: &str, domain: &str) -> String {
        format!("values Undefined {{ law: LawName {{ owner: \"NS\", local: \"{law}\" }}, domain: {domain} }}")
    }

    #[test]
    fn a_value_that_is_not_finite_fails_at_the_read_that_takes_it() {
        let (body, bound) = reads_before_a_divide();
        let rows = body.rows();
        let slot = |kind: RowKind| rows.iter().position(|row| row.kind() == kind).expect("a row");
        let (u, v, y, q) = (
            slot(memory("u")),
            slot(memory("v")),
            slot(memory("y")),
            slot(algebraic("q")),
        );
        let start: Vec<f64> = rows.iter().map(|row| bound.starts[row.id().index()]).collect();
        let (modes, parameters) = (&bound.modes, &bound.values);
        let tail = parameters.len() - 1;
        let mut trace = String::new();
        let mut named = |label: &str, parameters: &[f64], unknowns: &[f64]| {
            let seen = outcome(&body, modes, parameters, unknowns);
            writeln!(trace, "{label}: {seen}").expect("a string takes it");
            seen
        };
        named("start", parameters, &start);
        for at in 0..start.len() {
            for value in NOT_FINITE_OR_EDGE {
                let mut unknowns = start.clone();
                unknowns[at] = value;
                named(
                    &format!("unknown {at} {:016x}", value.to_bits()),
                    parameters,
                    &unknowns,
                );
            }
            for other in (0..start.len()).filter(|other| *other != at) {
                let mut unknowns = start.clone();
                (unknowns[at], unknowns[other]) = (f64::NAN, -1.0);
                named(&format!("unknown {at} nan, {other} -1"), parameters, &unknowns);
            }
        }
        for at in 0..parameters.len() {
            for value in NOT_FINITE_OR_EDGE {
                let mut changed = parameters.clone();
                changed[at] = value;
                named(
                    &format!("parameter {at} {:016x}", value.to_bits()),
                    &changed,
                    &start,
                );
            }
        }
        let mut unknowns = start.clone();
        (unknowns[u], unknowns[y]) = (f64::NAN, -1.0);
        let seen = named("a read before a failing divide", parameters, &unknowns);
        assert!(seen.starts_with(&failed("u", "NonFinite")), "{seen}");
        let (mut changed, mut unknowns) = (parameters.clone(), start.clone());
        (changed[tail], unknowns[y]) = (f64::NAN, -1.0);
        let seen = named("a failing divide before a read", &changed, &unknowns);
        assert!(seen.starts_with(&failed("u", "Divisor")), "{seen}");
        unknowns[u] = f64::NAN;
        let seen = named("the lowest failing read, whatever its kind", &changed, &unknowns);
        assert!(seen.starts_with(&failed("u", "NonFinite")), "{seen}");
        let mut unknowns = start.clone();
        unknowns[q] = f64::NAN;
        let seen = named("a bare read", parameters, &unknowns);
        assert!(seen.starts_with(&failed("q", "NonFinite")), "{seen}");
        let mut unknowns = start.clone();
        unknowns[v] = 1e-200;
        let seen = named(
            "a zero slope over a divisor that squares to zero",
            parameters,
            &unknowns,
        );
        assert!(!seen.contains("jacobian Undefined"), "{seen}");
        let mut unknowns = start.clone();
        unknowns[y] = -1.0;
        let seen = named(
            "a missing read after a failing divide",
            &parameters[..tail],
            &unknowns,
        );
        assert!(seen.starts_with(&failed("u", "Divisor")), "{seen}");
        let digest = Digest::of_canonical(trace.as_bytes());
        let hex: String = digest
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex, "8453b3441150be769319c476693feb6df57840ddda2117f3954bd6b7158db5d8",
            "{trace}"
        );
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn a_missing_read_with_nothing_failing_before_it_panics() {
        let (body, bound) = reads_before_a_divide();
        let start: Vec<f64> = body
            .rows()
            .iter()
            .map(|row| bound.starts[row.id().index()])
            .collect();
        let frame = DriveFrame::default();
        let inputs = Inputs {
            parameters: &bound.values[..bound.values.len() - 1],
            frame: &frame,
            unknowns: &start,
            modes: &bound.modes,
            draws: &[],
            factors: &[],
        };
        let _ = body.instance_values(&inputs);
    }
    fn switching() -> CompiledBody {
        let mut map = AnatomyBuilder::new();
        map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
        let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        assemble(schema, |mut body| {
            let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
            let (mut ns, _) = body.nervous()?.parts();
            let x = ns.state::<Dimensionless>("x", Admissible::finite());
            let threshold =
                ns.parameter::<Dimensionless>(ParameterKey::new("threshold"), Admissible::positive());
            let speed =
                ns.parameter::<Rate<Dimensionless>>(ParameterKey::new("speed"), Admissible::positive());
            let [a, b] = ns.modes("switch", ["a", "b"]);
            for mode in [&a, &b] {
                let rate = ns.kernel(|mut k| {
                    let (zero, current, speed) = (k.zero(), k.state(&x), k.parameter(&speed));
                    let rate = k.relax(zero, current, speed);
                    k.finish(rate, mode.only(), class)
                });
                ns.memory_rate(&x, rate);
            }
            let entering = ns.kernel(|mut k| {
                let (value, threshold) = (k.state(&x), k.parameter(&threshold));
                let gap = k.subtract(value, threshold);
                k.finish(gap, a.only(), class)
            });
            ns.event(Guard::new(entering, Crossing::Rising, a.to(&b)?));
            let leaving = ns.kernel(|mut k| {
                let (zero, value, threshold) = (k.zero(), k.state(&x), k.parameter(&threshold));
                let negative = k.subtract(zero, value);
                let gap = k.subtract(negative, threshold);
                k.finish(gap, b.only(), class)
            });
            ns.event(Guard::new(leaving, Crossing::Rising, b.to(&a)?));
            body.add(ns.finish());
            body.compile()
        })
        .expect("compiled")
    }

    fn lowered(op: &Op) -> Option<(&'static str, Vec<usize>)> {
        Some(match op {
            Op::Input(_) => return None,
            Op::Zero(_) => ("zero", Vec::new()),
            Op::Add(operands) => ("add", operands.to_vec()),
            Op::Subtract(operands) | Op::Deviation(operands) => ("subtract", operands.to_vec()),
            Op::Multiply(operands) | Op::Scale(operands) | Op::ConductanceFlow(operands) => {
                ("multiply", operands.to_vec())
            }
            Op::Divide(operands) => ("divide", operands.to_vec()),
            Op::AsDimension(operand) => ("copy", operand.to_vec()),
            Op::SmoothMax(operands) => ("smooth max", operands.to_vec()),
            Op::SmoothMin(operands) => ("smooth min", operands.to_vec()),
            Op::Exp(operand) => ("exp", operand.to_vec()),
            Op::Logistic(operand) => ("logistic", operand.to_vec()),
            Op::Ln(operand) => ("ln", operand.to_vec()),
            Op::Relax(operands) => ("relax", operands.to_vec()),
            Op::Volume(operand) => ("volume", operand.to_vec()),
            Op::Upwind(operands) => ("upwind", operands.to_vec()),
        })
    }

    fn named(instr: Instr) -> (&'static str, Vec<usize>) {
        match instr {
            Instr::Zero => ("zero", Vec::new()),
            Instr::Add(operands) => ("add", operands.to_vec()),
            Instr::Subtract(operands) => ("subtract", operands.to_vec()),
            Instr::Multiply(operands) => ("multiply", operands.to_vec()),
            Instr::Divide(operands) => ("divide", operands.to_vec()),
            Instr::Copy(operand) => ("copy", operand.to_vec()),
            Instr::SmoothMax(operands) => ("smooth max", operands.to_vec()),
            Instr::SmoothMin(operands) => ("smooth min", operands.to_vec()),
            Instr::Exp(operand) => ("exp", operand.to_vec()),
            Instr::Logistic(operand) => ("logistic", operand.to_vec()),
            Instr::Ln(operand) => ("ln", operand.to_vec()),
            Instr::Relax(operands) => ("relax", operands.to_vec()),
            Instr::Volume(operand) => ("volume", operand.to_vec()),
            Instr::Upwind(operands) => ("upwind", operands.to_vec()),
        }
    }

    fn reads_back(body: &CompiledBody) {
        let (layout, tape) = (&body.layout, &body.tape);
        assert_eq!(tape.programs.len(), layout.programs.len());
        assert_eq!(tape.instances.len(), layout.instances.len());
        assert_eq!(tape.guards.len(), layout.guards.len());
        let programs = (layout.programs.iter().map(|program| &program.code)).zip(&tape.programs);
        let guards =
            (layout.guards.iter().map(|guard| &guard.code)).zip(tape.guards.iter().map(|(kernel, _)| kernel));
        for (code, kernel) in programs.chain(guards) {
            let kernel = tape.arithmetic(kernel);
            assert_eq!(kernel.length, code.ops.len());
            let expected: Vec<_> = (code.ops.iter().enumerate())
                .filter_map(|(slot, op)| Some((slot, lowered(op)?)))
                .collect();
            let found: Vec<_> = kernel
                .steps
                .iter()
                .map(|&(slot, instr)| (slot, named(instr)))
                .collect();
            assert_eq!(found, expected);
        }
        let instances = (layout.instances.iter())
            .map(|instance| {
                (
                    &layout.programs[instance.program.index()].code.ops,
                    &instance.inputs,
                )
            })
            .zip(&tape.instances);
        let guards = (layout.guards.iter().map(|guard| (&guard.code.ops, &guard.inputs)))
            .zip(tape.guards.iter().map(|(_, reads)| reads));
        for ((ops, inputs), ranges) in instances.chain(guards) {
            let reads = tape.reads(ranges);
            let parameters = (reads.parameters.iter())
                .map(|&(slot, index)| (slot, Bound::Parameter(ParameterId::at(index))));
            let unknowns =
                (reads.unknowns.iter()).map(|&(slot, index)| (slot, Bound::Unknown(UnknownId::at(index))));
            let mut found: Vec<(usize, Bound)> = parameters
                .chain(unknowns)
                .chain(reads.others.iter().copied())
                .collect();
            found.sort_by_key(|&(slot, _)| slot);
            let slots = (ops.iter().enumerate()).filter(|(_, op)| matches!(op, Op::Input(_)));
            let expected: Vec<(usize, Bound)> =
                slots.map(|(slot, _)| slot).zip(inputs.iter().copied()).collect();
            assert_eq!(found, expected);
        }
        let sparsity = &layout.sparsity;
        let holds = |entry: usize, row: usize, unknown: UnknownId| {
            (sparsity.offsets[row]..sparsity.offsets[row + 1]).contains(&entry)
                && sparsity.unknowns[entry] == unknown
        };
        for (at, instance) in layout.instances.iter().enumerate() {
            let targets: Vec<usize> = match instance.target {
                Target::Column(column) => (layout.columns[column.index()].terms.iter())
                    .map(|(row, _)| row.index())
                    .collect(),
                Target::Row(row) => vec![row.index()],
            };
            let reads = instance.inputs.iter().filter_map(Bound::unknown);
            let expected: Vec<(usize, UnknownId)> = reads
                .flat_map(|unknown| targets.iter().map(move |&row| (row, unknown)))
                .collect();
            let entries = tape.entries(at);
            assert_eq!(entries.len(), expected.len());
            for (&entry, &(row, unknown)) in entries.iter().zip(&expected) {
                assert!(
                    holds(entry, row, unknown),
                    "instance {at}: entry {entry} is not row {row}'s {unknown:?}"
                );
            }
        }
        let differential =
            (layout.rows.iter().enumerate()).filter(|(_, row)| !matches!(row.kind, RowKind::Algebraic(_)));
        let expected: Vec<(usize, UnknownId)> = differential.map(|(at, row)| (at, row.unknown)).collect();
        assert_eq!(tape.diagonals.len(), expected.len());
        for (&entry, &(row, unknown)) in tape.diagonals.iter().zip(&expected) {
            assert!(
                holds(entry, row, unknown),
                "diagonal {entry} is not row {row}'s own entry"
            );
        }
    }

    #[test]
    fn the_tape_reads_back_its_layout() {
        reads_back(&reads_before_a_divide().0);
        let switching = switching();
        assert_eq!(switching.layout.guards.len(), 2);
        reads_back(&switching);
    }

    const STORE_SPEED: f64 = 0.001;

    fn stored() -> (CompiledBody, BoundParameters) {
        let origin = || Position::new(measure(0.0), measure(0.0), measure(0.0));
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", origin()).id();
        let vessel = map.site::<VesselSegment>("vessel", origin()).id();
        let bed = map.site::<CapillaryBed>("bed", origin()).id();
        map.contain(root, vessel);
        map.contain(root, bed);
        map.connect(Network::Vascular, vessel, bed);
        map.connect(Network::Vascular, bed, vessel);
        let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        for space in [Space::Plasma, Space::Interstitium, Space::Lumen] {
            schema.leave_out(space, "the bench reacts in the cells alone");
        }
        schema.carry::<Glucose, CapillaryBed>(Space::Cells);
        schema.carry::<Glycogen, CapillaryBed>(Space::Cells);
        schema.carry::<Oxygen, CapillaryBed>(Space::Cells);
        schema.carry::<CarbonDioxide, CapillaryBed>(Space::Cells);
        schema.carry::<Energy, CapillaryBed>(Space::Thermal);
        (schema.energy_content::<Glucose>(measure(1_000.0))).expect("a content");
        (schema.energy_content::<Glycogen>(measure(900.0))).expect("a content");
        let body = assemble(schema, |mut body| {
            let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), "synthetic");
            let (mut laws, _) = body.hold(held)?.parts();
            let class = RateClass::new(RateBand::Minutes, 10_000_000_000)?;
            let stored = Proportions::<Glycogen, 19>::making(Space::Cells)
                .uses::<Glucose, 20>(Space::Cells)
                .uses::<Oxygen, 6>(Space::Cells)
                .makes::<CarbonDioxide, 6>(Space::Cells)
                .makes::<Water, 25>(Space::Cells)
                .energy_into(Space::Thermal);
            let store = laws.reaction::<CapillaryBed, Glycogen, 19>("store", stored);
            let speed = laws.site_parameter::<CapillaryBed, Quotient<Rate<Glycogen>, Glucose>>(
                ParameterKey::new("mt.store-speed"),
                Admissible::nonnegative(),
            );
            let fuel = store.amount::<Glucose>();
            let rate = laws.site_kernel(|mut k| {
                let (fuel, speed) = (k.at_site(&fuel), k.site_parameter(&speed));
                let rate = k.multiply(speed, fuel);
                let rate = k.as_dimension::<_, Rate<Glycogen>>(rate);
                k.finish(rate, laws.every_mode(), class)
            });
            laws.react_at(&store, rate);
            let burnt = Proportions::<Glucose>::using(Space::Cells)
                .uses::<Oxygen, 6>(Space::Cells)
                .makes::<CarbonDioxide, 6>(Space::Cells)
                .makes::<Water, 6>(Space::Cells)
                .energy_into(Space::Thermal);
            let burn = laws.reaction::<CapillaryBed, Glucose, 1>("burn", burnt);
            let still = laws.site_parameter::<CapillaryBed, Rate<Glucose>>(
                ParameterKey::new("mt.burn-rate"),
                Admissible::nonnegative(),
            );
            let rate = laws.site_kernel(|mut k| {
                let still = k.site_parameter(&still);
                k.finish(still, laws.every_mode(), class)
            });
            laws.react_at(&burn, rate);
            body.add(laws.finish());
            body.compile()
        })
        .expect("compiled");
        let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
        let beds = Binding::Class(Class::CapillaryBed);
        let speed = measure::<Quotient<Rate<Glycogen>, Glucose>>(STORE_SPEED);
        (packet.synthetic(ParameterKey::new("mt.store-speed"), beds, speed, "synthetic")).expect("once");
        let still = measure::<Rate<Glucose>>(0.0);
        (packet.synthetic(ParameterKey::new("mt.burn-rate"), beds, still, "synthetic")).expect("once");
        (packet.start_rest(&body, |_| 100.0, "synthetic")).expect("a start for each row");
        let bound = body.bind(packet).expect("bound");
        (body, bound)
    }

    fn still_residuals(
        body: &CompiledBody,
        modes: &[usize],
        parameters: &[f64],
        unknowns: &[f64],
    ) -> Vec<f64> {
        let frame = DriveFrame::default();
        let inputs = Inputs {
            parameters,
            frame: &frame,
            unknowns,
            modes,
            draws: &[],
            factors: &[],
        };
        let values = body.instance_values(&inputs).expect("defined");
        let (columns, rows) = body.targets(&values);
        body.residuals(&columns, &rows, &vec![0.0; unknowns.len()])
    }

    #[test]
    fn the_slopes_of_a_reaction_counting_its_reference_nineteen_times_match_central_differences() {
        let (body, bound) = stored();
        let (modes, parameters) = (&bound.modes, &bound.values);
        let start: Vec<f64> = body
            .rows()
            .iter()
            .map(|row| bound.starts[row.id().index()])
            .collect();
        let row_of = |wanted: Species| {
            (body.rows().iter())
                .position(|row| matches!(row.kind(), RowKind::Amount { species, .. } if species == wanted))
                .expect("a pool at the bed")
        };
        let (glucose, glycogen) = (row_of(Species::Glucose), row_of(Species::Glycogen));
        let speed = (body.parameters().iter())
            .position(|slot| slot.key() == ParameterKey::new("mt.store-speed"))
            .expect("the store's speed");
        let frame = DriveFrame::default();
        let inputs = Inputs {
            parameters,
            frame: &frame,
            unknowns: &start,
            modes,
            draws: &[],
            factors: &[],
        };
        let jacobian = body.jacobian(&inputs, 0.0).expect("defined");
        let slopes = (body.parameter_slopes(&inputs, &[ParameterId::at(speed)])).expect("defined");
        let sparsity = body.sparsity();
        let entry = |row: usize| {
            (sparsity.offsets[row]..sparsity.offsets[row + 1])
                .find(|&at| sparsity.unknowns[at] == UnknownId::at(glucose))
                .map_or(0.0, |at| jacobian[at])
        };
        let near = |difference: f64, slope: f64| (difference - slope).abs() <= 1e-9 * difference.abs();
        let nudged = |values: &[f64], at: usize, by: f64| {
            let mut nudged = values.to_vec();
            nudged[at] += by;
            nudged
        };
        let (more, less) = (nudged(&start, glucose, 1.0), nudged(&start, glucose, -1.0));
        let (up, down) = (
            still_residuals(&body, modes, parameters, &more),
            still_residuals(&body, modes, parameters, &less),
        );
        for row in 0..start.len() {
            let difference = (up[row] - down[row]) / 2.0;
            assert!(
                near(difference, entry(row)),
                "row {row} against glucose: a central difference of {difference}, an entry of {}",
                entry(row)
            );
        }
        let step = STORE_SPEED / 10.0;
        let (faster, slower) = (nudged(parameters, speed, step), nudged(parameters, speed, -step));
        let (up, down) = (
            still_residuals(&body, modes, &faster, &start),
            still_residuals(&body, modes, &slower, &start),
        );
        for row in 0..start.len() {
            let difference = (up[row] - down[row]) / (2.0 * step);
            assert!(
                near(difference, slopes[row]),
                "row {row} against the speed: a central difference of {difference}, a slope of {}",
                slopes[row]
            );
        }
        assert!(
            (entry(glycogen) + STORE_SPEED).abs() <= 1e-12 * STORE_SPEED,
            "glycogen is made at the speed per glucose held, not {}",
            -entry(glycogen)
        );
    }
}
