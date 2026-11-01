use crate::reference::eliminate;
use ng_contract::*;

/// One Newton update on the ring `InitializationSystem::linearize` lays out: `points` blocks of `size` rows, each
/// coupled to the block before it by its stage's tangent alone, totals in the last block reading point 0, and fits
/// along the border. Dense, that is the cube of 24,000 unknowns.
///
/// Each point but the last factors over the body's elimination. Walking forward from point 0, every point becomes a
/// linear function of the last point's rows that point 0 reads and of the fits, so the ring shrinks to one dense corner
/// of the last point's rows and the fits' anchors. A second walk fills every point from the corner's answer.
pub(crate) fn solve_ring(
    system: &InitializationSystem<'_>,
    body: &CompiledBody,
    ring: &Jacobian,
    right: &[f64],
) -> Result<Vec<f64>, usize> {
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
        let lu = body.factor(&system.block(ring, point))?;
        let mut walked = vec![0.0; width * size];
        for (at, column) in walked.chunks_mut(size).enumerate() {
            for row in 0..size {
                let slot = point * size + row;
                let mut side = if at == seeds { right[slot] } else { 0.0 };
                for (place, value) in line(slot).filter(|(place, _)| !own(point).contains(place)) {
                    side -= value
                        * if place >= border {
                            f64::from(at == read.len() + place - border)
                        } else if point == 0 {
                            let seed = read.binary_search(&(place - last * size));
                            f64::from(seed == Ok(at))
                        } else {
                            let earlier: &Vec<f64> = &before;
                            earlier[at * size + place - (point - 1) * size]
                        };
                }
                column[row] = side;
            }
            lu.solve(column);
        }
        for (fit, row) in anchors.iter_mut().enumerate() {
            for (place, value) in line(border + fit).filter(|(place, _)| own(point).contains(place)) {
                for (at, cell) in row.iter_mut().enumerate() {
                    *cell += value * walked[at * size + place - point * size];
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
        for (at, &seed) in read.iter().enumerate() {
            cells[seed] += value * source[at * size + place];
        }
        for fit in 0..fits {
            cells[size + fit] += value * source[(read.len() + fit) * size + place];
        }
        cells[corner] -= value * source[seeds * size + place];
    };
    for row in 0..size {
        let slot = last * size + row;
        let cells = &mut matrix[row];
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
        lu.solve(&mut side);
        update[point * size..(point + 1) * size].copy_from_slice(&side);
    }
    Ok(update)
}
