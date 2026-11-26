use crate::body::{Bound, Instance, Layout, ParameterId, ProgramId, RowKind, Target, UnknownId};
use crate::boundary::DriveFrame;
use crate::law::{Op, Validity};
use crate::pinned;
use crate::{
    Compensated, CompiledBody, Context, Crossing, ExcursionSide, LawName, SimTime, ValidityExcursion,
};
use core::ops::{Deref, DerefMut, Index, IndexMut, Range};

mod enclosure;
#[cfg(test)]
mod terms;
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

const STACK_SLOTS: usize = 256;

struct Slots<'a>(&'a [f64]);
impl Index<u16> for Slots<'_> {
    type Output = f64;
    #[inline(always)]
    fn index(&self, slot: u16) -> &f64 {
        &self.0[usize::from(slot)]
    }
}
struct SlotsMut<'a>(&'a mut [f64]);
impl Index<u16> for SlotsMut<'_> {
    type Output = f64;
    #[inline(always)]
    fn index(&self, slot: u16) -> &f64 {
        &self.0[usize::from(slot)]
    }
}
impl IndexMut<u16> for SlotsMut<'_> {
    #[inline(always)]
    fn index_mut(&mut self, slot: u16) -> &mut f64 {
        &mut self.0[usize::from(slot)]
    }
}
struct Scratch(Vec<f64>);
impl Scratch {
    fn zeros(length: usize) -> Self {
        Self(vec![0.0; length])
    }
}
impl Index<u16> for Scratch {
    type Output = f64;
    #[inline(always)]
    fn index(&self, slot: u16) -> &f64 {
        &self.0[usize::from(slot)]
    }
}
impl Deref for Scratch {
    type Target = [f64];
    fn deref(&self) -> &[f64] {
        &self.0
    }
}
impl DerefMut for Scratch {
    fn deref_mut(&mut self) -> &mut [f64] {
        &mut self.0
    }
}

#[derive(Clone, Copy)]
enum Instr {
    Zero,
    Add([u16; 2]),
    Subtract([u16; 2]),
    Multiply([u16; 2]),
    Divide([u16; 2]),
    Copy([u16; 1]),
    SmoothMax([u16; 3]),
    SmoothMin([u16; 3]),
    Exp([u16; 1]),
    Logistic([u16; 1]),
    Ln([u16; 1]),
    Relax([u16; 3]),
    Volume([u16; 1]),
    Upwind([u16; 5]),
    Expm1([u16; 1]),
}

#[derive(Clone, Copy)]
enum FastInstr {
    Zero,
    Add([u8; 2]),
    Subtract([u8; 2]),
    Multiply([u8; 2]),
    Divide([u8; 2]),
    SmoothMax([u8; 3]),
    SmoothMin([u8; 3]),
    Exp([u8; 1]),
    Logistic([u8; 1]),
    Ln([u8; 1]),
    Relax([u8; 3]),
    Volume([u8; 1]),
    Upwind([u8; 5]),
    Expm1([u8; 1]),
}

#[derive(Clone, Copy)]
struct FastGeneral {
    steps_start: u32,
    steps_end: u32,
    param_start: u32,
    param_end: u32,
    unknown_start: u32,
    unknown_end: u32,
    other_start: u32,
    other_end: u32,
    output: u8,
}

#[derive(Clone, Copy)]
enum FastKind {
    Upwind5([u16; 5]),
    ReadUnknown(u16),
    ReadParameter(u16),
    General(u32),
}

#[derive(Clone, Copy)]
struct FastInstance {
    kind: FastKind,
    mode_var: u16,
    mode_val: u16,
}
impl FastInstance {
    #[inline(always)]
    fn holds(&self, modes: &[usize]) -> bool {
        self.mode_var == u16::MAX || modes[usize::from(self.mode_var)] == usize::from(self.mode_val)
    }
}

struct FastTape {
    instances: Vec<FastInstance>,
    generals: Vec<FastGeneral>,
    steps: Vec<(u8, FastInstr)>,
    parameters: Vec<(u8, u16)>,
    unknowns: Vec<(u8, u16)>,
    judged_ops: Vec<u8>,
    judged_ranges: Vec<Range<u32>>,
}

#[derive(Clone, Copy)]
struct ColumnTerm {
    column: u32,
    row: u32,
    coefficient: f64,
}

/// A read is split by where its value comes from, so a parameter or unknown is copied from the caller's slice with no
/// dispatch.
pub(crate) struct Tape {
    programs: Vec<Kernel>,
    instances: Vec<ReadRanges>,
    guards: Vec<(Kernel, ReadRanges)>,
    steps: Vec<(u16, Instr)>,
    parameters: Vec<(u16, u32)>,
    unknowns: Vec<(u16, u32)>,
    others: Vec<(u16, Bound)>,
    entries: Vec<u32>,
    entry_ranges: Vec<Range<u32>>,
    diagonals: Vec<u32>,
    longest: usize,
    parameters_needed: usize,
    unknowns_needed: usize,
    draws_needed: usize,
    factors_needed: usize,
    judged_total: usize,
    amounts_end: usize,
    memory_end: usize,
    contiguous_rows: bool,
    column_terms: Vec<ColumnTerm>,
    fast: Option<FastTape>,
}
struct Kernel {
    length: u16,
    steps: Range<u32>,
}
struct ReadRanges {
    parameters: Range<u32>,
    unknowns: Range<u32>,
    others: Range<u32>,
}
fn slot16(index: usize) -> u16 {
    u16::try_from(index).expect("a kernel fits in u16 slots")
}
fn range_u32(range: Range<usize>) -> Range<u32> {
    let start = u32::try_from(range.start).expect("tape offsets fit in u32");
    let end = u32::try_from(range.end).expect("tape offsets fit in u32");
    start..end
}
fn slice_range(range: &Range<u32>) -> Range<usize> {
    range.start as usize..range.end as usize
}
impl Tape {
    pub(crate) fn of(layout: &Layout) -> Self {
        let mut tape = Self {
            programs: Vec::with_capacity(layout.programs.len()),
            instances: Vec::with_capacity(layout.instances.len()),
            guards: Vec::with_capacity(layout.guards.len()),
            steps: Vec::new(),
            parameters: Vec::new(),
            unknowns: Vec::new(),
            others: Vec::new(),
            entries: Vec::new(),
            entry_ranges: Vec::with_capacity(layout.instances.len()),
            diagonals: Vec::new(),
            longest: 0,
            parameters_needed: 0,
            unknowns_needed: 0,
            draws_needed: 0,
            factors_needed: 0,
            judged_total: 0,
            amounts_end: 0,
            memory_end: 0,
            contiguous_rows: false,
            column_terms: Vec::new(),
            fast: None,
        };
        for program in &layout.programs {
            let kernel = tape.lower(&program.code.ops);
            tape.programs.push(kernel);
        }
        let sparsity = &layout.sparsity;
        let entry = |row: usize, unknown: UnknownId| {
            let range = sparsity.offsets[row]..sparsity.offsets[row + 1];
            let found = sparsity.unknowns[range.clone()].binary_search(&unknown);
            let at =
                range.start + found.expect("the compiled sparsity holds every unknown a row's laws read");
            u32::try_from(at).expect("sparsity entries fit in u32")
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
            tape.entry_ranges.push(range_u32(start..tape.entries.len()));
            let program = &layout.programs[instance.program.index()];
            tape.judged_total += program.code.judged.len();
            let reads = tape.bind(&program.code.ops, &instance.inputs);
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
        tape.amounts_end = layout
            .rows
            .iter()
            .take_while(|row| matches!(row.kind, RowKind::Amount { .. }))
            .count();
        tape.memory_end = tape.amounts_end
            + layout.rows[tape.amounts_end..]
                .iter()
                .take_while(|row| matches!(row.kind, RowKind::Memory(_)))
                .count();
        tape.contiguous_rows = layout.rows[tape.memory_end..]
            .iter()
            .all(|row| matches!(row.kind, RowKind::Algebraic(_)));
        for (column, col) in layout.columns.iter().enumerate() {
            let column = u32::try_from(column).expect("columns fit in u32");
            for &(row, coefficient) in &col.terms {
                tape.column_terms.push(ColumnTerm {
                    column,
                    row: u32::try_from(row.index()).expect("rows fit in u32"),
                    coefficient: coefficient as f64,
                });
            }
        }
        tape.fast = tape.build_fast(layout);
        tape
    }
    fn build_fast(&self, layout: &Layout) -> Option<FastTape> {
        if self.longest > STACK_SLOTS
            || self.parameters_needed > usize::from(u16::MAX) + 1
            || self.unknowns_needed > usize::from(u16::MAX) + 1
        {
            return None;
        }
        let mut steps = Vec::new();
        let mut program_fast = Vec::with_capacity(layout.programs.len());
        let mut program_alias = Vec::with_capacity(layout.programs.len());
        for program in &layout.programs {
            let ops = &program.code.ops;
            let mut alias = Vec::with_capacity(ops.len());
            let start = steps.len();
            for (at, op) in ops.iter().enumerate() {
                let at_u8 = at as u8;
                match *op {
                    Op::Input(_) => alias.push(at_u8),
                    Op::AsDimension([x]) => alias.push(alias[x]),
                    _ => {
                        alias.push(at_u8);
                        let instr = match *op {
                            Op::Zero(_) => FastInstr::Zero,
                            Op::Add([a, b]) => FastInstr::Add([alias[a], alias[b]]),
                            Op::Subtract([a, b]) | Op::Deviation([a, b]) => {
                                FastInstr::Subtract([alias[a], alias[b]])
                            }
                            Op::Multiply([a, b]) | Op::Scale([a, b]) | Op::ConductanceFlow([a, b]) => {
                                FastInstr::Multiply([alias[a], alias[b]])
                            }
                            Op::Divide([a, b]) => FastInstr::Divide([alias[a], alias[b]]),
                            Op::SmoothMax([a, b, w]) => FastInstr::SmoothMax([alias[a], alias[b], alias[w]]),
                            Op::SmoothMin([a, b, w]) => FastInstr::SmoothMin([alias[a], alias[b], alias[w]]),
                            Op::Exp([x]) => FastInstr::Exp([alias[x]]),
                            Op::Logistic([x]) => FastInstr::Logistic([alias[x]]),
                            Op::Ln([x]) => FastInstr::Ln([alias[x]]),
                            Op::Relax([a, b, c]) => FastInstr::Relax([alias[a], alias[b], alias[c]]),
                            Op::Volume([x]) => FastInstr::Volume([alias[x]]),
                            Op::Upwind([a, b, c, d, e]) => {
                                FastInstr::Upwind([alias[a], alias[b], alias[c], alias[d], alias[e]])
                            }
                            Op::Expm1([x]) => FastInstr::Expm1([alias[x]]),
                            Op::Input(_) | Op::AsDimension(_) => unreachable!(),
                        };
                        steps.push((at_u8, instr));
                    }
                }
            }
            let output = *alias.last().expect("a program has an output");
            program_fast.push((start as u32, steps.len() as u32, output));
            program_alias.push(alias);
        }
        let mut instances = Vec::with_capacity(layout.instances.len());
        let mut generals = Vec::new();
        let mut parameters = Vec::new();
        let mut unknowns = Vec::new();
        let mut judged_ops = Vec::with_capacity(self.judged_total);
        let mut judged_ranges = if self.judged_total > 0 {
            Vec::with_capacity(layout.instances.len())
        } else {
            Vec::new()
        };
        for (at, instance) in layout.instances.iter().enumerate() {
            let (mode_var, mode_val) = match instance.mode {
                None => (u16::MAX, 0),
                Some((var, val)) => (u16::try_from(var).ok()?, u16::try_from(val).ok()?),
            };
            if mode_var == u16::MAX && instance.mode.is_some() {
                return None;
            }
            let pid = instance.program.index();
            let (steps_start, steps_end, output) = program_fast[pid];
            let alias = &program_alias[pid];
            let judged_start = judged_ops.len() as u32;
            for &(op, ..) in &layout.programs[pid].code.judged {
                judged_ops.push(alias[op]);
            }
            let judged_end = judged_ops.len() as u32;
            if self.judged_total > 0 {
                judged_ranges.push(judged_start..judged_end);
            }
            let reads = self.instance(at);
            let p_steps = &steps[steps_start as usize..steps_end as usize];
            let kind = if judged_start == judged_end
                && reads.parameters.is_empty()
                && reads.others.is_empty()
                && reads.unknowns.len() == 5
                && matches!(p_steps, [(_, FastInstr::Upwind([0, 1, 2, 3, 4]))])
                && reads
                    .unknowns
                    .iter()
                    .enumerate()
                    .all(|(i, &(slot, _))| usize::from(slot) == i)
                && output == p_steps[0].0
            {
                FastKind::Upwind5([
                    reads.unknowns[0].1 as u16,
                    reads.unknowns[1].1 as u16,
                    reads.unknowns[2].1 as u16,
                    reads.unknowns[3].1 as u16,
                    reads.unknowns[4].1 as u16,
                ])
            } else if judged_start == judged_end
                && p_steps.is_empty()
                && reads.others.is_empty()
                && reads.parameters.is_empty()
                && reads.unknowns.len() == 1
                && reads.unknowns[0].0 == u16::from(output)
            {
                FastKind::ReadUnknown(reads.unknowns[0].1 as u16)
            } else if judged_start == judged_end
                && p_steps.is_empty()
                && reads.others.is_empty()
                && reads.unknowns.is_empty()
                && reads.parameters.len() == 1
                && reads.parameters[0].0 == u16::from(output)
            {
                FastKind::ReadParameter(reads.parameters[0].1 as u16)
            } else {
                let param_start = parameters.len() as u32;
                for &(slot, index) in reads.parameters {
                    parameters.push((slot as u8, index as u16));
                }
                let param_end = parameters.len() as u32;
                let unknown_start = unknowns.len() as u32;
                for &(slot, index) in reads.unknowns {
                    unknowns.push((slot as u8, index as u16));
                }
                let unknown_end = unknowns.len() as u32;
                let other_range = &self.instances[at].others;
                let general_at = generals.len() as u32;
                generals.push(FastGeneral {
                    steps_start,
                    steps_end,
                    param_start,
                    param_end,
                    unknown_start,
                    unknown_end,
                    other_start: other_range.start,
                    other_end: other_range.end,
                    output,
                });
                FastKind::General(general_at)
            };
            instances.push(FastInstance {
                kind,
                mode_var,
                mode_val,
            });
        }
        Some(FastTape {
            instances,
            generals,
            steps,
            parameters,
            unknowns,
            judged_ops,
            judged_ranges,
        })
    }
    fn clean_inputs(&self, inputs: &Inputs<'_>) -> bool {
        inputs.parameters.len() >= self.parameters_needed
            && inputs.unknowns.len() >= self.unknowns_needed
            && inputs.draws.len() >= self.draws_needed
            && inputs.factors.len() >= self.factors_needed
            && inputs
                .parameters
                .iter()
                .fold(true, |clean, value| clean & value.is_finite())
            && inputs
                .unknowns
                .iter()
                .fold(true, |clean, value| clean & value.is_finite())
            && inputs
                .draws
                .iter()
                .fold(true, |clean, value| clean & value.is_finite())
            && inputs
                .factors
                .iter()
                .fold(true, |clean, value| clean & value.is_finite())
    }
    fn lower(&mut self, ops: &[Op]) -> Kernel {
        let start = self.steps.len();
        for (at, op) in ops.iter().enumerate() {
            let instr = match *op {
                Op::Input(_) => continue,
                Op::Zero(_) => Instr::Zero,
                Op::Add([a, b]) => Instr::Add([slot16(a), slot16(b)]),
                Op::Subtract([a, b]) | Op::Deviation([a, b]) => Instr::Subtract([slot16(a), slot16(b)]),
                Op::Multiply([a, b]) | Op::Scale([a, b]) | Op::ConductanceFlow([a, b]) => {
                    Instr::Multiply([slot16(a), slot16(b)])
                }
                Op::Divide([a, b]) => Instr::Divide([slot16(a), slot16(b)]),
                Op::AsDimension([x]) => Instr::Copy([slot16(x)]),
                Op::SmoothMax([a, b, w]) => Instr::SmoothMax([slot16(a), slot16(b), slot16(w)]),
                Op::SmoothMin([a, b, w]) => Instr::SmoothMin([slot16(a), slot16(b), slot16(w)]),
                Op::Exp([x]) => Instr::Exp([slot16(x)]),
                Op::Logistic([x]) => Instr::Logistic([slot16(x)]),
                Op::Ln([x]) => Instr::Ln([slot16(x)]),
                Op::Relax([a, b, c]) => Instr::Relax([slot16(a), slot16(b), slot16(c)]),
                Op::Volume([x]) => Instr::Volume([slot16(x)]),
                Op::Upwind([a, b, c, d, e]) => {
                    Instr::Upwind([slot16(a), slot16(b), slot16(c), slot16(d), slot16(e)])
                }
                Op::Expm1([x]) => Instr::Expm1([slot16(x)]),
            };
            self.steps.push((slot16(at), instr));
        }
        self.longest = self.longest.max(ops.len());
        Kernel {
            length: slot16(ops.len()),
            steps: range_u32(start..self.steps.len()),
        }
    }
    fn bind(&mut self, ops: &[Op], inputs: &[Bound]) -> ReadRanges {
        let (parameters, unknowns, others) = (self.parameters.len(), self.unknowns.len(), self.others.len());
        let mut bounds = inputs.iter();
        for (at, _) in (ops.iter().enumerate()).filter(|(_, op)| matches!(op, Op::Input(_))) {
            let at = slot16(at);
            match *bounds.next().expect("one bound per input") {
                Bound::Parameter(parameter) => {
                    let index = parameter.index();
                    self.parameters_needed = self.parameters_needed.max(index + 1);
                    self.parameters
                        .push((at, u32::try_from(index).expect("parameter index fits in u32")));
                }
                Bound::Unknown(unknown) => {
                    let index = unknown.index();
                    self.unknowns_needed = self.unknowns_needed.max(index + 1);
                    self.unknowns
                        .push((at, u32::try_from(index).expect("unknown index fits in u32")));
                }
                other => {
                    match other {
                        Bound::Draw(stream) => self.draws_needed = self.draws_needed.max(stream + 1),
                        Bound::Forcing(forcing) => self.factors_needed = self.factors_needed.max(forcing + 1),
                        _ => {}
                    }
                    self.others.push((at, other));
                }
            }
        }
        ReadRanges {
            parameters: range_u32(parameters..self.parameters.len()),
            unknowns: range_u32(unknowns..self.unknowns.len()),
            others: range_u32(others..self.others.len()),
        }
    }
    fn arithmetic(&self, kernel: &Kernel) -> Arithmetic<'_> {
        Arithmetic {
            length: kernel.length,
            steps: &self.steps[slice_range(&kernel.steps)],
        }
    }
    fn reads(&self, ranges: &ReadRanges) -> Reads<'_> {
        Reads {
            parameters: &self.parameters[slice_range(&ranges.parameters)],
            unknowns: &self.unknowns[slice_range(&ranges.unknowns)],
            others: &self.others[slice_range(&ranges.others)],
        }
    }
    fn program(&self, program: ProgramId) -> Arithmetic<'_> {
        self.arithmetic(&self.programs[program.index()])
    }
    fn instance(&self, at: usize) -> Reads<'_> {
        self.reads(&self.instances[at])
    }
    fn entries(&self, at: usize) -> &[u32] {
        &self.entries[slice_range(&self.entry_ranges[at])]
    }
    fn guard(&self, guard: usize) -> (Arithmetic<'_>, Reads<'_>) {
        let (kernel, reads) = &self.guards[guard];
        (self.arithmetic(kernel), self.reads(reads))
    }
}

#[derive(Clone, Copy)]
struct Arithmetic<'t> {
    length: u16,
    steps: &'t [(u16, Instr)],
}

#[derive(Clone, Copy)]
struct Reads<'t> {
    parameters: &'t [(u16, u32)],
    unknowns: &'t [(u16, u32)],
    others: &'t [(u16, Bound)],
}
impl Reads<'_> {
    fn fill(&self, inputs: &Inputs<'_>, values: &mut [f64]) -> Option<(u16, Failure)> {
        let mut values = SlotsMut(values);
        let mut clean = true;
        for &(slot, index) in self.parameters {
            match inputs.parameters.get(index as usize) {
                Some(&value) => {
                    values[slot] = value;
                    clean &= value.is_finite();
                }
                None => clean = false,
            }
        }
        for &(slot, index) in self.unknowns {
            match inputs.unknowns.get(index as usize) {
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
    #[cold]
    #[inline(never)]
    fn first_failure(&self, inputs: &Inputs<'_>) -> (u16, Failure) {
        let failure = |value: Option<f64>, bound: Bound| match value {
            None => Some(Failure::PastItsSlice(bound)),
            Some(value) if !value.is_finite() => Some(Failure::NotFinite),
            Some(_) => None,
        };
        let parameters = (self.parameters.iter()).find_map(|&(slot, index)| {
            let bound = Bound::Parameter(ParameterId::at(index as usize));
            Some((
                slot,
                failure(inputs.parameters.get(index as usize).copied(), bound)?,
            ))
        });
        let unknowns = (self.unknowns.iter()).find_map(|&(slot, index)| {
            let bound = Bound::Unknown(UnknownId::at(index as usize));
            Some((
                slot,
                failure(inputs.unknowns.get(index as usize).copied(), bound)?,
            ))
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
        Bound::Boundary(_) | Bound::Gauge | Bound::Height(_) | Bound::Lasting(_) => {
            Some(CompiledBody::bound(bound, inputs))
        }
    }
}

#[inline(always)]
fn apply(instr: Instr, values: &[f64], partial_volume: f64) -> Result<f64, OperatorDomain> {
    let at = |slot: u16| values[usize::from(slot)];
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
        Instr::Expm1([x]) => pinned::expm1(at(x)),
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

#[inline(always)]
fn apply_fast(
    instr: FastInstr,
    values: &[f64; STACK_SLOTS],
    partial_volume: f64,
) -> Result<f64, OperatorDomain> {
    let at = |slot: u8| values[usize::from(slot)];
    let result = match instr {
        FastInstr::Zero => return Ok(0.0),
        FastInstr::Add([a, b]) => at(a) + at(b),
        FastInstr::Subtract([a, b]) => at(a) - at(b),
        FastInstr::Multiply([a, b]) => at(a) * at(b),
        FastInstr::Divide([a, b]) => {
            let divisor = at(b);
            if divisor <= 0.0 {
                return Err(OperatorDomain::Divisor);
            }
            at(a) / divisor
        }
        FastInstr::SmoothMax([a, b, width]) => smooth(at(a), at(b), at(width), 1.0).0,
        FastInstr::SmoothMin([a, b, width]) => smooth(at(a), at(b), at(width), -1.0).0,
        FastInstr::Exp([x]) => pinned::exp(at(x)),
        FastInstr::Expm1([x]) => pinned::expm1(at(x)),
        FastInstr::Logistic([x]) => 1.0 / (1.0 + pinned::exp(-at(x))),
        FastInstr::Ln([x]) => pinned::ln(at(x)).ok_or(OperatorDomain::LogarithmArgument)?,
        FastInstr::Relax([target, current, speed]) => at(speed) * (at(target) - at(current)),
        FastInstr::Volume([amount]) => at(amount) * partial_volume,
        FastInstr::Upwind([flow, from, from_water, to, to_water]) => {
            let flow = at(flow);
            let (amount, water) = if flow > 0.0 {
                (at(from), at(from_water))
            } else if flow < 0.0 {
                (at(to), at(to_water))
            } else {
                return Ok(0.0);
            };
            if water <= 0.0 {
                return Err(OperatorDomain::Divisor);
            }
            flow * (amount / water)
        }
    };
    if !result.is_finite() {
        return Err(OperatorDomain::NonFinite);
    }
    Ok(result)
}

#[inline(always)]
fn forward_fast(
    steps: &[(u8, FastInstr)],
    partial_volume: f64,
    values: &mut [f64; STACK_SLOTS],
) -> Result<(), OperatorDomain> {
    for &(at, instr) in steps {
        let value = apply_fast(instr, values, partial_volume)?;
        values[usize::from(at)] = value;
    }
    Ok(())
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
        values[usize::from(at)] = value;
    }
    match failed {
        None => Ok(values[usize::from(arithmetic.length) - 1]),
        Some((_, Failure::NotFinite)) => Err(OperatorDomain::NonFinite),
        Some((_, Failure::PastItsSlice(bound))) => {
            CompiledBody::bound(&bound, inputs);
            unreachable!("a read past its slice panics in bound")
        }
    }
}

/// Reruns `forward` on prefixes rather than call `apply` itself: a second caller of `apply` stopped its inlining into
/// `forward` and cost every step 12.6% more instructions.
#[cold]
fn refused_at(arithmetic: Arithmetic<'_>, reads: Reads<'_>, inputs: &Inputs<'_>, partial_volume: f64) -> u16 {
    let mut values = Scratch::zeros(usize::from(arithmetic.length));
    let none = Arithmetic {
        length: arithmetic.length,
        steps: &[],
    };
    let mut read = arithmetic.length;
    if forward(none, reads, inputs, partial_volume, &mut values).is_err() {
        let slots = (reads.parameters.iter().map(|read| read.0))
            .chain(reads.unknowns.iter().map(|read| read.0))
            .chain(reads.others.iter().map(|read| read.0));
        read = slots
            .filter(|&slot| !values[slot].is_finite())
            .min()
            .unwrap_or(read);
    }
    let (parameters, unknowns, others): (Vec<_>, Vec<_>, Vec<_>) = (
        reads
            .parameters
            .iter()
            .copied()
            .filter(|at| at.0 < read)
            .collect(),
        reads.unknowns.iter().copied().filter(|at| at.0 < read).collect(),
        reads.others.iter().copied().filter(|at| at.0 < read).collect(),
    );
    let before = Reads {
        parameters: &parameters,
        unknowns: &unknowns,
        others: &others,
    };
    let steps = &arithmetic.steps[..arithmetic.steps.partition_point(|&(at, _)| at < read)];
    let prefix = |count: usize| Arithmetic {
        length: arithmetic.length,
        steps: &steps[..count],
    };
    if forward(prefix(steps.len()), before, inputs, partial_volume, &mut values).is_ok() {
        return read.min(arithmetic.length - 1);
    }
    let (mut passed, mut failed) = (0, steps.len());
    while failed - passed > 1 {
        let middle = passed + (failed - passed) / 2;
        if forward(prefix(middle), before, inputs, partial_volume, &mut values).is_err() {
            failed = middle;
        } else {
            passed = middle;
        }
    }
    steps[failed - 1].0
}

fn backward(arithmetic: Arithmetic<'_>, values: &[f64], partial_volume: f64, slopes: &mut [f64]) {
    let length = usize::from(arithmetic.length);
    slopes[..length].fill(0.0);
    slopes[length - 1] = 1.0;
    let values = Slots(values);
    let mut slopes = SlotsMut(slopes);
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
            Instr::Expm1([x]) => slopes[x] += slope * pinned::exp(values[x]),
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

fn excursion(
    law: LawName,
    context: Context,
    at: SimTime,
    value: f64,
    lower: u64,
    upper: u64,
) -> Option<ValidityExcursion> {
    let side = if value < f64::from_bits(lower) {
        ExcursionSide::Below
    } else if value > f64::from_bits(upper) {
        ExcursionSide::Above
    } else {
        return None;
    };
    Some(ValidityExcursion {
        law,
        context,
        at,
        side,
        value: Compensated::exact(value),
    })
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
        let length = usize::from(arithmetic.length);
        let mut stack = [0.0; STACK_SLOTS];
        let mut heap = Vec::new();
        let values: &mut [f64] = if length <= STACK_SLOTS {
            &mut stack[..length]
        } else {
            heap.resize(length, 0.0);
            &mut heap
        };
        let value = forward(arithmetic, reads, inputs, self.partial_volume(), values)
            .map_err(|domain| self.guard_undefined(guard, inputs, domain))?;
        Ok(match layout.crossing {
            Crossing::Rising => value,
            Crossing::Falling => -value,
        })
    }
    #[cold]
    fn guard_undefined(&self, guard: usize, inputs: &Inputs<'_>, domain: OperatorDomain) -> Undefined {
        let layout = &self.layout.guards[guard];
        let mut law = self.layout.modes[layout.variable].variable.law;
        if !layout.code.origins.is_empty() {
            let (arithmetic, reads) = self.tape.guard(guard);
            let op = usize::from(refused_at(arithmetic, reads, inputs, self.partial_volume()));
            law = layout.code.origin(op).unwrap_or(law);
        }
        Undefined { law, domain }
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
        let length = usize::from(arithmetic.length);
        let (mut values_stack, mut slopes_stack) = ([0.0; STACK_SLOTS], [0.0; STACK_SLOTS]);
        let (mut values_heap, mut slopes_heap) = (Vec::new(), Vec::new());
        let (values, slopes): (&mut [f64], &mut [f64]) = if length <= STACK_SLOTS {
            (&mut values_stack[..length], &mut slopes_stack[..length])
        } else {
            values_heap.resize(length, 0.0);
            slopes_heap.resize(length, 0.0);
            (&mut values_heap, &mut slopes_heap)
        };
        let partial_volume = self.partial_volume();
        forward(arithmetic, reads, inputs, partial_volume, values)
            .map_err(|domain| self.guard_undefined(guard, inputs, domain))?;
        backward(arithmetic, values, partial_volume, slopes);
        let sign = self.guard_sign(guard);
        let finite = |&(slot, index): &(u16, u32)| {
            let slot = usize::from(slot);
            if slopes[slot].is_finite() {
                Ok((index as usize, sign * slopes[slot]))
            } else {
                let law = self.layout.modes[layout.variable].variable.law;
                let law = layout.code.origin(slot).unwrap_or(law);
                let domain = OperatorDomain::NonFinite;
                Err(Undefined { law, domain })
            }
        };
        let unknowns = reads.unknowns.iter().map(finite);
        let parameters = reads.parameters.iter().map(finite);
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
            Bound::Lasting(input) => inputs.frame.lasting(input),
        }
    }
    #[cold]
    fn undefined(&self, at: usize, inputs: &Inputs<'_>, domain: OperatorDomain) -> Undefined {
        let instance = &self.layout.instances[at];
        let program = &self.layout.programs[instance.program.index()];
        if program.code.origins.is_empty() {
            return Undefined {
                law: program.law,
                domain,
            };
        }
        let (arithmetic, reads) = (self.tape.program(instance.program), self.tape.instance(at));
        let op = usize::from(refused_at(arithmetic, reads, inputs, self.partial_volume()));
        self.undefined_at(instance, op, domain)
    }
    #[cold]
    fn undefined_at(&self, instance: &Instance, op: usize, domain: OperatorDomain) -> Undefined {
        let program = &self.layout.programs[instance.program.index()];
        let law = program.code.origin(op).unwrap_or(program.law);
        Undefined { law, domain }
    }
    pub(crate) fn instance_values(&self, inputs: &Inputs<'_>) -> Result<Vec<Option<f64>>, Undefined> {
        let partial_volume = self.partial_volume();
        if let Some(fast) = &self.tape.fast
            && self.tape.clean_inputs(inputs)
        {
            return self.instance_values_fast(fast, inputs, partial_volume);
        }
        let mut values = vec![0.0; self.tape.longest];
        let mut out = Vec::with_capacity(self.layout.instances.len() + self.tape.judged_total);
        let mut judged = Vec::with_capacity(self.tape.judged_total);
        for (at, instance) in self.layout.instances.iter().enumerate() {
            let program = &self.layout.programs[instance.program.index()];
            if !instance.holds(inputs.modes) {
                out.push(None);
                judged.extend(program.code.judged.iter().map(|_| None));
                continue;
            }
            let (arithmetic, reads) = (self.tape.program(instance.program), self.tape.instance(at));
            let value = forward(arithmetic, reads, inputs, partial_volume, &mut values);
            out.push(Some(value.map_err(|domain| self.undefined(at, inputs, domain))?));
            judged.extend(program.code.judged.iter().map(|&(op, ..)| Some(values[op])));
        }
        out.extend(judged);
        Ok(out)
    }
    fn instance_values_fast(
        &self,
        fast: &FastTape,
        inputs: &Inputs<'_>,
        partial_volume: f64,
    ) -> Result<Vec<Option<f64>>, Undefined> {
        let mut values = [0.0; STACK_SLOTS];
        let mut out = Vec::with_capacity(fast.instances.len() + self.tape.judged_total);
        let has_judged = self.tape.judged_total > 0;
        let mut judged = Vec::with_capacity(self.tape.judged_total);
        let parameters = inputs.parameters;
        let unknowns = inputs.unknowns;
        for (at, inst) in fast.instances.iter().enumerate() {
            if !inst.holds(inputs.modes) {
                out.push(None);
                if has_judged {
                    let range = &fast.judged_ranges[at];
                    judged.resize(judged.len() + (range.end - range.start) as usize, None);
                }
                continue;
            }
            let value = match inst.kind {
                FastKind::Upwind5(u) => {
                    let flow = unknowns[usize::from(u[0])];
                    let value = if flow > 0.0 {
                        let water = unknowns[usize::from(u[2])];
                        if water <= 0.0 {
                            return Err(self.undefined(at, inputs, OperatorDomain::Divisor));
                        }
                        flow * (unknowns[usize::from(u[1])] / water)
                    } else if flow < 0.0 {
                        let water = unknowns[usize::from(u[4])];
                        if water <= 0.0 {
                            return Err(self.undefined(at, inputs, OperatorDomain::Divisor));
                        }
                        flow * (unknowns[usize::from(u[3])] / water)
                    } else {
                        0.0
                    };
                    if !value.is_finite() {
                        return Err(self.undefined(at, inputs, OperatorDomain::NonFinite));
                    }
                    value
                }
                FastKind::ReadUnknown(u) => unknowns[usize::from(u)],
                FastKind::ReadParameter(p) => parameters[usize::from(p)],
                FastKind::General(general_at) => {
                    let g = fast.generals[general_at as usize];
                    for &(slot, index) in &fast.parameters[g.param_start as usize..g.param_end as usize] {
                        values[usize::from(slot)] = parameters[usize::from(index)];
                    }
                    for &(slot, index) in &fast.unknowns[g.unknown_start as usize..g.unknown_end as usize] {
                        values[usize::from(slot)] = unknowns[usize::from(index)];
                    }
                    let mut clean = true;
                    if g.other_start != g.other_end {
                        for &(slot, bound) in &self.tape.others[g.other_start as usize..g.other_end as usize]
                        {
                            match read(&bound, inputs) {
                                Some(value) if value.is_finite() => values[usize::from(slot)] = value,
                                _ => clean = false,
                            }
                        }
                    }
                    if clean {
                        forward_fast(
                            &fast.steps[g.steps_start as usize..g.steps_end as usize],
                            partial_volume,
                            &mut values,
                        )
                        .map_err(|domain| self.undefined(at, inputs, domain))?;
                        values[usize::from(g.output)]
                    } else {
                        let instance = &self.layout.instances[at];
                        let (arithmetic, reads) =
                            (self.tape.program(instance.program), self.tape.instance(at));
                        forward(arithmetic, reads, inputs, partial_volume, &mut values)
                            .map_err(|domain| self.undefined(at, inputs, domain))?
                    }
                }
            };
            out.push(Some(value));
            if has_judged {
                let range = &fast.judged_ranges[at];
                for &op in &fast.judged_ops[slice_range(range)] {
                    judged.push(Some(values[usize::from(op)]));
                }
            }
        }
        if has_judged {
            out.extend(judged);
        }
        Ok(out)
    }
    fn slopes(
        &self,
        inputs: &Inputs<'_>,
        mut each: impl FnMut(&Swept<'_>) -> Result<(), Undefined>,
    ) -> Result<(), Undefined> {
        let partial_volume = self.partial_volume();
        if let Some(fast) = &self.tape.fast
            && self.tape.clean_inputs(inputs)
        {
            return self.slopes_fast(fast, inputs, partial_volume, each);
        }
        let (mut values, mut slopes) = (vec![0.0; self.tape.longest], vec![0.0; self.tape.longest]);
        for (at, instance) in self.layout.instances.iter().enumerate() {
            if !instance.holds(inputs.modes) {
                continue;
            }
            let (arithmetic, reads) = (self.tape.program(instance.program), self.tape.instance(at));
            let value = forward(arithmetic, reads, inputs, partial_volume, &mut values)
                .map_err(|domain| self.undefined(at, inputs, domain))?;
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
    fn slopes_fast(
        &self,
        fast: &FastTape,
        inputs: &Inputs<'_>,
        partial_volume: f64,
        mut each: impl FnMut(&Swept<'_>) -> Result<(), Undefined>,
    ) -> Result<(), Undefined> {
        let (mut values, mut slopes) = ([0.0; STACK_SLOTS], [0.0; STACK_SLOTS]);
        let (parameters, unknowns) = (inputs.parameters, inputs.unknowns);
        for (at, (inst, instance)) in fast.instances.iter().zip(&self.layout.instances).enumerate() {
            if !inst.holds(inputs.modes) {
                continue;
            }
            let reads = self.tape.instance(at);
            match inst.kind {
                FastKind::Upwind5(u) => {
                    let flow = unknowns[usize::from(u[0])];
                    let mut upwind_slopes = [0.0; 5];
                    let value = if flow >= 0.0 {
                        let (amount, water) = (unknowns[usize::from(u[1])], unknowns[usize::from(u[2])]);
                        let value = if flow == 0.0 {
                            0.0
                        } else if water <= 0.0 {
                            return Err(self.undefined(at, inputs, OperatorDomain::Divisor));
                        } else {
                            flow * (amount / water)
                        };
                        if !value.is_finite() {
                            return Err(self.undefined(at, inputs, OperatorDomain::NonFinite));
                        }
                        if water > 0.0 {
                            let share = amount / water;
                            upwind_slopes[0] += share;
                            upwind_slopes[1] += flow / water;
                            upwind_slopes[2] -= flow * share / water;
                        }
                        value
                    } else {
                        let (amount, water) = (unknowns[usize::from(u[3])], unknowns[usize::from(u[4])]);
                        if water <= 0.0 {
                            return Err(self.undefined(at, inputs, OperatorDomain::Divisor));
                        }
                        let value = flow * (amount / water);
                        if !value.is_finite() {
                            return Err(self.undefined(at, inputs, OperatorDomain::NonFinite));
                        }
                        let share = amount / water;
                        upwind_slopes[0] += share;
                        upwind_slopes[3] += flow / water;
                        upwind_slopes[4] -= flow * share / water;
                        value
                    };
                    let flat = instance.rectified && value <= 0.0;
                    each(&Swept {
                        at,
                        instance,
                        reads,
                        slopes: &upwind_slopes,
                        flat,
                    })?;
                }
                FastKind::ReadUnknown(u) => {
                    let value = unknowns[usize::from(u)];
                    let flat = instance.rectified && value <= 0.0;
                    let slot = usize::from(reads.unknowns[0].0);
                    slopes[slot] = 1.0;
                    each(&Swept {
                        at,
                        instance,
                        reads,
                        slopes: &slopes,
                        flat,
                    })?;
                }
                FastKind::ReadParameter(p) => {
                    let value = parameters[usize::from(p)];
                    let flat = instance.rectified && value <= 0.0;
                    let slot = usize::from(reads.parameters[0].0);
                    slopes[slot] = 1.0;
                    each(&Swept {
                        at,
                        instance,
                        reads,
                        slopes: &slopes,
                        flat,
                    })?;
                }
                FastKind::General(general_at) => {
                    let g = fast.generals[general_at as usize];
                    let arithmetic = self.tape.program(instance.program);
                    for &(slot, index) in &fast.parameters[g.param_start as usize..g.param_end as usize] {
                        values[usize::from(slot)] = parameters[usize::from(index)];
                    }
                    for &(slot, index) in &fast.unknowns[g.unknown_start as usize..g.unknown_end as usize] {
                        values[usize::from(slot)] = unknowns[usize::from(index)];
                    }
                    let mut clean = true;
                    if g.other_start != g.other_end {
                        for &(slot, bound) in &self.tape.others[g.other_start as usize..g.other_end as usize]
                        {
                            match read(&bound, inputs) {
                                Some(value) if value.is_finite() => values[usize::from(slot)] = value,
                                _ => clean = false,
                            }
                        }
                    }
                    let value = if clean {
                        for &(slot, instr) in arithmetic.steps {
                            let val = apply(instr, &values, partial_volume)
                                .map_err(|domain| self.undefined(at, inputs, domain))?;
                            values[usize::from(slot)] = val;
                        }
                        values[usize::from(arithmetic.length) - 1]
                    } else {
                        forward(arithmetic, reads, inputs, partial_volume, &mut values)
                            .map_err(|domain| self.undefined(at, inputs, domain))?
                    };
                    backward(arithmetic, &values, partial_volume, &mut slopes);
                    let flat = instance.rectified && value <= 0.0;
                    each(&Swept {
                        at,
                        instance,
                        reads,
                        slopes: &slopes,
                        flat,
                    })?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn jacobian(&self, inputs: &Inputs<'_>, shift: f64) -> Result<Vec<f64>, Undefined> {
        enum TargetKind<'a> {
            Column(&'a crate::body::Column),
            Row(f64),
        }
        let mut values = vec![0.0; self.layout.sparsity.unknowns.len()];
        self.slopes(inputs, |swept| {
            let mut entries = self.tape.entries(swept.at).iter();
            let mut next = || *entries.next().expect("an entry per unknown read and term") as usize;
            let target = match swept.instance.target {
                Target::Column(column) => TargetKind::Column(&self.layout.columns[column.index()]),
                Target::Row(row) => TargetKind::Row(match self.layout.rows[row.index()].kind {
                    RowKind::Algebraic(_) => 1.0,
                    RowKind::Amount { .. } | RowKind::Memory(_) => -1.0,
                }),
            };
            for &(slot, _) in swept.reads.unknowns {
                let slot = usize::from(slot);
                let slope = swept.slope(slot);
                if !slope.is_finite() {
                    return Err(self.undefined_at(swept.instance, slot, OperatorDomain::NonFinite));
                }
                match target {
                    TargetKind::Column(column) => {
                        let slope = column.turnovers(slope);
                        for &(_, coefficient) in &column.terms {
                            values[next()] -= coefficient as f64 * slope;
                        }
                    }
                    TargetKind::Row(sign) => {
                        values[next()] += sign * slope;
                    }
                }
            }
            Ok(())
        })?;
        for &entry in &self.tape.diagonals {
            values[entry as usize] += shift;
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
                .filter(|&&(_, parameter)| slots.contains(&ParameterId::at(parameter as usize)));
            let slope: f64 = fitted.map(|&(slot, _)| swept.slope(usize::from(slot))).sum();
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
        if self.tape.contiguous_rows && rows.len() == self.layout.rows.len() && derivative.len() == rows.len()
        {
            residuals.extend_from_slice(&derivative[..self.tape.amounts_end]);
            for (&value, &tangent) in rows[self.tape.amounts_end..self.tape.memory_end]
                .iter()
                .zip(&derivative[self.tape.amounts_end..self.tape.memory_end])
            {
                residuals.push(tangent - value);
            }
            residuals.extend_from_slice(&rows[self.tape.memory_end..]);
        } else {
            for (row, (&value, &tangent)) in self.layout.rows.iter().zip(rows.iter().zip(derivative)) {
                residuals.push(match row.kind {
                    RowKind::Amount { .. } => tangent,
                    RowKind::Memory(_) => tangent - value,
                    RowKind::Algebraic(_) => value,
                });
            }
        }
        if columns.len() == self.layout.columns.len() {
            for &ColumnTerm {
                column,
                row,
                coefficient,
            } in &self.tape.column_terms
            {
                residuals[row as usize] -= coefficient * columns[column as usize];
            }
        } else {
            for (column, &rate) in self.layout.columns.iter().zip(columns) {
                for &(row, coefficient) in &column.terms {
                    residuals[row.index()] -= coefficient as f64 * rate;
                }
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
                excursions.extend(excursion(program.law, instance.context, at, value, lower, upper));
            }
        }
        let judged = (self.layout.instances.iter()).flat_map(|instance| {
            let program = &self.layout.programs[instance.program.index()];
            (program.code.judged.iter()).map(move |&(_, law, range)| (instance.context, law, range))
        });
        let tail = &values[self.layout.instances.len().min(values.len())..];
        let mut reported: Vec<(LawName, Context, Validity)> = Vec::new();
        for ((context, law, range), value) in judged.zip(tail) {
            if let Some(value) = *value
                && !reported.contains(&(law, context, range))
            {
                reported.push((law, context, range));
                excursions.extend(excursion(law, context, at, value, range.lower, range.upper));
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
            Op::Expm1(operand) => ("expm1", operand.to_vec()),
        })
    }

    fn named(instr: Instr) -> (&'static str, Vec<usize>) {
        let list = |slots: &[u16]| slots.iter().map(|&slot| usize::from(slot)).collect();
        match instr {
            Instr::Zero => ("zero", Vec::new()),
            Instr::Add(operands) => ("add", list(&operands)),
            Instr::Subtract(operands) => ("subtract", list(&operands)),
            Instr::Multiply(operands) => ("multiply", list(&operands)),
            Instr::Divide(operands) => ("divide", list(&operands)),
            Instr::Copy(operand) => ("copy", list(&operand)),
            Instr::SmoothMax(operands) => ("smooth max", list(&operands)),
            Instr::SmoothMin(operands) => ("smooth min", list(&operands)),
            Instr::Exp(operand) => ("exp", list(&operand)),
            Instr::Logistic(operand) => ("logistic", list(&operand)),
            Instr::Ln(operand) => ("ln", list(&operand)),
            Instr::Relax(operands) => ("relax", list(&operands)),
            Instr::Volume(operand) => ("volume", list(&operand)),
            Instr::Upwind(operands) => ("upwind", list(&operands)),
            Instr::Expm1(operand) => ("expm1", list(&operand)),
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
            assert_eq!(usize::from(kernel.length), code.ops.len());
            let expected: Vec<_> = (code.ops.iter().enumerate())
                .filter_map(|(slot, op)| Some((slot, lowered(op)?)))
                .collect();
            let found: Vec<_> = kernel
                .steps
                .iter()
                .map(|&(slot, instr)| (usize::from(slot), named(instr)))
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
            let parameters = (reads.parameters.iter()).map(|&(slot, index)| {
                (
                    usize::from(slot),
                    Bound::Parameter(ParameterId::at(index as usize)),
                )
            });
            let unknowns = (reads.unknowns.iter())
                .map(|&(slot, index)| (usize::from(slot), Bound::Unknown(UnknownId::at(index as usize))));
            let mut found: Vec<(usize, Bound)> = parameters
                .chain(unknowns)
                .chain(
                    reads
                        .others
                        .iter()
                        .map(|&(slot, bound)| (usize::from(slot), bound)),
                )
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
                    holds(entry as usize, row, unknown),
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
                holds(entry as usize, row, unknown),
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

    #[test]
    fn fast_tape_matches_general_evaluation_bit_for_bit() {
        let mut map = AnatomyBuilder::new();
        let origin = || Position::new(measure(0.0), measure(0.0), measure(0.0));
        let root = map.site::<TissueSite>("body", origin()).id();
        let a = map.site::<VesselSegment>("a", origin()).id();
        let b = map.site::<VesselSegment>("b", origin()).id();
        map.contain(root, a);
        map.contain(root, b);
        map.connect(Network::Vascular, a, b);
        let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
        for space in [Space::Plasma, Space::Interstitium, Space::Cells] {
            schema.leave_out(space, "vascular lumen only");
        }
        schema.carry::<Sodium, VesselSegment>(Space::Lumen);
        schema
            .partial_volume(Measure::new(1.0).expect("finite"))
            .expect("partial volume");
        let body = assemble(schema, |mut body| {
            let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut cv);
            let e = cv.edge_parameter::<_, Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
            let k = cv.edge_parameter::<_, Conductance<Water>>(
                ParameterKey::new("cv.g"),
                Admissible::nonnegative(),
            );
            let flow = cv.edge_kernel(|mut f| {
                let u = f.at_edge(&route.from_amount());
                let d = f.at_edge(&route.to_amount());
                let u = f.volume(u);
                let d = f.volume(d);
                let diff = f.subtract(u, d);
                let elastance = f.edge_parameter(&e);
                let pressure = f.multiply(elastance, diff);
                let head = f.as_dimension::<_, Pressure>(pressure);
                let conductance = f.edge_parameter(&k);
                let q = f.conductance_flow(conductance, head);
                f.finish(q, cv.every_mode(), class)
            });
            cv.transfer_along(&route, flow);
            body.add(cv.finish());
            body.compile()
        })
        .expect("compiled");
        let fast = body.tape.fast.as_ref().expect("fast tape");
        assert!(
            fast.instances
                .iter()
                .any(|inst| matches!(inst.kind, super::FastKind::Upwind5(_)))
        );
        let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
        packet
            .synthetic(
                ParameterKey::new("cv.e"),
                Binding::Network(Network::Vascular),
                measure::<Elastance>(1.0),
                "synthetic",
            )
            .expect("once");
        packet
            .synthetic(
                ParameterKey::new("cv.g"),
                Binding::Network(Network::Vascular),
                measure::<Conductance<Water>>(0.5),
                "synthetic",
            )
            .expect("once");
        packet.start_rest(&body, |_| 10.0, "synthetic").expect("starts");
        let bound = body.bind(packet).expect("bound");
        let frame = DriveFrame::default();
        let mut slow_params = bound.values.clone();
        slow_params.push(f64::NAN);
        let cases: &[[f64; 4]] = &[
            [10.0, 4.0, 2.0, 1.0],
            [4.0, 10.0, 2.0, 1.0],
            [5.0, 5.0, 2.0, 1.0],
            [5.0, 5.0, -0.0, -0.0],
            [0.0, 0.0, 2.0, 1.0],
            [10.0, 0.0, 2.0, 1.0],
            [0.0, 10.0, 2.0, 1.0],
        ];
        let derivative = vec![0.25; body.rows().len()];
        for case in cases {
            let mut unknowns = vec![0.0; body.rows().len()];
            for (dst, &src) in unknowns.iter_mut().zip(case) {
                *dst = src;
            }
            let fast_in = Inputs {
                parameters: &bound.values,
                frame: &frame,
                unknowns: &unknowns,
                modes: &bound.modes,
                draws: &[],
                factors: &[],
            };
            let slow_in = Inputs {
                parameters: &slow_params,
                ..fast_in
            };
            let fv = body.instance_values(&fast_in);
            let sv = body.instance_values(&slow_in);
            match (&fv, &sv) {
                (Ok(a), Ok(b)) => {
                    let bits =
                        |xs: &[Option<f64>]| xs.iter().map(|v| v.map(f64::to_bits)).collect::<Vec<_>>();
                    assert_eq!(bits(a), bits(b), "values differ on {case:?}");
                    let (fc, fr) = body.targets(a);
                    let fres: Vec<u64> = body
                        .residuals(&fc, &fr, &derivative)
                        .into_iter()
                        .map(f64::to_bits)
                        .collect();
                    let (sc, sr) = body.targets(b);
                    let sres: Vec<u64> = body
                        .residuals(&sc, &sr, &derivative)
                        .into_iter()
                        .map(f64::to_bits)
                        .collect();
                    assert_eq!(fres, sres, "residuals differ on {case:?}");
                }
                (Err(a), Err(b)) => assert_eq!(
                    (a.law, format!("{:?}", a.domain)),
                    (b.law, format!("{:?}", b.domain))
                ),
                _ => panic!("mismatch on {case:?}: {fv:?} vs {sv:?}"),
            }
            let fj = body.jacobian(&fast_in, 1.5);
            let sj = body.jacobian(&slow_in, 1.5);
            match (&fj, &sj) {
                (Ok(a), Ok(b)) => {
                    let bits = |xs: &[f64]| xs.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
                    assert_eq!(bits(a), bits(b), "jacobian differ on {case:?}");
                }
                (Err(a), Err(b)) => assert_eq!(
                    (a.law, format!("{:?}", a.domain)),
                    (b.law, format!("{:?}", b.domain))
                ),
                _ => panic!("jacobian mismatch on {case:?}: {fj:?} vs {sj:?}"),
            }
        }
    }
}
