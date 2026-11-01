use ng_contract::SparseView;

/// One Newton update on the ring `InitializationSystem::linearize` lays out: `points` blocks of `size` rows, each
/// coupled to the block before it by its stage's tangent alone, totals in the last block reading point 0, and fits
/// along the border. Dense, that is the cube of 24,000 unknowns. This is the slice's 80 ms.
///
/// Each point's own block is one sparse factor. Walking forward from point 0, every later point becomes a linear
/// function of point 0's differential rows and the fits, so the ring shrinks to one dense corner of about 170 rows
/// holding point 0's stage rows, the fits' anchors and a slack for each total's replaced row. Fixed order, so a run's
/// update is the same bits at every run.
pub(crate) fn solve_ring(
    matrix: SparseView<'_>,
    right: &[f64],
    size: usize,
    points: usize,
) -> Result<Vec<f64>, usize> {
    let _ = (matrix, right, size, points);
    // pseudocode, measured in probe/src/bin/speed.rs:
    //   split matrix by the slot layout (k * size + i, fits past points * size) into A_k (own block), B_k (the tangent's
    //     -1s on differential rows, absent at a reset), F_k (fit columns), the replaced total rows, and the anchor rows.
    //   seeds = one column per differential row of point 0, one per fit, one per total row (its slack), plus the right side.
    //   X_0 = identity on the seeds. For k in 1..points:
    //     factor A_k with `Elimination`, in the one order the pattern gave;
    //     for each seed: back-solve A_k x = right_k - B_k X_(k-1) - F_k, storing x as X_k.
    //   Accumulate each cycle-mean anchor's row as the mean of X_k over k.
    //   Corner: point 0's rows on X_(points - 1), the anchors, the totals on point 0. Solve it with `solve_dense`.
    //   Walk forward once more with the corner's solution to fill every point.
    todo!()
}
