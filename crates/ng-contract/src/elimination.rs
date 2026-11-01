use crate::body::{RowId, UnknownId};
use std::collections::BTreeSet;

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
    /// One per Jacobian entry: where it lands in the pattern.
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

    /// Factors `values`, one per Jacobian entry. A pivot of zero or past the finite range names its row: the
    /// elimination never moves a pivot, so a caller that needs one moved solves densely.
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
/// One Jacobian, factored over its body's elimination.
pub struct Factors<'b> {
    plan: &'b Elimination,
    lu: Vec<f64>,
}
impl Factors<'_> {
    /// Replaces a right side, one slot per row, with the update, one slot per unknown.
    pub fn solve(&self, side: &mut [f64]) {
        let Elimination {
            rows,
            unknowns,
            starts,
            columns,
            diagonals,
            ..
        } = self.plan;
        let mut pivoted: Vec<f64> = rows.iter().map(|&row| side[index(row)]).collect();
        for at in 0..rows.len() {
            let line = index(starts[at])..index(diagonals[at]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                sum -= cell * pivoted[index(column)];
            }
            pivoted[at] = sum;
        }
        for at in (0..rows.len()).rev() {
            let line = index(diagonals[at]) + 1..index(starts[at + 1]);
            let mut sum = pivoted[at];
            for (cell, &column) in self.lu[line.clone()].iter().zip(&columns[line]) {
                sum -= cell * pivoted[index(column)];
            }
            pivoted[at] = sum / self.lu[index(diagonals[at])];
        }
        for (&unknown, value) in unknowns.iter().zip(pivoted) {
            side[index(unknown)] = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Elimination;
    use crate::body::{RowId, UnknownId};

    /// A fixed mixer over a seed, so the matrices are a function of this file.
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
                for column in pivot..size {
                    let lead = a[pivot][column];
                    a[row][column] -= factor * lead;
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

    #[test]
    fn it_gives_the_dense_answer_on_sparse_systems_with_a_shuffled_matching() {
        let mut state = 0x9E37_79B9_7F4A_7C15;
        for size in [1, 2, 5, 17, 40] {
            let shuffle: Vec<usize> = (0..size).map(|at| (at * 7 + 3) % size).collect();
            let mut a = vec![vec![0.0; size]; size];
            for (unknown, &row) in shuffle.iter().enumerate() {
                a[row][unknown] = 4.0 + next(&mut state);
                for _ in 0..3 {
                    let other = (next(&mut state) * size as f64) as usize;
                    if other != unknown {
                        a[row][other] = next(&mut state) - 0.5;
                    }
                }
            }
            let (mut offsets, mut unknowns, mut values) = (vec![0], Vec::new(), Vec::new());
            for line in &a {
                for (column, &value) in line.iter().enumerate() {
                    if value != 0.0 {
                        unknowns.push(UnknownId::at(column));
                        values.push(value);
                    }
                }
                offsets.push(unknowns.len());
            }
            let matching: Vec<RowId> = shuffle.iter().map(|&row| RowId::at(row)).collect();
            let plan = Elimination::plan(&offsets, &unknowns, &matching);
            let side: Vec<f64> = (0..size).map(|_| next(&mut state) - 0.5).collect();
            let mut found = side.clone();
            plan.factor(&values).expect("a pivot").solve(&mut found);
            for (one, other) in found.iter().zip(dense_solve(a.clone(), side)) {
                assert!((one - other).abs() < 1e-11, "{one} against {other}");
            }
        }
    }

    #[test]
    fn a_zero_pivot_names_its_row() {
        let unknowns = [UnknownId::at(0), UnknownId::at(1), UnknownId::at(0), UnknownId::at(1)];
        let matching = [RowId::at(0), RowId::at(1)];
        let plan = Elimination::plan(&[0, 2, 4], &unknowns, &matching);
        assert!(matches!(plan.factor(&[1.0, 1.0, 1.0, 1.0]), Err(_)));
    }
}
