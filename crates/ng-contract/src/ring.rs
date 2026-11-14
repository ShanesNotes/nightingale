use crate::admission::{AdmissionWorkspace, Correction};
use crate::elimination::{Factors, LANES, Marks, Reach};
use crate::{InitializationSystem, Jacobian};

const ROUNDING: f64 = 8.0 * f64::EPSILON;
const PANEL: usize = 16;

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
    // Each cell takes the same terms in the same order as a pivot at a time gives it.
    let mut pending: Vec<[Option<f64>; PANEL]> = vec![[None; PANEL]; size];
    for start in (0..size).step_by(PANEL) {
        let end = (start + PANEL).min(size);
        for pivot in start..end {
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
            pending.swap(pivot, best);
            let (above, below) = matrix.split_at_mut(pivot + 1);
            let leading = &above[pivot][pivot..end];
            for (row, waiting) in below.iter_mut().zip(&mut pending[pivot + 1..]) {
                // A row with nothing under the pivot keeps its cells; a zero factor would only rewrite a zero's sign.
                if row[pivot] == 0.0 {
                    continue;
                }
                let factor = row[pivot] / leading[0];
                waiting[pivot - start] = Some(factor);
                for (cell, lead) in row[pivot..end].iter_mut().zip(leading) {
                    *cell -= factor * lead;
                }
            }
        }
        for (row, waiting) in pending.iter().enumerate().skip(start + 1) {
            let (above, below) = matrix.split_at_mut(row);
            let cells = &mut below[0][end..];
            for pivot in start..end.min(row) {
                if let Some(factor) = waiting[pivot - start] {
                    for (cell, lead) in cells.iter_mut().zip(&above[pivot][end..]) {
                        *cell -= factor * lead;
                    }
                }
            }
        }
        pending.fill([None; PANEL]);
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
    fn reach(&self) -> Option<Reach> {
        match self {
            Block::Fixed(factors) => factors.reach(),
            Block::Whole(_) => None,
        }
    }
    fn solve_group(
        &self,
        group: &mut [[f64; LANES]],
        reach: Option<&Reach>,
        (at_row, at_unknown): (&[usize], &[usize]),
        spare: &mut [[f64; LANES]],
        marks: &mut Marks,
    ) {
        match self {
            Block::Fixed(factors) => factors.solve_pivoted(group, reach, spare, marks),
            Block::Whole(_) => {
                let mut sides: Vec<Vec<f64>> = (0..LANES)
                    .map(|lane| at_row.iter().map(|&at| group[at][lane]).collect())
                    .collect();
                for side in &mut sides {
                    self.solve(side);
                }
                for (unknown, &at) in at_unknown.iter().enumerate() {
                    for (cell, side) in group[at].iter_mut().zip(&sides) {
                        *cell = side[unknown];
                    }
                }
            }
        }
    }
    fn solve(&self, side: &mut [f64]) {
        match self {
            Block::Fixed(factors) => factors.solve(side),
            Block::Whole(inverse) => {
                let answer = |row: &Vec<f64>| -> f64 {
                    row.iter().zip(&*side).map(|(cell, value)| cell * value).sum()
                };
                let answers: Vec<f64> = inverse.iter().map(answer).collect();
                side.copy_from_slice(&answers);
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
    /// One Newton update on the ring `linearize` lays out: a block of each leg's stages, each coupled to the
    /// block before it through that block's end stage alone, totals in the last block reading point 0, and fits along
    /// the border. Dense, that is the cube of `points` times the block's unknowns.
    ///
    /// Each block but the last factors over the interval's plan. Walking forward from point 0, every block becomes a
    /// linear function of the last block's slots that point 0 reads and of the fits, so the ring shrinks to one dense
    /// corner of the last block and the fits' anchors. A second walk fills every block from the corner's answer. A slot
    /// is named where its block or the corner has no pivot, and `admit` reads that the same way, on the whole ring.
    pub fn solve(
        &self,
        workspace: &AdmissionWorkspace,
        ring: &Jacobian,
        right: &[f64],
    ) -> Result<Correction, usize> {
        let update = self.ring(workspace, ring, right)?;
        let slots = workspace.fitted_slots().end;
        Ok(Correction::of(update[..slots].to_vec(), update[slots..].to_vec()))
    }
    pub(crate) fn ring(
        &self,
        workspace: &AdmissionWorkspace,
        ring: &Jacobian,
        right: &[f64],
    ) -> Result<Vec<f64>, usize> {
        let system = self;
        let view = ring.view();
        let (size, points) = (system.width(), workspace.leg_count());
        let plan = system.stage_plan().filter(|_| points > 1 && size > 0);
        let Some(plan) = plan else {
            let mut matrix = vec![vec![0.0; right.len() + 1]; right.len()];
            for (row, cells) in matrix.iter_mut().enumerate() {
                for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
                    cells[view.columns()[entry].index()] += view.values()[entry];
                }
                cells[right.len()] = right[row];
            }
            return eliminate(matrix);
        };
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
        let groups = width.div_ceil(LANES);
        let (at_row, at_unknown) = plan.places();
        let (mut order, mut slot_of): (Vec<usize>, Vec<usize>) = ((0..width).collect(), (0..width).collect());
        let (mut before, mut first): (Vec<[f64; LANES]>, Vec<[f64; LANES]>) = (Vec::new(), Vec::new());
        let mut anchors = vec![vec![0.0; width]; fits];
        let mut spare = vec![[0.0; LANES]; size];
        let mut marks = Marks::new(size);
        let target = |place: usize| {
            if place >= border {
                Ok(read.len() + place - border)
            } else {
                read.binary_search(&(place - last * size))
            }
        };
        for point in 0..last {
            let block = system.block(ring, point);
            let lu = match system.factor(&block) {
                Ok(factors) => Block::Fixed(factors),
                Err(_) => Block::Whole(inverse(&block)?),
            };
            let reach = lu.reach();
            let foreign: Vec<Vec<(usize, f64)>> = (0..size)
                .map(|row| {
                    (line(point * size + row).filter(|(place, _)| !own(point).contains(place))).collect()
                })
                .collect();
            let seeded = |place: usize| place >= border || point == 0;
            if let (0, Some(reach)) = (point, &reach) {
                let mut rows: Vec<Vec<usize>> = vec![Vec::new(); width];
                for (row, entries) in foreign.iter().enumerate() {
                    for column in entries.iter().filter_map(|&(place, _)| target(place).ok()) {
                        rows[column].push(row);
                    }
                }
                rows[seeds] = (0..size).collect();
                let keys: Vec<_> = rows
                    .iter()
                    .map(|rows| reach.key(rows, &at_row, &mut marks))
                    .collect();
                order.sort_by_key(|&column| (keys[column], column));
                for (slot, &column) in order.iter().enumerate() {
                    slot_of[column] = slot;
                }
            }
            let general =
                (foreign.iter().flatten()).any(|&(place, value)| seeded(place) && !value.is_finite());
            let mut reads: Vec<Vec<(usize, f64)>> = vec![Vec::new(); size];
            let mut terms: Vec<Vec<(usize, usize, usize, f64)>> = vec![Vec::new(); groups];
            for (row, entries) in foreign.iter().enumerate() {
                for &(place, value) in entries {
                    if !seeded(place) {
                        reads[row].push((at_unknown[place - (point - 1) * size], value));
                    } else if let Ok(column) = target(place) {
                        let slot = slot_of[column];
                        terms[slot / LANES].push((row, reads[row].len(), slot % LANES, value));
                    }
                }
            }
            let mut walked = vec![[0.0; LANES]; groups * size];
            for (group, cells) in walked.chunks_mut(size).enumerate() {
                let lanes = &order[group * LANES..((group + 1) * LANES).min(width)];
                let holds_right = group == slot_of[seeds] / LANES;
                if holds_right || general {
                    for (row, entries) in foreign.iter().enumerate() {
                        let sides = &mut cells[at_row[row]];
                        if holds_right {
                            sides[slot_of[seeds] % LANES] = right[point * size + row];
                        }
                        for &(place, value) in entries {
                            if seeded(place) {
                                let column = target(place);
                                if value.is_finite() {
                                    // A side for a column `read` names or for a fit is a positive zero or not zero,
                                    // so a finite term leaves every side but its own and the right side's as it is.
                                    for at in column.ok().into_iter().chain([seeds]) {
                                        if slot_of[at] / LANES == group {
                                            sides[slot_of[at] % LANES] -= value * f64::from(column == Ok(at));
                                        }
                                    }
                                } else {
                                    for (side, &at) in sides.iter_mut().zip(lanes) {
                                        *side -= value * f64::from(column == Ok(at));
                                    }
                                }
                            } else {
                                let earlier = &before[group * size + at_unknown[place - (point - 1) * size]];
                                // A side for a column `read` names or for a fit is a positive zero or not zero, so a
                                // finite term over a zero read leaves it as it is. The right side's can be a negative
                                // zero, and takes every term.
                                if !holds_right
                                    && value.is_finite()
                                    && earlier.iter().all(|value| *value == 0.0)
                                {
                                    continue;
                                }
                                for ((side, earlier), _) in sides.iter_mut().zip(earlier).zip(lanes) {
                                    *side -= value * earlier;
                                }
                            }
                        }
                    }
                } else {
                    let mut waiting = terms[group].iter().peekable();
                    for (row, reads) in reads.iter().enumerate() {
                        let sides = &mut cells[at_row[row]];
                        for at in 0..=reads.len() {
                            while let Some(&(_, _, lane, value)) =
                                waiting.next_if(|term| (term.0, term.1) == (row, at))
                            {
                                sides[lane] -= value;
                            }
                            let Some(&(unknown, value)) = reads.get(at) else {
                                continue;
                            };
                            let earlier = &before[group * size + unknown];
                            if value.is_finite() && earlier.iter().all(|value| *value == 0.0) {
                                continue;
                            }
                            for ((side, earlier), _) in sides.iter_mut().zip(earlier).zip(lanes) {
                                *side -= value * earlier;
                            }
                        }
                    }
                }
                lu.solve_group(
                    cells,
                    reach.as_ref(),
                    (&at_row, &at_unknown),
                    &mut spare,
                    &mut marks,
                );
            }
            for (fit, row) in anchors.iter_mut().enumerate() {
                let entries: Vec<(usize, f64)> = (line(border + fit)
                    .filter(|(place, _)| own(point).contains(place)))
                .map(|(place, value)| (at_unknown[place - point * size], value))
                .collect();
                for (group, cells) in walked.chunks(size).enumerate() {
                    let lanes = &order[group * LANES..((group + 1) * LANES).min(width)];
                    for &(at, value) in &entries {
                        // An anchor's cell is a positive zero or not zero, so a finite term over a zero read leaves it.
                        if value.is_finite() && cells[at].iter().all(|value| *value == 0.0) {
                            continue;
                        }
                        for (&column, walked) in lanes.iter().zip(&cells[at]) {
                            row[column] += value * walked;
                        }
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
        let add = |cells: &mut Vec<f64>, source: &[[f64; LANES]], place: usize, value: f64| {
            let side =
                |at: usize| source[slot_of[at] / LANES * size + at_unknown[place]][slot_of[at] % LANES];
            for (at, &seed) in read.iter().enumerate() {
                cells[seed] += value * side(at);
            }
            for fit in 0..fits {
                cells[size + fit] += value * side(read.len() + fit);
            }
            cells[corner] -= value * side(seeds);
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
            for (place, value) in line(border + fit) {
                if own(last).contains(&place) {
                    cells[place - last * size] += value;
                } else if place >= border {
                    cells[size + place - border] += value;
                }
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
            lu.solve(&mut side);
            update[point * size..(point + 1) * size].copy_from_slice(&side);
        }
        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, ROUNDING, inverse, reduced};
    use crate::elimination::{LANES, Marks};
    use crate::{Jacobian, UnknownId};

    fn reduced_by_pivot(mut matrix: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, usize> {
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
            let leading = &above[pivot][pivot..];
            // A row with nothing under the pivot keeps its cells; a zero factor would only rewrite a zero's sign.
            for row in below.iter_mut().filter(|row| row[pivot] != 0.0) {
                let factor = row[pivot] / leading[0];
                for (cell, lead) in row[pivot..].iter_mut().zip(leading) {
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

    #[test]
    fn a_panel_at_a_time_keeps_every_bit_of_a_pivot_at_a_time() {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1_u64 << 53) as f64
        };
        // The mirrored cases pivot off the diagonal, so rows swap inside a panel. The blocked cases keep the second
        // panel's columns zero under the first panel's rows and in the third panel's rows.
        let cases = [
            (1, 0),
            (2, 0),
            (15, 0),
            (16, 0),
            (17, 0),
            (40, 0),
            (70, 0),
            (17, 1),
            (40, 1),
            (70, 1),
            (40, 2),
            (48, 2),
        ];
        for (size, shape) in cases {
            let mut matrix = vec![vec![0.0; size + 3]; size];
            for row in matrix.iter_mut() {
                for cell in row.iter_mut() {
                    let pick = next();
                    *cell = if pick < 0.4 {
                        0.0
                    } else if pick < 0.45 {
                        -0.0
                    } else {
                        next() - 0.5
                    };
                }
            }
            for (at, row) in matrix.iter_mut().enumerate() {
                let column = if shape == 1 { size - 1 - at } else { at };
                row[column] += 2.0;
                if shape == 2 && !(16..32).contains(&at) {
                    row[16..32].fill(0.0);
                }
            }
            let one = reduced_by_pivot(matrix.clone()).expect("rank");
            let panel = reduced(matrix).expect("rank");
            for (a, b) in one.iter().flatten().zip(panel.iter().flatten()) {
                assert_eq!(a.to_bits(), b.to_bits(), "size {size}, shape {shape}");
            }
        }
    }

    fn next(state: &mut u64) -> f64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        (*state >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn chain_body() -> Result<crate::CompiledBody, crate::AdmissionError> {
        use crate::*;
        let mut map = AnatomyBuilder::new();
        let origin = || Measure::new(0.0).expect("a finite zero");
        map.site::<TissueSite>("synthetic body", Position::new(origin(), origin(), origin()));
        assemble(Schema::new(map.finish()?)?, |mut body| {
            let (mut ns, _) = body.nervous()?.parts();
            let names = ["a", "b", "c", "d", "e"];
            let rows: Vec<_> = (names.iter())
                .map(|name| ns.algebraic::<Dimensionless>(name, Admissible::finite()))
                .collect();
            let p = ns.parameter::<Dimensionless>(ParameterKey::new("synthetic p"), Admissible::finite());
            let class = RateClass::new(RateBand::Minutes, 1_000_000_000)?;
            for (at, row) in rows.iter().enumerate() {
                let other = &rows[(at + 1) % rows.len()];
                let residual = ns.kernel(|mut k| {
                    let own = k.algebraic(row);
                    let other = k.algebraic(other);
                    let coefficient = k.parameter(&p);
                    let scaled = k.scale(other, coefficient);
                    let value = k.add(own, scaled);
                    k.finish(value, ns.every_mode(), class)
                });
                ns.residual(row, residual);
            }
            body.add(ns.finish());
            body.compile()
        })
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Infinite {
        Seed,
        Read,
        Anchor,
    }

    /// `infinite` puts one infinite entry in the first row that holds one of its kind: a point 0 row's read of the
    /// last block, a point 1 row's read of point 0, or a fit's row's read of a block before the last.
    fn ring_bits(
        body: &crate::CompiledBody,
        (legs, fits): (u32, usize),
        drawn: f64,
        infinite: Option<Infinite>,
        state: &mut u64,
    ) -> Vec<u8> {
        use crate::*;
        use core::num::{NonZeroU32, NonZeroU64};
        let cycle = Cycle::new(
            NonZeroU64::new(u64::from(legs) * 1_000_000_000).expect("a period"),
            NonZeroU32::new(legs).expect("legs"),
        )
        .expect("an even cycle");
        let mut packet = ParameterPacket::periodic(cycle);
        (packet.synthetic(
            ParameterKey::new("synthetic p"),
            Binding::Body,
            Measure::<Dimensionless>::new(0.5).expect("finite"),
            "synthetic",
        ))
        .expect("a parameter");
        packet.start_rest(body, |_| 0.0, "synthetic").expect("a start");
        let system = (body.initialization(
            body.bind(packet).expect("bound"),
            BoundaryPort::new(vec![]).expect("no inputs"),
            InitializationRequest::at(SimTime::from_ns(0)),
        ))
        .expect("a system");
        let workspace = system.workspace();
        let laid = system.linearize(&workspace).expect("a ring");
        let view = laid.view();
        let (size, points) = (system.width(), workspace.leg_count());
        let border = points * size;
        let value = |state: &mut u64| match (next(state) * 10.0) as usize {
            0 => -0.0,
            1 => 0.0,
            2 => 1e-300 * (next(state) - 0.5),
            _ => next(state) - 0.5,
        };
        let (mut offsets, mut columns, mut values) = (vec![0], Vec::new(), Vec::new());
        let (mut right, mut placed) = (Vec::new(), false);
        for slot in 0..border + fits {
            let mut line = alloc::collections::BTreeMap::new();
            if slot < border {
                let point = slot / size;
                for entry in view.row_offsets()[slot]..view.row_offsets()[slot + 1] {
                    let column = view.columns()[entry].index();
                    if (point * size..(point + 1) * size).contains(&column) {
                        let (pick, sign) = (next(state), next(state));
                        let own = column - point * size == slot % size;
                        let magnitude = if pick < if own { 0.3 } else { 0.02 } {
                            0.0
                        } else {
                            1.0 + next(state)
                        };
                        line.insert(column, if sign < 0.5 { -magnitude } else { magnitude });
                    }
                }
                let earlier = (point + points - 1) % points;
                for column in earlier * size..(earlier + 1) * size {
                    if next(state) < 0.15 {
                        line.insert(column, value(state));
                    }
                }
            } else {
                line.insert(slot, 4.0 + next(state));
                for column in 0..border {
                    if next(state) < 0.1 {
                        line.insert(column, value(state));
                    }
                }
            }
            for fit in border..border + fits {
                if fit != slot && next(state) < 0.15 {
                    line.insert(fit, value(state));
                }
            }
            let kind = match slot {
                _ if slot >= border => Some((Infinite::Anchor, 0..(points - 1) * size)),
                _ if slot < size => Some((Infinite::Seed, (points - 1) * size..border)),
                _ if slot < 2 * size => Some((Infinite::Read, 0..size)),
                _ => None,
            };
            if let Some((kind, reads)) = kind.filter(|(kind, _)| !placed && infinite == Some(*kind))
                && let Some((_, cell)) = line.range_mut(reads).next()
            {
                *cell = if kind == Infinite::Read {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
                placed = true;
            }
            for (column, cell) in line {
                columns.push(UnknownId::at(column));
                values.push(cell);
            }
            offsets.push(columns.len());
            right.push(if next(state) < drawn { value(state) } else { -0.0 });
        }
        let ring = Jacobian::of_parts(offsets, columns, values);
        match system.ring(&workspace, &ring, &right) {
            Ok(update) => update
                .iter()
                .flat_map(|value| {
                    if value.is_nan() { f64::NAN } else { *value }
                        .to_bits()
                        .to_be_bytes()
                })
                .collect(),
            Err(row) => (row as u64).to_be_bytes().to_vec(),
        }
    }

    #[test]
    fn a_ring_keeps_the_bits_it_had_on_zeros_of_both_signs() {
        let body = chain_body().expect("a chain");
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut bits: Vec<u8> = Vec::new();
        for (legs, fits) in [
            (3_u32, 0),
            (3, 9),
            (4, 2),
            (5, 12),
            (6, 7),
            (4, 0),
            (5, 3),
            (6, 10),
        ] {
            bits.extend(ring_bits(&body, (legs, fits), 0.2, None, &mut state));
        }
        let sparse = [
            (7_u64, 4_u32, 0),
            (11, 3, 0),
            (11, 3, 4),
            (42, 3, 4),
            (49, 3, 4),
            (4, 5, 3),
            (10, 3, 4),
        ];
        let cases = (sparse
            .into_iter()
            .map(|(seed, legs, fits)| (seed, (legs, fits), 0.02, None)))
        .chain([
            (15, (4, 2), 0.02, Some(Infinite::Seed)),
            (24, (3, 4), 0.02, Some(Infinite::Seed)),
            (15, (4, 2), 0.02, Some(Infinite::Read)),
            (24, (3, 4), 0.02, Some(Infinite::Read)),
            (3, (3, 4), 0.02, Some(Infinite::Anchor)),
            (5, (4, 2), 0.2, Some(Infinite::Anchor)),
        ]);
        for (seed, shape, drawn, infinite) in cases {
            let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            bits.extend(ring_bits(&body, shape, drawn, infinite, &mut state));
        }
        let digest = crate::Digest::of_canonical(&bits);
        let hex: alloc::string::String = (digest.as_bytes().iter())
            .map(|byte| alloc::format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hex,
            "cf8a842fd3e17358f1d2980482d9999c17b4f452b6d2dd8eb7f2c537a3ecf10e"
        );
    }

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
        let (at_row, at_unknown) = ([2, 0, 3, 1], [1, 3, 0, 2]);
        let mut group = vec![[0.0; LANES]; size];
        for (lane, side) in sides.iter().enumerate() {
            for (row, &value) in side.iter().enumerate() {
                group[at_row[row]][lane] = value;
            }
        }
        let (mut spare, mut marks) = (vec![[0.0; LANES]; size], Marks::new(size));
        whole.solve_group(&mut group, None, (&at_row, &at_unknown), &mut spare, &mut marks);
        for (lane, mut alone) in sides.into_iter().enumerate() {
            whole.solve(&mut alone);
            for (unknown, value) in alone.iter().enumerate() {
                assert_eq!(value.to_bits(), group[at_unknown[unknown]][lane].to_bits());
            }
        }
    }
}
