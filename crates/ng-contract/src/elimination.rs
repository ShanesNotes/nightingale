use crate::body::{RowId, UnknownId};
use crate::graph::Digraph;
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

/// Stored pattern and operation counts of a compiled elimination plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EliminationMetrics {
    pub nonzeros: usize,
    pub factor_macs: usize,
    pub solve_macs: usize,
    pub blocks: usize,
    pub largest_block: usize,
}

/// A fixed block-triangular elimination over one sparsity. Strongly connected components of the structurally matched
/// dependency graph stand in topological order so no fill crosses block boundaries and every 1x1 block factors with no
/// elimination loop. Inside each block of more than one unknown, pivots follow exact directed minimum fill (breaking
/// ties by Markowitz product, then least unknown). The pattern is the union over modes, so a mode change moves no entry.
///
/// `own` stores off-diagonal reads raw beside each diagonal block's entries and fill, lower and upper apart. `closure`
/// symmetrizes each diagonal block: its extra entries hold zeros, and `ghosts` lists them so a solve keeps the bits a
/// walk over them gives. A value that is not finite takes `closure` whole, since a zero times it is not zero.
#[derive(Debug, PartialEq)]
pub(crate) struct Elimination {
    rows: Vec<u32>,
    unknowns: Vec<u32>,
    blocks: Vec<u32>,
    max_block: usize,
    own: Pattern,
    closure: Pattern,
    ghosts: Ghosts,
}

#[derive(Debug, PartialEq)]
struct Pattern {
    starts: Vec<u32>,
    intra_starts: Vec<u32>,
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

const TINY_PIVOT: f64 = 1e-12;

fn scales_of(pattern: &Pattern, values: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let Pattern {
        starts,
        intra_starts,
        columns,
        places,
        ..
    } = pattern;
    let size = starts.len() - 1;
    let mut raw = vec![0.0; columns.len()];
    for (&place, &value) in places.iter().zip(values) {
        raw[index(place)] += value;
    }
    let (mut row_max, mut col_max) = (vec![0.0; size], vec![0.0; size]);
    for at in 0..size {
        let range = index(intra_starts[at])..index(starts[at + 1]);
        let mut rmax = 0.0;
        for (&cell, &column) in raw[range.clone()].iter().zip(&columns[range]) {
            let abs = cell.abs();
            if abs.is_finite() {
                if abs > rmax {
                    rmax = abs;
                }
                let col = index(column);
                if abs > col_max[col] {
                    col_max[col] = abs;
                }
            }
        }
        row_max[at] = rmax;
    }
    (row_max, col_max)
}

#[cold]
fn bad_pivot_slow(
    diagonal: f64,
    at: usize,
    pattern: &Pattern,
    values: &[f64],
    scales: &mut Option<(Vec<f64>, Vec<f64>)>,
) -> bool {
    let (row_max, col_max) = scales.get_or_insert_with(|| scales_of(pattern, values));
    let reach = if row_max[at] < col_max[at] {
        row_max[at]
    } else {
        col_max[at]
    };
    diagonal.abs() <= TINY_PIVOT * reach
}

#[inline(always)]
fn bad_pivot(
    diagonal: f64,
    at: usize,
    row_max: f64,
    pattern: &Pattern,
    values: &[f64],
    scales: &mut Option<(Vec<f64>, Vec<f64>)>,
) -> bool {
    if diagonal == 0.0 || !diagonal.is_finite() {
        return true;
    }
    if diagonal.abs() > TINY_PIVOT * row_max {
        return false;
    }
    bad_pivot_slow(diagonal, at, pattern, values, scales)
}

fn index(value: u32) -> usize {
    value as usize
}

impl Pattern {
    fn of(
        lines: &[Vec<u32>],
        blocks: &[u32],
        rows: &[u32],
        offsets: &[usize],
        unknowns: &[UnknownId],
        position: &[u32],
    ) -> Self {
        let (mut starts, mut intra_starts, mut columns, mut diagonals) = (
            vec![0_u32],
            Vec::with_capacity(lines.len()),
            Vec::new(),
            Vec::with_capacity(lines.len()),
        );
        for block in blocks.windows(2) {
            let (lo, hi) = (block[0], block[1]);
            for (offset, line) in lines[index(lo)..index(hi)].iter().enumerate() {
                let at = lo + offset as u32;
                let base = columns.len() as u32;
                intra_starts.push(base + line.partition_point(|&column| column < lo) as u32);
                diagonals.push(base + line.partition_point(|&column| column < at) as u32);
                columns.extend_from_slice(line);
                starts.push(columns.len() as u32);
            }
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
            intra_starts,
            columns,
            diagonals,
            places,
        }
    }
    fn line(&self, at: usize) -> &[u32] {
        &self.columns[index(self.starts[at])..index(self.starts[at + 1])]
    }
}

fn order_block(members: &[u32], adj: &[Vec<u32>], local_of: &mut [u32], out: &mut Vec<u32>) {
    let k = members.len();
    if k <= 1 {
        out.extend_from_slice(members);
        return;
    }
    for (local, &node) in members.iter().enumerate() {
        local_of[index(node)] = local as u32;
    }
    let mut row_out: Vec<Vec<u32>> = vec![Vec::new(); k];
    let mut col_in: Vec<Vec<u32>> = vec![Vec::new(); k];
    let mut directed_edges = 0_usize;
    for (lu, &u) in members.iter().enumerate() {
        let lu_u = lu as u32;
        for &v in &adj[index(u)] {
            let lv = local_of[index(v)];
            if lv != u32::MAX && lv != lu_u {
                row_out[lu].push(lv);
                col_in[index(lv)].push(lu_u);
                directed_edges += 1;
            }
        }
    }
    if directed_edges == k * (k - 1) {
        out.extend_from_slice(members);
        for &node in members {
            local_of[index(node)] = u32::MAX;
        }
        return;
    }
    let mut mark = vec![0_u32; k];
    let mut tag = 0_u32;
    let fill_of = |p: usize,
                   row_out: &[Vec<u32>],
                   col_in: &[Vec<u32>],
                   mark: &mut [u32],
                   tag: &mut u32|
     -> (usize, usize) {
        let rdeg = row_out[p].len();
        let cdeg = col_in[p].len();
        let prod = rdeg * cdeg;
        if prod == 0 {
            return (0, 0);
        }
        if *tag == u32::MAX {
            mark.fill(0);
            *tag = 0;
        }
        *tag += 1;
        let t = *tag;
        for &col in &row_out[p] {
            mark[index(col)] = t;
        }
        let mut fill = 0_usize;
        for &r in &col_in[p] {
            let mut already = usize::from(mark[index(r)] == t);
            for &col in &row_out[index(r)] {
                already += usize::from(mark[index(col)] == t);
            }
            fill += rdeg - already;
        }
        (fill, prod)
    };
    let mut scores = vec![(0_usize, 0_usize); k];
    let mut heap: BinaryHeap<Reverse<(usize, usize, u32, u32)>> = BinaryHeap::with_capacity(k * 2);
    for lu in 0..k {
        let sc = fill_of(lu, &row_out, &col_in, &mut mark, &mut tag);
        scores[lu] = sc;
        heap.push(Reverse((sc.0, sc.1, members[lu], lu as u32)));
    }
    let mut eliminated = vec![false; k];
    let mut remaining = k;
    let mut touched_mark = vec![0_u32; k];
    let mut touched_tag = 0_u32;
    let mut touched: Vec<u32> = Vec::new();
    let mut added: Vec<(u32, u32)> = Vec::new();
    while let Some(Reverse((f, m, _, p_u))) = heap.pop() {
        let p = index(p_u);
        if eliminated[p] || scores[p] != (f, m) {
            continue;
        }
        eliminated[p] = true;
        remaining -= 1;
        out.push(members[p]);
        let ins = core::mem::take(&mut col_in[p]);
        let outs = core::mem::take(&mut row_out[p]);
        if touched_tag == u32::MAX {
            touched_mark.fill(0);
            touched_tag = 0;
        }
        touched_tag += 1;
        let ttag = touched_tag;
        touched.clear();
        added.clear();
        for &r in &ins {
            let ro = &mut row_out[index(r)];
            if let Some(pos) = ro.iter().position(|&x| x == p_u) {
                ro.swap_remove(pos);
            }
            if touched_mark[index(r)] != ttag {
                touched_mark[index(r)] = ttag;
                touched.push(r);
            }
        }
        for &col in &outs {
            let ci = &mut col_in[index(col)];
            if let Some(pos) = ci.iter().position(|&x| x == p_u) {
                ci.swap_remove(pos);
            }
            if touched_mark[index(col)] != ttag {
                touched_mark[index(col)] = ttag;
                touched.push(col);
            }
        }
        for &r in &ins {
            if tag == u32::MAX {
                mark.fill(0);
                tag = 0;
            }
            tag += 1;
            let t = tag;
            mark[index(r)] = t;
            for &col in &row_out[index(r)] {
                mark[index(col)] = t;
            }
            for &col in &outs {
                if mark[index(col)] != t {
                    mark[index(col)] = t;
                    row_out[index(r)].push(col);
                    col_in[index(col)].push(r);
                    added.push((r, col));
                }
            }
        }
        directed_edges = directed_edges - ins.len() - outs.len() + added.len();
        if remaining > 0 && directed_edges == remaining * (remaining - 1) {
            for (lu, &node) in members.iter().enumerate() {
                if !eliminated[lu] {
                    out.push(node);
                }
            }
            break;
        }
        for &(r, col) in &added {
            let ro = &row_out[index(r)];
            let ci = &col_in[index(col)];
            if tag == u32::MAX {
                mark.fill(0);
                tag = 0;
            }
            tag += 1;
            let t = tag;
            let (small, big) = if ro.len() <= ci.len() { (ro, ci) } else { (ci, ro) };
            for &q in small {
                mark[index(q)] = t;
            }
            for &q in big {
                if !eliminated[index(q)] && mark[index(q)] == t && touched_mark[index(q)] != ttag {
                    touched_mark[index(q)] = ttag;
                    touched.push(q);
                }
            }
        }
        for &u in &touched {
            let ui = index(u);
            if !eliminated[ui] {
                let ns = fill_of(ui, &row_out, &col_in, &mut mark, &mut tag);
                if ns != scores[ui] {
                    scores[ui] = ns;
                    heap.push(Reverse((ns.0, ns.1, members[ui], u)));
                }
            }
        }
    }
    for &node in members {
        local_of[index(node)] = u32::MAX;
    }
}

impl Elimination {
    pub(crate) fn plan(offsets: &[usize], unknowns: &[UnknownId], solved_by: &[RowId]) -> Self {
        let size = offsets.len() - 1;
        let mut node_of_row = vec![0_u32; size];
        for (unknown, row) in solved_by.iter().enumerate() {
            node_of_row[row.index()] = unknown as u32;
        }
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); size];
        let mut edges: Vec<(usize, usize)> = Vec::with_capacity(unknowns.len());
        for row in 0..size {
            let node = node_of_row[row];
            let line = &mut adj[index(node)];
            for read in &unknowns[offsets[row]..offsets[row + 1]] {
                let other = read.index() as u32;
                if other != node {
                    line.push(other);
                }
            }
            line.sort_unstable();
            line.dedup();
            for &other in line.iter() {
                edges.push((index(node), index(other)));
            }
        }
        // Tarjan emits SCCs in reverse topological order: every cross-block edge points to an earlier block.
        let comp = Digraph::new(size, &edges).strongly_connected_components();
        let num_comp = comp.iter().copied().max().map_or(0, |last| last + 1);
        let mut members: Vec<Vec<u32>> = vec![Vec::new(); num_comp];
        for (node, &c) in comp.iter().enumerate() {
            members[c].push(node as u32);
        }
        let mut order = Vec::with_capacity(size);
        let mut blocks = Vec::with_capacity(num_comp + 1);
        blocks.push(0_u32);
        let mut max_block = 0_usize;
        let mut local_of = vec![u32::MAX; size];
        for ms in &members {
            order_block(ms, &adj, &mut local_of, &mut order);
            blocks.push(order.len() as u32);
            max_block = max_block.max(ms.len());
        }
        let mut position = vec![u32::MAX; size];
        for (at, &node) in order.iter().enumerate() {
            position[index(node)] = at as u32;
        }
        let rows: Vec<u32> = (order.iter())
            .map(|&node| solved_by[index(node)].index() as u32)
            .collect();
        let mut own: Vec<Vec<u32>> = Vec::with_capacity(size);
        let mut stamp = vec![u32::MAX; size];
        for block in blocks.windows(2) {
            let (lo, hi) = (block[0], block[1]);
            for at in lo..hi {
                let row = rows[index(at)];
                let (mut line, mut pending) = (vec![at], Vec::new());
                stamp[index(at)] = at;
                let read = unknowns[offsets[index(row)]..offsets[index(row) + 1]].iter();
                let mut reach = |column: u32, line: &mut Vec<u32>, pending: &mut Vec<u32>| {
                    if stamp[index(column)] != at {
                        stamp[index(column)] = at;
                        line.push(column);
                        if column >= lo && column < at {
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
        }
        let mut closure: Vec<Vec<u32>> = Vec::with_capacity(size);
        stamp.fill(u32::MAX);
        for block in blocks.windows(2) {
            let (lo, hi) = (block[0], block[1]);
            if lo + 1 == hi {
                closure.push(own[index(lo)].clone());
                continue;
            }
            let len = index(hi - lo);
            let mut sym_intra: Vec<Vec<u32>> = vec![Vec::new(); len];
            for at in lo..hi {
                let row = rows[index(at)];
                for read in &unknowns[offsets[index(row)]..offsets[index(row) + 1]] {
                    let col = position[read.index()];
                    if col >= lo && col != at {
                        sym_intra[index(at - lo)].push(col);
                        sym_intra[index(col - lo)].push(at);
                    }
                }
            }
            for at in lo..hi {
                let (mut line, mut todo) = (vec![at], Vec::new());
                stamp[index(at)] = at;
                for &col in &own[index(at)] {
                    if col < lo && stamp[index(col)] != at {
                        stamp[index(col)] = at;
                        line.push(col);
                    }
                }
                let mut visit = |col: u32, line: &mut Vec<u32>, todo: &mut Vec<u32>| {
                    if stamp[index(col)] != at {
                        stamp[index(col)] = at;
                        line.push(col);
                        if col >= lo && col < at {
                            todo.push(col);
                        }
                    }
                };
                for &col in &sym_intra[index(at - lo)] {
                    visit(col, &mut line, &mut todo);
                }
                while let Some(piv) = todo.pop() {
                    for &col in &closure[index(piv)] {
                        if col > piv {
                            visit(col, &mut line, &mut todo);
                        }
                    }
                }
                line.sort_unstable();
                closure.push(line);
            }
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
            own: Pattern::of(&own, &blocks, &rows, offsets, unknowns, &position),
            closure: Pattern::of(&closure, &blocks, &rows, offsets, unknowns, &position),
            rows,
            unknowns: order,
            blocks,
            max_block,
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
    pub(crate) fn metrics(&self) -> EliminationMetrics {
        let Pattern {
            starts,
            intra_starts,
            columns,
            diagonals,
            ..
        } = &self.own;
        let size = self.rows.len();
        let mut factor_macs = 0_usize;
        for at in 0..size {
            for &pivot in &columns[index(intra_starts[at])..index(diagonals[at])] {
                let p = index(pivot);
                factor_macs += index(starts[p + 1]) - index(diagonals[p]) - 1;
            }
        }
        EliminationMetrics {
            nonzeros: columns.len(),
            factor_macs,
            solve_macs: columns.len().saturating_sub(size),
            blocks: self.blocks.len().saturating_sub(1),
            largest_block: self.max_block,
        }
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
        let cap = self.max_block;
        if cap <= 256 {
            let mut seen = [0.0; 256];
            self.eliminate_in(pattern, values, &mut seen[..cap])
        } else if cap <= 768 {
            let mut seen = [0.0; 768];
            self.eliminate_in(pattern, values, &mut seen[..cap])
        } else if cap <= 1536 {
            let mut seen = [0.0; 1536];
            self.eliminate_in(pattern, values, &mut seen[..cap])
        } else {
            let mut seen = vec![0.0; cap];
            self.eliminate_in(pattern, values, &mut seen)
        }
    }

    fn eliminate_in(&self, pattern: &Pattern, values: &[f64], seen: &mut [f64]) -> Eliminated {
        let Pattern {
            starts,
            intra_starts,
            columns,
            diagonals,
            places,
        } = pattern;
        let size = self.rows.len();
        let starts = &starts[..=size];
        let intra_starts = &intra_starts[..size];
        let diagonals = &diagonals[..size];
        let mut lu = vec![0.0; columns.len()];
        for (&place, &value) in places.iter().zip(values) {
            lu[index(place)] += value;
        }
        assert_eq!(lu.len(), columns.len());
        let mut scales: Option<(Vec<f64>, Vec<f64>)> = None;
        let mut finite = true;
        for block in self.blocks.windows(2) {
            let (lo, hi) = (index(block[0]), index(block[1]));
            if lo + 1 == hi {
                let (start, end) = (index(starts[lo]), index(starts[lo + 1]));
                for &cell in &lu[start..end] {
                    finite &= cell.is_finite();
                }
                let diagonal = lu[index(diagonals[lo])];
                if diagonal == 0.0 || !diagonal.is_finite() {
                    return Eliminated {
                        found: Err(lo),
                        finite,
                    };
                }
                continue;
            }
            for at in lo..hi {
                let (start, intra, diag, end) = (
                    index(starts[at]),
                    index(intra_starts[at]),
                    index(diagonals[at]),
                    index(starts[at + 1]),
                );
                for &cell in &lu[start..intra] {
                    finite &= cell.is_finite();
                }
                let mut row_max = 0.0;
                let (done, rest) = lu.split_at_mut(intra);
                let line = &mut rest[..end - intra];
                let diagonal = if intra == diag {
                    for &cell in line.iter() {
                        finite &= cell.is_finite();
                        let abs = cell.abs();
                        if abs > row_max {
                            row_max = abs;
                        }
                    }
                    line[0]
                } else {
                    for (&cell, &column) in line.iter().zip(&columns[intra..end]) {
                        seen[index(column) - lo] = cell;
                        let abs = cell.abs();
                        if abs > row_max {
                            row_max = abs;
                        }
                    }
                    let (lower, upper) = line.split_at_mut(diag - intra);
                    for (lower_cell, &pivot) in lower.iter_mut().zip(&columns[intra..diag]) {
                        let pivot = index(pivot);
                        let pivot_diag = index(diagonals[pivot]);
                        let factor = seen[pivot - lo] / done[pivot_diag];
                        *lower_cell = factor;
                        finite &= factor.is_finite();
                        let tail = pivot_diag + 1..index(starts[pivot + 1]);
                        for (&cell, &column) in done[tail.clone()].iter().zip(&columns[tail]) {
                            seen[index(column) - lo] -= factor * cell;
                        }
                    }
                    for (cell, &column) in upper.iter_mut().zip(&columns[diag..end]) {
                        let value = seen[index(column) - lo];
                        *cell = value;
                        finite &= value.is_finite();
                    }
                    upper[0]
                };
                if bad_pivot(diagonal, at, row_max, pattern, values, &mut scales) {
                    return Eliminated {
                        found: Err(at),
                        finite,
                    };
                }
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
        let size = self.plan.rows.len();
        if size <= 64 {
            let mut buf = [[0.0; 1]; 64];
            self.solve_in(side, &mut buf[..size]);
        } else if size <= 320 {
            let mut buf = [[0.0; 1]; 320];
            self.solve_in(side, &mut buf[..size]);
        } else if size <= 2048 {
            let mut buf = [[0.0; 1]; 2048];
            self.solve_in(side, &mut buf[..size]);
        } else if size <= 3072 {
            let mut buf = [[0.0; 1]; 3072];
            self.solve_in(side, &mut buf[..size]);
        } else {
            let mut buf = vec![[0.0; 1]; size];
            self.solve_in(side, &mut buf);
        }
    }
    fn solve_in(&self, side: &mut [f64], pivoted: &mut [[f64; 1]]) {
        let Elimination { rows, unknowns, .. } = self.plan;
        for (slot, &row) in pivoted.iter_mut().zip(rows) {
            *slot = [side[index(row)]];
        }
        if !self.substitute(pivoted) {
            for (slot, &row) in pivoted.iter_mut().zip(rows) {
                *slot = [side[index(row)]];
            }
            self.substitute_closed(pivoted);
        }
        for (&unknown, &[value]) in unknowns.iter().zip(pivoted.iter()) {
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
        if let Some(reach) = reach {
            if self.substitute_reached(group, reach, spare, marks) {
                return;
            }
            group.fill([0.0; LANES]);
            for &at in &marks.seeds {
                group[index(at)] = spare[index(at)];
            }
            if !self.substitute(group) {
                group.fill([0.0; LANES]);
                for &at in &marks.seeds {
                    group[index(at)] = spare[index(at)];
                }
                self.substitute_closed(group);
            }
        } else {
            spare.copy_from_slice(group);
            if !self.substitute(group) {
                group.copy_from_slice(spare);
                self.substitute_closed(group);
            }
        }
    }
    /// Walks only the rows a nonzero side reaches through a nonzero factor, block by block in the order a full walk
    /// takes them. A row whose side is a positive zero and whose nonzero factors read only such rows keeps that zero
    /// going forward and takes zero over its pivot going back, as a full walk gives it. A value that is not finite
    /// sends the group back to the full walk, since a zero factor times it is not zero.
    fn substitute_reached(
        &self,
        pivoted: &mut [[f64; LANES]],
        reach: &Reach,
        spare: &mut [[f64; LANES]],
        marks: &mut Marks,
    ) -> bool {
        match self.ghosts(pivoted) {
            Some(ghosts) => self.substitute_reached_with(pivoted, reach, spare, marks, Some(ghosts)),
            None => self.substitute_reached_with(pivoted, reach, spare, marks, None),
        }
    }
    #[inline(always)]
    fn substitute_reached_with(
        &self,
        pivoted: &mut [[f64; LANES]],
        reach: &Reach,
        spare: &mut [[f64; LANES]],
        marks: &mut Marks,
        ghosts: Option<&Ghosts>,
    ) -> bool {
        let pattern = self.pattern();
        let size = pivoted.len();
        let starts = &pattern.starts[..=size];
        let diagonals = &pattern.diagonals[..size];
        let columns = &pattern.columns;
        assert_eq!(self.lu.len(), columns.len());
        marks.clear();
        for (at, values) in pivoted.iter().enumerate() {
            if values.iter().any(|value| value.to_bits() != 0) {
                marks.forward[at] = true;
                marks.seeds.push(at as u32);
                spare[at] = *values;
            }
        }
        for (block_idx, block) in self.plan.blocks.windows(2).enumerate() {
            let (lo, hi) = (index(block[0]), index(block[1]));
            let mut any = false;
            for at in lo..hi {
                if marks.forward[at] {
                    any = true;
                    marks.stack.push(at as u32);
                }
            }
            if !any && !reach.block_neg_diag[block_idx] {
                continue;
            }
            if any {
                if lo + 1 == hi {
                    marks.stack.pop();
                    marks.back[lo] = true;
                    forward(starts, diagonals, columns, &self.lu, ghosts, pivoted, lo);
                    back(starts, diagonals, columns, &self.lu, ghosts, pivoted, lo);
                    for &reader in reach.off_readers(lo) {
                        marks.forward[index(reader)] = true;
                    }
                    continue;
                }
                marks.spread(|marks| &mut marks.forward, &reach.lower_starts, &reach.lower);
                for at in (lo..hi).filter(|&at| marks.forward[at]) {
                    forward(starts, diagonals, columns, &self.lu, ghosts, pivoted, at);
                    marks.back[at] = true;
                    marks.stack.push(at as u32);
                }
                marks.spread(|marks| &mut marks.back, &reach.upper_starts, &reach.upper);
            }
            for at in (lo..hi).filter(|&at| !marks.back[at]) {
                if self.lu[index(diagonals[at])].is_sign_negative() {
                    pivoted[at] = [0.0 / self.lu[index(diagonals[at])]; LANES];
                }
            }
            if any {
                for at in (lo..hi).rev().filter(|&at| marks.back[at]) {
                    back(starts, diagonals, columns, &self.lu, ghosts, pivoted, at);
                }
                for at in (lo..hi).filter(|&at| marks.back[at]) {
                    for &reader in reach.off_readers(at) {
                        marks.forward[index(reader)] = true;
                    }
                }
            }
        }
        (0..size)
            .filter(|&at| marks.back[at])
            .all(|at| pivoted[at].iter().all(|value| value.is_finite()))
    }
    fn substitute<const L: usize>(&self, pivoted: &mut [[f64; L]]) -> bool {
        match self.ghosts(pivoted) {
            Some(ghosts) => walk(&self.plan.blocks, self.pattern(), &self.lu, Some(ghosts), pivoted),
            None => walk(&self.plan.blocks, self.pattern(), &self.lu, None, pivoted),
        }
        self.closed || (pivoted.as_flattened().iter()).fold(true, |finite, value| finite & value.is_finite())
    }
    fn substitute_closed<const L: usize>(&self, pivoted: &mut [[f64; L]]) {
        let Elimination {
            own, closure, blocks, ..
        } = self.plan;
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
        walk(blocks, closure, &lu, None, pivoted);
    }
}

#[inline(always)]
fn walk<const L: usize>(
    blocks: &[u32],
    pattern: &Pattern,
    lu: &[f64],
    ghosts: Option<&Ghosts>,
    pivoted: &mut [[f64; L]],
) {
    let size = pivoted.len();
    let starts = &pattern.starts[..=size];
    let diagonals = &pattern.diagonals[..size];
    let columns = &pattern.columns;
    assert_eq!(lu.len(), columns.len());
    for block in blocks.windows(2) {
        let (lo, hi) = (index(block[0]), index(block[1]));
        if lo + 1 == hi {
            let (start, diag) = (index(starts[lo]), index(diagonals[lo]));
            let mut sum = pivoted[lo];
            if start < diag {
                for (&cell, &column) in lu[start..diag].iter().zip(&columns[start..diag]) {
                    let read = pivoted[index(column)];
                    for (value, read) in sum.iter_mut().zip(read) {
                        *value -= cell * read;
                    }
                }
            }
            let pivot = lu[diag];
            for value in &mut sum {
                *value /= pivot;
            }
            pivoted[lo] = sum;
            continue;
        }
        for at in lo..hi {
            forward(starts, diagonals, columns, lu, ghosts, pivoted, at);
        }
        for at in (lo..hi).rev() {
            back(starts, diagonals, columns, lu, ghosts, pivoted, at);
        }
    }
}

#[inline(always)]
fn forward<const L: usize>(
    starts: &[u32],
    diagonals: &[u32],
    columns: &[u32],
    lu: &[f64],
    ghosts: Option<&Ghosts>,
    pivoted: &mut [[f64; L]],
    at: usize,
) {
    let (start, diag) = (index(starts[at]), index(diagonals[at]));
    if start == diag && ghosts.is_none() {
        return;
    }
    let mut sum = pivoted[at];
    for (&cell, &column) in lu[start..diag].iter().zip(&columns[start..diag]) {
        let read = pivoted[index(column)];
        for (value, read) in sum.iter_mut().zip(read) {
            *value -= cell * read;
        }
    }
    if let Some(ghosts) = ghosts.filter(|_| sum.iter().any(|value| value.to_bits() == NEGATIVE_ZERO)) {
        let negative = |pivot: usize| lu[index(diagonals[pivot])].is_sign_negative();
        ghosts.settle(&mut sum, ghosts.lower(at), negative, pivoted);
    }
    pivoted[at] = sum;
}

#[inline(always)]
fn back<const L: usize>(
    starts: &[u32],
    diagonals: &[u32],
    columns: &[u32],
    lu: &[f64],
    ghosts: Option<&Ghosts>,
    pivoted: &mut [[f64; L]],
    at: usize,
) {
    let (diag, end) = (index(diagonals[at]), index(starts[at + 1]));
    let mut sum = pivoted[at];
    if diag + 1 < end {
        for (&cell, &column) in lu[diag + 1..end].iter().zip(&columns[diag + 1..end]) {
            let read = pivoted[index(column)];
            for (value, read) in sum.iter_mut().zip(read) {
                *value -= cell * read;
            }
        }
    }
    if let Some(ghosts) = ghosts.filter(|_| sum.iter().any(|value| value.to_bits() == NEGATIVE_ZERO)) {
        ghosts.settle(&mut sum, ghosts.upper(at), |_| false, pivoted);
    }
    let pivot = lu[diag];
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
    blocks: Vec<u32>,
    block_neg_diag: Vec<bool>,
    off_starts: Vec<u32>,
    off: Vec<u32>,
    lower_starts: Vec<u32>,
    lower: Vec<u32>,
    upper_starts: Vec<u32>,
    upper: Vec<u32>,
}
impl Reach {
    fn off_readers(&self, at: usize) -> &[u32] {
        &self.off[index(self.off_starts[at])..index(self.off_starts[at + 1])]
    }
    pub(crate) fn key(
        &self,
        rows: &[usize],
        at_row: &[usize],
        marks: &mut Marks,
    ) -> (usize, usize, usize, u64) {
        marks.clear();
        for &row in rows {
            marks.forward[at_row[row]] = true;
        }
        for block in self.blocks.windows(2) {
            let (lo, hi) = (index(block[0]), index(block[1]));
            if lo + 1 == hi {
                if marks.forward[lo] {
                    marks.back[lo] = true;
                    for &reader in self.off_readers(lo) {
                        marks.forward[index(reader)] = true;
                    }
                }
                continue;
            }
            let mut any = false;
            for at in lo..hi {
                if marks.forward[at] {
                    any = true;
                    marks.stack.push(at as u32);
                }
            }
            if !any {
                continue;
            }
            marks.spread(|marks| &mut marks.forward, &self.lower_starts, &self.lower);
            for at in lo..hi {
                if marks.forward[at] {
                    marks.back[at] = true;
                    marks.stack.push(at as u32);
                }
            }
            marks.spread(|marks| &mut marks.back, &self.upper_starts, &self.upper);
            for at in lo..hi {
                if marks.back[at] {
                    for &reader in self.off_readers(at) {
                        marks.forward[index(reader)] = true;
                    }
                }
            }
        }
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
            intra_starts,
            columns,
            diagonals,
            ..
        } = factors.pattern();
        if !factors.lu.iter().all(|cell| cell.is_finite()) {
            return None;
        }
        let size = diagonals.len();
        fn transpose(
            size: usize,
            lu: &[f64],
            columns: &[u32],
            lines: impl Fn(usize) -> core::ops::Range<usize>,
        ) -> (Vec<u32>, Vec<u32>) {
            let mut counts = vec![0_u32; size + 1];
            for at in 0..size {
                for entry in lines(at).filter(|&entry| lu[entry] != 0.0) {
                    counts[index(columns[entry]) + 1] += 1;
                }
            }
            for at in 0..size {
                counts[at + 1] += counts[at];
            }
            let mut filled = counts.clone();
            let mut readers = vec![0_u32; index(counts[size])];
            for at in 0..size {
                for entry in lines(at).filter(|&entry| lu[entry] != 0.0) {
                    let column = index(columns[entry]);
                    readers[index(filled[column])] = at as u32;
                    filled[column] += 1;
                }
            }
            (counts, readers)
        }
        let (off_starts, off) = transpose(size, &factors.lu, columns, |at| {
            index(starts[at])..index(intra_starts[at])
        });
        let (lower_starts, lower) = transpose(size, &factors.lu, columns, |at| {
            index(intra_starts[at])..index(diagonals[at])
        });
        let (upper_starts, upper) = transpose(size, &factors.lu, columns, |at| {
            index(diagonals[at]) + 1..index(starts[at + 1])
        });
        let block_neg_diag: Vec<bool> = factors
            .plan
            .blocks
            .windows(2)
            .map(|block| {
                (index(block[0])..index(block[1]))
                    .any(|at| factors.lu[index(diagonals[at])].is_sign_negative())
            })
            .collect();
        Some(Self {
            blocks: factors.plan.blocks.clone(),
            block_neg_diag,
            off_starts,
            off,
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
    seeds: Vec<u32>,
}
impl Marks {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            forward: vec![false; size],
            back: vec![false; size],
            stack: Vec::with_capacity(size),
            seeds: Vec::with_capacity(size),
        }
    }
    fn clear(&mut self) {
        self.forward.fill(false);
        self.back.fill(false);
        self.stack.clear();
        self.seeds.clear();
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
        let had = "8a58da40fecd96f6811d6858bfa0901640ea6d20c50d307fadfd508600e635a7";
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
            for block in plan.blocks.windows(2) {
                let (lo, hi) = (block[0] as usize, block[1] as usize);
                for pivot in lo..hi {
                    let (above, below) = filled.split_at_mut(pivot + 1);
                    for line in below[..hi - pivot - 1].iter_mut().filter(|line| line[pivot]) {
                        for (cell, &held) in line.iter_mut().zip(&above[pivot]).skip(pivot + 1) {
                            *cell |= held;
                        }
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
            let ghost_row = (0..size)
                .find_map(|at| {
                    plan.ghosts
                        .lower(at)
                        .first()
                        .or_else(|| plan.ghosts.upper(at).first())
                        .map(|&col| plan.rows[col as usize] as usize)
                })
                .unwrap_or(size / 2);
            for (side, value) in
                sides
                    .iter_mut()
                    .skip(LANES)
                    .zip([f64::INFINITY, f64::NAN, f64::NEG_INFINITY])
            {
                side[size / 2] = value;
                side[ghost_row] = value;
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
    fn a_negative_zero_in_any_lane_and_row_gives_its_group_the_ghosts() {
        let mut state = 0x510E_527F_ADE6_82D1;
        let mut turned = [0_usize; LANES];
        for (system, plan) in systems(&mut state) {
            let size = system.a.len();
            let own = plan.factor(&system.values).expect("a pivot");
            let today = plan.factor_closed(&system.values).expect("a pivot");
            let (at_row, at_unknown) = plan.places();
            let mut marks = Marks::new(size);
            for (lane, turned) in turned.iter_mut().enumerate().skip(1) {
                let mut sides = Vec::with_capacity(size * LANES);
                for row in 0..size {
                    for at in 0..LANES {
                        let side = (0..size).map(|other| match (at == lane, other == row) {
                            (true, true) => -0.0,
                            (true, false) => 0.0,
                            (false, _) => next(&mut state) + 0.5,
                        });
                        sides.push(side.collect::<Vec<f64>>());
                    }
                }
                for reached in [true, false] {
                    let (kept, had) = (
                        grouped(&own, &sides, reached, &mut marks),
                        grouped(&today, &sides, reached, &mut marks),
                    );
                    for (kept, had) in kept.iter().zip(&had) {
                        assert!(
                            same_bits(kept, had),
                            "lane {lane} keeps the symmetric plan's bits"
                        );
                    }
                }
                let had = grouped(&today, &sides, false, &mut marks);
                for (row, (side, had)) in sides.iter().zip(&had).skip(lane).step_by(LANES).enumerate() {
                    let mut pivoted = vec![[0.0]; size];
                    for (from, &value) in side.iter().enumerate() {
                        pivoted[at_row[from]] = [value];
                    }
                    super::walk(&plan.blocks, &plan.own, &own.lu, None, &mut pivoted);
                    let bare: Vec<f64> = (0..size).map(|unknown| pivoted[at_unknown[unknown]][0]).collect();
                    *turned += usize::from(at_row[row] > 0 && !same_bits(&bare, had));
                }
            }
        }
        for (lane, turned) in turned.iter().enumerate().skip(1) {
            assert!(
                *turned > 0,
                "a ghost turns a negative zero in lane {lane} after the first row"
            );
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

    #[test]
    fn a_tiny_pivot_relative_to_its_row_and_column_is_refused_while_a_scaled_diagonal_passes() {
        let unknowns = [
            UnknownId::at(0),
            UnknownId::at(1),
            UnknownId::at(0),
            UnknownId::at(1),
        ];
        let matching = [RowId::at(0), RowId::at(1)];
        let plan = Elimination::plan(&[0, 2, 4], &unknowns, &matching);
        for tiny in [1e-13, 1e-17, 1e-18, 1e-20] {
            assert_eq!(plan.factor(&[tiny, 1.0, 1.0, 1.0]).err(), Some(0));
        }
        let mut side = [2.0, 3.0];
        plan.factor(&[1e-20, 0.0, 0.0, 1.0])
            .expect("a uniformly scaled diagonal passes")
            .solve(&mut side);
        assert!((side[0] - 2e20).abs() <= 1e9);
        assert!((side[1] - 3.0).abs() <= 1e-12);
    }

    #[test]
    fn a_block_triangular_plan_adds_no_cross_block_fill() {
        let mut a = vec![vec![0.0; 10]; 10];
        for row in 0..4 {
            a[row][row] = 4.0;
            a[row][(row + 1) % 4] = 0.5;
            a[row][(row + 2) % 4] = -0.25;
        }
        for row in 4..8 {
            a[row][row] = 4.0;
            a[row][4 + (row - 3) % 4] = 0.5;
            a[row][0] = 0.3;
        }
        a[8][8] = 2.0;
        a[9][9] = 3.0;
        a[9][4] = 0.2;
        let matching: Vec<RowId> = (0..10).map(RowId::at).collect();
        let (plan, values) = planned(&a, &matching);
        assert_eq!(plan.blocks.len() - 1, 4);
        assert_eq!(plan.max_block, 4);
        let off_diag: usize = (0..10)
            .map(|at| (plan.own.intra_starts[at] - plan.own.starts[at]) as usize)
            .sum();
        assert_eq!(off_diag, 5, "only the 5 raw cross-block reads sit off-diagonal");
        let side: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let mut found = side.clone();
        plan.factor(&values).expect("a pivot").solve(&mut found);
        for (one, other) in found.iter().zip(dense_solve(a, side)) {
            assert!((one - other).abs() < 1e-11, "{one} against {other}");
        }
    }

    #[test]
    fn a_singleton_block_keeps_its_pivot_despite_larger_cross_block_reads() {
        let a = vec![vec![1.0, 0.0, 0.0], vec![1.0, 1e-16, 0.0], vec![0.0, 1.0, 1.0]];
        let matching = [RowId::at(0), RowId::at(1), RowId::at(2)];
        let (plan, values) = planned(&a, &matching);
        assert_eq!(plan.blocks.len() - 1, 3);
        let mut side = [0.0, 1e-16, 1.0];
        plan.factor(&values)
            .expect("singleton pivot is unaffected by cross-block scale")
            .solve(&mut side);
        assert_eq!(side, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_fully_connected_block_plans_in_canonical_order() {
        let size = 128;
        let mut offsets = Vec::with_capacity(size + 1);
        let mut unknowns = Vec::with_capacity(size * size);
        offsets.push(0);
        for _ in 0..size {
            for col in 0..size {
                unknowns.push(UnknownId::at(col));
            }
            offsets.push(unknowns.len());
        }
        let matching: Vec<RowId> = (0..size).map(RowId::at).collect();
        let plan = Elimination::plan(&offsets, &unknowns, &matching);
        assert_eq!(plan.blocks, vec![0, size as u32]);
        let expected: Vec<u32> = (0..size as u32).collect();
        assert_eq!(plan.unknowns, expected);
        assert_eq!(plan.rows, expected);
    }
}
