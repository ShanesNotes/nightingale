use crate::elimination::Factors;
use crate::{InitializationSystem, Jacobian};

const ROUNDING: f64 = 8.0 * f64::EPSILON;

/// `matrix` holds each row with its right side last. A row with no pivot left is named: it has no rank. A pivot is read
/// beside its row and its column as given, and the smaller of the two: after elimination a row's own largest entry can be
/// rounding, and a row and a column can each run at a scale of their own.
pub fn eliminate(matrix: Vec<Vec<f64>>) -> Result<Vec<f64>, usize> {
    Ok(reduced(matrix)?.into_iter().map(|answers| answers[0]).collect())
}

/// Each row holds its entries and then any number of right sides, and each row of the answer one value per side.
fn reduced(mut matrix: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, usize> {
    let size = matrix.len();
    let mut slots: Vec<usize> = (0..size).collect();
    let largest = |most: f64, cell: f64| if cell.abs() > most { cell.abs() } else { most };
    let mut across: Vec<f64> = (matrix.iter())
        .map(|row| row[..size].iter().copied().fold(0.0, largest))
        .collect();
    let down: Vec<f64> = (0..size)
        .map(|column| matrix.iter().map(|row| row[column]).fold(0.0, largest))
        .collect();
    for pivot in 0..size {
        let mut best = pivot;
        for row in pivot + 1..size {
            if matrix[row][pivot].abs() > matrix[best][pivot].abs() {
                best = row;
            }
        }
        let reach = if across[best] < down[pivot] {
            across[best]
        } else {
            down[pivot]
        };
        if matrix[best][pivot].abs() <= ROUNDING * reach {
            return Err(slots[pivot]);
        }
        matrix.swap(pivot, best);
        slots.swap(pivot, best);
        across.swap(pivot, best);
        let (above, below) = matrix.split_at_mut(pivot + 1);
        let leading = &above[pivot];
        for row in below {
            let factor = row[pivot] / leading[pivot];
            for (cell, lead) in row.iter_mut().zip(leading).skip(pivot) {
                *cell -= factor * lead;
            }
        }
    }
    let sides = matrix.first().map_or(0, |row| row.len() - size);
    let mut answers = vec![vec![0.0; sides]; size];
    for row in (0..size).rev() {
        for side in 0..sides {
            let known: f64 = (row + 1..size)
                .map(|column| matrix[row][column] * answers[column][side])
                .sum();
            answers[row][side] = (matrix[row][size + side] - known) / matrix[row][row];
        }
    }
    Ok(answers)
}

enum Block<'b> {
    Fixed(Factors<'b>),
    Whole(Vec<Vec<f64>>),
}
impl Block<'_> {
    fn solve(&self, sides: &mut [f64], width: usize) {
        match self {
            Block::Fixed(factors) => factors.solve_sides(sides, width),
            Block::Whole(inverse) => {
                let answer = |slot: usize| -> f64 {
                    let side = sides.iter().skip(slot % width).step_by(width);
                    inverse[slot / width]
                        .iter()
                        .zip(side)
                        .map(|(cell, value)| cell * value)
                        .sum()
                };
                let answers: Vec<f64> = (0..sides.len()).map(answer).collect();
                sides.copy_from_slice(&answers);
            }
        }
    }
}

/// A zero on the fixed path says nothing of rank, so the block is read whole before a row is named.
fn inverse(block: &Jacobian) -> Result<Vec<Vec<f64>>, usize> {
    let view = block.view();
    let size = view.row_offsets().len() - 1;
    let mut matrix = vec![vec![0.0; 2 * size]; size];
    for (row, cells) in matrix.iter_mut().enumerate() {
        for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
            cells[view.columns()[entry].index()] += view.values()[entry];
        }
        cells[size + row] = 1.0;
    }
    reduced(matrix)
}

impl InitializationSystem<'_> {
    /// One Newton update on the ring `linearize` lays out: `points` blocks of `size` rows, each coupled to the block
    /// before it by its stage's tangent alone, totals in the last block reading point 0, and fits along the border.
    /// Dense, that is the cube of `points` times `size` unknowns.
    ///
    /// Each point but the last factors over the body's elimination. Walking forward from point 0, every point becomes a
    /// linear function of the last point's rows that point 0 reads and of the fits, so the ring shrinks to one dense
    /// corner of the last point's rows and the fits' anchors. A second walk fills every point from the corner's answer.
    /// A row is named where its point's block or the corner has no pivot, and `admit` reads that the same
    /// way, on the whole ring.
    pub fn solve(&self, ring: &Jacobian, right: &[f64]) -> Result<Vec<f64>, usize> {
        let (system, body) = (self, self.body);
        let view = ring.view();
        let (size, points) = (system.rows(), system.points());
        let border = points * size;
        let fits = right.len() - border;
        let line = |slot: usize| {
            let entries = view.row_offsets()[slot]..view.row_offsets()[slot + 1];
            (view.columns()[entries.clone()]
                .iter()
                .map(|column| column.index()))
            .zip(view.values()[entries].iter().copied())
        };
        let own = |point: usize| point * size..(point + 1) * size;
        let last = points - 1;
        let mut read: Vec<usize> = (0..size)
            .flat_map(|row| line(row).map(|(column, _)| column))
            .filter(|column| own(last).contains(column) && last != 0)
            .map(|column| column - last * size)
            .collect();
        read.sort_unstable();
        read.dedup();
        let seeds = read.len() + fits;
        let width = seeds + 1;
        let mut factors = Vec::with_capacity(last);
        let (mut before, mut first) = (Vec::new(), Vec::new());
        let mut anchors = vec![vec![0.0; width]; fits];
        for point in 0..last {
            let block = system.block(ring, point);
            let lu = match body.factor(&block) {
                Ok(factors) => Block::Fixed(factors),
                Err(_) => Block::Whole(inverse(&block)?),
            };
            let mut walked = vec![0.0; size * width];
            for (row, sides) in walked.chunks_mut(width).enumerate() {
                let slot = point * size + row;
                sides[seeds] = right[slot];
                for (place, value) in line(slot).filter(|(place, _)| !own(point).contains(place)) {
                    if place >= border {
                        let fit = read.len() + place - border;
                        for (at, side) in sides.iter_mut().enumerate() {
                            *side -= value * f64::from(at == fit);
                        }
                    } else if point == 0 {
                        let seed = read.binary_search(&(place - last * size));
                        for (at, side) in sides.iter_mut().enumerate() {
                            *side -= value * f64::from(seed == Ok(at));
                        }
                    } else {
                        let earlier = &before[(place - (point - 1) * size) * width..][..width];
                        for (side, earlier) in sides.iter_mut().zip(earlier) {
                            *side -= value * earlier;
                        }
                    }
                }
            }
            lu.solve(&mut walked, width);
            for (fit, row) in anchors.iter_mut().enumerate() {
                for (place, value) in line(border + fit).filter(|(place, _)| own(point).contains(place)) {
                    let walked = &walked[(place - point * size) * width..][..width];
                    for (cell, walked) in row.iter_mut().zip(walked) {
                        *cell += value * walked;
                    }
                }
            }
            if point == 0 {
                first.clone_from(&walked);
            }
            before = walked;
            factors.push(lu);
        }
        let corner = size + fits;
        let mut matrix = vec![vec![0.0; corner + 1]; corner];
        let add = |cells: &mut Vec<f64>, source: &[f64], place: usize, value: f64| {
            let sides = &source[place * width..][..width];
            for (at, &seed) in read.iter().enumerate() {
                cells[seed] += value * sides[at];
            }
            for fit in 0..fits {
                cells[size + fit] += value * sides[read.len() + fit];
            }
            cells[corner] -= value * sides[seeds];
        };
        for (row, cells) in matrix.iter_mut().enumerate().take(size) {
            let slot = last * size + row;
            cells[corner] = right[slot];
            for (place, value) in line(slot) {
                if own(last).contains(&place) {
                    cells[place - last * size] += value;
                } else if place >= border {
                    cells[size + place - border] += value;
                } else if place < size {
                    add(cells, &first, place, value);
                } else {
                    add(cells, &before, place - (last - 1) * size, value);
                }
            }
        }
        for (fit, row) in anchors.iter().enumerate() {
            let cells = &mut matrix[size + fit];
            cells[corner] = right[border + fit] - row[seeds];
            for (at, &seed) in read.iter().enumerate() {
                cells[seed] += row[at];
            }
            for other in 0..fits {
                cells[size + other] += row[read.len() + other];
            }
            for (place, value) in line(border + fit).filter(|(place, _)| own(last).contains(place)) {
                cells[place - last * size] += value;
            }
        }
        let answer = eliminate(matrix)?;
        let mut update = vec![0.0; right.len()];
        update[last * size..points * size].copy_from_slice(&answer[..size]);
        update[border..].copy_from_slice(&answer[size..]);
        for (point, lu) in factors.iter().enumerate() {
            let mut side = vec![0.0; size];
            for (row, cell) in side.iter_mut().enumerate() {
                let slot = point * size + row;
                let foreign = line(slot).filter(|(place, _)| !own(point).contains(place));
                *cell = right[slot] - foreign.map(|(place, value)| value * update[place]).sum::<f64>();
            }
            lu.solve(&mut side, 1);
            update[point * size..(point + 1) * size].copy_from_slice(&side);
        }
        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, inverse};
    use crate::{Jacobian, UnknownId};

    #[test]
    fn a_whole_block_reads_each_side_as_it_reads_it_alone() {
        let size = 4;
        let (mut offsets, mut columns, mut values) = (vec![0], Vec::new(), Vec::new());
        for row in 0..size {
            for column in 0..size {
                columns.push(UnknownId::at(column));
                let off = 0.25 * (row as f64 - column as f64);
                values.push(if row == column { 3.0 + row as f64 } else { off });
            }
            offsets.push(columns.len());
        }
        let block = Jacobian::of_parts(offsets, columns, values);
        let whole = Block::Whole(inverse(&block).expect("a pivot"));
        let width = 5;
        let value = |side: usize, row: usize| match (side + row) % 3 {
            0 => -0.0,
            1 => 0.1 * (side + 1) as f64,
            _ => -(row as f64) - 0.5,
        };
        let sides: Vec<Vec<f64>> = (0..width)
            .map(|side| (0..size).map(|row| value(side, row)).collect())
            .collect();
        let mut together: Vec<f64> = (0..size * width)
            .map(|slot| sides[slot % width][slot / width])
            .collect();
        whole.solve(&mut together, width);
        for (at, mut alone) in sides.into_iter().enumerate() {
            whole.solve(&mut alone, 1);
            for (row, value) in alone.iter().enumerate() {
                assert_eq!(value.to_bits(), together[row * width + at].to_bits());
            }
        }
    }
}
