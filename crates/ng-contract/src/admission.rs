use crate::body::{Bound, Instance, ModeLayout, Target};
use crate::boundary::{Delivery, DriveFrame, PendingInputs};
use crate::chance::{HELD_DRAW, HELD_FACTOR};
use crate::elimination::{Elimination, Factors};
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
/// Two-stage Radau IIA, the one tableau of a cycle's interval (Hairer and Wanner, *Solving Ordinary Differential
/// Equations II*, 2nd ed., 1996, section IV.5, Radau IIA of order 3): stage `j` of an interval of length `h` from `P`
/// holds `Y_j = P + h sum_k A[j][k] F(Y_k)`, the interior stage at a third of the interval and the end stage at its
/// end. It damps a stiff row to zero, and the end stage is the interval's end, so its row is the weights.
const RADAU: [[f64; 2]; 2] = [[5.0 / 12.0, -1.0 / 12.0], [3.0 / 4.0, 1.0 / 4.0]];
const INTERIOR: usize = 0;
const END: usize = 1;

/// When the start stands and what reads the anchors. The anchors themselves and the tolerance are the
/// packet's.
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
            // A drain's law holds its flow row, and the columns read that.
            Target::Row(row) => matches!(
                self.rows()[row.index()].kind(),
                RowKind::Algebraic(slot) if slot.law.local == boundary
            ),
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
        let staged = |row: usize| {
            let hazard = (self.layout.hazards.iter()).any(|hazard| hazard.integral.index() == row);
            !hazard && !algebraic(self.rows()[row].kind())
        };
        let pattern = interval.map(|_| StagePattern::radau(self, staged));
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
            pattern,
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

/// One interval's two stages as a block of a cycle's matrix: end rows then interior rows, over end then interior
/// unknowns. Each stage keeps the body's pivots, so a closed store stays paired with the power that holds it, and an
/// interval factors over a plan made once, as a body's stage does.
struct StagePattern {
    offsets: Vec<usize>,
    unknowns: Vec<UnknownId>,
    plan: Elimination,
}
impl StagePattern {
    fn radau(body: &CompiledBody, staged: impl Fn(usize) -> bool) -> Self {
        let sparsity = body.sparsity();
        let size = body.rows().len();
        let (mut offsets, mut unknowns) = (vec![0], Vec::new());
        for (own, other) in [(0, size), (size, 0)] {
            for row in 0..size {
                let reads =
                    &sparsity.unknowns()[sparsity.row_offsets()[row]..sparsity.row_offsets()[row + 1]];
                let shifted = |by: usize| {
                    reads
                        .iter()
                        .map(move |unknown| UnknownId::at(by + unknown.index()))
                };
                unknowns.extend(shifted(own));
                if staged(row) {
                    unknowns.extend(shifted(other));
                }
                offsets.push(unknowns.len());
            }
        }
        let pivots = body.pivots();
        let lifted: Vec<RowId> = (0..2 * size)
            .map(|unknown| RowId::at(unknown / size * size + pivots[unknown % size].index()))
            .collect();
        let plan = Elimination::plan(&offsets, &unknowns, &lifted);
        Self {
            offsets,
            unknowns,
            plan,
        }
    }
}

/// Adds `value` at `column`, beside any entry the line already holds there.
fn add(line: &mut Vec<(usize, f64)>, column: usize, value: f64) {
    match line.iter_mut().find(|(known, _)| *known == column) {
        Some((_, held)) => *held += value,
        None => line.push((column, value)),
    }
}

/// A square system over the body's rows at each stage of `N` intervals and one unknown per fit. Point 0, the end of
/// the interval that closes the ring, is the start state.
///
/// In a workspace, interval `k` holds its end stage, point `k`, in slots `2 k n` to `2 k n + n` and its interior stage
/// in the next `n`, for `n` rows, and fit `j` is slot `2 N n + j`; `endpoint` and `interior` name them. In the
/// residual, the same slots hold each stage's rows, with a total in place of one end row of the last interval under a
/// cycle, and then fit `j`'s anchor row. That layout is the block ring an elimination reads, so a solver for a large
/// body needs no other surface.
///
/// An instant has one point, its only stage, and no ring. A cycle's interval is two Radau IIA stages from the point
/// before it, on the rates a core's stage evaluates, so the rows need no second formula and `admit` needs no solver.
///
/// Each interval holds its own modes, at both stages. A guard crossed at point `k` fires on the way to point `k + 1`, as
/// a step that ends there would: the next interval holds in what it left and starts from its reset. `sweep` reads that
/// off the workspace, and `admit` runs the same reading and refuses a ring it would change.
pub struct InitializationSystem<'m> {
    pub(crate) body: &'m CompiledBody,
    parameters: BoundParameters,
    modes: Vec<Vec<usize>>,
    resets: Vec<Vec<RowId>>,
    pending: PendingInputs,
    anchor_moment: SimTime,
    tolerance: ErrorScales,
    /// One per point: the drives frozen at the start of the interval it opens, as a ticket's are. The interval ending
    /// at point `k` reads the frame of the point before it at both its stages.
    frames: Vec<DriveFrame>,
    /// One interval in seconds, and `None` for an instant.
    interval: Option<f64>,
    /// A cycle's interval block, and `None` for an instant, whose one stage is the body's.
    pattern: Option<StagePattern>,
    draws: Vec<f64>,
    factors: Vec<f64>,
}
impl InitializationSystem<'_> {
    /// Slots per interval: both stages under a cycle, the one point of an instant.
    pub(crate) fn width(&self) -> usize {
        let rows = self.body.rows().len();
        if self.interval.is_some() { 2 * rows } else { rows }
    }
    fn stage_list(&self) -> &'static [usize] {
        if self.interval.is_some() {
            &[END, INTERIOR]
        } else {
            &[END]
        }
    }
    fn slots(&self, point: usize, stage: usize) -> Range<usize> {
        let rows = self.body.rows().len();
        let start = point * self.width() + if stage == END { 0 } else { rows };
        start..start + rows
    }
    fn unknowns(&self) -> usize {
        self.frames.len() * self.width() + self.parameters.owed.fits.len()
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
        &unknowns[self.slots(point, END)]
    }
    fn before(&self, point: usize) -> usize {
        (point + self.frames.len() - 1) % self.frames.len()
    }
    fn fitted<'u>(&self, unknowns: &'u [f64]) -> &'u [f64] {
        &unknowns[self.frames.len() * self.width()..]
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
    /// The interval an amount or memory row's stage spans, and `None` for a row that states the packet's value: every
    /// such row of an instant, and a hazard integral, which only grows until it fires, so it never returns over a cycle
    /// and accrues from the anchor moment.
    fn stage_interval(&self, row: usize) -> Option<f64> {
        let hazard = (self.body.layout.hazards.iter()).any(|hazard| hazard.integral.index() == row);
        self.interval.filter(|_| !hazard)
    }
    /// The total row of a combination replaces its least row's end stage in the last interval, because the end rows sum
    /// to the combination's net boundary flow over the cycle, whatever the state: they alone leave its total free. A
    /// regulated total answers its own flows, so its rows pin it and it needs no row. Charge follows from the ions.
    fn replaced(&self) -> impl Iterator<Item = (usize, QuantityId)> + '_ {
        let last = self.frames.len() - 1;
        let combinations = self.body.conserved_basis().combinations();
        (combinations.iter())
            .filter(|combination| !combination.regulated() && combination.book() != Book::Charge)
            .filter(|_| self.interval.is_some())
            .map(move |combination| {
                let row = combination.terms()[0].0.index();
                (self.slots(last, END).start + row, combination.id())
            })
    }
    /// A slot's natural size, so a solver reads a small row at its own size. A fit reads as one.
    pub fn size(&self, slot: usize) -> f64 {
        let rows = self.body.rows().len();
        if slot < self.frames.len() * self.width() {
            self.parameters.sizes[slot % rows]
        } else {
            1.0
        }
    }
    /// How many rows a stage holds.
    pub fn rows(&self) -> usize {
        self.body.rows().len()
    }
    /// How many points the ring holds, one for an instant.
    pub fn points(&self) -> usize {
        self.frames.len()
    }
    /// The slots of the state at `point`: the end stage of the interval that ends there.
    pub fn endpoint(&self, point: usize) -> Range<usize> {
        self.slots(point, END)
    }
    /// The slots of the interior stage of the interval that ends at `point`, empty at an instant.
    pub fn interior(&self, point: usize) -> Range<usize> {
        if self.interval.is_some() {
            self.slots(point, INTERIOR)
        } else {
            let end = self.slots(point, END).end;
            end..end
        }
    }
    /// Every slot `stage` solves for the interval ending at `point`: its end stage, then its interior one.
    pub fn interval_slots(&self, point: usize) -> Range<usize> {
        point * self.width()..(point + 1) * self.width()
    }
    /// The fits' slots, after every stage.
    pub fn fitted_slots(&self) -> Range<usize> {
        self.frames.len() * self.width()..self.unknowns()
    }
    /// Seeded with the packet's start values at every stage and each fit's guess.
    pub fn workspace(&self) -> SolveWorkspace {
        let mut unknowns = Vec::with_capacity(self.unknowns());
        for _ in 0..self.frames.len() * self.stage_list().len() {
            unknowns.extend_from_slice(&self.parameters.starts);
        }
        unknowns.extend(self.parameters.owed.fits.iter().map(|fit| fit.guess));
        SolveWorkspace::seeded(self.body, unknowns)
    }
    /// One row per body row at each stage, none replaced. Each is in its row's own unit, so one scale reads it: a
    /// stage's rates are multiplied by the interval, as `verify` does. A row that states the packet's value is its value
    /// less the packet's.
    fn stages(&self, unknowns: &[f64]) -> Result<Vec<f64>, InitializationFailure> {
        let values = self.values(unknowns);
        let mut out = Vec::with_capacity(self.unknowns());
        for point in 0..self.frames.len() {
            self.stage_rows(unknowns, &values, point, &mut out)?;
        }
        Ok(out)
    }
    /// Each stage's residual with no tangent: an amount's or memory's rate negated, an algebraic row's own value.
    fn rates(
        &self,
        unknowns: &[f64],
        values: &[f64],
        point: usize,
    ) -> Result<[Vec<f64>; 2], InitializationFailure> {
        let mut rates = [Vec::new(), Vec::new()];
        let still = vec![0.0; self.body.rows().len()];
        for &stage in self.stage_list() {
            let inputs = self.inputs(point, &unknowns[self.slots(point, stage)], values);
            let laws = self.body.instance_values(&inputs).map_err(undefined)?;
            let (columns, rows) = self.body.targets(&laws);
            rates[stage] = self.body.residuals(&columns, &rows, &still);
        }
        Ok(rates)
    }
    fn stage_rows(
        &self,
        unknowns: &[f64],
        values: &[f64],
        point: usize,
        out: &mut Vec<f64>,
    ) -> Result<(), InitializationFailure> {
        let rates = self.rates(unknowns, values, point)?;
        let before = self.point(unknowns, self.before(point));
        for &stage in self.stage_list() {
            let state = &unknowns[self.slots(point, stage)];
            for (at, row) in self.body.rows().iter().enumerate() {
                out.push(match self.stage_interval(at) {
                    _ if algebraic(row.kind()) => rates[stage][at],
                    Some(seconds) => {
                        let then = if self.reset(point, at) { 0.0 } else { before[at] };
                        let [interior, end] = RADAU[stage];
                        (state[at] - then) + seconds * (interior * rates[INTERIOR][at] + end * rates[END][at])
                    }
                    None => state[at] - self.parameters.starts[at],
                });
            }
        }
        Ok(())
    }
    /// Each row of one interval's block, end rows then interior rows, with the columns it reads in the workspace's
    /// slots. `chained` adds the point before's end and the fits, which a stage alone holds fixed.
    fn interval_lines(
        &self,
        unknowns: &[f64],
        values: &[f64],
        point: usize,
        chained: bool,
    ) -> Result<Vec<Vec<(usize, f64)>>, InitializationFailure> {
        let rows = self.body.rows();
        let sparsity = self.body.sparsity();
        let fits = &self.parameters.owed.fits;
        let border = self.frames.len() * self.width();
        let mut slopes: [(Vec<f64>, Vec<Vec<f64>>); 2] = Default::default();
        for &stage in self.stage_list() {
            let inputs = self.inputs(point, &unknowns[self.slots(point, stage)], values);
            let entries = self.body.jacobian(&inputs, 0.0).map_err(undefined)?;
            let mut by_fit = Vec::with_capacity(fits.len());
            for fit in fits.iter().filter(|_| chained) {
                by_fit.push(
                    self.body
                        .parameter_slopes(&inputs, &fit.slots)
                        .map_err(undefined)?,
                );
            }
            slopes[stage] = (entries, by_fit);
        }
        let mut lines = Vec::with_capacity(self.width());
        for &stage in self.stage_list() {
            for (at, row) in rows.iter().enumerate() {
                let own = self.slots(point, stage).start;
                let reads = sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1];
                let mut line = Vec::new();
                let read = |line: &mut Vec<(usize, f64)>, from: usize, scale: f64| {
                    let (entries, by_fit) = &slopes[from];
                    let first = self.slots(point, from).start;
                    for entry in reads.clone() {
                        add(
                            line,
                            first + sparsity.unknowns()[entry].index(),
                            scale * entries[entry],
                        );
                    }
                    for (fit, slope) in by_fit.iter().enumerate() {
                        add(line, border + fit, scale * slope[at]);
                    }
                };
                match self.stage_interval(at) {
                    _ if algebraic(row.kind()) => read(&mut line, stage, 1.0),
                    Some(seconds) => {
                        for from in [INTERIOR, END] {
                            read(&mut line, from, seconds * RADAU[stage][from]);
                        }
                        add(&mut line, own + at, 1.0);
                        if chained && !self.reset(point, at) {
                            add(&mut line, self.slots(self.before(point), END).start + at, -1.0);
                        }
                    }
                    None => line.push((own + at, 1.0)),
                }
                lines.push(line);
            }
        }
        Ok(lines)
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
        let values = self.values(unknowns);
        let fits = &self.parameters.owed.fits;
        let mut lines: Vec<Vec<(usize, f64)>> = Vec::with_capacity(self.unknowns());
        for point in 0..self.frames.len() {
            lines.extend(self.interval_lines(unknowns, &values, point, true)?);
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
                    let points = self.frames.len();
                    let share = 1.0 / points as f64;
                    (0..points)
                        .map(|point| (self.slots(point, END).start + row, share))
                        .collect()
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
    fn pattern(&self) -> (&[usize], &[UnknownId]) {
        match &self.pattern {
            Some(pattern) => (&pattern.offsets, &pattern.unknowns),
            None => (
                self.body.sparsity().row_offsets(),
                self.body.sparsity().unknowns(),
            ),
        }
    }
    /// One interval's block in its own pattern, each entry read by its column from `entry`.
    fn local_block(&self, point: usize, entry: impl Fn(usize, usize) -> f64) -> Jacobian {
        let (offsets, unknowns) = self.pattern();
        let first = point * self.width();
        let mut values = Vec::with_capacity(unknowns.len());
        for row in 0..offsets.len() - 1 {
            let pattern = &unknowns[offsets[row]..offsets[row + 1]];
            values.extend(pattern.iter().map(|unknown| entry(row, first + unknown.index())));
        }
        Jacobian::of_parts(offsets.to_vec(), unknowns.to_vec(), values)
    }
    /// A block of a `stage` or a `block`, factored over this system's plan, or the row whose pivot it cannot take. The
    /// plan keeps the body's pivots at both stages, so a closed store factors here as it does in a core's stage.
    pub fn factor(&self, block: &Jacobian) -> Result<Factors<'_>, usize> {
        match &self.pattern {
            Some(pattern) => pattern.plan.factor(&block.values),
            None => self.body.factor(block),
        }
    }
    /// A Newton update for one `stage`: its block factored over the plan, or read whole where a pivot of the plan is
    /// zero, or the row with no pivot at all.
    pub fn solve_stage(&self, block: &Jacobian, residuals: &[f64]) -> Result<Vec<f64>, usize> {
        if let Ok(factors) = self.factor(block) {
            let mut update = residuals.to_vec();
            factors.solve(&mut update);
            return Ok(update);
        }
        let mut matrix = self.dense(block);
        for (row, &residual) in matrix.iter_mut().zip(residuals) {
            row[residuals.len()] = residual;
        }
        eliminate(matrix)
    }
    /// One interval's stages on their own: each row's residual, as `evaluate` gives it before a total replaces a row,
    /// and the block of the interval's own unknowns, in the slots `interval_slots` names. An admission solver marches the ring
    /// with this and `sweep`, to reach a guess Newton can start from. Nothing here is judged: `admit` is.
    pub fn stage(
        &self,
        workspace: &SolveWorkspace,
        point: usize,
    ) -> Result<(Vec<f64>, Jacobian), InitializationFailure> {
        self.check(workspace)?;
        let unknowns = workspace.coordinates();
        let values = self.values(unknowns);
        let lines = self.interval_lines(unknowns, &values, point, false)?;
        let block = self.local_block(point, |row, column| {
            (lines[row].iter())
                .find(|(at, _)| *at == column)
                .map_or(0.0, |(_, value)| *value)
        });
        let mut rows = Vec::with_capacity(self.width());
        self.stage_rows(unknowns, &values, point, &mut rows)?;
        Ok((rows, block))
    }
    /// One interval's own block of a `linearize`, in the pattern a `stage` block has, so a ring elimination factors it
    /// over the same plan. A cycle's last interval holds totals in place of rows, so its block is not this. Each entry
    /// is read by its column, because a row that states the packet's value holds fewer entries than the pattern.
    pub fn block(&self, ring: &Jacobian, point: usize) -> Jacobian {
        let view = ring.view();
        let first = point * self.width();
        self.local_block(point, |row, column| {
            let line = view.row_offsets()[first + row]..view.row_offsets()[first + row + 1];
            (view.columns()[line.clone()].iter().zip(&view.values()[line]))
                .find(|(at, _)| at.index() == column)
                .map_or(0.0, |(_, value)| *value)
        })
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
        let size = self.width();
        let values = self.values(unknowns);
        // A law with no slope here says nothing of rank. The first stage that needs one refuses it.
        let Ok(lines) = self.interval_lines(unknowns, &values, 0, false) else {
            return Ok(None);
        };
        let mut matrix = vec![vec![0.0; size + 1]; size];
        for (cells, line) in matrix.iter_mut().zip(&lines) {
            for &(column, value) in line.iter().filter(|(column, _)| *column < size) {
                cells[column] += value;
            }
        }
        Ok(eliminate(matrix)
            .err()
            .map(|slot| RowId::at(slot % self.body.rows().len())))
    }
    /// A system small enough to read whole: each row with a zero right side, as `eliminate` takes it.
    fn dense(&self, lines: &Jacobian) -> Vec<Vec<f64>> {
        let view = lines.view();
        let size = view.row_offsets().len() - 1;
        let mut matrix = vec![vec![0.0; size + 1]; size];
        for (row, cells) in matrix.iter_mut().enumerate() {
            for entry in view.row_offsets()[row]..view.row_offsets()[row + 1] {
                cells[view.columns()[entry].index()] += view.values()[entry];
            }
        }
        matrix
    }
    /// One defect over its scale, `None` when the tolerance refuses it. What overflowed is refused, because `<=` is
    /// false on a NaN but true of two infinities, and a defect of zero holds even where an overflow leaves no scale.
    fn held(&self, defect: f64, read: f64, size: f64) -> Option<f64> {
        let scale = self.tolerance.at(read, size);
        let within = defect == 0.0 || (defect.is_finite() && defect <= scale);
        within.then(|| if scale > 0.0 { defect / scale } else { 0.0 })
    }
    /// An instant reads its row at point 0. A cycle mean reads its row's mean over the points, the samples an observer
    /// takes on the cycle's grid, not the stages' quadrature. It sums as an observer sums a window, so the two read one
    /// number however far the row swings.
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
    /// A combination's net boundary flow over the cycle, one column at a time as the ledger sums it, whichever rows
    /// the workspace spread it over, at each stage by its weight: the quadrature the end rows' sum closes. A row's own
    /// sum can lose a small flow beside large ones, and the ledger keeps it.
    fn net_flow(&self, unknowns: &[f64], quantity: QuantityId) -> Result<f64, InitializationFailure> {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let crossing = |terms: &[(RowId, i64)]| match *terms {
            [(row, sign)] => (combination.terms().iter())
                .filter(|(known, _)| *known == row)
                .map(|(_, coefficient)| sign * coefficient)
                .sum(),
            _ => 0,
        };
        let values = self.values(unknowns);
        let mut net = Compensated::ZERO;
        for point in 0..self.frames.len() {
            for &stage in self.stage_list() {
                let seconds = self.interval.map_or(1.0, |seconds| seconds * RADAU[END][stage]);
                let inputs = self.inputs(point, &unknowns[self.slots(point, stage)], &values);
                let laws = self.body.instance_values(&inputs).map_err(undefined)?;
                let (rates, _) = self.body.targets(&laws);
                for (column, rate) in self.body.columns().iter().zip(rates) {
                    net = net.plus(Compensated::exact(seconds * rate).scaled(crossing(&column.terms)));
                }
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
        for (slot, &value) in unknowns[..self.frames.len() * self.width()].iter().enumerate() {
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
            let row = slot % size;
            let judge = self.body.judged_at(row);
            let held = self.held(stages[slot].abs(), unknowns[slot - row + judge], sizes[judge]);
            held.ok_or(inconsistent(body_row(row)))
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
        // One stage can have rank while the whole system has a free level: a total nothing truly regulates, or two fits
        // of one anchor's worth of information.
        let Ok(lines) = self.lines(unknowns) else {
            return Ok(worst);
        };
        let free = if self.frames.len() > 1 {
            self.solve(&lines, &vec![0.0; unknowns.len()]).err()
        } else {
            eliminate(self.dense(&lines)).err()
        };
        if let Some(slot) = free {
            let row = match slot.checked_sub(self.width()) {
                None => rows[slot % size].id(),
                Some(fit) => rows[owed.anchors[owed.fits[fit].anchor].row.index()].id(),
            };
            return Err(InitializationFailure::NoRank { row });
        }
        Ok(worst)
    }
    fn felt(&self, unknowns: &[f64], fit: &Fit, value: f64) -> Result<bool, InitializationFailure> {
        let rows = self.body.rows();
        let values = self.values(unknowns);
        for point in 0..self.frames.len() {
            for &stage in self.stage_list() {
                let state = &unknowns[self.slots(point, stage)];
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
            pattern: _,
            draws: _,
            factors: _,
        } = self;
        let solved = modes.swap_remove(0);
        let unread = parameters.packet;
        parameters.packet = unread.reading(time, start, &solved, fitted, &history);
        parameters.values = values;
        let stated = core::mem::replace(&mut parameters.modes, solved);
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
        let mode_of = |((mode, &value), &guess): ((&ModeLayout, &usize), &usize)| ValueRecord {
            name: ValueName::Mode {
                variable: mode.variable,
                value: mode.values[value],
            },
            provenance: if value == guess {
                ValueProvenance::Fixture
            } else {
                ValueProvenance::Solved
            },
            value: Compensated::exact(value as f64),
        };
        let starts = body.rows().iter().zip(start).zip(&parameters.starts);
        let modes = body.layout.modes.iter().zip(&parameters.modes).zip(&stated);
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
            unread,
            start: start.to_vec(),
            fitted: fitted.to_vec(),
        })
    }
}

/// A judged start. It holds what its hash read, so the same start can close under another core's history.
pub struct InitialRevision {
    parts: ImageParts,
    provenance: InitializationProvenance,
    unread: PacketHash,
    start: Vec<f64>,
    fitted: Vec<f64>,
}
impl InitialRevision {
    /// This start under `history`: what `admit` would give the same witness with it, with no second judgment. Its
    /// state, modes, fits, pending inputs and provenance are this one's, and its hash closes on `history`. Nothing
    /// else can change, so a start cannot be reused against another timeline.
    pub fn with_history(&self, history: MethodHistory) -> Self {
        let parts = &self.parts;
        let mut parameters = parts.parameters.copied();
        parameters.packet =
            (self.unread).reading(parts.time, &self.start, &parameters.modes, &self.fitted, &history);
        Self {
            parts: ImageParts {
                time: parts.time,
                state: parts.state.copied(),
                continuation: history,
                dense: DenseSegment::empty(),
                pending: parts.pending.copied(),
                parameters,
            },
            provenance: self.provenance.clone(),
            unread: self.unread,
            start: self.start.clone(),
            fitted: self.fitted.clone(),
        }
    }
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
    /// A law a workspace leaves without a value, such as a divisor at zero.
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
    /// A row with no pivot on the solved workspace, in its own stage or on the ring, which structural
    /// matching cannot see.
    NoRank {
        row: RowId,
    },
}
impl From<AdmissionError> for InitializationFailure {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}

#[cfg(test)]
mod tests {
    use super::{END, INTERIOR, RADAU};

    /// One interval of `y' = speed (target - y)` from `start`, both stages from their two linear rows.
    fn interval(start: f64, h: f64, speed: f64, target: f64) -> [f64; 2] {
        let a = |j: usize, k: usize| h * speed * RADAU[j][k];
        let matrix = [[1.0 + a(0, 0), a(0, 1)], [a(1, 0), 1.0 + a(1, 1)]];
        let right = RADAU.map(|row| start + h * speed * target * (row[0] + row[1]));
        let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        [
            (right[0] * matrix[1][1] - matrix[0][1] * right[1]) / det,
            (matrix[0][0] * right[1] - matrix[1][0] * right[0]) / det,
        ]
    }

    /// The verdict's counterexample on this tableau: a drive relaxing at speed 2 to 3 for the first of four intervals
    /// and at speed 5 to 1 after, on its periodic orbit, scaled so the mean of its four points is one. A phase that
    /// runs at it advances by the stage weights' integral over the beat, which falls short of one, so a sampled mean
    /// anchor on a varying rate cannot place a wrap on the grid.
    #[test]
    fn a_sampled_mean_of_one_leaves_the_stage_integral_of_a_varying_rate_short_of_one() {
        let (points, h) = (4, 0.25);
        let run = |mut start: f64| {
            let mut stages = Vec::with_capacity(points);
            for point in 0..points {
                let (speed, target) = if point == 0 { (2.0, 3.0) } else { (5.0, 1.0) };
                let both = interval(start, h, speed, target);
                start = both[END];
                stages.push(both);
            }
            stages
        };
        let from_zero = run(0.0)[points - 1][END];
        let slope = run(1.0)[points - 1][END] - from_zero;
        let orbit = run(from_zero / (1.0 - slope));
        let sampled: f64 = orbit.iter().map(|both| both[END]).sum::<f64>() / points as f64;
        let weighted: f64 = (orbit.iter())
            .map(|both| h * (RADAU[END][INTERIOR] * both[INTERIOR] + RADAU[END][END] * both[END]))
            .sum();
        let advance = weighted / sampled;
        assert!((advance - 0.992_687_681_7).abs() < 1e-9, "{advance}");
    }
}
