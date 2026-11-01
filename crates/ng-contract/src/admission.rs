use crate::body::{Bound, ModeLayout};
use crate::boundary::{Delivery, DriveFrame, PendingInputs};
use crate::chance::{HELD_DRAW, HELD_FACTOR};
use crate::evaluate::{Inputs, Undefined};
use crate::packet::{BoundAnchor, Owed};
use crate::problem::{Jacobian, SolveWorkspace};
use crate::{
    AdmissionError, AnchorId, AnchorStatistic, BoundParameters, BoundaryPort, Candidate, Compensated,
    CompiledBody, DenseSegment, ErrorScales, ImageParts, InitializationProvenance, LawName, MethodHistory,
    ModelHash, ObservationLens, PacketHash, QuantityId, Row, RowId, RowKind, SimTime, UnknownId, ValueName,
    ValueProvenance, ValueRecord,
};

const NANOSECONDS_PER_SECOND: f64 = 1e9;

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
    fn reads_drive(&self, boundary: &str) -> bool {
        (self.layout.instances.iter().flat_map(|instance| &instance.inputs))
            .any(|input| matches!(input, Bound::Boundary(name) if *name == boundary))
    }
    pub(crate) fn check_delivery(
        &self,
        delivery: &Delivery,
        earliest: SimTime,
    ) -> Result<(), AdmissionError> {
        let boundaries = &self.layout.schema.boundaries;
        if !boundaries.iter().any(|known| known.name == delivery.boundary) {
            return Err(AdmissionError::UnknownBoundary {
                name: delivery.boundary,
            });
        }
        if delivery.at < earliest {
            return Err(AdmissionError::EventBeforeHead { id: delivery.id });
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
    /// window to a reading the body can give. A lens is the one place a method meets a row.
    fn check_readings(&self, owed: &Owed, lens: Option<&ObservationLens>) -> Result<(), AdmissionError> {
        for anchor in &owed.anchors {
            let read = lens.is_some_and(|lens| {
                let (window_ns, samples) = match (anchor.statistic, owed.cycle) {
                    (AnchorStatistic::CycleMean, Some(cycle)) => (cycle.period_ns(), cycle.intervals()),
                    _ => (lens.period_ns(), 1),
                };
                lens.model() == self.model_hash() && lens.reads(anchor.method, anchor.row, window_ns, samples)
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
        mut parameters: BoundParameters,
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
        self.check_readings(&parameters.owed, lens)?;
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
        parameters.packet = parameters.packet.reading(&frames);
        let tolerance = parameters.owed.tolerance.unwrap_or(ErrorScales::EXACT);
        Ok(InitializationSystem {
            body: self,
            parameters,
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

/// A square system over `N` points of the body's rows and one unknown per fit. Node 0 is the start state.
///
/// In a workspace, point `k` row `i` is slot `k n + i` and fit `j` is slot `N n + j`, for `n` rows. In the
/// residual, the same slots hold the point's row and then fit `j`'s anchor row. That layout is the band a block
/// elimination reads, so a solver for a large body needs no other surface.
///
/// An instant has one point and no ring. A cycle has one backward Euler stage per interval, around the ring, on
/// the surface a core's stage evaluates, so the rows need no second formula and `admit` needs no solver.
pub struct InitializationSystem<'m> {
    body: &'m CompiledBody,
    parameters: BoundParameters,
    pending: PendingInputs,
    anchor_moment: SimTime,
    tolerance: ErrorScales,
    /// One per interval: the drives frozen at its start, as a ticket's are. The stage at point `k` reads the
    /// interval that ends there.
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
    /// The bound values with each fit's unknown in every slot it fills.
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
            modes: &self.parameters.modes,
            draws: &self.draws,
            factors: &self.factors,
        }
    }
    /// The total row of a combination replaces its least row at the last point, because the point rows sum to the
    /// combination's net boundary flow over the cycle, whatever the state: they alone leave its total free.
    fn replaced(&self) -> impl Iterator<Item = (usize, QuantityId)> + '_ {
        let last = self.frames.len() - 1;
        let rows = self.body.rows().len();
        let combinations = self.body.conserved_basis().combinations();
        (combinations.iter())
            .filter(|_| self.interval.is_some())
            .map(move |combination| (last * rows + combination.terms()[0].0.index(), combination.id()))
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
        let rows = self.body.rows();
        let values = self.values(unknowns);
        let mut out = Vec::with_capacity(self.unknowns());
        for point in 0..self.frames.len() {
            let state = self.point(unknowns, point);
            let before = self.point(unknowns, self.before(point));
            let inputs = self.inputs(point, state, &values);
            let laws = self.body.instance_values(&inputs).map_err(undefined)?;
            let (columns, rates) = self.body.targets(&laws);
            let tangent = |seconds: f64| {
                let changes = state.iter().zip(before).map(|(now, then)| (now - then) / seconds);
                changes.collect::<Vec<f64>>()
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
        Ok(out)
    }
    /// A combination's sum at the start, in the workspace and in the packet.
    fn totals(&self, unknowns: &[f64], quantity: QuantityId) -> (Compensated, Compensated) {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let mut sums = (Compensated::ZERO, Compensated::ZERO);
        for (row, coefficient) in combination.terms() {
            let term = |value: f64| Compensated::exact(value).scaled(*coefficient);
            sums.0 = sums.0.plus(term(unknowns[row.index()]));
            sums.1 = sums.1.plus(term(self.parameters.starts[row.index()]));
        }
        sums
    }
    /// One slot per unknown, in the workspace's order: a body row or a total at each point, then each fit's anchor.
    pub fn evaluate(&self, workspace: &SolveWorkspace) -> Result<Vec<f64>, InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
        let mut out = self.stages(unknowns)?;
        for (slot, quantity) in self.replaced() {
            let (now, packet) = self.totals(unknowns, quantity);
            out[slot] = now.minus(packet).rounded();
        }
        let owed = &self.parameters.owed;
        for fit in &owed.fits {
            let anchor = &owed.anchors[fit.anchor];
            out.push(self.anchor_read(unknowns, anchor) - anchor.value);
        }
        Ok(out)
    }
    /// Over the slots `workspace` holds, a column's `UnknownId::index` is its slot. Nothing but a stage's own
    /// tangent couples two points, so the matrix is a block ring with a border of fits.
    pub fn linearize(&self, workspace: &SolveWorkspace) -> Result<Jacobian, InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
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
                if self.interval.is_some() && !algebraic(row.kind()) {
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
    /// One defect over its scale, `None` when the tolerance refuses it. `<=` is false on a NaN, so what overflowed
    /// is refused, and an exact packet's scale of zero passes only a defect of zero.
    fn held(&self, defect: f64, read: f64) -> Option<f64> {
        let scale = self.tolerance.at(read);
        (defect <= scale).then(|| if scale > 0.0 { defect / scale } else { 0.0 })
    }
    /// Whether `admit` would pass this workspace, so a solve stops where the judge would, and a guess that already
    /// holds stands.
    pub fn holds(&self, workspace: &SolveWorkspace) -> bool {
        self.check(workspace).is_ok() && self.recheck(workspace.coordinates()).is_ok()
    }
    /// An instant reads its row at point 0. A cycle mean reads its row's mean over the points, the quadrature the
    /// ring's own sum closes, so a linear law's mean is exact at any point count.
    fn anchor_read(&self, unknowns: &[f64], anchor: &BoundAnchor) -> f64 {
        let row = anchor.row.index();
        match anchor.statistic {
            AnchorStatistic::Instant => unknowns[row],
            AnchorStatistic::CycleMean => {
                let points = self.frames.len();
                let read = |point| self.point(unknowns, point)[row];
                (0..points).map(read).sum::<f64>() / points as f64
            }
        }
    }
    /// A combination's rows summed around the ring. The tangents cancel, so this is its net boundary flow over the
    /// cycle, whichever rows the workspace spread it over.
    fn net_flow(&self, stages: &[f64], quantity: QuantityId) -> f64 {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let size = self.body.rows().len();
        let mut net = Compensated::ZERO;
        for point in 0..self.frames.len() {
            for (row, coefficient) in combination.terms() {
                let stage = stages[point * size + row.index()];
                net = net.plus(Compensated::exact(stage).scaled(*coefficient));
            }
        }
        net.rounded()
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
        for (_, quantity) in self.replaced() {
            let row = AdmissionRow::Total(quantity);
            let (now, packet) = self.totals(unknowns, quantity);
            let moved = self.held(now.minus(packet).abs().rounded(), packet.rounded());
            note(row, moved.ok_or(inconsistent(row))?);
            let flow = self.held(self.net_flow(&stages, quantity).abs(), packet.rounded());
            note(
                row,
                flow.ok_or(InitializationFailure::NetBoundaryFlow { quantity })?,
            );
        }
        for (slot, &defect) in stages.iter().enumerate() {
            let row = body_row(slot % size);
            let held = self.held(defect.abs(), unknowns[slot]);
            note(row, held.ok_or(inconsistent(row))?);
        }
        for anchor in &owed.anchors {
            let row = AdmissionRow::Anchor(anchor.id);
            let held = self.held(
                (self.anchor_read(unknowns, anchor) - anchor.value).abs(),
                anchor.value,
            );
            note(row, held.ok_or(inconsistent(row))?);
        }
        Ok(worst)
    }
    /// The only constructor of `InitialRevision`. A workspace from another system is refused. Node 0 becomes the
    /// start state and each fit lands in `BoundParameters::values`, which the chain's first link seals, so a fit is
    /// bound once and a resume never refits. The other points end here: they were this cycle's witness.
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
            pending,
            anchor_moment: time,
            tolerance,
            frames: _,
            interval: _,
            draws: _,
            factors: _,
        } = self;
        parameters.values = values;
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
}
impl From<AdmissionError> for InitializationFailure {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}
