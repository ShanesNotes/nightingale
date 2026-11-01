use crate::body::{Bound, Instance, ModeLayout, Target};
use crate::boundary::{Delivery, DriveFrame, PendingInputs};
use crate::chance::{HELD_DRAW, HELD_FACTOR};
use crate::evaluate::{Inputs, Undefined};
use crate::packet::{BoundAnchor, Fit, Owed};
use crate::problem::{Jacobian, SolveWorkspace};
use crate::{
    AdmissionError, AnchorId, AnchorStatistic, Book, BoundParameters, BoundaryPort, Candidate, Combination,
    Compensated, CompiledBody, DenseSegment, ErrorScales, ImageParts, InitializationProvenance, LawName,
    MethodHistory, ModelHash, ObservationLens, PacketHash, QuantityId, Row, RowId, RowKind, SimTime,
    UnknownId, ValueName, ValueProvenance, ValueRecord, eliminate,
};
use core::ops::Range;

const NANOSECONDS_PER_SECOND: f64 = 1e9;
/// What rounding leaves of a value, past which a change to it can be told from nothing.
const ROUNDING: f64 = 8.0 * f64::EPSILON;

/// When the start stands and what reads the anchors. The goal and the tolerance are the packet's.
pub struct InitializationRequest<'l> {
    anchor_moment: SimTime,
    lens: Option<&'l ObservationLens>,
}
impl InitializationRequest<'static> {
    /// A packet with an anchor needs `reading` too.
    pub fn at(anchor_moment: SimTime) -> Self {
        Self {
            anchor_moment,
            lens: None,
        }
    }
}
impl InitializationRequest<'_> {
    /// Every anchor must be read by a channel of `lens` with its method and window (rule 17). A lens only refuses,
    /// so it shapes no bit of the start and stays out of the run's identity.
    pub fn reading(self, lens: &ObservationLens) -> InitializationRequest<'_> {
        InitializationRequest {
            anchor_moment: self.anchor_moment,
            lens: Some(lens),
        }
    }
}

impl CompiledBody {
    /// Whether the route's own law reads its drive in every mode. Another row may sense a drive, and moves nothing, and
    /// a law of one mode alone would leave a delivery's amount waiting on that mode.
    fn reads_drive(&self, boundary: &str) -> bool {
        let own = |instance: &&Instance| match instance.target {
            Target::Column(column) => (self.layout.columns[column.index()].receipt)
                .is_some_and(|receipt| receipt.boundary == boundary),
            Target::Row(_) => false,
        };
        let reads = |instance: &&Instance| {
            let read = |input: &Bound| matches!(input, Bound::Boundary(name) if *name == boundary);
            instance.inputs.iter().any(read)
        };
        let held: Vec<Option<(usize, usize)>> = (self.layout.instances.iter())
            .filter(own)
            .filter(reads)
            .map(|instance| instance.mode)
            .collect();
        let every_value = |&(variable, _): &(usize, usize)| {
            let values = self.layout.modes[variable].values.len();
            (0..values).all(|value| held.contains(&Some((variable, value))))
        };
        held.contains(&None) || held.iter().flatten().any(every_value)
    }
    pub(crate) fn check_delivery(
        &self,
        delivery: &Delivery,
        earliest: SimTime,
    ) -> Result<(), AdmissionError> {
        if delivery.at < earliest {
            return Err(AdmissionError::EventBeforeHead { id: delivery.id });
        }
        if delivery.posture_event() {
            return Ok(());
        }
        let boundaries = &self.layout.schema.boundaries;
        if !(boundaries.iter())
            .any(|known| known.name == delivery.boundary && delivery.carries(known.pool.species))
        {
            return Err(AdmissionError::UnknownBoundary {
                name: delivery.boundary,
            });
        }
        if !self.reads_drive(delivery.boundary) {
            return Err(AdmissionError::DriveNotRead {
                name: delivery.boundary,
            });
        }
        Ok(())
    }
    pub(crate) fn boundary_name(&self, name: &str) -> Option<&'static str> {
        let boundaries = &self.layout.schema.boundaries;
        boundaries
            .iter()
            .find(|known| known.name == name)
            .map(|known| known.name)
    }
    /// Rule 17's other half: `bind` matched the window to the row and the packet, and this matches the method and
    /// window to a reading the body can give. A lens is the one place a method meets a row, and its windows sit on
    /// whole multiples of their length, so a run that starts inside one never reads the beat admission matched.
    fn check_readings(
        &self,
        owed: &Owed,
        moment: SimTime,
        lens: Option<&ObservationLens>,
    ) -> Result<(), AdmissionError> {
        for anchor in &owed.anchors {
            let read = lens.is_some_and(|lens| {
                let (window_ns, points) = match (anchor.statistic, owed.cycle) {
                    (AnchorStatistic::CycleMean, Some(cycle)) => (cycle.period_ns(), cycle.intervals()),
                    _ => (lens.period_ns(), 1),
                };
                lens.model() == self.model_hash()
                    && moment.nanoseconds().is_multiple_of(window_ns)
                    && lens.reads(anchor.method, anchor.row, window_ns, points)
            });
            if !read {
                return Err(AdmissionError::AnchorUnread { anchor: anchor.id });
            }
        }
        Ok(())
    }
    /// The square admission system. Refuses parameters bound on another body, an event `check_delivery` refuses
    /// at the anchor moment, an anchor no channel reads and a delivery edge off the cycle's grid.
    pub fn initialization<'m>(
        &'m self,
        parameters: BoundParameters,
        boundary: BoundaryPort,
        request: InitializationRequest<'_>,
    ) -> Result<InitializationSystem<'m>, AdmissionError> {
        let InitializationRequest { anchor_moment, lens } = request;
        if parameters.model != self.model_hash() {
            return Err(AdmissionError::ForeignParameters);
        }
        for delivery in &boundary.deliveries {
            self.check_delivery(delivery, anchor_moment)?;
        }
        self.check_readings(&parameters.owed, anchor_moment, lens)?;
        let start = anchor_moment.nanoseconds();
        let (instants, interval, fence) = match parameters.owed.cycle {
            None => {
                let fence = anchor_moment.plus_ns(1).unwrap_or(anchor_moment);
                (vec![start], None, fence)
            }
            Some(cycle) => {
                if let Some(id) = boundary.off_grid(anchor_moment, cycle) {
                    return Err(AdmissionError::DeliveryOffGrid { id });
                }
                let step_ns = cycle.step_ns();
                let instants = (0..cycle.intervals()).map(|point| point * step_ns);
                let instants = instants.map(|offset| start.saturating_add(offset)).collect();
                let seconds = step_ns as f64 / NANOSECONDS_PER_SECOND;
                // Saturating fails closed: past the clock's end every late stamp is refused.
                let fence = SimTime::from_ns(start.saturating_add(cycle.period_ns()));
                (instants, Some(seconds), fence)
            }
        };
        let pending = PendingInputs::opening(boundary, fence);
        let frame_at = |ns: &u64| pending.frame_at(SimTime::from_ns(*ns));
        let frames: Vec<DriveFrame> = instants.iter().map(frame_at).collect();
        let tolerance = parameters.owed.tolerance.unwrap_or(ErrorScales::EXACT);
        let modes = vec![parameters.modes.clone(); frames.len()];
        let resets = vec![Vec::new(); frames.len()];
        Ok(InitializationSystem {
            body: self,
            parameters,
            modes,
            resets,
            pending,
            anchor_moment,
            tolerance,
            frames,
            interval,
            draws: vec![HELD_DRAW; self.layout.streams.len()],
            factors: vec![HELD_FACTOR; self.layout.forcings.len()],
        })
    }
}

fn undefined(undefined: Undefined) -> InitializationFailure {
    InitializationFailure::Undefined { law: undefined.law }
}

fn algebraic(kind: RowKind) -> bool {
    matches!(kind, RowKind::Algebraic(_))
}

/// A square system over `N` points of the body's rows and one unknown per fit. Point 0 is the start state.
///
/// In a workspace, point `k` row `i` is slot `k n + i` and fit `j` is slot `N n + j`, for `n` rows. In the
/// residual, the same slots hold the point's row, with a total in one row of the last point under a cycle, and
/// then fit `j`'s anchor row. That layout is the block ring an elimination reads, so a solver for a large body
/// needs no other surface.
///
/// An instant has one point and no ring. A cycle has one backward Euler stage per interval, around the ring, on
/// the surface a core's stage evaluates, so the rows need no second formula and `admit` needs no solver.
///
/// Each point holds its own modes. A guard crossed at point `k` fires on the way to point `k + 1`, as a step that ends
/// there would: the next point's stage holds in what it left and starts from its reset. `sweep` reads that off the
/// workspace, and `admit` runs the same reading and refuses a ring it would change.
pub struct InitializationSystem<'m> {
    pub(crate) body: &'m CompiledBody,
    parameters: BoundParameters,
    modes: Vec<Vec<usize>>,
    resets: Vec<Vec<RowId>>,
    pending: PendingInputs,
    anchor_moment: SimTime,
    tolerance: ErrorScales,
    /// One per point: the drives frozen at the start of the interval it opens, as a ticket's are. The stage at
    /// point `k` reads the interval that ends there.
    frames: Vec<DriveFrame>,
    /// One interval in seconds, and `None` for an instant.
    interval: Option<f64>,
    draws: Vec<f64>,
    factors: Vec<f64>,
}
impl InitializationSystem<'_> {
    fn unknowns(&self) -> usize {
        self.frames.len() * self.body.rows().len() + self.parameters.owed.fits.len()
    }
    fn check(&self, workspace: &SolveWorkspace) -> Result<(), InitializationFailure> {
        let foreign = workspace.layout() != self.body.model_hash().digest()
            || workspace.coordinates().len() != self.unknowns();
        if foreign {
            return Err(AdmissionError::ForeignWorkspace.into());
        }
        Ok(())
    }
    fn point<'u>(&self, unknowns: &'u [f64], point: usize) -> &'u [f64] {
        let rows = self.body.rows().len();
        &unknowns[point * rows..][..rows]
    }
    /// The point before `point`, around the ring. An instant's one point is its own.
    fn before(&self, point: usize) -> usize {
        (point + self.frames.len() - 1) % self.frames.len()
    }
    fn fitted<'u>(&self, unknowns: &'u [f64]) -> &'u [f64] {
        &unknowns[self.frames.len() * self.body.rows().len()..]
    }
    fn values(&self, unknowns: &[f64]) -> Vec<f64> {
        let mut values = self.parameters.values.clone();
        for (fit, &value) in self.parameters.owed.fits.iter().zip(self.fitted(unknowns)) {
            for slot in &fit.slots {
                values[slot.index()] = value;
            }
        }
        values
    }
    fn inputs<'a>(&'a self, point: usize, state: &'a [f64], values: &'a [f64]) -> Inputs<'a> {
        Inputs {
            parameters: values,
            frame: &self.frames[self.before(point)],
            unknowns: state,
            modes: &self.modes[point],
            draws: &self.draws,
            factors: &self.factors,
        }
    }
    fn reset(&self, point: usize, row: usize) -> bool {
        self.resets[point].contains(&RowId::at(row))
    }
    /// The total row of a combination replaces its least row at the last point, because the point rows sum to the
    /// combination's net boundary flow over the cycle, whatever the state: they alone leave its total free. A regulated
    /// total answers its own flows, so its point rows pin it and it needs no row. Charge follows from the ions.
    fn replaced(&self) -> impl Iterator<Item = (usize, QuantityId)> + '_ {
        let last = self.frames.len() - 1;
        let rows = self.body.rows().len();
        let combinations = self.body.conserved_basis().combinations();
        (combinations.iter())
            .filter(|combination| !combination.regulated() && combination.book() != Book::Charge)
            .filter(|_| self.interval.is_some())
            .map(move |combination| (last * rows + combination.terms()[0].0.index(), combination.id()))
    }
    /// A slot's natural size, so a solver reads a small row at its own size. A fit reads as one.
    pub fn size(&self, slot: usize) -> f64 {
        let rows = self.body.rows().len();
        if slot < self.frames.len() * rows {
            self.parameters.sizes[slot % rows]
        } else {
            1.0
        }
    }
    /// How many rows a point holds.
    pub fn rows(&self) -> usize {
        self.body.rows().len()
    }
    /// How many points the ring holds, one for an instant.
    pub fn points(&self) -> usize {
        self.frames.len()
    }
    /// Seeded with the packet's start values at every point and each fit's guess.
    pub fn workspace(&self) -> SolveWorkspace {
        let mut unknowns = Vec::with_capacity(self.unknowns());
        for _ in 0..self.frames.len() {
            unknowns.extend_from_slice(&self.parameters.starts);
        }
        unknowns.extend(self.parameters.owed.fits.iter().map(|fit| fit.guess));
        SolveWorkspace::seeded(self.body, unknowns)
    }
    /// One row per body row at each point, none replaced. Each is in its row's own unit, so one scale reads it:
    /// a stage's rate is multiplied by the interval, as `verify` does. An amount or memory row of an instant is
    /// its value less the packet's.
    fn stages(&self, unknowns: &[f64]) -> Result<Vec<f64>, InitializationFailure> {
        let values = self.values(unknowns);
        let mut out = Vec::with_capacity(self.unknowns());
        for point in 0..self.frames.len() {
            self.stage_rows(unknowns, &values, point, &mut out)?;
        }
        Ok(out)
    }
    fn stage_rows(
        &self,
        unknowns: &[f64],
        values: &[f64],
        point: usize,
        out: &mut Vec<f64>,
    ) -> Result<(), InitializationFailure> {
        let rows = self.body.rows();
        {
            let state = self.point(unknowns, point);
            let before = self.point(unknowns, self.before(point));
            let inputs = self.inputs(point, state, values);
            let laws = self.body.instance_values(&inputs).map_err(undefined)?;
            let (columns, rates) = self.body.targets(&laws);
            let tangent = |seconds: f64| {
                let change = |(at, (now, then)): (usize, (&f64, &f64))| {
                    let then = if self.reset(point, at) { 0.0 } else { *then };
                    (now - then) / seconds
                };
                state
                    .iter()
                    .zip(before)
                    .enumerate()
                    .map(change)
                    .collect::<Vec<f64>>()
            };
            let derivative = self.interval.map_or(vec![0.0; rows.len()], tangent);
            let residuals = self.body.residuals(&columns, &rates, &derivative);
            for (at, row) in rows.iter().enumerate() {
                out.push(match self.interval {
                    _ if algebraic(row.kind()) => residuals[at],
                    Some(seconds) => seconds * residuals[at],
                    None => state[at] - self.parameters.starts[at],
                });
            }
        }
        Ok(())
    }
    /// How far a combination's sum at the start stands from the packet's, and the packet's sum. The first is summed
    /// term by term, so a total too large to hold still reads as unmoved.
    fn total(&self, unknowns: &[f64], quantity: QuantityId) -> (f64, f64) {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let (mut moved, mut packet) = (Compensated::ZERO, Compensated::ZERO);
        for (row, coefficient) in combination.terms() {
            let start = Compensated::exact(self.parameters.starts[row.index()]).scaled(*coefficient);
            let now = Compensated::exact(unknowns[row.index()]).scaled(*coefficient);
            moved = moved.plus(now.minus(start));
            packet = packet.plus(start);
        }
        (moved.rounded(), packet.rounded())
    }
    fn total_size(&self, quantity: QuantityId) -> f64 {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let sizes = combination
            .terms()
            .iter()
            .map(|(row, _)| self.parameters.sizes[row.index()]);
        sizes.fold(0.0, |most, size| if size > most { size } else { most })
    }
    /// One slot per unknown, in the workspace's order: each point's body rows, a total in one row of the last point
    /// under a cycle, then each fit's anchor.
    pub fn evaluate(&self, workspace: &SolveWorkspace) -> Result<Vec<f64>, InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
        let mut out = self.stages(unknowns)?;
        for (slot, quantity) in self.replaced() {
            out[slot] = self.total(unknowns, quantity).0;
        }
        let owed = &self.parameters.owed;
        for fit in &owed.fits {
            let anchor = &owed.anchors[fit.anchor];
            out.push(self.anchor_read(unknowns, anchor) - anchor.value);
        }
        Ok(out)
    }
    /// Over the slots `workspace` holds, a column's `UnknownId::index` is its slot. Only a stage's own tangent and a
    /// total's row couple two points, so the matrix is a block ring with a border of fits.
    pub fn linearize(&self, workspace: &SolveWorkspace) -> Result<Jacobian, InitializationFailure> {
        self.check(workspace)?;
        self.lines(workspace.coordinates())
    }
    fn lines(&self, unknowns: &[f64]) -> Result<Jacobian, InitializationFailure> {
        let rows = self.body.rows();
        let (size, points) = (rows.len(), self.frames.len());
        let values = self.values(unknowns);
        let fits = &self.parameters.owed.fits;
        let shift = self.interval.map_or(0.0, |seconds| 1.0 / seconds);
        let sparsity = self.body.sparsity();
        let mut lines: Vec<Vec<(usize, f64)>> = Vec::with_capacity(self.unknowns());
        for point in 0..points {
            let inputs = self.inputs(point, self.point(unknowns, point), &values);
            let entries = self.body.jacobian(&inputs, shift).map_err(undefined)?;
            let mut slopes = Vec::with_capacity(fits.len());
            for fit in fits {
                slopes.push(
                    self.body
                        .parameter_slopes(&inputs, &fit.slots)
                        .map_err(undefined)?,
                );
            }
            for (at, row) in rows.iter().enumerate() {
                let scale = match self.interval {
                    _ if algebraic(row.kind()) => 1.0,
                    Some(seconds) => seconds,
                    None => {
                        lines.push(vec![(point * size + at, 1.0)]);
                        continue;
                    }
                };
                let reads = sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1];
                let read = |entry: usize| {
                    (
                        point * size + sparsity.unknowns()[entry].index(),
                        scale * entries[entry],
                    )
                };
                let mut line: Vec<(usize, f64)> = reads.map(read).collect();
                if self.interval.is_some() && !algebraic(row.kind()) && !self.reset(point, at) {
                    let then = self.before(point) * size + at;
                    match line.iter_mut().find(|(column, _)| *column == then) {
                        Some((_, value)) => *value -= 1.0,
                        None => line.push((then, -1.0)),
                    }
                }
                for (fit, slope) in slopes.iter().enumerate() {
                    line.push((points * size + fit, scale * slope[at]));
                }
                lines.push(line);
            }
        }
        for (slot, quantity) in self.replaced() {
            let terms = self.body.conserved_basis().combinations()[quantity.index()].terms();
            let term = |(row, coefficient): &(RowId, i64)| (row.index(), *coefficient as f64);
            lines[slot] = terms.iter().map(term).collect();
        }
        for fit in fits {
            let row = self.parameters.owed.anchors[fit.anchor].row.index();
            lines.push(match self.parameters.owed.anchors[fit.anchor].statistic {
                AnchorStatistic::Instant => vec![(row, 1.0)],
                AnchorStatistic::CycleMean => {
                    let share = 1.0 / points as f64;
                    (0..points).map(|point| (point * size + row, share)).collect()
                }
            });
        }
        let mut row_offsets = Vec::with_capacity(lines.len() + 1);
        row_offsets.push(0);
        let (mut columns, mut entries) = (Vec::new(), Vec::new());
        for line in lines {
            for (column, value) in line {
                columns.push(UnknownId::at(column));
                entries.push(value);
            }
            row_offsets.push(columns.len());
        }
        Ok(Jacobian::of_parts(row_offsets, columns, entries))
    }
    /// One point's stage on its own, in the workspace's slots: each row's residual, as `evaluate` gives it, and the block
    /// of the point's own unknowns, whose view has one row and one column per body row. An admission solver marches
    /// the ring with this and `sweep`, to reach a guess Newton can start from. Nothing here is judged: `admit` is.
    pub fn stage(
        &self,
        workspace: &SolveWorkspace,
        point: usize,
    ) -> Result<(Vec<f64>, Jacobian), InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
        let shift = self.interval.map_or(0.0, |seconds| 1.0 / seconds);
        let values = self.values(unknowns);
        let inputs = self.inputs(point, self.point(unknowns, point), &values);
        let mut entries = self.body.jacobian(&inputs, shift).map_err(undefined)?;
        let sparsity = self.body.sparsity();
        if let Some(seconds) = self.interval {
            for (at, row) in self
                .body
                .rows()
                .iter()
                .enumerate()
                .filter(|(_, row)| !algebraic(row.kind()))
            {
                let _ = row;
                entries[sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1]]
                    .iter_mut()
                    .for_each(|entry| *entry *= seconds);
            }
        }
        let block = Jacobian::of_parts(
            sparsity.row_offsets().to_vec(),
            sparsity.unknowns().to_vec(),
            entries,
        );
        let mut rows = Vec::with_capacity(self.body.rows().len());
        self.stage_rows(unknowns, &values, point, &mut rows)?;
        Ok((rows, block))
    }
    /// One point's own block of a `linearize`, in the body's pattern, so a ring elimination factors it over the body's
    /// elimination. A cycle's last point holds totals in place of rows, so its block is not this.
    pub fn block(&self, ring: &Jacobian, point: usize) -> Jacobian {
        let sparsity = self.body.sparsity();
        let size = self.body.rows().len();
        let view = ring.view();
        let mut entries = Vec::with_capacity(sparsity.unknowns().len());
        for row in 0..size {
            let count = sparsity.row_offsets()[row + 1] - sparsity.row_offsets()[row];
            let start = view.row_offsets()[point * size + row];
            entries.extend_from_slice(&view.values()[start..start + count]);
        }
        Jacobian::of_parts(
            sparsity.row_offsets().to_vec(),
            sparsity.unknowns().to_vec(),
            entries,
        )
    }
    /// Reads, from each point in `points`, the modes and resets the next point starts with: every armed guard crossed
    /// there fires, and the point after it holds what that left. Returns how many points changed, so a solver alternates
    /// it with Newton until it returns zero. It reads the same surface `admit` does, on the workspace alone.
    pub fn sweep(
        &mut self,
        workspace: &SolveWorkspace,
        points: Range<usize>,
    ) -> Result<usize, InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
        let mut changed = 0;
        for point in points {
            let (modes, resets) = self.next(unknowns, point)?;
            let next = (point + 1) % self.frames.len();
            if modes != self.modes[next] || resets != self.resets[next] {
                changed += 1;
                self.modes[next] = modes;
                self.resets[next] = resets;
            }
        }
        Ok(changed)
    }
    /// Puts point 0 back on the packet's modes, as if every guard that enters them fired on the way there. A pass of a
    /// march ends where its guess leaves it, so the next pass starts the cycle where the packet says it starts.
    pub fn restart(&mut self) {
        let last = &self.modes[self.modes.len() - 1];
        let start = &self.parameters.modes;
        let entering = (self.body.layout.guards.iter())
            .filter(|guard| start[guard.variable] == guard.enters && last[guard.variable] == guard.leaves);
        self.resets[0] = entering.filter_map(|guard| guard.resets).collect();
        self.modes[0].clone_from(start);
    }
    fn next(
        &self,
        unknowns: &[f64],
        point: usize,
    ) -> Result<(Vec<usize>, Vec<RowId>), InitializationFailure> {
        let values = self.values(unknowns);
        let inputs = self.inputs(point, self.point(unknowns, point), &values);
        self.body.fired_at(&inputs, &self.tolerance).map_err(undefined)
    }
    /// The first row whose stage has no pivot at the start state: a coupling loop such as `x = y`, `y = x`, a pressure
    /// with no gauge, or a closed row where no cycle gives its tangent. Structural matching cannot see these, and a
    /// run from a consistent start would hold whatever the packet gave them. One dense elimination, once a run.
    fn rankless(&self, unknowns: &[f64]) -> Result<Option<RowId>, InitializationFailure> {
        let size = self.body.rows().len();
        let shift = self.interval.map_or(0.0, |seconds| 1.0 / seconds);
        let values = self.values(unknowns);
        let inputs = self.inputs(0, self.point(unknowns, 0), &values);
        // A law with no slope here says nothing of rank. The first stage that needs one refuses it.
        let Ok(entries) = self.body.jacobian(&inputs, shift) else {
            return Ok(None);
        };
        let sparsity = self.body.sparsity();
        let mut matrix = vec![vec![0.0; size]; size];
        for (row, cells) in matrix.iter_mut().enumerate() {
            for entry in sparsity.row_offsets()[row]..sparsity.row_offsets()[row + 1] {
                cells[sparsity.unknowns()[entry].index()] = entries[entry];
            }
        }
        // At an instant an amount or a memory row states the packet's value, so it pivots on itself.
        if self.interval.is_none() {
            for (at, row) in self.body.rows().iter().enumerate() {
                if !algebraic(row.kind()) {
                    matrix[at].fill(0.0);
                    matrix[at][at] = 1.0;
                }
            }
        }
        for row in &mut matrix {
            row.push(0.0);
        }
        Ok(eliminate(matrix).err().map(RowId::at))
    }
    /// One defect over its scale, `None` when the tolerance refuses it. What overflowed is refused, because `<=` is
    /// false on a NaN but true of two infinities, and a defect of zero holds even where an overflow leaves no scale.
    fn held(&self, defect: f64, read: f64, size: f64) -> Option<f64> {
        let scale = self.tolerance.at(read, size);
        let within = defect == 0.0 || (defect.is_finite() && defect <= scale);
        within.then(|| if scale > 0.0 { defect / scale } else { 0.0 })
    }
    /// An instant reads its row at point 0. A cycle mean reads its row's mean over the points, the quadrature the
    /// ring's own sum closes, so a linear law's mean is exact at any point count. It sums as an observer sums a
    /// window, so the two read one number however far the row swings.
    fn anchor_read(&self, unknowns: &[f64], anchor: &BoundAnchor) -> f64 {
        let row = anchor.row.index();
        match anchor.statistic {
            AnchorStatistic::Instant => unknowns[row],
            AnchorStatistic::CycleMean => {
                let points = self.frames.len();
                let read = |point| Compensated::exact(self.point(unknowns, point)[row]);
                let sum = (0..points).map(read).fold(Compensated::ZERO, Compensated::plus);
                Compensated::between(Compensated::ZERO, sum, 1, points as u64).rounded()
            }
        }
    }
    /// A combination's net boundary flow over the cycle, one column at a time as the ledger books it, whichever rows
    /// the workspace spread it over. A row's own sum can lose a small flow beside large ones, and the ledger keeps it.
    fn net_flow(&self, unknowns: &[f64], quantity: QuantityId) -> Result<f64, InitializationFailure> {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let crossing = |terms: &[(RowId, i64)]| match *terms {
            [(row, sign)] => (combination.terms().iter())
                .filter(|(known, _)| *known == row)
                .map(|(_, coefficient)| sign * coefficient)
                .sum(),
            _ => 0,
        };
        let seconds = self.interval.unwrap_or(1.0);
        let values = self.values(unknowns);
        let mut net = Compensated::ZERO;
        for point in 0..self.frames.len() {
            let inputs = self.inputs(point, self.point(unknowns, point), &values);
            let laws = self.body.instance_values(&inputs).map_err(undefined)?;
            let (rates, _) = self.body.targets(&laws);
            for (column, rate) in self.body.columns().iter().zip(rates) {
                net = net.plus(Compensated::exact(seconds * rate).scaled(crossing(&column.terms)));
            }
        }
        Ok(net.rounded())
    }
    /// Every row of the system again, on this workspace alone: no core, no solver and nothing the initializer
    /// said. The first failure in this order names its row, so a test can name it too. Returns the row nearest
    /// the tolerance, with its defect over its scale.
    fn recheck(&self, unknowns: &[f64]) -> Result<Option<(AdmissionRow, f64)>, InitializationFailure> {
        let rows = self.body.rows();
        let inconsistent = |row| InitializationFailure::Inconsistent { row };
        let body_row = |at: usize| AdmissionRow::Body(rows[at].id());
        let size = rows.len();
        for (slot, &value) in unknowns[..self.frames.len() * size].iter().enumerate() {
            if !rows[slot % size].admissible().holds(Compensated::exact(value)) {
                return Err(inconsistent(body_row(slot % size)));
            }
        }
        let owed = &self.parameters.owed;
        for (fit, &value) in owed.fits.iter().zip(self.fitted(unknowns)) {
            let slots = fit.slots.iter().map(|slot| &self.body.parameters()[slot.index()]);
            if !slots
                .into_iter()
                .all(|slot| slot.admissible().holds(Compensated::exact(value)))
            {
                let (key, binding) = (fit.key, fit.binding);
                return Err(AdmissionError::ParameterOutsideSet { key, binding }.into());
            }
        }
        let mut worst: Option<(AdmissionRow, f64)> = None;
        let mut note = |row, normalized: f64| {
            if worst.is_none_or(|(_, known)| known < normalized) {
                worst = Some((row, normalized));
            }
        };
        let stages = self.stages(unknowns)?;
        let sizes = &self.parameters.sizes;
        let stage = |slot: usize| {
            let held = self.held(stages[slot].abs(), unknowns[slot], sizes[slot % size]);
            held.ok_or(inconsistent(body_row(slot % size)))
        };
        let replaced = |slot: &usize| self.replaced().any(|(known, _)| known == *slot);
        for slot in (0..stages.len()).filter(|slot| !replaced(slot)) {
            note(body_row(slot % size), stage(slot)?);
        }
        // Every conserved total but charge, regulated or not: each stage's defect can pass while their sum over the
        // cycle does not. A net flow lands on the row its total replaced, so the flow is read first, to name it.
        let combinations = self.body.conserved_basis().combinations();
        let books = (combinations.iter()).filter(|combination| combination.book() != Book::Charge);
        for quantity in books.filter(|_| self.interval.is_some()).map(Combination::id) {
            let (moved, packet) = self.total(unknowns, quantity);
            let flow = self.net_flow(unknowns, quantity)?.abs();
            let held = self.held(flow, packet + moved, self.total_size(quantity));
            note(
                AdmissionRow::Total(quantity),
                held.ok_or(InitializationFailure::NetBoundaryFlow { quantity })?,
            );
        }
        for (slot, _) in self.replaced() {
            note(body_row(slot % size), stage(slot)?);
        }
        let specified = (self.body.conserved_basis().combinations().iter())
            .filter(|combination| !combination.regulated() && combination.book() != Book::Charge);
        for quantity in specified.map(|combination| combination.id()) {
            let row = AdmissionRow::Total(quantity);
            let (moved, packet) = self.total(unknowns, quantity);
            let bulk = self.total_size(quantity);
            note(
                row,
                self.held(moved.abs(), packet, bulk).ok_or(inconsistent(row))?,
            );
        }
        for anchor in &owed.anchors {
            let row = AdmissionRow::Anchor(anchor.id);
            let held = self.held(
                (self.anchor_read(unknowns, anchor) - anchor.value).abs(),
                anchor.value,
                sizes[anchor.row.index()],
            );
            note(row, held.ok_or(inconsistent(row))?);
        }
        for (fit, &value) in owed.fits.iter().zip(self.fitted(unknowns)) {
            if !self.felt(unknowns, fit, value)? {
                let (key, binding) = (fit.key, fit.binding);
                return Err(AdmissionError::FitUnread { key, binding }.into());
            }
        }
        for point in 0..self.frames.len() {
            let next = (point + 1) % self.frames.len();
            let (modes, resets) = self.next(unknowns, point)?;
            if modes != self.modes[next] || resets != self.resets[next] {
                return Err(InitializationFailure::GuardSweep { point: next });
            }
        }
        if let Some(row) = self.rankless(unknowns)? {
            return Err(InitializationFailure::NoRank { row });
        }
        // One stage can have rank while the ring has a free level: a total a law seems to regulate but does not.
        if self.frames.len() > 1 {
            let ring = self.lines(unknowns)?;
            if let Err(row) = self.solve(&ring, &vec![0.0; unknowns.len()]) {
                let row = rows[row % size].id();
                return Err(InitializationFailure::NoRank { row });
            }
        }
        Ok(worst)
    }
    /// Whether a row behind this fit's anchor feels it anywhere on this workspace: the fit's value, through its slope,
    /// moves that row's stage past rounding on the row's own value.
    fn felt(&self, unknowns: &[f64], fit: &Fit, value: f64) -> Result<bool, InitializationFailure> {
        let rows = self.body.rows();
        let values = self.values(unknowns);
        for point in 0..self.frames.len() {
            let state = self.point(unknowns, point);
            let inputs = self.inputs(point, state, &values);
            let slopes = (self.body.parameter_slopes(&inputs, &fit.slots)).map_err(undefined)?;
            let felt = |row: &RowId| {
                let at = row.index();
                let interval = self.interval.filter(|_| !algebraic(rows[at].kind()));
                (interval.unwrap_or(1.0) * slopes[at] * value).abs() > ROUNDING * state[at].abs()
            };
            if fit.rows.iter().any(felt) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// The only constructor of `InitialRevision`. A workspace from another body or of another shape is refused.
    /// Point 0 becomes the start state and each fit lands in `BoundParameters::values`, which the chain's first link
    /// seals, so a fit is bound once and a resume never refits. The other points end here: they were this cycle's
    /// witness.
    pub fn admit(
        self,
        solved: SolveWorkspace,
        history: MethodHistory,
    ) -> Result<InitialRevision, InitializationFailure> {
        self.check(&solved)?;
        let unknowns = solved.coordinates();
        let worst = self.recheck(unknowns)?;
        let values = self.values(unknowns);
        let start = self.point(unknowns, 0);
        let fitted = self.fitted(unknowns);
        let Self {
            body,
            mut parameters,
            mut modes,
            resets: _,
            pending,
            anchor_moment: time,
            tolerance,
            frames: _,
            interval: _,
            draws: _,
            factors: _,
        } = self;
        parameters.packet = parameters.packet.reading(time, start, fitted, &history);
        parameters.values = values;
        parameters.modes = modes.swap_remove(0);
        let owed = core::mem::take(&mut parameters.owed);
        let value_of = |entry: &(ValueName, ValueProvenance, f64)| {
            let (name, provenance, value) = *entry;
            let fit = (owed.fits.iter()).position(|fit| name == fit.name());
            ValueRecord {
                name,
                provenance,
                value: Compensated::exact(fit.map_or(value, |at| fitted[at])),
            }
        };
        let start_of = |((row, &value), &guess): ((&Row, &f64), &f64)| {
            let moved = value.to_bits() != guess.to_bits();
            ValueRecord {
                name: ValueName::Start(row.kind()),
                provenance: if moved {
                    ValueProvenance::Solved
                } else {
                    ValueProvenance::Fixture
                },
                value: Compensated::exact(value),
            }
        };
        let mode_of = |(mode, &value): (&ModeLayout, &usize)| ValueRecord {
            name: ValueName::Mode {
                variable: mode.variable,
                value: mode.values[value],
            },
            provenance: ValueProvenance::Fixture,
            value: Compensated::exact(value as f64),
        };
        let starts = body.rows().iter().zip(start).zip(&parameters.starts);
        let modes = body.layout.modes.iter().zip(&parameters.modes);
        let records = (owed.entries.iter().map(value_of))
            .chain(starts.map(start_of))
            .chain(modes.map(mode_of));
        let provenance = InitializationProvenance {
            values: records.collect(),
            tolerance,
            worst: worst.map(|(row, normalized)| (row, Compensated::exact(normalized))),
        };
        Ok(InitialRevision {
            parts: ImageParts {
                time,
                state: Candidate::of_coordinates(body, start, parameters.modes.clone()),
                continuation: history,
                dense: DenseSegment::empty(),
                pending,
                parameters,
            },
            provenance,
        })
    }
}

pub struct InitialRevision {
    parts: ImageParts,
    provenance: InitializationProvenance,
}
impl InitialRevision {
    pub fn packet_hash(&self) -> PacketHash {
        self.parts.parameters.packet
    }
    pub fn model_hash(&self) -> ModelHash {
        self.parts.parameters.model
    }
    pub fn provenance(&self) -> &InitializationProvenance {
        &self.provenance
    }
    pub fn into_parts(self) -> ImageParts {
        self.parts
    }
}

/// What a refusal names: a body row, a conserved total or an anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionRow {
    Body(RowId),
    Total(QuantityId),
    Anchor(AnchorId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitializationFailure {
    Admission(AdmissionError),
    /// A row the solved workspace leaves inconsistent with the packet, its set or the tolerance.
    Inconsistent {
        row: AdmissionRow,
    },
    /// A law the solved workspace leaves without a value, such as a divisor at zero.
    Undefined {
        law: LawName,
    },
    /// A conserved combination whose boundary flows do not net to zero over the cycle: no state returns.
    NetBoundaryFlow {
        quantity: QuantityId,
    },
    /// A guard that crossed on the way to this point and not in the modes the workspace was solved in.
    GuardSweep {
        point: usize,
    },
    /// A row with no pivot at the start state, in its own stage or on the ring, which structural matching cannot see.
    NoRank {
        row: RowId,
    },
}
impl From<AdmissionError> for InitializationFailure {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}
