//! The admission system's matrix assembled with no `linearize`, from central differences of `evaluate`, and a dense
//! solve of it with no ring, so the lifted ring answers to an independent reading of the same rows.

use ng_contract::{InitializationSystem, SolveWorkspace};

fn differenced(system: &InitializationSystem<'_>, workspace: &mut SolveWorkspace) -> Vec<Vec<f64>> {
    let size = workspace.unknowns_mut().len();
    let mut matrix = vec![vec![0.0; size]; size];
    for column in 0..size {
        let held = workspace.unknowns_mut()[column];
        let step = 1e-6 * (1.0 + held.abs());
        workspace.unknowns_mut()[column] = held + step;
        let up = system.evaluate(workspace).expect("every row");
        workspace.unknowns_mut()[column] = held - step;
        let down = system.evaluate(workspace).expect("every row");
        workspace.unknowns_mut()[column] = held;
        for (row, cells) in matrix.iter_mut().enumerate() {
            cells[column] = (up[row] - down[row]) / (2.0 * step);
        }
    }
    matrix
}

fn solved(mut matrix: Vec<Vec<f64>>, mut right: Vec<f64>) -> Vec<f64> {
    let size = right.len();
    for pivot in 0..size {
        let best = (pivot..size)
            .max_by(|&one, &other| matrix[one][pivot].abs().total_cmp(&matrix[other][pivot].abs()))
            .expect("a row");
        matrix.swap(pivot, best);
        right.swap(pivot, best);
        for row in pivot + 1..size {
            let factor = matrix[row][pivot] / matrix[pivot][pivot];
            let (above, below) = matrix.split_at_mut(row);
            for (cell, lead) in below[0].iter_mut().zip(&above[pivot]).skip(pivot) {
                *cell -= factor * lead;
            }
            right[row] -= factor * right[pivot];
        }
    }
    for row in (0..size).rev() {
        let known: f64 = (row + 1..size)
            .map(|column| matrix[row][column] * right[column])
            .sum();
        right[row] = (right[row] - known) / matrix[row][row];
    }
    right
}

/// Every entry of `linearize` against the differenced matrix, and the ring's update against the dense one.
pub fn cross_check(system: &InitializationSystem<'_>, workspace: &mut SolveWorkspace, tag: &str) {
    let matrix = differenced(system, workspace);
    let lifted = system.linearize(workspace).expect("a matrix");
    let view = lifted.view();
    let size = matrix.len();
    for (row, cells) in matrix.iter().enumerate() {
        let mut line = vec![0.0; size];
        for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
            line[view.columns()[entry].index()] += view.values()[entry];
        }
        for (column, (&slope, &difference)) in line.iter().zip(cells).enumerate() {
            let scale = 1.0 + slope.abs();
            assert!(
                (slope - difference).abs() < 1e-5 * scale,
                "{tag}: row {row} column {column}: {slope} against {difference}"
            );
        }
    }
    let residuals = system.evaluate(workspace).expect("every row");
    let ring = system.solve(&lifted, &residuals).expect("a pivot");
    let dense = solved(matrix, residuals);
    let largest = dense.iter().fold(0.0_f64, |most, value| most.max(value.abs()));
    for (slot, (one, other)) in ring.iter().zip(&dense).enumerate() {
        assert!(
            (one - other).abs() <= 1e-5 * (1.0 + largest),
            "{tag}: slot {slot}: {one} against {other}"
        );
    }
}
