use crate::body::{RowId, UnknownId};
use alloc::collections::BTreeSet;

/// A fixed elimination over one sparsity. Each pivot pairs an unknown with the row the structural matching gave it,
/// in least-degree order on the symmetric pattern, ties to the least unknown, so a sparse body factors in about its
/// nonzeros and every run fills the same entries. The pattern is the union over modes, so a mode change moves no entry.
///
/// `own` stores the entries the rows hold and those their elimination fills, lower and upper apart. `closure` is the
/// symmetric pattern the order is read from: the extra entries hold zeros, and `ghosts` lists them so a solve keeps
/// the bits a walk over them gives. A value that is not finite takes `closure` whole, since a zero times it is not zero.
#[derive(Debug, PartialEq)]
pub(crate) struct Elimination {
    rows: Vec<u32>,
    unknowns: Vec<u32>,
    own: Pattern,
    closure: Pattern,
    ghosts: Ghosts,
}

#[derive(Debug, PartialEq)]
struct Pattern {
    starts: Vec<u32>,
    columns: Vec<u32>,
    diagonals: Vec<u32>,
    places: Vec<u32>,
}

#[derive(Debug, PartialEq)]
struct Ghosts {
    starts: Vec<u32>,
    splits: Vec<u32>,
    columns: Vec<u32>,
}

struct Eliminated {
    found: Result<Vec<f64>, usize>,
    finite: bool,
}

fn index(value: u32) -> usize {
    value as usize
}

impl Pattern {
    fn of(
        lines: &[Vec<u32>],
        rows: &[u32],
        offsets: &[usize],
        unknowns: &[UnknownId],
        position: &[u32],
    ) -> Self {
        let (mut starts, mut columns, mut diagonals) =
            (vec![0_u32], Vec::new(), Vec::with_capacity(lines.len()));
        for (at, line) in lines.iter().enumerate() {
            diagonals.push(columns.len() as u32 + line.partition_point(|&column| column < at as u32) as u32);
            columns.extend_from_slice(line);
            starts.push(columns.len() as u32);
        }
        let mut places = vec![0_u32; unknowns.len()];
        for (at, &row) in rows.iter().enumerate() {
            let row = index(row);
            let pattern = &columns[index(starts[at])..index(starts[at + 1])];
            for entry in offsets[row]..offsets[row + 1] {
                let column = position[unknowns[entry].index()];
                let found = pattern.binary_search(&column);
                let found = found.expect("every entry of a row lies on its pivot's pattern");
                places[entry] = starts[at] + found as u32;
            }
        }
        Self {
            starts,
            columns,
            diagonals,
            places,
        }
    }
    fn line(&self, at: usize) -> &[u32] {
        &self.columns[index(self.starts[at])..index(self.starts[at + 1])]
    }
}

impl Elimination {
    pub(crate) fn plan(offsets: &[usize], unknowns: &[UnknownId], solved_by: &[RowId]) -> Self {
        let size = offsets.len() - 1;
        let mut node_of_row = vec![0_u32; size];
        for (unknown, row) in solved_by.iter().enumerate() {
            node_of_row[row.index()] = unknown as u32;
        }
        let mut neighbours: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); size];
        for row in 0..size {
            let node = node_of_row[row];
            for read in &unknowns[offsets[row]..offsets[row + 1]] {
                let other = read.index() as u32;
                if other != node {
                    neighbours[index(node)].insert(other);
                    neighbours[index(other)].insert(node);
                }
            }
        }
        let mut position = vec![u32::MAX; size];
        let mut order = Vec::with_capacity(size);
        let mut waiting: BTreeSet<(usize, u32)> = (0..size as u32)
            .map(|node| (neighbours[index(node)].len(), node))
            .collect();
        let mut fill: Vec<Vec<u32>> = vec![Vec::new(); size];
        while let Some((_, node)) = waiting.pop_first() {
            position[index(node)] = order.len() as u32;
            order.push(node);
            let adjacent: Vec<u32> = neighbours[index(node)].iter().copied().collect();
            fill[index(node)] = adjacent.clone();
            for &one in &adjacent {
                waiting.remove(&(neighbours[index(one)].len(), one));
                neighbours[index(one)].remove(&node);
                for &other in &adjacent {
                    if other != one {
                        neighbours[index(one)].insert(other);
                    }
                }
                waiting.insert((neighbours[index(one)].len(), one));
            }
        }
        let rows: Vec<u32> = (order.iter())
            .map(|&node| solved_by[index(node)].index() as u32)
            .collect();
        let mut closure: Vec<Vec<u32>> = vec![Vec::new(); size];
        for (at, &node) in order.iter().enumerate() {
            for &other in &fill[index(node)] {
                closure[index(position[index(other)])].push(at as u32);
            }
        }
        for (at, &node) in order.iter().enumerate() {
            closure[at].push(at as u32);
            let mut upper: Vec<u32> = (fill[index(node)].iter())
                .map(|&other| position[index(other)])
                .collect();
            upper.sort_unstable();
            closure[at].extend_from_slice(&upper);
        }
        let mut own: Vec<Vec<u32>> = Vec::with_capacity(size);
        let mut stamp = vec![u32::MAX; size];
        for (at, &row) in rows.iter().enumerate() {
            let at = at as u32;
            let (mut line, mut pending) = (vec![at], Vec::new());
            stamp[index(at)] = at;
            let read = unknowns[offsets[index(row)]..offsets[index(row) + 1]].iter();
            let mut reach = |column: u32, line: &mut Vec<u32>, pending: &mut Vec<u32>| {
                if stamp[index(column)] != at {
                    stamp[index(column)] = at;
                    line.push(column);
                    if column < at {
                        pending.push(column);
                    }
                }
            };
            for unknown in read {
                reach(position[unknown.index()], &mut line, &mut pending);
            }
            while let Some(pivot) = pending.pop() {
                for &column in &own[index(pivot)] {
                    if column > pivot {
                        reach(column, &mut line, &mut pending);
                    }
                }
            }
            line.sort_unstable();
            own.push(line);
        }
        let mut ghosts = Ghosts {
            starts: vec![0],
            splits: Vec::with_capacity(size),
            columns: Vec::new(),
        };
        for (at, (closed, line)) in closure.iter().zip(&own).enumerate() {
            let left = |column: &&u32| line.binary_search(column).is_err();
            ghosts
                .columns
                .extend(closed.iter().filter(|column| **column < at as u32).filter(left));
            ghosts.splits.push(ghosts.columns.len() as u32);
            ghosts
                .columns
                .extend(closed.iter().filter(|column| **column > at as u32).filter(left));
            ghosts.starts.push(ghosts.columns.len() as u32);
        }
        Self {
            own: Pattern::of(&own, &rows, offsets, unknowns, &position),
            closure: Pattern::of(&closure, &rows, offsets, unknowns, &position),
            rows,
            unknowns: order,
            ghosts,
        }
    }

    pub(crate) fn places(&self) -> (Vec<usize>, Vec<usize>) {
        let (mut rows, mut unknowns) = (vec![0; self.rows.len()], vec![0; self.rows.len()]);
        for (at, (&row, &unknown)) in self.rows.iter().zip(&self.unknowns).enumerate() {
            rows[index(row)] = at;
            unknowns[index(unknown)] = at;
        }
        (rows, unknowns)
    }
    pub(crate) fn solved_by(&self) -> Vec<RowId> {
        let mut solved_by = vec![RowId::at(0); self.rows.len()];
        for (&unknown, &row) in self.unknowns.iter().zip(&self.rows) {
            solved_by[index(unknown)] = RowId::at(index(row));
        }
        solved_by
    }

    pub(crate) fn factor(&self, values: &[f64]) -> Result<Factors<'_>, usize> {
        let (found, closed) = match self.eliminate(&self.own, values) {
            Eliminated { found, finite: true } => (found, false),
            Eliminated { finite: false, .. } => (self.eliminate(&self.closure, values).found, true),
        };
        let lu = found.map_err(|at| index(self.rows[at]))?;
        Ok(Factors {
            plan: self,
            closed,
            lu,
        })
    }

    fn eliminate(&self, pattern: &Pattern, values: &[f64]) -> Eliminated {
        let Pattern {
            starts,
            columns,
            diagonals,
            places,
        } = pattern;
        let mut lu = vec![0.0; columns.len()];
        for (&place, &value) in places.iter().zip(values) {
            lu[index(place)] += value;
        }
        let (mut seen, mut finite) = (vec![0.0; self.rows.len()], true);
        for at in 0..self.rows.len() {
            let (start, end) = (index(starts[at]), index(starts[at + 1]));
            let (done, rest) = lu.split_at_mut(start);
            let line = &mut rest[..end - start];
            for (cell, &column) in line.iter().zip(&columns[start..end]) {
                seen[index(column)] = *cell;
            }
            for &pivot in &columns[start..index(diagonals[at])] {
                let pivot = index(pivot);
                let factor = seen[pivot] / done[index(diagonals[pivot])];
                seen[pivot] = factor;
                let tail = index(diagonals[pivot]) + 1..index(starts[pivot + 1]);
                for (cell, &column) in done[tail.clone()].iter().zip(&columns[tail]) {
                    seen[index(column)] -= factor * cell;
                }
            }
            for (cell, &column) in line.iter_mut().zip(&columns[start..end]) {
                *cell = seen[index(column)];
                finite &= cell.is_finite();
            }
            let diagonal = seen[at];
            if diagonal == 0.0 || !diagonal.is_finite() {
                return Eliminated {
                    found: Err(at),
                    finite,
                };
            }
        }
        Eliminated {
            found: Ok(lu),
            finite,
        }
    }
}
pub struct Factors<'b> {
    plan: &'b Elimination,
    closed: bool,
    lu: Vec<f64>,
}
pub(crate) const LANES: usize = 8;
const NEGATIVE_ZERO: u64 = 0x8000_0000_0000_0000;

impl Factors<'_> {
    fn pattern(&self) -> &Pattern {
        if self.closed {
            &self.plan.closure
        } else {
            &self.plan.own
        }
    }
    /// A sum ends as a negative zero only where its side was one, so a group with none needs no ghost.
    fn ghosts<const L: usize>(&self, sides: &[[f64; L]]) -> Option<&Ghosts> {
        let signed = || {
            (sides.as_flattened().iter())
                .fold(false, |found, value| found | (value.to_bits() == NEGATIVE_ZERO))
        };
        (!self.closed && signed()).then_some(&self.plan.ghosts)
    }
    /// Replaces a right side, one slot per row, with the update, one slot per unknown.
    pub fn solve(&self, side: &mut [f64]) {
        let Elimination { rows, unknowns, .. } = self.plan;
        let pivot = |side: &[f64]| -> Vec<[f64; 1]> { rows.iter().map(|&row| [side[index(row)]]).collect() };
        let mut pivoted = pivot(side);
        if !self.substitute(&mut pivoted) {
            pivoted = pivot(side);
            self.substitute_closed(&mut pivoted);
        }
        for (&unknown, [value]) in unknowns.iter().zip(pivoted) {
            side[index(unknown)] = value;
        }
    }
    pub(crate) fn reach(&self) -> Option<Reach> {
        Reach::of(self)
    }
    pub(crate) fn solve_pivoted(
        &self,
        group: &mut [[f64; LANES]],
        reach: Option<&Reach>,
        spare: &mut [[f64; LANES]],
        marks: &mut Marks,
    ) {
        spare.copy_from_slice(group);
        if !reach.is_some_and(|reach| self.substitute_reached(group, reach, marks)) {
            group.copy_from_slice(spare);
            if !self.substitute(group) {
                group.copy_from_slice(spare);
                self.substitute_closed(group);
            }
        }
    }
    /// Walks only the rows a nonzero side reaches through a nonzero factor, in the order a full walk takes them. A row
    /// whose side is a positive zero and whose nonzero factors read only such rows keeps that zero going forward and
    /// takes zero over its pivot going back, as a full walk gives it. A value that is not finite sends the group back
    /// to the full walk, since a zero factor times it is not zero.
    fn substitute_reached(&self, pivoted: &mut [[f64; LANES]], reach: &Reach, marks: &mut Marks) -> bool {
        let pattern = self.pattern();
        let (diagonals, ghosts) = (&pattern.diagonals, self.ghosts(pivoted));
        let size = pivoted.len();
        marks.clear();
        for (at, values) in pivoted.iter().enumerate() {
            if values.iter().any(|value| value.to_bits() != 0) {
                marks.forward[at] = true;
                marks.stack.push(at as u32);
            }
        }
        marks.spread(|marks| &mut marks.forward, &reach.lower_starts, &reach.lower);
        for at in (0..size).filter(|&at| marks.forward[at]) {
            forward(pattern, &self.lu, ghosts, pivoted, at);
        }
        marks.back.copy_from_slice(&marks.forward);
        marks
            .stack
            .extend((0..size as u32).filter(|&at| marks.forward[index(at)]));
        marks.spread(|marks| &mut marks.back, &reach.upper_starts, &reach.upper);
        for at in (0..size).filter(|&at| !marks.back[at]) {
            pivoted[at] = [0.0 / self.lu[index(diagonals[at])]; LANES];
        }
        for at in (0..size).rev().filter(|&at| marks.back[at]) {
            back(pattern, &self.lu, ghosts, pivoted, at);
        }
        (0..size)
            .filter(|&at| marks.back[at])
            .all(|at| pivoted[at].iter().all(|value| value.is_finite()))
    }
    fn substitute<const L: usize>(&self, pivoted: &mut [[f64; L]]) -> bool {
        match self.ghosts(pivoted) {
            Some(ghosts) => walk(self.pattern(), &self.lu, Some(ghosts), pivoted),
            None => walk(self.pattern(), &self.lu, None, pivoted),
        }
        self.closed || (pivoted.as_flattened().iter()).fold(true, |finite, value| finite & value.is_finite())
    }
    fn substitute_closed<const L: usize>(&self, pivoted: &mut [[f64; L]]) {
        let Elimination { own, closure, .. } = self.plan;
        let mut lu = Vec::with_capacity(closure.columns.len());
        for at in 0..own.diagonals.len() {
            let (line, values) = (
                own.line(at),
                &self.lu[index(own.starts[at])..index(own.starts[at + 1])],
            );
            lu.extend(
                closure
                    .line(at)
                    .iter()
                    .map(|column| match line.binary_search(column) {
                        Ok(found) => values[found],
                        Err(_) if index(*column) < at => 0.0 / self.lu[index(own.diagonals[index(*column)])],
                        Err(_) => 0.0,
                    }),
            );
        }
        walk(closure, &lu, None, pivoted);
    }
}

#[inline(always)]
fn walk<const L: usize>(pattern: &Pattern, lu: &[f64], ghosts: Option<&Ghosts>, pivoted: &mut [[f64; L]]) {
    for at in 0..pivoted.len() {
        forward(pattern, lu, ghosts, pivoted, at);
    }
    for at in (0..pivoted.len()).rev() {
        back(pattern, lu, ghosts, pivoted, at);
    }
}

#[inline(always)]
fn forward<const L: usize>(
    pattern: &Pattern,
    lu: &[f64],
    ghosts: Option<&Ghosts>,
    pivoted: &mut [[f64; L]],
    at: usize,
) {
    let line = index(pattern.starts[at])..index(pattern.diagonals[at]);
    let mut sum = pivoted[at];
    for (cell, &column) in lu[line.clone()].iter().zip(&pattern.columns[line]) {
        for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
            *value -= cell * read;
        }
    }
    if let Some(ghosts) = ghosts.filter(|_| sum.iter().any(|value| value.to_bits() == NEGATIVE_ZERO)) {
        let negative = |pivot: usize| lu[index(pattern.diagonals[pivot])].is_sign_negative();
        ghosts.settle(&mut sum, ghosts.lower(at), negative, pivoted);
    }
    pivoted[at] = sum;
}

#[inline(always)]
fn back<const L: usize>(
    pattern: &Pattern,
    lu: &[f64],
    ghosts: Option<&Ghosts>,
    pivoted: &mut [[f64; L]],
    at: usize,
) {
    let line = index(pattern.diagonals[at]) + 1..index(pattern.starts[at + 1]);
    let mut sum = pivoted[at];
    for (cell, &column) in lu[line.clone()].iter().zip(&pattern.columns[line]) {
        for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
            *value -= cell * read;
        }
    }
    if let Some(ghosts) = ghosts.filter(|_| sum.iter().any(|value| value.to_bits() == NEGATIVE_ZERO)) {
        ghosts.settle(&mut sum, ghosts.upper(at), |_| false, pivoted);
    }
    let pivot = lu[index(pattern.diagonals[at])];
    for value in &mut sum {
        *value /= pivot;
    }
    pivoted[at] = sum;
}

impl Ghosts {
    fn lower(&self, at: usize) -> &[u32] {
        &self.columns[index(self.starts[at])..index(self.splits[at])]
    }
    fn upper(&self, at: usize) -> &[u32] {
        &self.columns[index(self.splits[at])..index(self.starts[at + 1])]
    }
    /// A ghost holds a zero: a positive one above the diagonal, and below it a zero over its pivot, so of that pivot's
    /// sign. Times a finite read it is a zero whose sign is the two signs' difference. Subtracted, it changes only a
    /// sum that is a negative zero, and a negative zero turns positive and stays so.
    fn settle<const L: usize>(
        &self,
        sum: &mut [f64; L],
        ghosts: &[u32],
        negative: impl Fn(usize) -> bool,
        pivoted: &[[f64; L]],
    ) {
        for (lane, value) in sum.iter_mut().enumerate() {
            let flips =
                |ghost: &u32| negative(index(*ghost)) != pivoted[index(*ghost)][lane].is_sign_negative();
            if value.to_bits() == NEGATIVE_ZERO && ghosts.iter().any(flips) {
                *value = 0.0;
            }
        }
    }
}

pub(crate) struct Reach {
    lower_starts: Vec<u32>,
    lower: Vec<u32>,
    upper_starts: Vec<u32>,
    upper: Vec<u32>,
}
impl Reach {
    pub(crate) fn key(
        &self,
        rows: &[usize],
        at_row: &[usize],
        marks: &mut Marks,
    ) -> (usize, usize, usize, u64) {
        marks.clear();
        for &row in rows {
            let at = at_row[row];
            if !marks.forward[at] {
                marks.forward[at] = true;
                marks.stack.push(at as u32);
            }
        }
        marks.spread(|marks| &mut marks.forward, &self.lower_starts, &self.lower);
        marks.back.copy_from_slice(&marks.forward);
        marks
            .stack
            .extend((0..marks.forward.len() as u32).filter(|&at| marks.forward[index(at)]));
        marks.spread(|marks| &mut marks.back, &self.upper_starts, &self.upper);
        let mut key = (usize::MAX, 0, 0, 0_u64);
        for at in (0..marks.back.len()).filter(|&at| marks.back[at]) {
            key.0 = key.0.min(at);
            key.1 = at;
            key.2 += 1;
            key.3 = key
                .3
                .wrapping_add((at as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        }
        key
    }
    /// `None` where a factor is not finite, since a zero times it is not zero. `factor` refuses that case first, so
    /// this is a second guard.
    fn of(factors: &Factors<'_>) -> Option<Self> {
        let Pattern {
            starts,
            columns,
            diagonals,
            ..
        } = factors.pattern();
        if !factors.lu.iter().all(|cell| cell.is_finite()) {
            return None;
        }
        let size = diagonals.len();
        let transpose = |lines: &dyn Fn(usize) -> core::ops::Range<usize>| {
            let mut counts = vec![0_u32; size + 1];
            for at in 0..size {
                for entry in lines(at).filter(|&entry| factors.lu[entry] != 0.0) {
                    counts[index(columns[entry]) + 1] += 1;
                }
            }
            for at in 0..size {
                counts[at + 1] += counts[at];
            }
            let mut filled = counts.clone();
            let mut readers = vec![0_u32; index(counts[size])];
            for at in 0..size {
                for entry in lines(at).filter(|&entry| factors.lu[entry] != 0.0) {
                    let column = index(columns[entry]);
                    readers[index(filled[column])] = at as u32;
                    filled[column] += 1;
                }
            }
            (counts, readers)
        };
        let (lower_starts, lower) = transpose(&|at| index(starts[at])..index(diagonals[at]));
        let (upper_starts, upper) = transpose(&|at| index(diagonals[at]) + 1..index(starts[at + 1]));
        Some(Self {
            lower_starts,
            lower,
            upper_starts,
            upper,
        })
    }
}

pub(crate) struct Marks {
    forward: Vec<bool>,
    back: Vec<bool>,
    stack: Vec<u32>,
}
impl Marks {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            forward: vec![false; size],
            back: vec![false; size],
            stack: Vec::with_capacity(size),
        }
    }
    fn clear(&mut self) {
        self.forward.fill(false);
        self.back.fill(false);
        self.stack.clear();
    }
    fn spread(&mut self, set: impl Fn(&mut Self) -> &mut Vec<bool>, starts: &[u32], readers: &[u32]) {
        while let Some(row) = self.stack.pop() {
            let row = index(row);
            for &reader in &readers[index(starts[row])..index(starts[row + 1])] {
                let marked = &mut set(self)[index(reader)];
                if !*marked {
                    *marked = true;
                    self.stack.push(reader);
                }
            }
        }
    }
    #[cfg(test)]
    fn walked(&self) -> usize {
        self.back.iter().filter(|&&marked| marked).count()
    }
}

#[cfg(test)]
mod tests {
    use super::{Elimination, Factors, LANES, Marks};
    use crate::body::{RowId, UnknownId};

    fn next(state: &mut u64) -> f64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        (*state >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn dense_solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
        let size = b.len();
        for pivot in 0..size {
            let best = (pivot..size)
                .max_by(|&x, &y| a[x][pivot].abs().total_cmp(&a[y][pivot].abs()))
                .expect("a row");
            a.swap(pivot, best);
            b.swap(pivot, best);
            for row in pivot + 1..size {
                let factor = a[row][pivot] / a[pivot][pivot];
                let (above, below) = a.split_at_mut(row);
                for (cell, lead) in below[0].iter_mut().zip(&above[pivot]).skip(pivot) {
                    *cell -= factor * lead;
                }
                b[row] -= factor * b[pivot];
            }
        }
        for row in (0..size).rev() {
            let known: f64 = (row + 1..size).map(|column| a[row][column] * b[column]).sum();
            b[row] = (b[row] - known) / a[row][row];
        }
        b
    }

    /// A negative zero in `a` is an entry of the pattern whose value is zero.
    fn planned(a: &[Vec<f64>], matching: &[RowId]) -> (Elimination, Vec<f64>) {
        let (mut offsets, mut unknowns, mut values) = (vec![0], Vec::new(), Vec::new());
        for line in a {
            for (column, &value) in line.iter().enumerate() {
                if value.to_bits() != 0 {
                    unknowns.push(UnknownId::at(column));
                    values.push(value);
                }
            }
            offsets.push(unknowns.len());
        }
        (Elimination::plan(&offsets, &unknowns, matching), values)
    }

    fn grouped(factors: &Factors<'_>, sides: &[Vec<f64>], reached: bool, marks: &mut Marks) -> Vec<Vec<f64>> {
        let (at_row, at_unknown) = factors.plan.places();
        let size = at_row.len();
        let reach = if reached { factors.reach() } else { None };
        let mut answers = vec![vec![0.0; size]; sides.len()];
        let mut spare = vec![[0.0; LANES]; size];
        for (chunk, answers) in sides.chunks(LANES).zip(answers.chunks_mut(LANES)) {
            let mut slab = vec![[0.0; LANES]; size];
            for (lane, side) in chunk.iter().enumerate() {
                for (row, &value) in side.iter().enumerate() {
                    slab[at_row[row]][lane] = value;
                }
            }
            factors.solve_pivoted(&mut slab, reach.as_ref(), &mut spare, marks);
            for (lane, answer) in answers.iter_mut().enumerate() {
                for (unknown, value) in answer.iter_mut().enumerate() {
                    *value = slab[at_unknown[unknown]][lane];
                }
            }
        }
        answers
    }

    fn shuffled(size: usize, state: &mut u64) -> (Vec<Vec<f64>>, Elimination, Vec<f64>) {
        let shuffle: Vec<usize> = (0..size).map(|at| (at * 7 + 3) % size).collect();
        let mut a = vec![vec![0.0; size]; size];
        for (unknown, &row) in shuffle.iter().enumerate() {
            a[row][unknown] = 4.0 + next(state);
            for _ in 0..3 {
                let other = (next(state) * size as f64) as usize;
                if other != unknown {
                    a[row][other] = next(state) - 0.5;
                }
            }
        }
        let matching: Vec<RowId> = shuffle.iter().map(|&row| RowId::at(row)).collect();
        let (plan, values) = planned(&a, &matching);
        (a, plan, values)
    }

    fn sides(size: usize, state: &mut u64) -> Vec<Vec<f64>> {
        let value = |side: usize, row: usize, state: &mut u64| match (side, (side + row) % 5) {
            (0, _) => -0.0,
            (1, _) => 0.0,
            (_, 0) => -0.0,
            (_, 1) => 0.0,
            (_, 2) => 1e-300 * (next(state) - 0.5),
            (_, 3) => 1e150 * (next(state) - 0.5),
            _ => next(state) - 0.5,
        };
        (0..LANES + 5)
            .map(|side| (0..size).map(|row| value(side, row, state)).collect())
            .collect()
    }

    fn hex(bits: &[u8]) -> String {
        let digest = crate::Digest::of_canonical(bits);
        digest
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn it_gives_the_dense_answer_on_sparse_systems_with_a_shuffled_matching() {
        let mut state = 0x9E37_79B9_7F4A_7C15;
        for size in [1, 2, 5, 17, 40] {
            let (a, plan, values) = shuffled(size, &mut state);
            let side: Vec<f64> = (0..size).map(|_| next(&mut state) - 0.5).collect();
            let mut found = side.clone();
            plan.factor(&values).expect("a pivot").solve(&mut found);
            for (one, other) in found.iter().zip(dense_solve(a.clone(), side)) {
                assert!((one - other).abs() < 1e-11, "{one} against {other}");
            }
        }
    }

    #[test]
    fn each_side_keeps_the_bits_it_had_alone_or_beside_others() {
        let mut state = 0x2545_F491_4F6C_DD1D;
        let (mut alone, mut reached, mut full) = (Vec::new(), Vec::new(), Vec::new());
        let bits = |side: &[f64]| {
            side.iter()
                .flat_map(|value| value.to_bits().to_be_bytes())
                .collect::<Vec<u8>>()
        };
        for size in [1, 2, 5, 17, 40] {
            let (_, plan, values) = shuffled(size, &mut state);
            let factors = plan.factor(&values).expect("a pivot");
            let sides = sides(size, &mut state);
            let mut marks = Marks::new(size);
            let walked = grouped(&factors, &sides, true, &mut marks);
            let whole = grouped(&factors, &sides, false, &mut marks);
            for ((mut side, walked), whole) in sides.into_iter().zip(walked).zip(whole) {
                factors.solve(&mut side);
                alone.extend(bits(&side));
                reached.extend(bits(&walked));
                full.extend(bits(&whole));
            }
        }
        let had = "9c64d90d69b25c98bf72b65448a978ef42f6406db1317f7541e070b3d7eb3478";
        assert_eq!(hex(&alone), had);
        assert_eq!(hex(&reached), had);
        assert_eq!(hex(&full), had);
    }

    /// Two parts of `part` rows, each reading only its own rows, but for one entry of the second part that reads the
    /// first with a value of zero, and a last row that reads only itself. Odd rows are negated, so some pivots are
    /// negative.
    fn two_parts(part: usize) -> (Elimination, Vec<f64>) {
        let mut state = 0x9E37_79B9_7F4A_7C15;
        let mut a = vec![vec![0.0; 2 * part + 1]; 2 * part + 1];
        for (row, line) in a.iter_mut().enumerate() {
            let first = row / part * part;
            line[row] = 4.0 + next(&mut state);
            if row < 2 * part {
                line[first + (row + 1) % part] = next(&mut state) - 0.5;
                line[first + (row + 5) % part] = next(&mut state) - 0.5;
            }
            if row % 2 == 1 {
                line.iter_mut()
                    .filter(|cell| **cell != 0.0)
                    .for_each(|cell| *cell = -*cell);
            }
        }
        a[part + 3][2] = -0.0;
        let matching: Vec<RowId> = (0..=2 * part).map(RowId::at).collect();
        planned(&a, &matching)
    }

    #[test]
    fn a_group_walks_only_the_rows_its_sides_reach() {
        let mut state = 0x9E37_79B9_7F4A_7C15;
        let part = 12;
        let mut a = vec![vec![0.0; 2 * part]; 2 * part];
        for (row, line) in a.iter_mut().enumerate() {
            let first = row / part * part;
            line[row] = 4.0 + next(&mut state);
            line[first + (row + 1) % part] = next(&mut state) - 0.5;
            line[first + (row + 5) % part] = next(&mut state) - 0.5;
        }
        let matching: Vec<RowId> = (0..2 * part).map(RowId::at).collect();
        let (plan, values) = planned(&a, &matching);
        let factors = plan.factor(&values).expect("a pivot");
        let sides: Vec<Vec<f64>> = (0..LANES)
            .map(|lane| {
                (0..2 * part)
                    .map(|row| {
                        if row < part && row % 3 == lane % 3 {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        let mut marks = Marks::new(2 * part);
        let walked = grouped(&factors, &sides, true, &mut marks);
        assert_eq!(marks.walked(), part, "the group walks its own part's rows");
        for (mut side, walked) in sides.into_iter().zip(walked) {
            factors.solve(&mut side);
            let same = side
                .iter()
                .zip(&walked)
                .all(|(one, other)| one.to_bits() == other.to_bits());
            assert!(same, "a short walk keeps every bit");
        }
    }

    fn alone_and_walked(factors: &Factors<'_>, sides: Vec<Vec<f64>>) -> bool {
        let size = sides[0].len();
        let walked = grouped(factors, &sides, true, &mut Marks::new(size));
        sides.into_iter().zip(walked).all(|(mut side, walked)| {
            factors.solve(&mut side);
            side.iter()
                .zip(&walked)
                .all(|(one, other)| one.to_bits() == other.to_bits())
        })
    }

    /// A row the walk leaves out reads an infinite one through a factor of zero, which the full walk turns into NaN.
    #[test]
    fn a_value_that_is_not_finite_takes_the_full_walk_and_keeps_its_bits() {
        let (plan, values) = two_parts(12);
        let factors = plan.factor(&values).expect("a pivot");
        let sides: Vec<Vec<f64>> = (0..LANES)
            .map(|lane| {
                (0..25)
                    .map(|row| match (lane, row) {
                        (2, 2) => f64::INFINITY,
                        (5, 7) => f64::NEG_INFINITY,
                        _ => 0.0,
                    })
                    .collect()
            })
            .collect();
        assert!(
            alone_and_walked(&factors, sides),
            "a value that is not finite keeps the full walk's bits"
        );
    }

    /// Rows whose pivots are negative turn a zero over the pivot into a negative zero.
    #[test]
    fn a_negative_zero_side_keeps_its_sign_through_negative_pivots() {
        let (plan, values) = two_parts(12);
        let factors = plan.factor(&values).expect("a pivot");
        let sides: Vec<Vec<f64>> = (0..LANES)
            .map(|lane| {
                (0..25)
                    .map(|row| if lane > 0 && row == lane + 17 { -0.0 } else { 0.0 })
                    .collect()
            })
            .collect();
        assert!(alone_and_walked(&factors, sides), "zeros keep their signs");
    }

    impl Elimination {
        fn factor_closed(&self, values: &[f64]) -> Result<Factors<'_>, usize> {
            let lu = self.eliminate(&self.closure, values).found;
            let lu = lu.map_err(|at| self.rows[at] as usize)?;
            Ok(Factors {
                plan: self,
                closed: true,
                lu,
            })
        }
    }

    struct System {
        a: Vec<Vec<f64>>,
        offsets: Vec<usize>,
        unknowns: Vec<UnknownId>,
        values: Vec<f64>,
        matching: Vec<RowId>,
    }

    fn system(size: usize, reads: usize, state: &mut u64) -> System {
        let mut lines = vec![alloc::collections::BTreeMap::new(); size];
        let matching: Vec<RowId> = (0..size)
            .map(|unknown| RowId::at((unknown * 7 + 3) % size))
            .collect();
        for (unknown, row) in matching.iter().enumerate() {
            let sign = if next(state) < 0.5 { -1.0 } else { 1.0 };
            lines[row.index()].insert(unknown, sign * (4.0 + next(state)));
            for _ in 0..reads {
                let other = (next(state) * size as f64) as usize;
                let value = match (next(state) * 8.0) as usize {
                    0 => -0.0,
                    1 => 0.0,
                    _ => next(state) - 0.5,
                };
                lines[row.index()].entry(other).or_insert(value);
            }
        }
        let (mut a, mut offsets, mut unknowns, mut values) =
            (vec![vec![0.0; size]; size], vec![0], vec![], vec![]);
        for (row, line) in lines.iter().enumerate() {
            for (&unknown, &value) in line {
                a[row][unknown] = value;
                unknowns.push(UnknownId::at(unknown));
                values.push(value);
            }
            offsets.push(unknowns.len());
        }
        System {
            a,
            offsets,
            unknowns,
            values,
            matching,
        }
    }

    fn systems(state: &mut u64) -> Vec<(System, Elimination)> {
        let mut found = Vec::new();
        for size in [1, 2, 5, 17, 40, 64] {
            for reads in [1, 2, 3] {
                let system = system(size, reads, state);
                let plan = Elimination::plan(&system.offsets, &system.unknowns, &system.matching);
                found.push((system, plan));
            }
        }
        found
    }

    fn same_bits(one: &[f64], other: &[f64]) -> bool {
        one.len() == other.len()
            && one
                .iter()
                .zip(other)
                .all(|(one, other)| one.to_bits() == other.to_bits())
    }

    #[test]
    fn own_holds_each_rows_entries_and_their_fill_and_no_more() {
        let mut state = 0x6A09_E667_F3BC_C908;
        for (system, plan) in systems(&mut state) {
            let (at_row, at_unknown) = plan.places();
            let size = at_row.len();
            let mut filled = vec![vec![false; size]; size];
            for row in 0..size {
                filled[at_row[row]][at_row[row]] = true;
                for unknown in &system.unknowns[system.offsets[row]..system.offsets[row + 1]] {
                    filled[at_row[row]][at_unknown[unknown.index()]] = true;
                }
            }
            for pivot in 0..size {
                let (above, below) = filled.split_at_mut(pivot + 1);
                for line in below.iter_mut().filter(|line| line[pivot]) {
                    for (cell, &held) in line.iter_mut().zip(&above[pivot]).skip(pivot + 1) {
                        *cell |= held;
                    }
                }
            }
            for (at, line) in filled.iter().enumerate() {
                let wanted: Vec<u32> = (0..size as u32).filter(|&column| line[column as usize]).collect();
                assert_eq!(plan.own.line(at), wanted, "line {at} holds its entries and fill");
                let closed = plan.closure.line(at);
                assert!(
                    wanted.iter().all(|column| closed.contains(column)),
                    "the symmetric pattern holds line {at}"
                );
            }
        }
    }

    #[test]
    fn own_gives_the_dense_answer_on_systems_that_read_one_way() {
        let mut state = 0xBB67_AE85_84CA_A73B;
        for (system, plan) in systems(&mut state) {
            let size = system.a.len();
            let side: Vec<f64> = (0..size).map(|_| next(&mut state) - 0.5).collect();
            let mut found = side.clone();
            plan.factor(&system.values).expect("a pivot").solve(&mut found);
            for (one, other) in found.iter().zip(dense_solve(system.a.clone(), side)) {
                assert!((one - other).abs() < 1e-11, "{one} against {other}");
            }
        }
    }

    #[test]
    fn own_keeps_every_bit_of_the_symmetric_plan() {
        let mut state = 0x3C6E_F372_FE94_F82B;
        for (system, plan) in systems(&mut state) {
            let size = system.a.len();
            let own = plan.factor(&system.values).expect("a pivot");
            let today = plan.factor_closed(&system.values).expect("a pivot");
            assert!(!own.closed, "finite values factor over the body's own pattern");
            for at in 0..size {
                let (line, closed) = (plan.own.line(at), plan.closure.line(at));
                let (from, to) = (plan.own.starts[at] as usize, plan.closure.starts[at] as usize);
                for (place, column) in closed.iter().enumerate() {
                    let had = today.lu[to + place];
                    let kept = match line.binary_search(column) {
                        Ok(found) => own.lu[from + found],
                        Err(_) if (*column as usize) < at => {
                            0.0 / own.lu[plan.own.diagonals[*column as usize] as usize]
                        }
                        Err(_) => 0.0,
                    };
                    assert_eq!(kept.to_bits(), had.to_bits(), "line {at}, column {column}");
                }
            }
            let mut sides = sides(size, &mut state);
            for (side, value) in
                sides
                    .iter_mut()
                    .skip(LANES)
                    .zip([f64::INFINITY, f64::NAN, f64::NEG_INFINITY])
            {
                side[size / 2] = value;
            }
            let mut marks = Marks::new(size);
            for reached in [true, false] {
                let (kept, had) = (
                    grouped(&own, &sides, reached, &mut marks),
                    grouped(&today, &sides, reached, &mut marks),
                );
                for (kept, had) in kept.iter().zip(&had) {
                    assert!(same_bits(kept, had), "a grouped walk keeps today's bits");
                }
            }
            for side in sides {
                let (mut kept, mut had) = (side.clone(), side);
                own.solve(&mut kept);
                today.solve(&mut had);
                assert!(same_bits(&kept, &had), "a solve alone keeps today's bits");
            }
        }
    }

    #[test]
    fn a_value_that_is_not_finite_factors_as_the_symmetric_plan_does() {
        let mut state = 0xA54F_F53A_5F1D_36F1;
        let mut apart = 0;
        for (mut system, plan) in systems(&mut state)
            .into_iter()
            .filter(|(system, _)| system.a.len() > 4)
        {
            let (size, entries) = (system.a.len(), system.values.len());
            for (at, value) in [
                (entries / 3, f64::INFINITY),
                (entries / 2, f64::NAN),
                (entries - 1, -1e308),
            ] {
                let held = core::mem::replace(&mut system.values[at], value);
                let side: Vec<f64> = (0..size).map(|_| next(&mut state) - 0.5).collect();
                let answer = |factors: Result<Factors<'_>, usize>| {
                    factors.map(|factors| {
                        let mut side = side.clone();
                        factors.solve(&mut side);
                        side.iter().map(|value| value.to_bits()).collect::<Vec<u64>>()
                    })
                };
                let had = answer(plan.factor_closed(&system.values));
                assert_eq!(
                    answer(plan.factor(&system.values)),
                    had,
                    "the factors and a solve keep today's bits"
                );
                let own = (plan.eliminate(&plan.own, &system.values).found).map(|lu| Factors {
                    plan: &plan,
                    closed: false,
                    lu,
                });
                apart += usize::from(answer(own.map_err(|at| plan.rows[at] as usize)) != had);
                system.values[at] = held;
            }
        }
        assert!(
            apart > 0,
            "a value that is not finite meets a zero today's plan stores"
        );
    }

    #[test]
    fn a_zero_pivot_names_its_row() {
        let unknowns = [
            UnknownId::at(0),
            UnknownId::at(1),
            UnknownId::at(0),
            UnknownId::at(1),
        ];
        let matching = [RowId::at(0), RowId::at(1)];
        let plan = Elimination::plan(&[0, 2, 4], &unknowns, &matching);
        assert!(plan.factor(&[1.0, 1.0, 1.0, 1.0]).is_err());
    }
}
