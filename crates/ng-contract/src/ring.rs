use crate::admission::{AdmissionWorkspace, Correction};
use crate::elimination::{Factors, LANES, Marks, RING_LANES, Reach};
use crate::{InitializationSystem, Jacobian};

const ROUNDING: f64 = 8.0 * f64::EPSILON;
const PANEL: usize = 16;
const TILE_W: usize = 64;
const ROW_BLOCK: usize = 8;
const NEGATIVE_ZERO: u64 = 0x8000_0000_0000_0000;

#[inline(always)]
fn sub_fused<const K: usize>(cells: &mut [f64], leads: [&[f64]; K], factors: [f64; K]) {
    let n = cells.len();
    for lead in leads {
        assert!(lead.len() >= n);
    }
    let mut base = 0;
    while base + 8 <= n {
        let mut acc = [0.0_f64; 8];
        acc.copy_from_slice(&cells[base..base + 8]);
        for k in 0..K {
            let lead = &leads[k][base..base + 8];
            let f = factors[k];
            for i in 0..8 {
                acc[i] -= f * lead[i];
            }
        }
        cells[base..base + 8].copy_from_slice(&acc);
        base += 8;
    }
    while base < n {
        let mut acc = cells[base];
        for k in 0..K {
            acc -= factors[k] * leads[k][base];
        }
        cells[base] = acc;
        base += 1;
    }
}

#[inline(always)]
fn apply_fused(
    cells: &mut [f64],
    above: &[f64],
    start: usize,
    stride: usize,
    lead_col: usize,
    width: usize,
    factors: &[(u8, f64)],
) {
    let lead_at = |k: u8| -> &[f64] {
        let base = (start + k as usize) * stride + lead_col;
        &above[base..base + width]
    };
    let mut pos = 0;
    while pos + 8 <= factors.len() {
        sub_fused(
            cells,
            [
                lead_at(factors[pos].0),
                lead_at(factors[pos + 1].0),
                lead_at(factors[pos + 2].0),
                lead_at(factors[pos + 3].0),
                lead_at(factors[pos + 4].0),
                lead_at(factors[pos + 5].0),
                lead_at(factors[pos + 6].0),
                lead_at(factors[pos + 7].0),
            ],
            [
                factors[pos].1,
                factors[pos + 1].1,
                factors[pos + 2].1,
                factors[pos + 3].1,
                factors[pos + 4].1,
                factors[pos + 5].1,
                factors[pos + 6].1,
                factors[pos + 7].1,
            ],
        );
        pos += 8;
    }
    if pos + 4 <= factors.len() {
        sub_fused(
            cells,
            [
                lead_at(factors[pos].0),
                lead_at(factors[pos + 1].0),
                lead_at(factors[pos + 2].0),
                lead_at(factors[pos + 3].0),
            ],
            [
                factors[pos].1,
                factors[pos + 1].1,
                factors[pos + 2].1,
                factors[pos + 3].1,
            ],
        );
        pos += 4;
    }
    match factors.len() - pos {
        3 => sub_fused(
            cells,
            [
                lead_at(factors[pos].0),
                lead_at(factors[pos + 1].0),
                lead_at(factors[pos + 2].0),
            ],
            [factors[pos].1, factors[pos + 1].1, factors[pos + 2].1],
        ),
        2 => sub_fused(
            cells,
            [lead_at(factors[pos].0), lead_at(factors[pos + 1].0)],
            [factors[pos].1, factors[pos + 1].1],
        ),
        1 => sub_fused(cells, [lead_at(factors[pos].0)], [factors[pos].1]),
        _ => {}
    }
}

/// `matrix` holds each row with its right side last. An unknown with no pivot left is named: it has no rank. A pivot
/// is read beside its row and its column as given, and the smaller of the two: after elimination a row's own largest
/// entry can be rounding, and a row and a column can each run at a scale of their own.
pub fn eliminate(matrix: Vec<Vec<f64>>) -> Result<Vec<f64>, usize> {
    let size = matrix.len();
    if size == 0 {
        return Ok(Vec::new());
    }
    let stride = size + 1;
    let mut flat = Vec::with_capacity(size * stride);
    for row in matrix {
        flat.extend_from_slice(&row[..stride]);
    }
    eliminate_flat(&mut flat, size)
}

pub(crate) fn eliminate_flat(flat: &mut [f64], size: usize) -> Result<Vec<f64>, usize> {
    reduced_flat(flat, size, 1)
}

/// Each row holds its entries and then any number of right sides, and each row of the answer one value per side.
#[cfg(test)]
fn reduced(matrix: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, usize> {
    let size = matrix.len();
    let sides = matrix.first().map_or(0, |row| row.len() - size);
    if size == 0 {
        return Ok(Vec::new());
    }
    let stride = size + sides;
    let mut flat = Vec::with_capacity(size * stride);
    for row in matrix {
        flat.extend_from_slice(&row[..stride]);
    }
    let flat_answers = reduced_flat(&mut flat, size, sides)?;
    Ok(flat_answers.chunks(sides).map(|row| row.to_vec()).collect())
}

/// Flat row-major Gaussian elimination with partial pivoting and L1-tiled panel updates.
/// Every cell takes the exact same terms in the exact same pivot order as one pivot at a time.
fn reduced_flat(flat: &mut [f64], size: usize, sides: usize) -> Result<Vec<f64>, usize> {
    if size == 0 {
        return Ok(Vec::new());
    }
    let stride = size + sides;
    let mut across = vec![0.0_f64; size];
    let mut down = vec![0.0_f64; size];
    let mut has_neg_zero = false;
    for row in 0..size {
        let cells = &flat[row * stride..row * stride + stride];
        for &cell in cells {
            if cell.to_bits() == NEGATIVE_ZERO {
                has_neg_zero = true;
            }
        }
        let mut row_max = 0.0;
        for (column, &cell) in cells[..size].iter().enumerate() {
            let mag = cell.abs();
            if mag > row_max {
                row_max = mag;
            }
            if mag > down[column] {
                down[column] = mag;
            }
        }
        across[row] = row_max;
    }
    struct ActiveRow {
        row: usize,
        mask: u16,
        count: usize,
        factors: [(u8, f64); PANEL],
    }
    let mut pending: Vec<[Option<f64>; PANEL]> = vec![[None; PANEL]; size];
    let mut active_rows: Vec<ActiveRow> = Vec::with_capacity(size);
    let mut tile_masks: Vec<u16> = vec![0; stride.div_ceil(TILE_W)];
    for start in (0..size).step_by(PANEL) {
        let end = (start + PANEL).min(size);
        for pivot in start..end {
            let mut best = pivot;
            let mut best_mag = flat[pivot * stride + pivot].abs();
            for row in pivot + 1..size {
                let mag = flat[row * stride + pivot].abs();
                if mag > best_mag {
                    best = row;
                    best_mag = mag;
                }
            }
            let reach = if across[best] < down[pivot] {
                across[best]
            } else {
                down[pivot]
            };
            if best_mag <= ROUNDING * reach {
                return Err(pivot);
            }
            if best != pivot {
                let (top, bottom) = flat.split_at_mut(best * stride);
                top[pivot * stride..(pivot + 1) * stride].swap_with_slice(&mut bottom[..stride]);
            }
            across.swap(pivot, best);
            pending.swap(pivot, best);
            let (above, below) = flat.split_at_mut((pivot + 1) * stride);
            let leading = &above[pivot * stride + pivot..pivot * stride + end];
            let lead_diag = leading[0];
            let panel_width = end - pivot;
            for (offset, waiting) in pending[pivot + 1..].iter_mut().enumerate() {
                let row = &mut below[offset * stride + pivot..offset * stride + end];
                // A row with nothing under the pivot keeps its cells; a zero factor would only rewrite a zero's sign.
                if row[0] == 0.0 {
                    continue;
                }
                let factor = row[0] / lead_diag;
                waiting[pivot - start] = Some(factor);
                for at in 0..panel_width {
                    row[at] -= factor * leading[at];
                }
            }
        }
        active_rows.clear();
        let mut all_factors_finite = true;
        for (row, waiting) in pending.iter().enumerate().skip(start + 1) {
            let mut list = [(0_u8, 0.0_f64); PANEL];
            let mut count = 0_usize;
            let mut mask = 0_u16;
            for pivot in start..end.min(row) {
                if let Some(factor) = waiting[pivot - start] {
                    let k = (pivot - start) as u8;
                    list[count] = (k, factor);
                    mask |= 1_u16 << k;
                    count += 1;
                    all_factors_finite &= factor.is_finite();
                }
            }
            if count > 0 {
                active_rows.push(ActiveRow {
                    row,
                    mask,
                    count,
                    factors: list,
                });
            }
        }
        if !active_rows.is_empty() && end < stride {
            let split = active_rows.partition_point(|active| active.row < end);
            for active in &active_rows[..split] {
                let (above, below) = flat.split_at_mut(active.row * stride);
                let cells = &mut below[end..stride];
                for &(k, factor) in &active.factors[..active.count] {
                    let lead =
                        &above[(start + k as usize) * stride + end..(start + k as usize) * stride + stride];
                    for (cell, &l) in cells.iter_mut().zip(lead) {
                        *cell -= factor * l;
                    }
                }
            }
            let trailing = &active_rows[split..];
            if !trailing.is_empty() {
                let skip_zeros = !has_neg_zero && all_factors_finite;
                let panel_rows = end - start;
                let rem = stride - end;
                let num_tiles = rem.div_ceil(TILE_W);
                let (above, below) = flat.split_at_mut(end * stride);
                if skip_zeros {
                    tile_masks[..num_tiles].fill(0);
                    for k in 0..panel_rows {
                        let bit = 1_u16 << k;
                        let lead = &above[(start + k) * stride + end..(start + k) * stride + stride];
                        for (t, tmask) in tile_masks[..num_tiles].iter_mut().enumerate() {
                            let col = t * TILE_W;
                            let width = (rem - col).min(TILE_W);
                            if lead[col..col + width].iter().any(|v| v.to_bits() != 0) {
                                *tmask |= bit;
                            }
                        }
                    }
                } else {
                    let all_bits = if panel_rows == PANEL {
                        u16::MAX
                    } else {
                        (1_u16 << panel_rows) - 1
                    };
                    tile_masks[..num_tiles].fill(all_bits);
                }
                let mut filtered = [(0_u8, 0.0_f64); PANEL];
                for row_group in trailing.chunks(ROW_BLOCK) {
                    let group_mask = row_group.iter().fold(0_u16, |acc, active| acc | active.mask);
                    for (t, &tmask) in tile_masks[..num_tiles].iter().enumerate() {
                        if (group_mask & tmask) == 0 {
                            continue;
                        }
                        let col = t * TILE_W;
                        let width = (rem - col).min(TILE_W);
                        let lead_col = end + col;
                        for active in row_group {
                            let active_mask = active.mask & tmask;
                            if active_mask == 0 {
                                continue;
                            }
                            let cell_start = (active.row - end) * stride + lead_col;
                            let cells = &mut below[cell_start..cell_start + width];
                            if active_mask == active.mask {
                                apply_fused(
                                    cells,
                                    above,
                                    start,
                                    stride,
                                    lead_col,
                                    width,
                                    &active.factors[..active.count],
                                );
                            } else {
                                let mut n = 0;
                                for &(k, factor) in &active.factors[..active.count] {
                                    if (tmask & (1_u16 << k)) != 0 {
                                        filtered[n] = (k, factor);
                                        n += 1;
                                    }
                                }
                                apply_fused(cells, above, start, stride, lead_col, width, &filtered[..n]);
                            }
                        }
                    }
                }
            }
        }
        pending.fill([None; PANEL]);
    }
    let mut answers = vec![0.0_f64; size * sides];
    for row in (0..size).rev() {
        let row_slice = &flat[row * stride..row * stride + stride];
        let diag = row_slice[row];
        if sides == 1 {
            let known: f64 = row_slice[row + 1..size]
                .iter()
                .zip(&answers[row + 1..size])
                .map(|(&a, &b)| a * b)
                .sum();
            answers[row] = (row_slice[size] - known) / diag;
        } else {
            for side in 0..sides {
                let known: f64 = (row + 1..size)
                    .map(|column| row_slice[column] * answers[column * sides + side])
                    .sum();
                answers[row * sides + side] = (row_slice[size + side] - known) / diag;
            }
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
        group: &mut [[f64; RING_LANES]],
        active_lanes: usize,
        reach: Option<&Reach>,
        (at_row, at_unknown): (&[usize], &[usize]),
        spare: &mut [[f64; RING_LANES]],
        marks: &mut Marks,
    ) -> bool {
        match self {
            Block::Fixed(factors) => factors.solve_pivoted_wide(group, reach, spare, marks),
            Block::Whole(_) => {
                let mut sides: Vec<Vec<f64>> = (0..active_lanes)
                    .map(|lane| at_row.iter().map(|&at| group[at][lane]).collect())
                    .collect();
                for side in &mut sides {
                    self.solve(side);
                }
                for (unknown, &at) in at_unknown.iter().enumerate() {
                    for (cell, side) in group[at][..active_lanes].iter_mut().zip(&sides) {
                        *cell = side[unknown];
                    }
                }
                false
            }
        }
    }
    fn solve_group_seeded_no_neg_zero(
        &self,
        (group, active_lanes): (&mut [[f64; RING_LANES]], usize),
        reach: Option<&Reach>,
        places: (&[usize], &[usize]),
        marks: &mut Marks,
        touched: &[u32],
        rebuild: impl FnOnce(&mut [[f64; RING_LANES]]),
    ) -> bool {
        match (self, reach) {
            (Block::Fixed(factors), Some(reach)) => {
                if factors.solve_pivoted_seeded_no_neg_zero(group, reach, marks, touched) {
                    return true;
                }
                group.fill([0.0; RING_LANES]);
                rebuild(group);
                factors.solve_pivoted_wide_fallback(group, Some(reach), marks);
                false
            }
            (Block::Fixed(factors), None) => {
                factors.solve_pivoted_wide_fallback(group, None, marks);
                false
            }
            (Block::Whole(_), _) => {
                let mut unused = Vec::new();
                self.solve_group(group, active_lanes, reach, places, &mut unused, marks)
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
    let stride = 2 * size;
    let mut flat = vec![0.0; size * stride];
    for row in 0..size {
        let cells = &mut flat[row * stride..row * stride + stride];
        for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
            cells[view.columns()[entry].index()] += view.values()[entry];
        }
        cells[size + row] = 1.0;
    }
    let flat_answers = reduced_flat(&mut flat, size, size)?;
    Ok(flat_answers.chunks(size).map(|row| row.to_vec()).collect())
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
            let n = right.len();
            let stride = n + 1;
            let mut matrix = vec![0.0; n * stride];
            for row in 0..n {
                let cells = &mut matrix[row * stride..row * stride + stride];
                for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
                    cells[view.columns()[entry].index()] += view.values()[entry];
                }
                cells[n] = right[row];
            }
            return eliminate_flat(&mut matrix, n);
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
        let group_size = if width > RING_LANES { RING_LANES } else { LANES };
        let groups = width.div_ceil(group_size);
        let base_places = plan.places();
        let (mut order, mut slot_of): (Vec<usize>, Vec<usize>) = ((0..width).collect(), (0..width).collect());
        let mut anchors = vec![vec![0.0; width]; fits];
        let mut spare = vec![[0.0; RING_LANES]; size];
        let mut marks = Marks::new(size);
        let target = |place: usize| {
            if place >= border {
                Ok(read.len() + place - border)
            } else {
                read.binary_search(&(place - last * size))
            }
        };

        let mut needed_from_prev: Vec<Vec<usize>> = (0..points).map(|_| Vec::new()).collect();
        let mut needed_from_first: Vec<usize> = Vec::new();
        let mut point_needs_dense_all = vec![false; points];
        for p in 1..points {
            let prev_range = own(p - 1);
            let mut needed = Vec::new();
            let mut nonfinite = false;
            for row in 0..size {
                for (place, value) in line(p * size + row) {
                    if !own(p).contains(&place) {
                        if !value.is_finite() {
                            nonfinite = true;
                        }
                        if p == last && place < size {
                            needed_from_first.push(place);
                        } else if prev_range.contains(&place) {
                            needed.push(place - (p - 1) * size);
                        }
                    }
                }
            }
            needed.sort_unstable();
            needed.dedup();
            needed_from_prev[p] = needed;
            point_needs_dense_all[p] = nonfinite;
        }
        needed_from_first.sort_unstable();
        needed_from_first.dedup();
        let mut first_pos = vec![0_usize; size];
        for (idx, &local) in needed_from_first.iter().enumerate() {
            first_pos[local] = idx;
        }
        let mut last_pos = vec![0_usize; size];
        for (idx, &local) in needed_from_prev[last].iter().enumerate() {
            last_pos[local] = idx;
        }

        let mut first_by_unknown = vec![0.0_f64; needed_from_first.len() * width];
        let mut last_by_unknown = vec![0.0_f64; needed_from_prev[last].len() * width];
        let mut cells = vec![[0.0; RING_LANES]; size];
        let mut foreign: Vec<Vec<(usize, f64)>> = vec![Vec::new(); size];
        let mut reads: Vec<Vec<(usize, f64)>> = vec![Vec::new(); size];
        let mut terms: Vec<Vec<(usize, usize, usize, f64)>> = vec![Vec::new(); groups];
        let mut anchor_entries: Vec<Vec<(usize, f64)>> = vec![Vec::new(); fits];
        let mut prev_pos = vec![0_usize; size];
        let mut readers_offsets: Vec<usize> = Vec::new();
        let mut readers_data: Vec<(u32, f64)> = Vec::new();
        let mut prev_scratch: Vec<[f64; RING_LANES]> = Vec::new();
        let mut touched: Vec<u32> = Vec::with_capacity(size);

        let mut cur_sparse_offsets: Vec<usize> = vec![0; groups + 1];
        let mut cur_sparse_idx: Vec<u32> = Vec::new();
        let mut cur_sparse_val: Vec<[f64; RING_LANES]> = Vec::new();
        let mut cur_dense: Vec<[f64; RING_LANES]> = Vec::new();
        let mut next_sparse_offsets: Vec<usize> = vec![0; groups + 1];
        let mut next_sparse_idx: Vec<u32> = Vec::new();
        let mut next_sparse_val: Vec<[f64; RING_LANES]> = Vec::new();
        let mut next_dense: Vec<[f64; RING_LANES]> = Vec::new();

        let mut block_cells = vec![0.0_f64; size];
        let mut block_values = Vec::new();
        for point in 0..last {
            system.block_values(ring, point, &mut block_cells, &mut block_values);
            let lu = match plan.factor(&block_values) {
                Ok(factors) => Block::Fixed(factors),
                Err(_) => {
                    let block = system.block(ring, point);
                    Block::Whole(inverse(&block).map_err(|at| point * size + at)?)
                }
            };
            let (at_row, at_unknown) = match &lu {
                Block::Fixed(factors) => factors.places(),
                Block::Whole(_) => base_places.clone(),
            };
            let reach = lu.reach();
            let own_range = own(point);
            for (row, list) in foreign.iter_mut().enumerate() {
                list.clear();
                for (place, value) in line(point * size + row) {
                    if !own_range.contains(&place) {
                        list.push((place, value));
                    }
                }
            }
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
            let prev_needed = &needed_from_prev[point];
            let prev_count = prev_needed.len();
            for (idx, &local) in prev_needed.iter().enumerate() {
                prev_pos[local] = idx;
            }
            for list in &mut terms {
                list.clear();
            }
            let mut max_reads_per_row = 0_usize;
            let mut any_nonfinite_read = false;
            for (row, entries) in foreign.iter().enumerate() {
                let row_reads = &mut reads[row];
                row_reads.clear();
                for &(place, value) in entries {
                    if !seeded(place) {
                        let compact = prev_pos[place - (point - 1) * size];
                        row_reads.push((compact, value));
                        if !value.is_finite() {
                            any_nonfinite_read = true;
                        }
                    } else if let Ok(column) = target(place) {
                        let slot = slot_of[column];
                        terms[slot / group_size].push((row, row_reads.len(), slot % group_size, value));
                    }
                }
                if row_reads.len() > max_reads_per_row {
                    max_reads_per_row = row_reads.len();
                }
            }
            let single_read_fast = point > 0 && max_reads_per_row <= 1 && !any_nonfinite_read;
            if single_read_fast {
                readers_offsets.clear();
                readers_offsets.resize(prev_count + 1, 0);
                for row_reads in &reads {
                    if let Some(&(compact, _)) = row_reads.first() {
                        readers_offsets[compact + 1] += 1;
                    }
                }
                for c in 0..prev_count {
                    readers_offsets[c + 1] += readers_offsets[c];
                }
                let total_readers = readers_offsets[prev_count];
                readers_data.clear();
                readers_data.resize(total_readers, (0, 0.0));
                let mut fill = readers_offsets.clone();
                for (row, row_reads) in reads.iter().enumerate() {
                    if let Some(&(compact, value)) = row_reads.first() {
                        let dst = fill[compact];
                        readers_data[dst] = (at_row[row] as u32, value);
                        fill[compact] = dst + 1;
                    }
                }
            }
            for (fit, entries) in anchor_entries.iter_mut().enumerate() {
                entries.clear();
                for (place, value) in line(border + fit) {
                    if own_range.contains(&place) {
                        entries.push((at_unknown[place - point * size], value));
                    }
                }
            }

            let cur_has_dense_all = point_needs_dense_all[point];
            let next_needed = &needed_from_prev[point + 1];
            let next_count = next_needed.len();
            let next_has_dense_all = point + 1 < last && point_needs_dense_all[point + 1];
            let next_unknown: Vec<usize> = next_needed.iter().map(|&local| at_unknown[local]).collect();
            let first_unknown_at: Vec<usize> = if point == 0 {
                needed_from_first.iter().map(|&local| at_unknown[local]).collect()
            } else {
                Vec::new()
            };
            if point + 1 < last {
                next_sparse_offsets.clear();
                next_sparse_offsets.push(0);
                next_sparse_idx.clear();
                next_sparse_val.clear();
                let dense_len = if next_has_dense_all {
                    groups * next_count
                } else {
                    next_count
                };
                next_dense.resize(dense_len, [0.0; RING_LANES]);
            }
            if prev_scratch.len() < prev_count {
                prev_scratch.resize(prev_count, [0.0; RING_LANES]);
            }

            let right_group = slot_of[seeds] / group_size;
            let right_lane = slot_of[seeds] % group_size;
            for group in 0..groups {
                let lanes = &order[group * group_size..((group + 1) * group_size).min(width)];
                let holds_right = group == right_group;
                let used_reach = if holds_right || general {
                    let use_dense = holds_right || cur_has_dense_all;
                    let dense_base = if cur_has_dense_all { group * prev_count } else { 0 };
                    if !use_dense && point > 0 {
                        let span = cur_sparse_offsets[group]..cur_sparse_offsets[group + 1];
                        for (&idx, &val) in cur_sparse_idx[span.clone()].iter().zip(&cur_sparse_val[span]) {
                            prev_scratch[idx as usize] = val;
                        }
                    }
                    for (row, entries) in foreign.iter().enumerate() {
                        let sides = &mut cells[at_row[row]];
                        if holds_right {
                            sides[right_lane] = right[point * size + row];
                        }
                        for &(place, value) in entries {
                            if seeded(place) {
                                let column = target(place);
                                if value.is_finite() {
                                    // A side for a column `read` names or for a fit is a positive zero or not zero,
                                    // so a finite term leaves every side but its own and the right side's as it is.
                                    for at in column.ok().into_iter().chain([seeds]) {
                                        if slot_of[at] / group_size == group {
                                            sides[slot_of[at] % group_size] -=
                                                value * f64::from(column == Ok(at));
                                        }
                                    }
                                } else {
                                    for (side, &at) in sides.iter_mut().zip(lanes) {
                                        *side -= value * f64::from(column == Ok(at));
                                    }
                                }
                            } else {
                                let compact = prev_pos[place - (point - 1) * size];
                                let earlier = if use_dense {
                                    &cur_dense[dense_base + compact]
                                } else {
                                    &prev_scratch[compact]
                                };
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
                    if !use_dense && point > 0 {
                        let span = cur_sparse_offsets[group]..cur_sparse_offsets[group + 1];
                        for &idx in &cur_sparse_idx[span] {
                            prev_scratch[idx as usize] = [0.0; RING_LANES];
                        }
                    }
                    lu.solve_group(
                        &mut cells,
                        lanes.len(),
                        reach.as_ref(),
                        (&at_row, &at_unknown),
                        &mut spare,
                        &mut marks,
                    )
                } else if point == 0 {
                    touched.clear();
                    for &(row, _, lane, value) in &terms[group] {
                        let r = at_row[row];
                        cells[r][lane] -= value;
                        touched.push(r as u32);
                    }
                    lu.solve_group_seeded_no_neg_zero(
                        (&mut cells, lanes.len()),
                        reach.as_ref(),
                        (&at_row, &at_unknown),
                        &mut marks,
                        &touched,
                        |rebuilt| {
                            for &(row, _, lane, value) in &terms[group] {
                                rebuilt[at_row[row]][lane] -= value;
                            }
                        },
                    )
                } else if single_read_fast && terms[group].is_empty() {
                    touched.clear();
                    let span = cur_sparse_offsets[group]..cur_sparse_offsets[group + 1];
                    let lanes_len = lanes.len();
                    if lanes_len == RING_LANES {
                        for (&compact, earlier) in cur_sparse_idx[span.clone()]
                            .iter()
                            .zip(&cur_sparse_val[span.clone()])
                        {
                            let c = compact as usize;
                            for &(row_at, value) in &readers_data[readers_offsets[c]..readers_offsets[c + 1]]
                            {
                                let r = row_at as usize;
                                let mut sides = cells[r];
                                for (side, &val) in sides.iter_mut().zip(earlier) {
                                    *side -= value * val;
                                }
                                cells[r] = sides;
                                touched.push(row_at);
                            }
                        }
                    } else {
                        for (&compact, earlier) in cur_sparse_idx[span.clone()]
                            .iter()
                            .zip(&cur_sparse_val[span.clone()])
                        {
                            let c = compact as usize;
                            for &(row_at, value) in &readers_data[readers_offsets[c]..readers_offsets[c + 1]]
                            {
                                let sides = &mut cells[row_at as usize];
                                for lane in 0..lanes_len {
                                    sides[lane] -= value * earlier[lane];
                                }
                                touched.push(row_at);
                            }
                        }
                    }
                    lu.solve_group_seeded_no_neg_zero(
                        (&mut cells, lanes_len),
                        reach.as_ref(),
                        (&at_row, &at_unknown),
                        &mut marks,
                        &touched,
                        |rebuilt| {
                            for (&compact, earlier) in cur_sparse_idx[span.clone()]
                                .iter()
                                .zip(&cur_sparse_val[span.clone()])
                            {
                                let c = compact as usize;
                                for &(row_at, value) in
                                    &readers_data[readers_offsets[c]..readers_offsets[c + 1]]
                                {
                                    let sides = &mut rebuilt[row_at as usize];
                                    for lane in 0..lanes_len {
                                        sides[lane] -= value * earlier[lane];
                                    }
                                }
                            }
                        },
                    )
                } else {
                    if !cur_has_dense_all {
                        let span = cur_sparse_offsets[group]..cur_sparse_offsets[group + 1];
                        for (&idx, &val) in cur_sparse_idx[span.clone()].iter().zip(&cur_sparse_val[span]) {
                            prev_scratch[idx as usize] = val;
                        }
                    }
                    let dense_base = group * prev_count;
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
                            let earlier = if cur_has_dense_all {
                                &cur_dense[dense_base + unknown]
                            } else {
                                &prev_scratch[unknown]
                            };
                            if value.is_finite() && earlier.iter().all(|value| *value == 0.0) {
                                continue;
                            }
                            for ((side, earlier), _) in sides.iter_mut().zip(earlier).zip(lanes) {
                                *side -= value * earlier;
                            }
                        }
                    }
                    if !cur_has_dense_all {
                        let span = cur_sparse_offsets[group]..cur_sparse_offsets[group + 1];
                        for &idx in &cur_sparse_idx[span] {
                            prev_scratch[idx as usize] = [0.0; RING_LANES];
                        }
                    }
                    lu.solve_group(
                        &mut cells,
                        lanes.len(),
                        reach.as_ref(),
                        (&at_row, &at_unknown),
                        &mut spare,
                        &mut marks,
                    )
                };

                for (row, entries) in anchors.iter_mut().zip(&anchor_entries) {
                    for &(at, value) in entries {
                        // An anchor's cell is a positive zero or not zero, so a finite term over a zero read leaves it.
                        if value.is_finite() && cells[at].iter().all(|value| *value == 0.0) {
                            continue;
                        }
                        for (&column, walked) in lanes.iter().zip(&cells[at]) {
                            row[column] += value * walked;
                        }
                    }
                }
                let live = marks.live_slice();
                if point == 0 {
                    for (k, &at) in first_unknown_at.iter().enumerate() {
                        if used_reach && !live[at] {
                            continue;
                        }
                        let dst = &mut first_by_unknown[k * width..(k + 1) * width];
                        for (&column, &val) in lanes.iter().zip(&cells[at]) {
                            dst[column] = val;
                        }
                    }
                }
                if point + 1 == last {
                    for (k, &at) in next_unknown.iter().enumerate() {
                        if used_reach && !live[at] {
                            continue;
                        }
                        let dst = &mut last_by_unknown[k * width..(k + 1) * width];
                        for (&column, &val) in lanes.iter().zip(&cells[at]) {
                            dst[column] = val;
                        }
                    }
                } else {
                    if used_reach {
                        for (k, &at) in next_unknown.iter().enumerate() {
                            if !live[at] {
                                continue;
                            }
                            let value = cells[at];
                            if value.iter().any(|&v| v != 0.0) {
                                next_sparse_idx.push(k as u32);
                                next_sparse_val.push(value);
                            }
                        }
                    } else {
                        for (k, &at) in next_unknown.iter().enumerate() {
                            let value = cells[at];
                            if value.iter().any(|&v| v != 0.0) {
                                next_sparse_idx.push(k as u32);
                                next_sparse_val.push(value);
                            }
                        }
                    }
                    next_sparse_offsets.push(next_sparse_idx.len());
                    if next_has_dense_all {
                        let base = group * next_count;
                        for (k, &at) in next_unknown.iter().enumerate() {
                            next_dense[base + k] = cells[at];
                        }
                    } else if holds_right {
                        for (k, &at) in next_unknown.iter().enumerate() {
                            next_dense[k] = cells[at];
                        }
                    }
                }
                if used_reach {
                    for (cell, &is_live) in cells.iter_mut().zip(live) {
                        if is_live {
                            *cell = [0.0; RING_LANES];
                        }
                    }
                } else {
                    cells.fill([0.0; RING_LANES]);
                }
            }
            if point + 1 < last {
                core::mem::swap(&mut cur_sparse_offsets, &mut next_sparse_offsets);
                core::mem::swap(&mut cur_sparse_idx, &mut next_sparse_idx);
                core::mem::swap(&mut cur_sparse_val, &mut next_sparse_val);
                core::mem::swap(&mut cur_dense, &mut next_dense);
            }
            factors.push(lu);
        }
        let corner = size + fits;
        let stride = corner + 1;
        let mut matrix = vec![0.0; corner * stride];
        let add = |cells: &mut [f64], source: &[f64], compact: usize, value: f64| {
            let row = &source[compact * width..(compact + 1) * width];
            for (&seed, &side) in read.iter().zip(&row[..read.len()]) {
                cells[seed] += value * side;
            }
            for (cell, &side) in cells[size..size + fits].iter_mut().zip(&row[read.len()..seeds]) {
                *cell += value * side;
            }
            cells[corner] -= value * row[seeds];
        };
        for row in 0..size {
            let cells = &mut matrix[row * stride..row * stride + stride];
            let slot = last * size + row;
            cells[corner] = right[slot];
            for (place, value) in line(slot) {
                if own(last).contains(&place) {
                    cells[place - last * size] += value;
                } else if place >= border {
                    cells[size + place - border] += value;
                } else if place < size {
                    add(cells, &first_by_unknown, first_pos[place], value);
                } else {
                    add(
                        cells,
                        &last_by_unknown,
                        last_pos[place - (last - 1) * size],
                        value,
                    );
                }
            }
        }
        for (fit, row) in anchors.iter().enumerate() {
            let cells = &mut matrix[(size + fit) * stride..(size + fit) * stride + stride];
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
        let answer = reduced_flat(&mut matrix, corner, 1).map_err(|at| {
            if at < size {
                last * size + at
            } else {
                border + at - size
            }
        })?;
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
    use crate::elimination::{Marks, RING_LANES};
    use crate::{Jacobian, UnknownId};

    fn reduced_by_pivot(mut matrix: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, usize> {
        let size = matrix.len();
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
                return Err(pivot);
            }
            matrix.swap(pivot, best);
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

    #[test]
    fn a_positive_zero_panel_keeps_every_bit_across_partial_tiles() {
        let mut state = 0xD1B5_4A32_D192_ED03_u64;
        let half = super::PANEL / 2;
        for (size, sides, swap_half) in [(96, 3, false), (145, 5, false), (145, 3, true)] {
            let mut matrix = vec![vec![0.0; size + sides]; size];
            for (row_idx, row) in matrix.iter_mut().enumerate() {
                for (col_idx, cell) in row.iter_mut().enumerate() {
                    if row_idx < super::PANEL {
                        let in_first_half = row_idx < half;
                        if col_idx < super::PANEL {
                            if (col_idx < half) != in_first_half {
                                continue;
                            }
                        } else {
                            let tile = (col_idx - super::PANEL) / super::TILE_W;
                            if (in_first_half && tile == 0) || (!in_first_half && tile == 1) {
                                continue;
                            }
                        }
                    } else if next(&mut state) < 0.2 {
                        continue;
                    }
                    let scale = match (row_idx + col_idx) % 3 {
                        0 => 1e-4,
                        1 => 1.0,
                        _ => 1e4,
                    };
                    let v = (next(&mut state) - 0.5) * scale;
                    *cell = if v == 0.0 { 0.0 } else { v };
                }
                let diag = if swap_half && row_idx < half {
                    half - 1 - row_idx
                } else if swap_half && row_idx < super::PANEL {
                    super::PANEL + half - 1 - row_idx
                } else {
                    row_idx
                };
                row[diag] = 2e4 + next(&mut state) * 1e4;
            }
            let one = reduced_by_pivot(matrix.clone()).expect("rank");
            let panel = reduced(matrix).expect("rank");
            for (a, b) in one.iter().flatten().zip(panel.iter().flatten()) {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "size {size}, sides {sides}, swap_half {swap_half}"
                );
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
            "14cb548d51049e50ddb154b3f2990ce73c1faf38db65a372e8fbf1f6b3901ebb"
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
        let mut group = vec![[0.0; RING_LANES]; size];
        for (lane, side) in sides.iter().enumerate() {
            for (row, &value) in side.iter().enumerate() {
                group[at_row[row]][lane] = value;
            }
        }
        let (mut spare, mut marks) = (vec![[0.0; RING_LANES]; size], Marks::new(size));
        group[at_row[0]][width] = 7.0;
        whole.solve_group(
            &mut group,
            width,
            None,
            (&at_row, &at_unknown),
            &mut spare,
            &mut marks,
        );
        assert_eq!(group[at_unknown[0]][width].to_bits(), 0.0_f64.to_bits());
        assert_eq!(group[at_row[0]][width].to_bits(), 7.0_f64.to_bits());
        for (lane, mut alone) in sides.into_iter().enumerate() {
            whole.solve(&mut alone);
            for (unknown, value) in alone.iter().enumerate() {
                assert_eq!(value.to_bits(), group[at_unknown[unknown]][lane].to_bits());
            }
        }
    }

    #[test]
    fn reduced_names_the_unknown_without_a_pivot_after_row_swaps() {
        let matrix = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![2.0, 1.0, 1.0, 0.0],
            vec![0.0, 1.0, 1.0, 0.0],
        ];
        assert_eq!(reduced(matrix.clone()), Err(2));
        assert_eq!(reduced_by_pivot(matrix), Err(2));
    }
}
