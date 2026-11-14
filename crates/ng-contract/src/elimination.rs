use crate::body::{RowId, UnknownId};
use alloc::collections::BTreeSet;

/// A fixed elimination over one sparsity. Each pivot pairs an unknown with the row the structural matching gave it,
/// in least-degree order, ties to the least unknown, so a sparse body factors in about its nonzeros and every run
/// fills the same entries. The pattern is the union over modes, so a mode change moves no entry.
#[derive(Debug, PartialEq)]
pub(crate) struct Elimination {
    rows: Vec<u32>,
    unknowns: Vec<u32>,
    starts: Vec<u32>,
    columns: Vec<u32>,
    diagonals: Vec<u32>,
    places: Vec<u32>,
}

fn index(value: u32) -> usize {
    value as usize
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
        let mut lower: Vec<Vec<u32>> = vec![Vec::new(); size];
        for (at, &node) in order.iter().enumerate() {
            for &other in &fill[index(node)] {
                lower[index(position[index(other)])].push(at as u32);
            }
        }
        let (mut starts, mut columns, mut diagonals) = (vec![0_u32], Vec::new(), Vec::with_capacity(size));
        for (at, &node) in order.iter().enumerate() {
            columns.extend_from_slice(&lower[at]);
            diagonals.push(columns.len() as u32);
            columns.push(at as u32);
            let mut upper: Vec<u32> = (fill[index(node)].iter())
                .map(|&other| position[index(other)])
                .collect();
            upper.sort_unstable();
            columns.extend_from_slice(&upper);
            starts.push(columns.len() as u32);
        }
        let rows: Vec<u32> = (order.iter())
            .map(|&node| solved_by[index(node)].index() as u32)
            .collect();
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
            rows,
            unknowns: order,
            starts,
            columns,
            diagonals,
            places,
        }
    }

    /// Where each row pivots, and where each unknown.
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
        let mut lu = vec![0.0; self.columns.len()];
        for (&place, &value) in self.places.iter().zip(values) {
            lu[index(place)] += value;
        }
        let mut seen = vec![0.0; self.rows.len()];
        for at in 0..self.rows.len() {
            let (start, end) = (index(self.starts[at]), index(self.starts[at + 1]));
            let (done, rest) = lu.split_at_mut(start);
            let line = &mut rest[..end - start];
            for (cell, &column) in line.iter().zip(&self.columns[start..end]) {
                seen[index(column)] = *cell;
            }
            for &pivot in &self.columns[start..index(self.diagonals[at])] {
                let pivot = index(pivot);
                let factor = seen[pivot] / done[index(self.diagonals[pivot])];
                seen[pivot] = factor;
                let tail = index(self.diagonals[pivot]) + 1..index(self.starts[pivot + 1]);
                for (cell, &column) in done[tail.clone()].iter().zip(&self.columns[tail]) {
                    seen[index(column)] -= factor * cell;
                }
            }
            for (cell, &column) in line.iter_mut().zip(&self.columns[start..end]) {
                *cell = seen[index(column)];
            }
            let diagonal = seen[at];
            if diagonal == 0.0 || !diagonal.is_finite() {
                return Err(index(self.rows[at]));
            }
        }
        Ok(Factors { plan: self, lu })
    }
}
pub struct Factors<'b> {
    plan: &'b Elimination,
    lu: Vec<f64>,
}
pub(crate) const LANES: usize = 8;

impl Factors<'_> {
    /// Replaces a right side, one slot per row, with the update, one slot per unknown.
    pub fn solve(&self, side: &mut [f64]) {
        let Elimination { rows, unknowns, .. } = self.plan;
        let mut pivoted: Vec<[f64; 1]> = rows.iter().map(|&row| [side[index(row)]]).collect();
        self.substitute(&mut pivoted);
        for (&unknown, [value]) in unknowns.iter().zip(pivoted) {
            side[index(unknown)] = value;
        }
    }
    pub(crate) fn reach(&self) -> Option<Reach> {
        Reach::of(self)
    }
    /// One group of sides in pivot order, each row's side where its row pivots, replaced by their updates where each
    /// unknown pivots.
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
            self.substitute(group);
        }
    }
    /// Walks only the rows a nonzero side reaches through a nonzero factor, in the order a full walk takes them. A row
    /// whose side is a positive zero and whose nonzero factors read only such rows keeps that zero going forward and
    /// takes zero over its pivot going back, as a full walk gives it. A value that is not finite sends the group back
    /// to the full walk, since a zero factor times it is not zero.
    fn substitute_reached(&self, pivoted: &mut [[f64; LANES]], reach: &Reach, marks: &mut Marks) -> bool {
        let Elimination {
            starts,
            columns,
            diagonals,
            ..
        } = self.plan;
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
            let line = index(starts[at])..index(diagonals[at]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
                    *value -= cell * read;
                }
            }
            pivoted[at] = sum;
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
            let line = index(diagonals[at]) + 1..index(starts[at + 1]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
                    *value -= cell * read;
                }
            }
            let pivot = self.lu[index(diagonals[at])];
            for value in &mut sum {
                *value /= pivot;
            }
            pivoted[at] = sum;
        }
        (0..size)
            .filter(|&at| marks.back[at])
            .all(|at| pivoted[at].iter().all(|value| value.is_finite()))
    }
    fn substitute<const L: usize>(&self, pivoted: &mut [[f64; L]]) {
        let Elimination {
            starts,
            columns,
            diagonals,
            ..
        } = self.plan;
        for at in 0..pivoted.len() {
            let line = index(starts[at])..index(diagonals[at]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
                    *value -= cell * read;
                }
            }
            pivoted[at] = sum;
        }
        for at in (0..pivoted.len()).rev() {
            let line = index(diagonals[at]) + 1..index(starts[at + 1]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                for (value, read) in sum.iter_mut().zip(pivoted[index(column)]) {
                    *value -= cell * read;
                }
            }
            let pivot = self.lu[index(diagonals[at])];
            for value in &mut sum {
                *value /= pivot;
            }
            pivoted[at] = sum;
        }
    }
}

/// For each row, the rows whose nonzero factor reads it: below the diagonal going forward, above it going back.
pub(crate) struct Reach {
    lower_starts: Vec<u32>,
    lower: Vec<u32>,
    upper_starts: Vec<u32>,
    upper: Vec<u32>,
}
impl Reach {
    /// `None` where a factor is not finite, since a zero times it is not zero.
    fn of(factors: &Factors<'_>) -> Option<Self> {
        let Elimination {
            starts,
            columns,
            diagonals,
            ..
        } = factors.plan;
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

/// The rows a reach walk marks, kept from one group to the next so no group allocates.
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

    fn planned(a: &[Vec<f64>], matching: &[RowId]) -> (Elimination, Vec<f64>) {
        let (mut offsets, mut unknowns, mut values) = (vec![0], Vec::new(), Vec::new());
        for line in a {
            for (column, &value) in line.iter().enumerate() {
                if value != 0.0 {
                    unknowns.push(UnknownId::at(column));
                    values.push(value);
                }
            }
            offsets.push(unknowns.len());
        }
        (Elimination::plan(&offsets, &unknowns, matching), values)
    }

    /// Each group of `LANES` sides laid in pivot order, solved, and read back one side at a time.
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

    /// Two parts that never read each other: sides nonzero on one part walk that part's rows and no others.
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

    #[test]
    fn a_value_that_is_not_finite_takes_the_full_walk_and_keeps_its_bits() {
        let mut state = 0x2545_F491_4F6C_DD1D;
        let (_, plan, values) = shuffled(17, &mut state);
        let factors = plan.factor(&values).expect("a pivot");
        let sides: Vec<Vec<f64>> = (0..LANES)
            .map(|lane| {
                (0..17)
                    .map(|row| match (lane + row) % 7 {
                        0 if lane == 2 => f64::INFINITY,
                        0 if lane == 5 => f64::NAN,
                        1 => -0.0,
                        _ => 0.0,
                    })
                    .collect()
            })
            .collect();
        let walked = grouped(&factors, &sides, true, &mut Marks::new(17));
        for (mut side, walked) in sides.into_iter().zip(walked) {
            factors.solve(&mut side);
            let same = side
                .iter()
                .zip(&walked)
                .all(|(one, other)| one.to_bits() == other.to_bits());
            assert!(same, "a value that is not finite keeps the full walk's bits");
        }
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
