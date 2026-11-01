use ng_contract::SparseView;

/// A fixed elimination order for one pattern, found once and read on every update, so a step's solve costs what the
/// map's fill costs and not the cube of the rows. On the slice's pattern it spends 32,000 multiplies a factor against
/// 4,600,000 dense, and 65 us against 519 us in the probe (`probe/src/bin/speed.rs`).
///
/// The order is a pure function of the pattern, so it holds nothing a resume could miss, and every tie breaks to the
/// least row, so it is one order on every host. A rate column's pivot can read zero in one mode and not another, so a
/// pivot below rounding is refused here and the caller's `Err` names its slot.
pub(crate) struct Elimination {
    order: Vec<usize>,
}
impl Elimination {
    /// Minimum degree on the symmetrized pattern, least row first on a tie. The pattern is the union over modes, so
    /// one order serves every mode.
    pub(crate) fn of(pattern: SparseView<'_>) -> Self {
        let _ = (pattern, &Self::of as *const _);
        // pseudocode, as `analyze` in probe/src/bin/speed.rs:
        //   adjacency from row_offsets and columns, both directions; repeat for each row: take the live row of least
        //   degree, ties to the least index, join its live neighbours pairwise, retire it; the order is the sequence.
        todo!()
    }
    /// Solves `A x = b` in this order, with a row swap inside a block of the matched pattern where a closed row's
    /// diagonal reads zero. `Err` names the slot with no pivot.
    pub(crate) fn solve(&mut self, matrix: SparseView<'_>, right: &[f64]) -> Result<Vec<f64>, usize> {
        let _ = (matrix, right, &self.order);
        // pseudocode, as `factor` and `back` in probe/src/bin/speed.rs, with the pivot row taken among the rows the
        // order has not eliminated, first largest wins, so a tie settles the same way on every run.
        todo!()
    }
}
