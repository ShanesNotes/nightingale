use crate::body::{Bound, Instance, ModeLayout, Target};
use crate::boundary::{Delivery, DriveFrame, PendingInputs};
use crate::chance::{HELD_DRAW, HELD_FACTOR};
use crate::elimination::{Elimination, Factors};
use crate::evaluate::{Inputs, Undefined};
use crate::identity::{Canonical, Seal};
use crate::packet::{BoundAnchor, Fit, Owed};
use crate::problem::Jacobian;
use crate::{
    AdmissionError, AnchorId, AnchorStatistic, Book, BoundParameters, BoundaryPort, Candidate, Combination,
    Compensated, CompiledBody, DenseSegment, Digest, ErrorScales, ImageParts, InitializationProvenance,
    LawName, Measure, MethodHistory, ModelHash, ObservationLens, PacketHash, QuantityId, Row, RowId, RowKind,
    SimTime, Slot, Time, UnknownId, ValueName, ValueProvenance, ValueRecord, eliminate,
};
use core::ops::Range;

mod crossings;
pub use crossings::Relocated;

const NANOSECONDS_PER_SECOND: f64 = 1e9;
/// What rounding leaves of a value, past which a change to it can be told from nothing.
const ROUNDING: f64 = 8.0 * f64::EPSILON;
/// Two-stage Radau IIA, the one tableau of a cycle's leg (Hairer and Wanner, *Solving Ordinary Differential
/// Equations II*, 2nd ed., 1996, section IV.5, Radau IIA of order 3): stage `j` of a leg of length `h` from `P` holds
/// `Y_j = P + h sum_k A[j][k] F(Y_k)`, the interior stage at a third of the leg and the end stage at its end. It
/// damps a stiff row to zero, and the end stage is the leg's end, so its row is the weights.
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

/// The clock a paced cycle names, proved at `initialization`: a memory whose rate is the anchored row, reset to zero by
/// its one rising guard, the clock less a threshold, which the anchor's time mean reaches in exactly one period. Its
/// guard's crossing is the cycle's own end, so it holds no time of its own.
#[derive(Clone, Copy)]
struct PeriodIdentity {
    clock: usize,
    guard: usize,
    anchor: usize,
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
    /// whole multiples of their length, so a run that starts inside one never reads the beat admission matched. A cycle
    /// time mean is read only by a channel that integrates over its window, never by one that counts samples.
    fn check_readings(
        &self,
        owed: &Owed,
        moment: SimTime,
        lens: Option<&ObservationLens>,
    ) -> Result<(), AdmissionError> {
        for anchor in &owed.anchors {
            let read = lens.is_some_and(|lens| {
                let on_model = lens.model() == self.model_hash();
                match (anchor.statistic, owed.cycle) {
                    (AnchorStatistic::CycleTimeMean, Some(cycle)) => {
                        on_model
                            && moment.nanoseconds().is_multiple_of(cycle.period_ns())
                            && lens.reads_time_mean(anchor.method, anchor.row, cycle.period_ns())
                    }
                    (statistic, cycle) => {
                        let (window_ns, points) = match (statistic, cycle) {
                            (AnchorStatistic::CycleMean, Some(cycle)) => {
                                (cycle.period_ns(), cycle.intervals())
                            }
                            _ => (lens.period_ns(), 1),
                        };
                        on_model
                            && moment.nanoseconds().is_multiple_of(window_ns)
                            && lens.reads(anchor.method, anchor.row, window_ns, points)
                    }
                }
            });
            if !read {
                return Err(AdmissionError::AnchorUnread { anchor: anchor.id });
            }
        }
        Ok(())
    }
    /// The identity a paced cycle claims, checked against the compiled laws, or the refusal that names what fails it.
    fn period_identity(
        &self,
        parameters: &BoundParameters,
    ) -> Result<Option<PeriodIdentity>, AdmissionError> {
        let owed = &parameters.owed;
        let Some((cycle, (clock, anchor))) = owed.cycle.and_then(|cycle| Some((cycle, cycle.pacing()?)))
        else {
            return Ok(None);
        };
        let at = (owed.anchors.iter()).position(|bound| bound.id == anchor);
        let Some(at) = at.filter(|&at| owed.anchors[at].statistic == AnchorStatistic::CycleTimeMean) else {
            return Err(AdmissionError::PeriodAnchor { anchor });
        };
        let rate = owed.anchors[at].row;
        let refused = AdmissionError::PeriodClock { clock };
        let row = (self.rows().iter())
            .position(|row| row.kind() == RowKind::Memory(clock))
            .ok_or(refused)?;
        let runs = (self.layout.instances.iter()).filter(|law| law.target == Target::Row(RowId::at(row)));
        let mut runs = runs.peekable();
        runs.peek().ok_or(refused)?;
        for law in runs {
            let ops = &self.layout.programs[law.program.index()].code.ops;
            let copy = ops.len() == 1 && law.inputs[..] == [Bound::Unknown(UnknownId::at(rate.index()))];
            if !copy || law.mode.is_some() {
                return Err(refused);
            }
        }
        let resetting: Vec<usize> = (self.layout.guards.iter().enumerate())
            .filter(|(_, guard)| guard.resets == Some(RowId::at(row)))
            .map(|(at, _)| at)
            .collect();
        let [guard] = resetting[..] else {
            return Err(refused);
        };
        let layout = &self.layout.guards[guard];
        let threshold = match (layout.crossing, &layout.inputs[..], layout.code.ops.len()) {
            (crate::Crossing::Rising, [Bound::Unknown(read), Bound::Parameter(threshold)], 3)
                if read.index() == row && matches!(layout.code.ops[2], crate::law::Op::Subtract([0, 1])) =>
            {
                parameters.values[threshold.index()]
            }
            _ => return Err(refused),
        };
        let period = cycle.period_ns() as f64 / NANOSECONDS_PER_SECOND;
        let reached = owed.anchors[at].value * period;
        if (reached - threshold).abs() > ROUNDING * threshold.abs() {
            return Err(AdmissionError::PeriodMismatch { anchor });
        }
        Ok(Some(PeriodIdentity {
            clock: row,
            guard,
            anchor: at,
        }))
    }
    /// The square admission system. Refuses parameters bound on another body, an event `check_delivery` refuses
    /// at the anchor moment, an anchor no channel reads, a delivery edge off the cycle's grid and a paced cycle whose
    /// clock, anchor or period do not agree.
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
        let period = self.period_identity(&parameters)?;
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
        let staged = |row: usize| {
            let hazard = (self.layout.hazards.iter()).any(|hazard| hazard.integral.index() == row);
            !hazard && !algebraic(self.rows()[row].kind())
        };
        let pattern = interval.map(|_| StagePattern::radau(self, staged));
        let mut identity = Canonical::new("ng-contract admission system 1");
        self.model_hash().digest().seal(&mut identity);
        parameters.packet.digest().seal(&mut identity);
        anchor_moment.seal(&mut identity);
        parameters.owed.cycle.seal(&mut identity);
        Ok(InitializationSystem {
            body: self,
            parameters,
            pending,
            anchor_moment,
            tolerance,
            frames,
            interval,
            pattern,
            period,
            identity: identity.digest(),
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

/// One leg's two stages as a block of a cycle's matrix: end rows then interior rows, over end then interior unknowns.
/// Each stage keeps the body's pivots, so a closed store stays paired with the power that holds it, and a leg factors
/// over a plan made once, as a body's stage does.
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

/// A guard's crossing inside one interval: the guards that share its zero, the first the one its root row reads, and
/// how far into the interval it lies, in seconds.
#[derive(Clone, Debug, PartialEq)]
struct Located {
    interval: usize,
    guards: Vec<usize>,
    offset: f64,
}

/// Where a cycle's modes change: the modes the cycle opens in after the wrap's firings, each located crossing in
/// interval and time order, and for each leg the guards that fire where it starts, in firing order. A leg's modes and
/// resets follow from these.
#[derive(Clone, Debug, PartialEq)]
struct Topology {
    opening: Vec<usize>,
    crossings: Vec<Located>,
    knots: Vec<Vec<usize>>,
}

/// A part of one interval on which the modes and drives are fixed, solved by one pair of Radau IIA stages: in seconds,
/// where it starts in its interval and how long it lasts, the modes it holds, the rows its start resets, and the
/// crossings that end and start it.
#[derive(Clone, Debug, PartialEq)]
struct Leg {
    interval: usize,
    start: f64,
    length: f64,
    modes: Vec<usize>,
    resets: Vec<RowId>,
    ends_at: Option<usize>,
    starts_at: Option<usize>,
}

/// The coordinates of one admission solve and the topology they are solved on. Its stage values and fits are slots a
/// solver writes; its crossing times are not, and change only by a correction, a sweep, a march's relocation or a
/// checked shift. It is bound to the system that made it, so another system's workspace is refused.
#[derive(Clone)]
pub struct AdmissionWorkspace {
    system: Digest,
    rows: usize,
    width: usize,
    interval: Option<f64>,
    unknowns: Vec<f64>,
    topology: Topology,
    legs: Vec<Leg>,
}
impl AdmissionWorkspace {
    /// Every stage value, leg by leg, then each fit.
    pub fn unknowns_mut(&mut self) -> &mut [f64] {
        &mut self.unknowns
    }
    pub(crate) fn leg_count(&self) -> usize {
        self.legs.len()
    }
    fn block(&self, leg: usize) -> usize {
        leg * self.width
    }
    fn base(&self, point: usize) -> usize {
        (self.legs.iter())
            .rposition(|leg| leg.interval == point)
            .expect("every interval has a leg")
    }
    /// The slots of the state at grid point `point`: the end stage of the leg that ends there, before any firing.
    pub fn endpoint(&self, point: usize) -> Range<usize> {
        let start = self.block(self.base(point));
        start..start + self.rows
    }
    /// The slots of the interior stage of the leg that ends at `point`, empty at an instant.
    pub fn interior(&self, point: usize) -> Range<usize> {
        let end = self.endpoint(point).end;
        if self.interval.is_some() {
            end..end + self.rows
        } else {
            end..end
        }
    }
    /// Every slot of every leg of the interval that ends at `point`.
    pub fn interval_slots(&self, point: usize) -> Range<usize> {
        let first = (self.legs.iter())
            .position(|leg| leg.interval == point)
            .expect("a leg");
        self.block(first)..self.block(self.base(point) + 1)
    }
    /// The fits' slots, after every stage.
    pub fn fitted_slots(&self) -> Range<usize> {
        self.block(self.legs.len())..self.unknowns.len()
    }
    /// How many guard crossings the workspace places inside its intervals.
    pub fn crossing_count(&self) -> usize {
        self.topology.crossings.len()
    }
    /// The legs of the interval that ends at `point`, in time order.
    pub fn legs_of(&self, point: usize) -> Vec<LegHandle> {
        (self.legs.iter().enumerate())
            .filter(|(_, leg)| leg.interval == point)
            .map(|(index, _)| LegHandle { index })
            .collect()
    }
    /// The slots of one leg's two stages, end then interior.
    pub fn leg_slots(&self, leg: LegHandle) -> Range<usize> {
        self.block(leg.index)..self.block(leg.index + 1)
    }
    /// Moves one crossing by `by`. Refused where it would leave its interval or pass a neighbour, so every leg keeps a
    /// positive length; a moved crossing is a guess like any other, and `admit` judges it.
    pub fn shift_crossing(&mut self, crossing: usize, by: Measure<Time>) -> Result<(), AdmissionError> {
        let offset = self
            .topology
            .crossings
            .get(crossing)
            .ok_or(AdmissionError::CrossingOutside)?
            .offset;
        let moved = offset + by.value();
        if !self.fits(crossing, moved) {
            return Err(AdmissionError::CrossingOutside);
        }
        self.topology.crossings[crossing].offset = moved;
        self.place();
        Ok(())
    }
    /// Whether crossing `at` can stand at `offset`: inside its interval and after and before its neighbours there.
    fn fits(&self, at: usize, offset: f64) -> bool {
        let Some(interval) = self.interval else {
            return false;
        };
        let crossings = &self.topology.crossings;
        let here = crossings[at].interval;
        let clearance = interval * crossings::CLEARANCE;
        let before = (at.checked_sub(1)).filter(|&before| crossings[before].interval == here);
        let after =
            Some(at + 1).filter(|&after| after < crossings.len() && crossings[after].interval == here);
        let low = before.map_or(0.0, |before| crossings[before].offset);
        let high = after.map_or(interval, |after| crossings[after].offset);
        offset.is_finite() && offset - low >= clearance && high - offset >= clearance
    }
    /// Each leg's start and length again, from its crossings.
    fn place(&mut self) {
        let interval = self.interval.unwrap_or(0.0);
        for leg in &mut self.legs {
            let start = leg.starts_at.map_or(0.0, |at| self.topology.crossings[at].offset);
            let end = leg
                .ends_at
                .map_or(interval, |at| self.topology.crossings[at].offset);
            leg.start = start;
            leg.length = end - start;
        }
    }
}

/// Whether `apply` moved every crossing time as far as its correction asked or held one short of a knot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    Inside,
    Clamped,
}

/// One leg of a workspace, for a march that solves the cycle leg by leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegHandle {
    index: usize,
}

/// A Newton update for a workspace: one change per slot, and one per crossing time, which only `apply` reads.
pub struct Correction {
    slots: Vec<f64>,
    times: Vec<f64>,
}
impl Correction {
    pub(crate) fn of(slots: Vec<f64>, times: Vec<f64>) -> Self {
        Self { slots, times }
    }
    /// The change to each slot `unknowns_mut` holds.
    pub fn slots(&self) -> &[f64] {
        &self.slots
    }
    /// The largest change to a crossing time as a share of the interval.
    pub fn time_share(&self, workspace: &AdmissionWorkspace) -> f64 {
        let interval = workspace.interval.unwrap_or(1.0);
        (self.times.iter()).fold(0.0, |most, change| {
            let share = change.abs() / interval;
            if share > most { share } else { most }
        })
    }
}

/// A located crossing of an admitted cycle: the guard's mode variable, the values it leaves and enters, and the time
/// it stands at from the anchor moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocatedCrossing {
    pub variable: Slot,
    pub leaves: &'static str,
    pub enters: &'static str,
    seconds: f64,
}
impl LocatedCrossing {
    pub fn at(&self) -> Measure<Time> {
        Measure::new(self.seconds).expect("a finite time")
    }
}

/// A square system over the body's rows at both stages of every leg, one unknown per fit and one per located
/// crossing. Grid point 0, at the anchor moment after any firing there, is the start state.
///
/// The packet's equal intervals form a fixed grid. Admission splits an interval at each guard crossing it locates, so
/// each leg holds one set of modes and one drive frame and is two Radau IIA stages from the state before it. Each
/// crossing adds its time and its guard's root row; the reset it fires is a projection of the state before it. A
/// workspace holds the stages leg by leg in time order from the anchor moment, `[end, interior]` each, and then the
/// fits; the residual holds each stage's
/// rows in the same slots, with a total in place of one end row and, for a paced cycle, its clock's root in place of
/// the clock's last end row, then each fit's anchor and each crossing's root. That layout is the block ring an
/// elimination reads, with fits and crossing times on its border.
///
/// An instant has one point, its only stage, and no ring.
pub struct InitializationSystem<'m> {
    pub(crate) body: &'m CompiledBody,
    parameters: BoundParameters,
    pending: PendingInputs,
    anchor_moment: SimTime,
    tolerance: ErrorScales,
    /// One per grid point: the drives frozen at the start of the interval it opens, as a ticket's are. The interval
    /// ending at point `k` reads the frame of the point before it at every stage of every leg.
    frames: Vec<DriveFrame>,
    /// One interval in seconds, and `None` for an instant.
    interval: Option<f64>,
    /// A cycle's leg block, and `None` for an instant, whose one stage is the body's.
    pattern: Option<StagePattern>,
    period: Option<PeriodIdentity>,
    identity: Digest,
    draws: Vec<f64>,
    factors: Vec<f64>,
}
impl InitializationSystem<'_> {
    /// Slots per leg: both stages under a cycle, the one point of an instant.
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
    fn slots(&self, leg: usize, stage: usize) -> Range<usize> {
        let rows = self.body.rows().len();
        let start = leg * self.width() + if stage == END { 0 } else { rows };
        start..start + rows
    }
    fn check(&self, workspace: &AdmissionWorkspace) -> Result<(), InitializationFailure> {
        let legs = self.derive(&workspace.topology);
        let foreign = workspace.system != self.identity
            || workspace.unknowns.len()
                != workspace.legs.len() * self.width() + self.parameters.owed.fits.len()
            || legs.as_ref() != Some(&workspace.legs);
        if foreign {
            return Err(AdmissionError::ForeignWorkspace.into());
        }
        Ok(())
    }
    fn before(&self, interval: usize) -> usize {
        (interval + self.frames.len() - 1) % self.frames.len()
    }
    /// The intervals in time order from the anchor moment: interval 1 first and interval 0, which ends at the wrap,
    /// last. Legs lie in this order, so the leg that ends at grid point 0 is the ring's last block, the one an
    /// elimination reads whole, and the totals and a clock's root replace rows there.
    fn chronological(&self) -> impl Iterator<Item = usize> + use<> {
        let points = self.frames.len();
        (0..points).map(move |step| (step + 1) % points)
    }
    fn rank(&self, interval: usize) -> usize {
        (interval + self.frames.len() - 1) % self.frames.len()
    }
    /// The leg that follows the wrap: the first of interval 1, or of the one interval.
    fn first_leg(&self, legs: &[Leg]) -> usize {
        let opening = 1 % self.frames.len();
        (legs.iter())
            .position(|leg| leg.interval == opening)
            .expect("every interval has a leg")
    }
    fn previous(legs: &[Leg], leg: usize) -> usize {
        (leg + legs.len() - 1) % legs.len()
    }
    fn fitted<'u>(&self, workspace: &'u AdmissionWorkspace) -> &'u [f64] {
        &workspace.unknowns[workspace.fitted_slots()]
    }
    fn values(&self, workspace: &AdmissionWorkspace) -> Vec<f64> {
        let mut values = self.parameters.values.clone();
        for (fit, &value) in self.parameters.owed.fits.iter().zip(self.fitted(workspace)) {
            for slot in &fit.slots {
                values[slot.index()] = value;
            }
        }
        values
    }
    fn inputs<'a>(&'a self, leg: &'a Leg, state: &'a [f64], values: &'a [f64]) -> Inputs<'a> {
        Inputs {
            parameters: values,
            frame: &self.frames[self.before(leg.interval)],
            unknowns: state,
            modes: &leg.modes,
            draws: &self.draws,
            factors: &self.factors,
        }
    }
    /// The state a leg starts from: the end of the leg before it, with each row its start's firings reset at zero.
    fn origin(&self, workspace: &AdmissionWorkspace, leg: usize) -> Vec<f64> {
        let previous = Self::previous(&workspace.legs, leg);
        let mut origin = workspace.unknowns[self.slots(previous, END)].to_vec();
        for row in &workspace.legs[leg].resets {
            origin[row.index()] = 0.0;
        }
        origin
    }
    /// The start state: grid point 0 after the firings that open the cycle.
    fn start_state(&self, workspace: &AdmissionWorkspace) -> Vec<f64> {
        if self.interval.is_none() {
            return workspace.unknowns[self.slots(0, END)].to_vec();
        }
        self.origin(workspace, self.first_leg(&workspace.legs))
    }
    /// Legs, modes and resets from a topology, or `None` where it holds the wrong number of knots or a crossing
    /// outside its interval or out of order.
    fn derive(&self, topology: &Topology) -> Option<Vec<Leg>> {
        let points = self.frames.len();
        let Some(interval) = self.interval else {
            let modes = self.parameters.modes.clone();
            let instant = Leg {
                interval: 0,
                start: 0.0,
                length: 0.0,
                modes,
                resets: Vec::new(),
                ends_at: None,
                starts_at: None,
            };
            let alone = topology.crossings.is_empty()
                && topology.knots.len() == 1
                && topology.opening == instant.modes;
            return alone.then(|| vec![instant]);
        };
        let crossings = &topology.crossings;
        let rank = |crossing: &Located| self.rank(crossing.interval);
        let ordered = (crossings.windows(2)).all(|pair| {
            rank(&pair[0]) < rank(&pair[1])
                || (rank(&pair[0]) == rank(&pair[1]) && pair[0].offset < pair[1].offset)
        });
        let inside = (crossings.iter()).all(|crossing| {
            crossing.interval < points
                && crossing.offset > 0.0
                && crossing.offset < interval
                && !crossing.guards.is_empty()
        });
        if !ordered || !inside || topology.knots.len() != points + crossings.len() {
            return None;
        }
        let mut legs = Vec::with_capacity(topology.knots.len());
        let mut next = 0;
        for point in self.chronological() {
            let mut start = (0.0, None);
            while next < crossings.len() && crossings[next].interval == point {
                let offset = crossings[next].offset;
                legs.push((point, start, (offset, Some(next))));
                start = (offset, Some(next));
                next += 1;
            }
            legs.push((point, start, (interval, None)));
        }
        let mut legs: Vec<Leg> = (legs.into_iter())
            .map(|(point, (start, starts_at), (end, ends_at))| Leg {
                interval: point,
                start,
                length: end - start,
                modes: Vec::new(),
                resets: Vec::new(),
                ends_at,
                starts_at,
            })
            .collect();
        let first = self.first_leg(&legs);
        let guards = &self.body.layout.guards;
        if topology.opening.len() != self.parameters.modes.len() {
            return None;
        }
        let mut modes = topology.opening.clone();
        for step in 0..legs.len() {
            let at = (first + step) % legs.len();
            let knot = &topology.knots[at];
            if step > 0 {
                for &guard in knot {
                    if guards[guard].armed(&modes) {
                        modes[guards[guard].variable] = guards[guard].enters;
                    }
                }
            }
            legs[at].modes = modes.clone();
            legs[at].resets = knot.iter().filter_map(|&guard| guards[guard].resets).collect();
        }
        Some(legs)
    }
    fn workspace_of(&self, topology: Topology, unknowns: Vec<f64>) -> AdmissionWorkspace {
        let legs = self.derive(&topology).expect("a topology this system made");
        AdmissionWorkspace {
            system: self.identity,
            rows: self.body.rows().len(),
            width: self.width(),
            interval: self.interval,
            unknowns,
            topology,
            legs,
        }
    }
    /// The interval an amount or memory row's stage legs, and `None` for a row that states the packet's value: every
    /// such row of an instant, and a hazard integral, which only grows until it fires, so it never returns over a cycle
    /// and accrues from the anchor moment.
    fn staged(&self, row: usize) -> bool {
        let hazard = (self.body.layout.hazards.iter()).any(|hazard| hazard.integral.index() == row);
        self.interval.is_some() && !hazard
    }
    /// The total row of a combination replaces its least row's end stage at the wrap, in the leg whose end it reads,
    /// because the end rows sum to the combination's net boundary flow over the cycle, whatever the state: they alone
    /// leave its total free. A regulated total answers its own flows, so its rows pin it and it needs no row. Charge
    /// follows from the ions.
    fn replaced<'w>(
        &'w self,
        workspace: &'w AdmissionWorkspace,
    ) -> impl Iterator<Item = (usize, QuantityId)> + 'w {
        let last = workspace.endpoint(0).start;
        let combinations = self.body.conserved_basis().combinations();
        (combinations.iter())
            .filter(|combination| !combination.regulated() && combination.book() != Book::Charge)
            .filter(|_| self.interval.is_some())
            .map(move |combination| (last + combination.terms()[0].0.index(), combination.id()))
    }
    /// The slot whose end row a paced cycle's clock root replaces: the clock's end row in the leg before the wrap.
    fn clocked(&self, workspace: &AdmissionWorkspace) -> Option<usize> {
        let period = self.period?;
        let last = Self::previous(&workspace.legs, self.first_leg(&workspace.legs));
        Some(self.slots(last, END).start + period.clock)
    }
    /// A slot's natural size, so a solver reads a small row at its own size. A fit and a root read as one.
    pub fn size(&self, workspace: &AdmissionWorkspace, slot: usize) -> f64 {
        if slot < workspace.fitted_slots().start {
            self.parameters.sizes[slot % self.body.rows().len()]
        } else {
            1.0
        }
    }
    /// The sum of each row's square over its natural size: a slot's size and value, a fit's one, and a root's reach
    /// over the rows its guard reads. A line search compares two of these; neither reads a tolerance.
    pub fn merit(&self, workspace: &AdmissionWorkspace, residuals: &[f64]) -> f64 {
        let slots = workspace.fitted_slots().end;
        let mut sum = 0.0;
        for (slot, residual) in residuals.iter().enumerate().take(slots) {
            let share = residual / (self.size(workspace, slot) + workspace.unknowns[slot].abs());
            sum += share * share;
        }
        let values = self.values(workspace);
        for (at, crossing) in workspace.topology.crossings.iter().enumerate() {
            let leg = Self::crossing_leg(workspace, at);
            let Ok((read, size)) = self.root_scale(workspace, &values, leg, crossing.guards[0]) else {
                return f64::INFINITY;
            };
            let share = residuals
                .get(slots + at)
                .map_or(0.0, |residual| residual / (read + size));
            sum += share * share;
        }
        sum
    }
    /// Whether the system is a cycle, which a march walks, rather than an instant.
    pub fn cycles(&self) -> bool {
        self.interval.is_some()
    }
    /// How many rows a stage holds.
    pub fn rows(&self) -> usize {
        self.body.rows().len()
    }
    /// How many grid points the ring holds, one for an instant.
    pub fn points(&self) -> usize {
        self.frames.len()
    }
    /// Seeded with the packet's start values at every stage and each fit's guess, with no crossing located.
    pub fn workspace(&self) -> AdmissionWorkspace {
        let legs = self.frames.len();
        let mut unknowns = Vec::with_capacity(legs * self.width() + self.parameters.owed.fits.len());
        for _ in 0..legs * self.stage_list().len() {
            unknowns.extend_from_slice(&self.parameters.starts);
        }
        unknowns.extend(self.parameters.owed.fits.iter().map(|fit| fit.guess));
        let topology = Topology {
            opening: self.parameters.modes.clone(),
            crossings: Vec::new(),
            knots: vec![Vec::new(); legs],
        };
        self.workspace_of(topology, unknowns)
    }
    /// The crossings `workspace` places, with their guards' names and times from the anchor moment.
    pub fn crossings(&self, workspace: &AdmissionWorkspace) -> Vec<LocatedCrossing> {
        let step = self.interval.unwrap_or(0.0);
        let anchor = self.anchor_moment.nanoseconds() as f64 / NANOSECONDS_PER_SECOND;
        (workspace.topology.crossings.iter())
            .map(|crossing| {
                let guard = &self.body.layout.guards[crossing.guards[0]];
                let mode = &self.body.layout.modes[guard.variable];
                let point = (crossing.interval + self.frames.len() - 1) % self.frames.len();
                let seconds = anchor + point as f64 * step + crossing.offset;
                LocatedCrossing {
                    variable: mode.variable,
                    leaves: mode.values[guard.leaves],
                    enters: mode.values[guard.enters],
                    seconds,
                }
            })
            .collect()
    }
    /// One row per body row at each stage of each leg, none replaced. Each is in its row's own unit, so one scale
    /// reads it: a stage's rates are multiplied by its leg, as `verify` does. A row that states the packet's value is
    /// its value less the packet's.
    fn stages(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
    ) -> Result<Vec<f64>, InitializationFailure> {
        let mut out = Vec::with_capacity(workspace.unknowns.len());
        for leg in 0..workspace.legs.len() {
            self.stage_rows(workspace, values, leg, &mut out)?;
        }
        Ok(out)
    }
    /// Each stage's residual with no tangent: an amount's or memory's rate negated, an algebraic row's own value.
    fn rates(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
    ) -> Result<[Vec<f64>; 2], InitializationFailure> {
        let mut rates = [Vec::new(), Vec::new()];
        let still = vec![0.0; self.body.rows().len()];
        let held = &workspace.legs[leg];
        for &stage in self.stage_list() {
            let inputs = self.inputs(held, &workspace.unknowns[self.slots(leg, stage)], values);
            let laws = self.body.instance_values(&inputs).map_err(undefined)?;
            let (columns, rows) = self.body.targets(&laws);
            rates[stage] = self.body.residuals(&columns, &rows, &still);
        }
        Ok(rates)
    }
    fn stage_rows(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        out: &mut Vec<f64>,
    ) -> Result<(), InitializationFailure> {
        let rates = self.rates(workspace, values, leg)?;
        let origin = if self.interval.is_some() {
            self.origin(workspace, leg)
        } else {
            Vec::new()
        };
        let seconds = workspace.legs[leg].length;
        for &stage in self.stage_list() {
            let state = &workspace.unknowns[self.slots(leg, stage)];
            for (at, row) in self.body.rows().iter().enumerate() {
                out.push(if algebraic(row.kind()) {
                    rates[stage][at]
                } else if self.staged(at) {
                    let [interior, end] = RADAU[stage];
                    (state[at] - origin[at])
                        + seconds * (interior * rates[INTERIOR][at] + end * rates[END][at])
                } else {
                    state[at] - self.parameters.starts[at]
                });
            }
        }
        Ok(())
    }
    fn root_column(&self, workspace: &AdmissionWorkspace, crossing: usize) -> usize {
        workspace.unknowns.len() + crossing
    }
    /// Each row of one leg's block, end rows then interior rows, with the columns it reads in the ring's order.
    /// `chained` adds the leg before's end, the fits and the crossing times that bound it, which a stage alone holds
    /// fixed.
    fn leg_lines(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        chained: bool,
    ) -> Result<Vec<Vec<(usize, f64)>>, InitializationFailure> {
        let rows = self.body.rows();
        let sparsity = self.body.sparsity();
        let fits = &self.parameters.owed.fits;
        let border = workspace.fitted_slots().start;
        let held = &workspace.legs[leg];
        let mut slopes: [(Vec<f64>, Vec<Vec<f64>>); 2] = Default::default();
        for &stage in self.stage_list() {
            let inputs = self.inputs(held, &workspace.unknowns[self.slots(leg, stage)], values);
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
        let timed = chained && (held.ends_at.is_some() || held.starts_at.is_some());
        let rates = if timed {
            Some(self.rates(workspace, values, leg)?)
        } else {
            None
        };
        let mut lines = Vec::with_capacity(self.width());
        for &stage in self.stage_list() {
            for (at, row) in rows.iter().enumerate() {
                let own = self.slots(leg, stage).start;
                let reads = sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1];
                let mut line = Vec::new();
                let read = |line: &mut Vec<(usize, f64)>, from: usize, scale: f64| {
                    let (entries, by_fit) = &slopes[from];
                    let first = self.slots(leg, from).start;
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
                if algebraic(row.kind()) {
                    read(&mut line, stage, 1.0);
                } else if self.staged(at) {
                    for from in [INTERIOR, END] {
                        read(&mut line, from, held.length * RADAU[stage][from]);
                    }
                    add(&mut line, own + at, 1.0);
                    if chained && !held.resets.contains(&RowId::at(at)) {
                        let previous = Self::previous(&workspace.legs, leg);
                        add(&mut line, self.slots(previous, END).start + at, -1.0);
                    }
                    if let Some(rates) = &rates {
                        let along =
                            RADAU[stage][INTERIOR] * rates[INTERIOR][at] + RADAU[stage][END] * rates[END][at];
                        if let Some(end) = held.ends_at {
                            add(&mut line, self.root_column(workspace, end), along);
                        }
                        if let Some(start) = held.starts_at {
                            add(&mut line, self.root_column(workspace, start), -along);
                        }
                    }
                } else {
                    line.push((own + at, 1.0));
                }
                lines.push(line);
            }
        }
        Ok(lines)
    }
    /// A guard's directed gap at the end of `leg`, the root a crossing there holds.
    fn root(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        guard: usize,
    ) -> Result<f64, InitializationFailure> {
        let inputs = self.inputs(
            &workspace.legs[leg],
            &workspace.unknowns[self.slots(leg, END)],
            values,
        );
        self.body.guard_gap(guard, &inputs).map_err(undefined)
    }
    /// A root row's columns: the end stage of the leg it ends and each fit whose slots the guard reads.
    fn root_line(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        guard: usize,
    ) -> Result<Vec<(usize, f64)>, InitializationFailure> {
        let state = &workspace.unknowns[self.slots(leg, END)];
        let inputs = self.inputs(&workspace.legs[leg], state, values);
        let slopes = self.body.guard_slopes(guard, &inputs).map_err(undefined)?;
        let first = self.slots(leg, END).start;
        let mut line = Vec::new();
        for (unknown, slope) in slopes.unknowns {
            add(&mut line, first + unknown, slope);
        }
        let border = workspace.fitted_slots().start;
        for (parameter, slope) in slopes.parameters {
            for (fit, held) in self.parameters.owed.fits.iter().enumerate() {
                if held.slots.iter().any(|slot| slot.index() == parameter) {
                    add(&mut line, border + fit, slope);
                }
            }
        }
        Ok(line)
    }
    /// What a root row is held to: its guard's reach over the rows it reads, at their values and at their sizes, each
    /// in the guard's own unit.
    fn root_scale(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        guard: usize,
    ) -> Result<(f64, f64), InitializationFailure> {
        let state = &workspace.unknowns[self.slots(leg, END)];
        let inputs = self.inputs(&workspace.legs[leg], state, values);
        let slopes = self.body.guard_slopes(guard, &inputs).map_err(undefined)?;
        let sizes = &self.parameters.sizes;
        let read = |(unknown, slope): &(usize, f64)| slope.abs() * state[*unknown].abs();
        let size = |(unknown, slope): &(usize, f64)| slope.abs() * sizes[*unknown];
        Ok((
            slopes.unknowns.iter().map(read).sum(),
            slopes.unknowns.iter().map(size).sum(),
        ))
    }
    fn crossing_leg(workspace: &AdmissionWorkspace, crossing: usize) -> usize {
        (workspace.legs.iter())
            .position(|leg| leg.ends_at == Some(crossing))
            .expect("a crossing ends a leg")
    }
    /// How far a combination's sum at the start stands from the packet's, and the packet's sum. The first is summed
    /// term by term, so a total too large to hold still reads as unmoved.
    fn total(&self, start: &[f64], quantity: QuantityId) -> (f64, f64) {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let (mut moved, mut packet) = (Compensated::ZERO, Compensated::ZERO);
        for (row, coefficient) in combination.terms() {
            let then = Compensated::exact(self.parameters.starts[row.index()]).scaled(*coefficient);
            let now = Compensated::exact(start[row.index()]).scaled(*coefficient);
            moved = moved.plus(now.minus(then));
            packet = packet.plus(then);
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
    /// The ring's rows in its order: each leg's stage rows, a total in one end row of the last interval, a paced
    /// clock's root in its last end row, then each fit's anchor and each crossing's root.
    pub fn evaluate(&self, workspace: &AdmissionWorkspace) -> Result<Vec<f64>, InitializationFailure> {
        self.check(workspace)?;
        let values = self.values(workspace);
        let mut out = self.stages(workspace, &values)?;
        let start = self.start_state(workspace);
        for (slot, quantity) in self.replaced(workspace) {
            out[slot] = self.total(&start, quantity).0;
        }
        if let (Some(slot), Some(period)) = (self.clocked(workspace), self.period) {
            let last = slot / self.width();
            out[slot] = self.root(workspace, &values, last, period.guard)?;
        }
        let owed = &self.parameters.owed;
        for fit in &owed.fits {
            let anchor = &owed.anchors[fit.anchor];
            out.push(self.anchor_read(workspace, &values, anchor)? - anchor.value);
        }
        for (at, crossing) in workspace.topology.crossings.iter().enumerate() {
            let leg = Self::crossing_leg(workspace, at);
            out.push(self.root(workspace, &values, leg, crossing.guards[0])?);
        }
        Ok(out)
    }
    /// Over the slots `workspace` holds and then its crossing times, a column's `UnknownId::index` is its place. Only a
    /// stage's own tangent, a total's row and a crossing's time couple two legs, so the matrix is a block ring with a
    /// border of fits and times.
    pub fn linearize(&self, workspace: &AdmissionWorkspace) -> Result<Jacobian, InitializationFailure> {
        self.check(workspace)?;
        self.lines(workspace)
    }
    fn lines(&self, workspace: &AdmissionWorkspace) -> Result<Jacobian, InitializationFailure> {
        let values = self.values(workspace);
        let fits = &self.parameters.owed.fits;
        let crossings = &workspace.topology.crossings;
        let mut lines: Vec<Vec<(usize, f64)>> =
            Vec::with_capacity(workspace.unknowns.len() + crossings.len());
        for leg in 0..workspace.legs.len() {
            lines.extend(self.leg_lines(workspace, &values, leg, true)?);
        }
        let start = workspace.endpoint(0).start;
        for (slot, quantity) in self.replaced(workspace) {
            let terms = self.body.conserved_basis().combinations()[quantity.index()].terms();
            let term = |(row, coefficient): &(RowId, i64)| (start + row.index(), *coefficient as f64);
            lines[slot] = terms.iter().map(term).collect();
        }
        if let (Some(slot), Some(period)) = (self.clocked(workspace), self.period) {
            lines[slot] = self.root_line(workspace, &values, slot / self.width(), period.guard)?;
        }
        for fit in fits {
            let anchor = &self.parameters.owed.anchors[fit.anchor];
            lines.push(self.anchor_line(workspace, &values, anchor)?);
        }
        for (at, crossing) in crossings.iter().enumerate() {
            let leg = Self::crossing_leg(workspace, at);
            lines.push(self.root_line(workspace, &values, leg, crossing.guards[0])?);
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
    /// One leg's block in its own pattern, each entry read by its column from `entry`.
    fn local_block(&self, leg: usize, entry: impl Fn(usize, usize) -> f64) -> Jacobian {
        let (offsets, unknowns) = self.pattern();
        let first = leg * self.width();
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
    /// One leg's stages on their own: each row's residual, as `evaluate` gives it before a total or a clock root
    /// replaces a row, and the block of the leg's own unknowns, in the slots `leg_slots` names. An admission solver
    /// marches the ring with this, `seed`, `settle` and `relocate`, to reach a guess Newton can start from. Nothing here
    /// is judged: `admit` is.
    pub fn stage(
        &self,
        workspace: &AdmissionWorkspace,
        leg: LegHandle,
    ) -> Result<(Vec<f64>, Jacobian), InitializationFailure> {
        self.check(workspace)?;
        let values = self.values(workspace);
        let lines = self.leg_lines(workspace, &values, leg.index, false)?;
        let block = self.local_block(leg.index, |row, column| {
            (lines[row].iter())
                .find(|(at, _)| *at == column)
                .map_or(0.0, |(_, value)| *value)
        });
        let mut rows = Vec::with_capacity(self.width());
        self.stage_rows(workspace, &values, leg.index, &mut rows)?;
        Ok((rows, block))
    }
    /// One leg's rows alone, as `stage` gives them, for a solver that holds a factored block of the leg.
    pub fn stage_residuals(
        &self,
        workspace: &AdmissionWorkspace,
        leg: LegHandle,
    ) -> Result<Vec<f64>, InitializationFailure> {
        self.check(workspace)?;
        let values = self.values(workspace);
        let mut rows = Vec::with_capacity(self.width());
        self.stage_rows(workspace, &values, leg.index, &mut rows)?;
        Ok(rows)
    }
    /// Starts both of a leg's stages at the state it starts from.
    pub fn seed(&self, workspace: &mut AdmissionWorkspace, leg: LegHandle) {
        let origin = if self.interval.is_some() {
            self.origin(workspace, leg.index)
        } else {
            return;
        };
        for stage in [END, INTERIOR] {
            let slots = self.slots(leg.index, stage);
            workspace.unknowns[slots].copy_from_slice(&origin);
        }
    }
    /// One leg's own block of a `linearize`, in the pattern a `stage` block has, so a ring elimination factors it
    /// over the same plan. A cycle's last interval holds totals in place of rows, so its block is not this. Each entry
    /// is read by its column, because a row that states the packet's value holds fewer entries than the pattern.
    pub fn block(&self, ring: &Jacobian, leg: usize) -> Jacobian {
        let view = ring.view();
        let first = leg * self.width();
        self.local_block(leg, |row, column| {
            let line = view.row_offsets()[first + row]..view.row_offsets()[first + row + 1];
            (view.columns()[line.clone()].iter().zip(&view.values()[line]))
                .find(|(at, _)| at.index() == column)
                .map_or(0.0, |(_, value)| *value)
        })
    }
    /// Takes `share` of `correction` into `workspace`: each slot and each crossing time less its change. A time that
    /// would leave its interval or pass a neighbour stops short of it, by the clearance a leg keeps, and the result is
    /// `Clamped`: Newton cannot move a crossing across a knot, and a sweep, which reads where each guard crosses on the
    /// curves, does.
    pub fn apply(&self, workspace: &mut AdmissionWorkspace, correction: &Correction, share: f64) -> Applied {
        let mut clamped = false;
        let interval = self.interval.unwrap_or(0.0);
        let clearance = interval * crossings::CLEARANCE;
        for at in 0..workspace.topology.crossings.len() {
            let crossings = &workspace.topology.crossings;
            let here = crossings[at].interval;
            let before = at
                .checked_sub(1)
                .filter(|&before| crossings[before].interval == here);
            let after =
                Some(at + 1).filter(|&after| after < crossings.len() && crossings[after].interval == here);
            let low = before.map_or(0.0, |before| crossings[before].offset) + clearance;
            let high = after.map_or(interval, |after| {
                crossings[after].offset - correction.times[after] * share
            }) - clearance;
            let wanted = crossings[at].offset - share * correction.times[at];
            let offset = if wanted < low {
                low
            } else if wanted > high {
                high
            } else {
                wanted
            };
            clamped |= offset != wanted;
            if offset.is_finite() && offset > 0.0 && offset < interval {
                workspace.topology.crossings[at].offset = offset;
            }
        }
        workspace.place();
        for (unknown, change) in workspace.unknowns.iter_mut().zip(&correction.slots) {
            *unknown -= share * change;
        }
        if clamped {
            Applied::Clamped
        } else {
            Applied::Inside
        }
    }
    /// The first row whose stage has no pivot at the start state: a coupling loop such as `x = y`, `y = x`, a pressure
    /// with no gauge, or a closed row where no cycle gives its tangent. Structural matching cannot see these, and a
    /// run from a consistent start would hold whatever the packet gave them. One dense elimination, once a run.
    fn rankless(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
    ) -> Result<Option<RowId>, InitializationFailure> {
        let size = self.width();
        let lines = self.leg_lines(workspace, values, 0, false)?;
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
    /// The value an observer reads at grid point `point`: the state there after any firing, as a step that ends there
    /// leaves it.
    fn point_read(&self, workspace: &AdmissionWorkspace, point: usize, row: usize) -> Option<usize> {
        let after =
            (workspace.legs.iter()).position(|leg| leg.interval == (point + 1) % self.frames.len())?;
        let reset = workspace.legs[after].resets.contains(&RowId::at(row));
        (!reset).then(|| workspace.endpoint(point).start + row)
    }
    /// An instant reads its row at the start. A cycle mean reads its row's mean over the grid points, the samples an
    /// observer takes, summed as an observer sums a window, so the two read one number however far the row swings. A
    /// cycle time mean reads the stages' quadrature over every leg, divided by the period: the integral a clock's
    /// period closes on, and what an observer's integral of the row over its window approaches.
    fn anchor_read(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        anchor: &BoundAnchor,
    ) -> Result<f64, InitializationFailure> {
        let row = anchor.row.index();
        let read = |slot: Option<usize>| slot.map_or(0.0, |slot| workspace.unknowns[slot]);
        Ok(match anchor.statistic {
            AnchorStatistic::Instant => self.start_state(workspace)[row],
            AnchorStatistic::CycleMean => {
                let points = self.frames.len();
                let read = |point| Compensated::exact(read(self.point_read(workspace, point, row)));
                let sum = (0..points).map(read).fold(Compensated::ZERO, Compensated::plus);
                Compensated::between(Compensated::ZERO, sum, 1, points as u64).rounded()
            }
            AnchorStatistic::CycleTimeMean => {
                let _ = values;
                let mut sum = Compensated::ZERO;
                for (leg, held) in workspace.legs.iter().enumerate() {
                    for stage in [INTERIOR, END] {
                        let value = workspace.unknowns[self.slots(leg, stage).start + row];
                        sum = sum.plus(Compensated::exact(held.length * RADAU[END][stage] * value));
                    }
                }
                sum.rounded() / self.period_seconds()
            }
        })
    }
    fn period_seconds(&self) -> f64 {
        self.interval
            .map_or(1.0, |interval| interval * self.frames.len() as f64)
    }
    fn anchor_line(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        anchor: &BoundAnchor,
    ) -> Result<Vec<(usize, f64)>, InitializationFailure> {
        let row = anchor.row.index();
        let _ = values;
        Ok(match anchor.statistic {
            AnchorStatistic::Instant => {
                let first = self.first_leg(&workspace.legs);
                let reset = self.interval.is_some() && workspace.legs[first].resets.contains(&anchor.row);
                let slot = if self.interval.is_some() {
                    workspace.endpoint(0).start
                } else {
                    0
                };
                if reset {
                    Vec::new()
                } else {
                    vec![(slot + row, 1.0)]
                }
            }
            AnchorStatistic::CycleMean => {
                let points = self.frames.len();
                let share = 1.0 / points as f64;
                (0..points)
                    .filter_map(|point| self.point_read(workspace, point, row))
                    .map(|slot| (slot, share))
                    .collect()
            }
            AnchorStatistic::CycleTimeMean => {
                let period = self.period_seconds();
                let mut line = Vec::new();
                for (leg, held) in workspace.legs.iter().enumerate() {
                    let mut quadrature = 0.0;
                    for stage in [INTERIOR, END] {
                        let slot = self.slots(leg, stage).start + row;
                        add(&mut line, slot, held.length * RADAU[END][stage] / period);
                        quadrature += RADAU[END][stage] * workspace.unknowns[slot];
                    }
                    if let Some(end) = held.ends_at {
                        add(&mut line, self.root_column(workspace, end), quadrature / period);
                    }
                    if let Some(start) = held.starts_at {
                        add(
                            &mut line,
                            self.root_column(workspace, start),
                            -quadrature / period,
                        );
                    }
                }
                line
            }
        })
    }
    /// A combination's net boundary flow over the cycle, one column at a time as the ledger sums it, whichever rows
    /// the workspace spread it over, at each stage of each leg by its weight: the quadrature the end rows' sum closes.
    /// A row's own sum can lose a small flow beside large ones, and the ledger keeps it.
    fn net_flow(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        quantity: QuantityId,
    ) -> Result<f64, InitializationFailure> {
        let combination = &self.body.conserved_basis().combinations()[quantity.index()];
        let crossing = |terms: &[(RowId, i64)]| match *terms {
            [(row, sign)] => (combination.terms().iter())
                .filter(|(known, _)| *known == row)
                .map(|(_, coefficient)| sign * coefficient)
                .sum(),
            _ => 0,
        };
        let mut net = Compensated::ZERO;
        for (leg, held) in workspace.legs.iter().enumerate() {
            for &stage in self.stage_list() {
                let seconds = held.length * RADAU[END][stage];
                let inputs = self.inputs(held, &workspace.unknowns[self.slots(leg, stage)], values);
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
    fn recheck(
        &self,
        workspace: &AdmissionWorkspace,
    ) -> Result<Option<(AdmissionRow, f64)>, InitializationFailure> {
        let rows = self.body.rows();
        let inconsistent = |row| InitializationFailure::Inconsistent { row };
        let body_row = |at: usize| AdmissionRow::Body(rows[at].id());
        let size = rows.len();
        let unknowns = &workspace.unknowns;
        for (slot, &value) in unknowns[..workspace.fitted_slots().start].iter().enumerate() {
            if !rows[slot % size].admissible().holds(Compensated::exact(value)) {
                return Err(inconsistent(body_row(slot % size)));
            }
        }
        let owed = &self.parameters.owed;
        for (fit, &value) in owed.fits.iter().zip(self.fitted(workspace)) {
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
        let values = self.values(workspace);
        let stages = self.stages(workspace, &values)?;
        let sizes = &self.parameters.sizes;
        let stage = |slot: usize| {
            let row = slot % size;
            let judge = self.body.judged_at(row);
            let held = self.held(stages[slot].abs(), unknowns[slot - row + judge], sizes[judge]);
            held.ok_or(inconsistent(body_row(row)))
        };
        let replaced = |slot: &usize| {
            self.replaced(workspace).any(|(known, _)| known == *slot)
                || self.clocked(workspace) == Some(*slot)
        };
        for slot in (0..stages.len()).filter(|slot| !replaced(slot)) {
            note(body_row(slot % size), stage(slot)?);
        }
        let mut roots: Vec<(usize, usize)> = (0..workspace.topology.crossings.len())
            .map(|at| {
                (
                    Self::crossing_leg(workspace, at),
                    workspace.topology.crossings[at].guards[0],
                )
            })
            .collect();
        roots.extend(self.period.map(|period| {
            let first = self.first_leg(&workspace.legs);
            (Self::previous(&workspace.legs, first), period.guard)
        }));
        for (leg, guard) in roots {
            let variable = self.body.layout.modes[self.body.layout.guards[guard].variable].variable;
            let row = AdmissionRow::Crossing { variable };
            let gap = self.root(workspace, &values, leg, guard)?;
            let (read, reach) = self.root_scale(workspace, &values, leg, guard)?;
            note(row, self.held(gap.abs(), read, reach).ok_or(inconsistent(row))?);
        }
        // Every conserved total but charge, regulated or not: each stage's defect can pass while their sum over the
        // cycle does not. A net flow lands on the row its total replaced, so the flow is read first, to name it.
        let start = self.start_state(workspace);
        let combinations = self.body.conserved_basis().combinations();
        let books = (combinations.iter()).filter(|combination| combination.book() != Book::Charge);
        for quantity in books.filter(|_| self.interval.is_some()).map(Combination::id) {
            let (moved, packet) = self.total(&start, quantity);
            let flow = self.net_flow(workspace, &values, quantity)?.abs();
            let held = self.held(flow, packet + moved, self.total_size(quantity));
            note(
                AdmissionRow::Total(quantity),
                held.ok_or(InitializationFailure::NetBoundaryFlow { quantity })?,
            );
        }
        for slot in (0..stages.len()).filter(replaced) {
            note(body_row(slot % size), stage(slot)?);
        }
        let specified = (self.body.conserved_basis().combinations().iter())
            .filter(|combination| !combination.regulated() && combination.book() != Book::Charge);
        for quantity in specified.map(|combination| combination.id()) {
            let row = AdmissionRow::Total(quantity);
            let (moved, packet) = self.total(&start, quantity);
            let bulk = self.total_size(quantity);
            note(
                row,
                self.held(moved.abs(), packet, bulk).ok_or(inconsistent(row))?,
            );
        }
        for anchor in &owed.anchors {
            let row = AdmissionRow::Anchor(anchor.id);
            let held = self.held(
                (self.anchor_read(workspace, &values, anchor)? - anchor.value).abs(),
                anchor.value,
                sizes[anchor.row.index()],
            );
            note(row, held.ok_or(inconsistent(row))?);
        }
        for (fit, &value) in owed.fits.iter().zip(self.fitted(workspace)) {
            if !self.felt(workspace, &values, fit, value)? {
                let (key, binding) = (fit.key, fit.binding);
                return Err(AdmissionError::FitUnread { key, binding }.into());
            }
        }
        if self.interval.is_some() {
            self.settled(workspace)?;
            // Under drives that hold still all beat, a guard that fires on a periodic orbit times the beat itself, and
            // only the grid's samples would say where it starts: such a beat must name its clock.
            let still = self.frames.windows(2).all(|pair| pair[0] == pair[1]);
            let fires = workspace.topology.knots.iter().any(|knot| !knot.is_empty());
            if still && fires && self.period.is_none() {
                return Err(InitializationFailure::Unpaced);
            }
        }
        if let Some(row) = self.rankless(workspace, &values)? {
            return Err(InitializationFailure::NoRank { row });
        }
        // One stage can have rank while the whole system has a free level: a total nothing truly regulates, two fits
        // of one anchor's worth of information, or an autonomous cycle no clock fixes. The whole bordered ring is
        // factored, and a law with no slope here is refused, not passed over.
        let lines = self.lines(workspace)?;
        let right = vec![0.0; unknowns.len() + workspace.topology.crossings.len()];
        let free = self.ring(workspace, &lines, &right).err();
        if let Some(slot) = free {
            let border = workspace.fitted_slots().start;
            if slot < border {
                return Err(InitializationFailure::NoRank {
                    row: rows[slot % size].id(),
                });
            }
            let at = slot - border;
            if let Some(fit) = owed.fits.get(at) {
                return Err(InitializationFailure::NoRank {
                    row: rows[owed.anchors[fit.anchor].row.index()].id(),
                });
            }
            let guard = workspace.topology.crossings[at - owed.fits.len()].guards[0];
            let variable = self.body.layout.modes[self.body.layout.guards[guard].variable].variable;
            return Err(InitializationFailure::CrossingRank { variable });
        }
        Ok(worst)
    }
    fn felt(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        fit: &Fit,
        value: f64,
    ) -> Result<bool, InitializationFailure> {
        let rows = self.body.rows();
        for (leg, held) in workspace.legs.iter().enumerate() {
            for &stage in self.stage_list() {
                let state = &workspace.unknowns[self.slots(leg, stage)];
                let inputs = self.inputs(held, state, values);
                let slopes = (self.body.parameter_slopes(&inputs, &fit.slots)).map_err(undefined)?;
                let felt = |row: &RowId| {
                    let at = row.index();
                    let seconds = if self.interval.is_some() && !algebraic(rows[at].kind()) {
                        held.length
                    } else {
                        1.0
                    };
                    (seconds * slopes[at] * value).abs() > ROUNDING * state[at].abs()
                };
                if fit.rows.iter().any(felt) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    /// What an admitted start's hash reads of the cycle that made it: the scheme, every leg's interval, start,
    /// length and resets, every crossing's guards and time, every knot's firings, the clock that paces it and every
    /// stage coordinate. The modes follow from the start's, which the hash reads beside this, and the firings.
    fn witness(&self, workspace: &AdmissionWorkspace) -> Digest {
        let mut out = Canonical::new("ng-contract admission witness 1");
        out.text(if self.interval.is_some() {
            "radau-iia-2 located crossings"
        } else {
            "instant"
        });
        Witness {
            legs: &workspace.legs,
            topology: &workspace.topology,
            period: self.period.as_ref(),
            unknowns: &workspace.unknowns,
        }
        .seal(&mut out);
        out.digest()
    }
    /// The only constructor of `InitialRevision`. A workspace from another system, of another shape or with a
    /// topology its own values do not give is refused. The start state, grid point 0 after the firings that open the
    /// cycle, and each fit land in the revision, and the fits in `BoundParameters::values`, which the chain's first link
    /// seals, so a fit is bound once and a resume never refits. The other legs end here: they were this cycle's
    /// witness, and the hash reads a digest of them.
    pub fn admit(
        self,
        solved: AdmissionWorkspace,
        history: MethodHistory,
    ) -> Result<InitialRevision, InitializationFailure> {
        self.check(&solved)?;
        let worst = self.recheck(&solved)?;
        let values = self.values(&solved);
        let start = self.start_state(&solved);
        let fitted = self.fitted(&solved).to_vec();
        let witness = self.interval.map(|_| self.witness(&solved));
        let crossings = self.crossings(&solved);
        let first = if self.interval.is_some() {
            self.first_leg(&solved.legs)
        } else {
            0
        };
        let solved_modes = solved.legs[first].modes.clone();
        let Self {
            body,
            mut parameters,
            pending,
            anchor_moment: time,
            tolerance,
            frames: _,
            interval: _,
            pattern: _,
            period: _,
            identity: _,
            draws: _,
            factors: _,
        } = self;
        let unread = parameters.packet;
        parameters.packet = unread.reading(time, &start, &solved_modes, &fitted, witness.as_ref(), &history);
        parameters.values = values;
        let stated = core::mem::replace(&mut parameters.modes, solved_modes);
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
        let starts = body.rows().iter().zip(&start).zip(&parameters.starts);
        let modes = body.layout.modes.iter().zip(&parameters.modes).zip(&stated);
        let records = (owed.entries.iter().map(value_of))
            .chain(starts.map(start_of))
            .chain(modes.map(mode_of));
        let provenance = InitializationProvenance {
            values: records.collect(),
            tolerance,
            worst: worst.map(|(row, normalized)| (row, Compensated::exact(normalized))),
            crossings,
        };
        Ok(InitialRevision {
            parts: ImageParts {
                time,
                state: Candidate::of_coordinates(body, &start, parameters.modes.clone()),
                continuation: history,
                dense: DenseSegment::empty(),
                pending,
                parameters,
            },
            provenance,
            unread,
            start,
            fitted,
            witness,
        })
    }
}

/// What `InitializationSystem::witness` seals. Every field is named, so a new one cannot be left out by accident.
struct Witness<'w> {
    legs: &'w [Leg],
    topology: &'w Topology,
    period: Option<&'w PeriodIdentity>,
    unknowns: &'w [f64],
}
impl Seal for Witness<'_> {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            legs,
            topology,
            period,
            unknowns,
        } = self;
        out.number(legs.len() as u64);
        for leg in legs.iter() {
            let Leg {
                interval,
                start,
                length,
                modes: _,
                resets,
                ends_at,
                starts_at,
            } = leg;
            interval.seal(out);
            start.seal(out);
            length.seal(out);
            out.number(resets.len() as u64);
            for row in resets {
                row.index().seal(out);
            }
            ends_at.seal(out);
            starts_at.seal(out);
        }
        let Topology {
            opening: _,
            crossings,
            knots,
        } = topology;
        out.number(crossings.len() as u64);
        for crossing in crossings.iter() {
            let Located {
                interval,
                guards,
                offset,
            } = crossing;
            interval.seal(out);
            guards.seal(out);
            offset.seal(out);
        }
        out.number(knots.len() as u64);
        for knot in knots.iter() {
            knot.seal(out);
        }
        match period {
            Some(PeriodIdentity { clock, guard, anchor }) => {
                out.number(1);
                clock.seal(out);
                guard.seal(out);
                anchor.seal(out);
            }
            None => out.number(0),
        }
        unknowns.seal(out);
    }
}

/// A judged start. It holds what its hash read, so the same start can close under another core's history.
pub struct InitialRevision {
    parts: ImageParts,
    provenance: InitializationProvenance,
    unread: PacketHash,
    start: Vec<f64>,
    fitted: Vec<f64>,
    witness: Option<Digest>,
}
impl InitialRevision {
    /// This start under `history`: what `admit` would give the same witness with it, with no second judgment. Its
    /// state, modes, fits, pending inputs, witness and provenance are this one's, and its hash closes on `history`.
    /// Nothing else can change, so a start cannot be reused against another timeline.
    pub fn with_history(&self, history: MethodHistory) -> Self {
        let parts = &self.parts;
        let mut parameters = parts.parameters.copied();
        parameters.packet = (self.unread).reading(
            parts.time,
            &self.start,
            &parameters.modes,
            &self.fitted,
            self.witness.as_ref(),
            &history,
        );
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
            witness: self.witness,
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
    /// A guard's root at a crossing it fires, named by its mode variable.
    Crossing {
        variable: Slot,
    },
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
    /// A guard that crosses inside the interval that ends at this grid point and not where the workspace places it,
    /// or a cycle whose modes do not return to the packet's at the wrap.
    GuardSweep {
        point: usize,
    },
    /// A guard whose crossing no scan can settle: it touches zero with no slope, two guards cross at one time with no
    /// zero in common, it fires twice at one knot, or its gap has no bound on part of a leg.
    Unresolved {
        variable: Slot,
    },
    /// A paced cycle's clock that wraps inside the cycle as well as at its end.
    SecondWrap,
    /// A cycle under drives that hold still all beat whose guards fire, so it times itself, with no clock named.
    Unpaced,
    /// A row with no pivot on the solved workspace, in its own stage or on the ring, which structural
    /// matching cannot see.
    NoRank {
        row: RowId,
    },
    /// A crossing time the whole ring leaves free.
    CrossingRank {
        variable: Slot,
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
