use crate::accuracy::{faulted, net_at};
use crate::initialize::{admit_start, solved_start};
use crate::run::advance;
use crate::{
    AccuracyFailure, MemoryStore, ReferenceIntegrator, anchor_moment, arena_body, arena_lens, arena_packet,
};
use core::num::NonZeroU64;
use ng_contract::{
    AdmissionError, BoundaryEvent, BoundaryInput, BoundaryPort, ColumnId, Compensated, CompiledBody,
    CoupledSolver, Delivered, Digest, Dimensionless, ErrorScales, InitialRevision, InputId, Legs, Measure,
    MethodHistory, Posture, RowId, Seed, SimTime, Target,
};

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const REFINEMENT: u64 = 4;
const LENGTH_NS: u64 = 6 * SECOND;

/// The policy values a window judges by, all model defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPolicy {
    margin: u64,
    distance_floor: f64,
    landing_ns: NonZeroU64,
    grid_ns: u64,
    past_end_ns: u64,
    refinements_ns: [u64; 3],
    refinement_scales: ErrorScales,
}
impl WindowPolicy {
    pub(crate) fn of(longest: u64) -> Self {
        let scale = |value| Measure::<Dimensionless>::new(value).expect("a finite constant");
        Self {
            margin: 2,
            distance_floor: 1e-6,
            // At 1 µs the reference's Newton could not settle on a landing's span and the run faulted.
            landing_ns: NonZeroU64::MIN.saturating_add(100_000 - 1),
            grid_ns: 10 * MILLISECOND,
            past_end_ns: 500 * MILLISECOND,
            refinements_ns: [longest, longest / REFINEMENT, longest / (REFINEMENT * REFINEMENT)],
            refinement_scales: ErrorScales::uniform(scale(1e9), scale(1.0)).expect("positive constants"),
        }
    }
    /// How many times the coarser refinements' spread a core may stand from the finest, on values, transitions and
    /// instants in other modes.
    pub fn margin(&self) -> u64 {
        self.margin
    }
    /// The least a quantity's distance may be, as a share of the most the finest refinement holds of it.
    pub fn distance_floor(&self) -> Compensated {
        Compensated::of(&self.floor())
    }
    /// The refinements' minimum interval, which locates each landing, and the least a transition's distance may be.
    pub fn landing_ns(&self) -> NonZeroU64 {
        self.landing_ns
    }
    pub fn grid_ns(&self) -> u64 {
        self.grid_ns
    }
    /// How far past the window's end a run goes on recording transitions, so one just before the end finds its
    /// counterpart in a run that makes it just after.
    pub fn past_end_ns(&self) -> u64 {
        self.past_end_ns
    }
    /// The reference's intervals, coarsest first: the body's longest step, then each a quarter of the last.
    pub fn refinements_ns(&self) -> [u64; 3] {
        self.refinements_ns
    }
    /// So loose the estimate never refuses, and each refinement takes the step it names.
    pub fn refinement_scales(&self) -> ErrorScales {
        self.refinement_scales
    }
    pub(crate) fn finest(&self) -> ReferenceIntegrator {
        let [.., finest] = references(self.refinements_ns, self.refinement_scales, self.landing_ns);
        finest
    }
    fn floor(&self) -> Measure<Dimensionless> {
        Measure::new(self.distance_floor).expect("a finite constant")
    }
    fn seal(&self, bytes: &mut Vec<u8>) {
        let Self {
            margin,
            distance_floor,
            landing_ns,
            grid_ns,
            past_end_ns,
            refinements_ns,
            refinement_scales,
        } = *self;
        for value in [
            margin,
            distance_floor.to_bits(),
            landing_ns.get(),
            grid_ns,
            past_end_ns,
        ] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        for reference in references(refinements_ns, refinement_scales, landing_ns) {
            bytes.extend_from_slice(reference.resolution().digest().as_bytes());
        }
    }
}

fn references(intervals_ns: [u64; 3], scales: ErrorScales, floor: NonZeroU64) -> [ReferenceIntegrator; 3] {
    intervals_ns.map(|interval_ns| {
        ReferenceIntegrator::new(interval_ns)
            .with_scales(scales)
            .with_floor(floor)
    })
}

/// One quantity the window compares at each instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compared {
    /// An amount, a memory or a produced quantity.
    Row(RowId),
    Exchange(ColumnId),
}

struct Instant {
    at: SimTime,
    modes: Vec<usize>,
    values: Vec<Compensated>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Kind {
    mode: usize,
    entered: usize,
}

struct Transition {
    kind: Kind,
    at: SimTime,
}

struct Trace {
    instants: Vec<Instant>,
    transitions: Vec<Transition>,
}
impl Trace {
    fn apart(&self, target: &Trace) -> usize {
        let other = |(instant, target): &(&Instant, &Instant)| instant.modes != target.modes;
        self.instants.iter().zip(&target.instants).filter(other).count()
    }
    /// Only this way round: a coarser step passes over brief excursions a finer one resolves, and what a run steps
    /// over shows as instants in other modes.
    fn farthest_from(&self, target: &Trace, kind: Kind, end: SimTime) -> Option<(SimTime, u64)> {
        let mut worst: Option<(SimTime, u64)> = None;
        for transition in (self.transitions.iter()).filter(|seen| seen.kind == kind && seen.at <= end) {
            let nearest = (target.transitions.iter().filter(|other| other.kind == kind))
                .map(|other| transition.at.nanoseconds().abs_diff(other.at.nanoseconds()))
                .min()
                .unwrap_or(u64::MAX);
            if worst.is_none_or(|(_, apart)| nearest > apart) {
                worst = Some((transition.at, nearest));
            }
        }
        worst
    }
    fn kinds(&self) -> Vec<Kind> {
        let mut kinds: Vec<Kind> = self
            .transitions
            .iter()
            .map(|transition| transition.kind)
            .collect();
        kinds.sort();
        kinds.dedup();
        kinds
    }
}

fn high(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn larger(one: f64, other: f64) -> f64 {
    if other > one { other } else { one }
}

struct Measured {
    compared: Vec<Compared>,
    target: Trace,
    distances: Vec<Compensated>,
    transition_distances: Vec<(Kind, u64)>,
    most_apart: usize,
    end: SimTime,
}
impl Measured {
    fn new(
        policy: &WindowPolicy,
        compared: Vec<Compared>,
        target: Trace,
        rungs: &[&Trace],
        end: SimTime,
    ) -> Self {
        let (floor, margin) = (policy.floor(), policy.margin);
        let distances = (0..compared.len())
            .map(|quantity| {
                let mut most = Compensated::ZERO;
                for instant in &target.instants {
                    let held = instant.values[quantity].abs();
                    if !held.at_most(most) {
                        most = held;
                    }
                }
                let mut distance = most.times(&floor);
                for rung in rungs {
                    for (instant, exact) in rung.instants.iter().zip(&target.instants) {
                        if instant.modes != exact.modes {
                            continue;
                        }
                        let error = instant.values[quantity].minus(exact.values[quantity]).abs();
                        if !error.at_most(distance) {
                            distance = error;
                        }
                    }
                }
                distance.scaled(margin as i64)
            })
            .collect();
        let transition_distances = (target.kinds().into_iter())
            .map(|kind| {
                let most = (rungs.iter())
                    .filter_map(|rung| rung.farthest_from(&target, kind, end))
                    .map(|(_, apart)| apart)
                    .fold(policy.landing_ns.get(), u64::max);
                (kind, margin * most)
            })
            .collect();
        let most_apart = rungs.iter().map(|rung| rung.apart(&target)).max().unwrap_or(0);
        Self {
            compared,
            distances,
            transition_distances,
            most_apart: margin as usize * most_apart,
            target,
            end,
        }
    }
    fn verdict(&self, run: &Trace, case: usize) -> Result<Compensated, AccuracyFailure> {
        let apart = run.apart(&self.target);
        if apart > self.most_apart {
            return Err(AccuracyFailure::ModesApart {
                case,
                apart,
                most: self.most_apart,
            });
        }
        let mut worst = 0.0_f64;
        for (instant, target) in run.instants.iter().zip(&self.target.instants) {
            if instant.modes != target.modes {
                continue;
            }
            let quantities = self.compared.iter().zip(&self.distances);
            for ((value, exact), (quantity, distance)) in
                instant.values.iter().zip(&target.values).zip(quantities)
            {
                let error = value.minus(*exact).abs();
                if !error.at_most(*distance) {
                    return Err(AccuracyFailure::Outside {
                        case,
                        at: instant.at,
                        quantity: *quantity,
                        error,
                        distance: *distance,
                    });
                }
                if high(*distance) > 0.0 {
                    worst = larger(worst, high(error) / high(*distance));
                }
            }
        }
        for kind in run.kinds() {
            let distance_ns = (self.transition_distances.iter())
                .find(|(known, _)| *known == kind)
                .map_or(0, |(_, distance_ns)| *distance_ns);
            let Some((at, apart_ns)) = run.farthest_from(&self.target, kind, self.end) else {
                continue;
            };
            if apart_ns > distance_ns {
                return Err(AccuracyFailure::TransitionMoved {
                    case,
                    mode: kind.mode,
                    at,
                    apart_ns,
                    distance_ns,
                });
            }
            worst = larger(worst, apart_ns as f64 / distance_ns as f64);
        }
        let worst = Measure::<Dimensionless>::new(worst).expect("a share of a positive distance is finite");
        Ok(Compensated::of(&worst))
    }
    fn seal(&self, number: &mut impl FnMut(u64)) {
        let Self {
            compared,
            target,
            distances,
            transition_distances,
            most_apart,
            end,
        } = self;
        for quantity in compared {
            let (tag, index) = match quantity {
                Compared::Row(row) => (0, row.index()),
                Compared::Exchange(column) => (1, column.index()),
            };
            number(tag);
            number(index as u64);
        }
        let Trace {
            instants,
            transitions,
        } = target;
        for Instant { at, modes, values } in instants {
            number(at.nanoseconds());
            modes.iter().for_each(|mode| number(*mode as u64));
            values
                .iter()
                .flat_map(|value| value.bits())
                .for_each(&mut *number);
        }
        for Transition { kind, at } in transitions {
            number(kind.mode as u64);
            number(kind.entered as u64);
            number(at.nanoseconds());
        }
        distances
            .iter()
            .flat_map(|distance| distance.bits())
            .for_each(&mut *number);
        for (Kind { mode, entered }, distance_ns) in transition_distances {
            number(*mode as u64);
            number(*entered as u64);
            number(*distance_ns);
        }
        number(*most_apart as u64);
        number(end.nanoseconds());
    }
}

fn admission(error: AdmissionError) -> AccuracyFailure {
    AccuracyFailure::Admission(error.into())
}

/// §7's first window of the slice, measured once against `ReferenceIntegrator` at three refinements: admission at
/// 06:55, then a ramp from supine to seated past the first cycle, then a meal of water, sodium, potassium and
/// chloride, on a compressed synthetic day. The finest refinement stands for the solution. A core passes where it
/// stands no more than twice as far from it as the coarser two do, the coarsest being the body's longest step. Values
/// are compared where a run holds the finest's modes, and where the modes differ the window compares when each run
/// made its transitions and how many instants it spent in other modes.
pub struct SliceWindow {
    body: CompiledBody,
    start: Box<InitialRevision>,
    policy: WindowPolicy,
    measured: Measured,
}
impl SliceWindow {
    pub fn first() -> Result<Self, AccuracyFailure> {
        let body = arena_body().map_err(admission)?;
        let (packet, lens) = (
            arena_packet(&body).map_err(admission)?,
            arena_lens(&body).map_err(admission)?,
        );
        let boundary = BoundaryPort::new(timeline().map_err(admission)?).map_err(admission)?;
        let policy = WindowPolicy::of(body.longest_step_ns());
        let [coarse, middle, finest] =
            references(policy.refinements_ns, policy.refinement_scales, policy.landing_ns);
        let start = solved_start(&body, packet, boundary, &coarse, &lens, anchor_moment())
            .map(Box::new)
            .map_err(AccuracyFailure::Admission)?;
        let [coarse, middle, finest] =
            [&coarse, &middle, &finest].map(|reference| trace(&body, &start, &policy, reference));
        let measured = Measured::new(&policy, compared(&body), finest?, &[&coarse?, &middle?], end());
        Ok(Self {
            body,
            start,
            policy,
            measured,
        })
    }
    pub fn policy(&self) -> &WindowPolicy {
        &self.policy
    }
    pub(crate) fn judge(
        &self,
        core: &dyn CoupledSolver,
        case: usize,
    ) -> Result<Compensated, AccuracyFailure> {
        let run = trace(&self.body, &self.start, &self.policy, core)?;
        self.measured.verdict(&run, case)
    }
    /// Everything the window judges by, so two windows that would judge a core apart never share an identity.
    pub fn digest(&self) -> Digest {
        let Self {
            body,
            start,
            policy,
            measured,
        } = self;
        let mut bytes = body.model_hash().digest().as_bytes().to_vec();
        policy.seal(&mut bytes);
        bytes.extend_from_slice(start.packet_hash().digest().as_bytes());
        let mut number = |value: u64| bytes.extend_from_slice(&value.to_be_bytes());
        measured.seal(&mut number);
        Digest::of_canonical(&bytes)
    }
}

fn end() -> SimTime {
    SimTime::from_ns(anchor_moment().nanoseconds() + LENGTH_NS)
}

fn compared(body: &CompiledBody) -> Vec<Compared> {
    let mut columns: Vec<ColumnId> = (body.instances().iter())
        .filter_map(|instance| match instance.target() {
            Target::Column(column) => Some(column),
            Target::Row(_) => None,
        })
        .collect();
    columns.sort();
    columns.dedup();
    let rows = body.rows().iter().map(|row| Compared::Row(row.id()));
    rows.chain(columns.into_iter().map(Compared::Exchange)).collect()
}

fn trace(
    body: &CompiledBody,
    start: &InitialRevision,
    policy: &WindowPolicy,
    core: &dyn CoupledSolver,
) -> Result<Trace, AccuracyFailure> {
    let compared = compared(body);
    let history = MethodHistory::new(core.resolution().policy(), anchor_moment(), Vec::new());
    let start = start.with_history(history);
    let store = MemoryStore::default();
    let seed = Seed::new([1, 2, 3, 4]);
    let mut session = admit_start(body, core, start, seed, &store).map_err(AccuracyFailure::Admission)?;
    let end = end().nanoseconds();
    let mut trace = Trace {
        instants: Vec::new(),
        transitions: Vec::new(),
    };
    let mut next = anchor_moment().nanoseconds() + policy.grid_ns;
    let mut reached = vec![Compensated::ZERO; compared.len()];
    while session.record().accepted().time().nanoseconds() < end + policy.past_end_ns {
        let before = session.record().accepted().modes().to_vec();
        session = advance(session).map_err(faulted)?;
        let accepted = session.record().accepted();
        let (now, after) = (accepted.time(), accepted.modes());
        for (mode, (was, is)) in before.iter().zip(after).enumerate() {
            if was != is {
                let kind = Kind { mode, entered: *is };
                trace.transitions.push(Transition { kind, at: now });
            }
        }
        let segment = session.conservation().head().expect("a step was accepted");
        let dense = accepted.last_segment();
        while next <= now.nanoseconds() && next <= end {
            let at = SimTime::from_ns(next);
            // A segment ends at a firing's left limit, so its end instant reads the accepted state, as an
            // observation does.
            let read = |compared: &Compared| match *compared {
                Compared::Row(row) if at == now => Reading::Level(accepted.value(row)),
                Compared::Row(row) => Reading::Level(
                    dense
                        .sample(row, at)
                        .expect("a compared instant is inside the segment"),
                ),
                Compared::Exchange(column) => Reading::Total(net_at(&segment, column, at)),
            };
            trace.instants.push(Instant {
                at,
                modes: if at == now { after.to_vec() } else { before.clone() },
                values: over_interval(compared.iter().map(read), &mut reached),
            });
            next += policy.grid_ns;
        }
    }
    Ok(trace)
}

/// A compared value at a grid instant: a level as it stands, or an exchange's total since admission.
enum Reading {
    Level(Compensated),
    Total(Compensated),
}

/// A level as it stands, and an exchange's net over the interval the instant ends. A running total carries its signed
/// error from interval to interval, where it cancels or grows by where the grid falls in the beat.
fn over_interval(readings: impl Iterator<Item = Reading>, reached: &mut [Compensated]) -> Vec<Compensated> {
    (readings.zip(reached.iter_mut()))
        .map(|(reading, before)| match reading {
            Reading::Level(value) => value,
            Reading::Total(total) => {
                let net = total.minus(*before);
                *before = total;
                net
            }
        })
        .collect()
}

fn after(ns: u64, tag: &[u8], payload: BoundaryInput) -> BoundaryEvent {
    let at = SimTime::from_ns(anchor_moment().nanoseconds() + ns);
    BoundaryEvent::new(InputId::new(Digest::of_canonical(tag)), at, payload)
}

fn timeline() -> Result<Vec<BoundaryEvent>, AdmissionError> {
    let mut events = Vec::new();
    for step in 0..=10_u8 {
        let legs = if step < 5 {
            Legs::Horizontal
        } else {
            Legs::SeatedDependent
        };
        let posture = Posture::new(Measure::new(f64::from(step) / 10.0)?, legs)?;
        let at = 1_200 * MILLISECOND + u64::from(step) * 100 * MILLISECOND;
        events.push(after(at, &[b'p', step], BoundaryInput::Posture(posture)));
    }
    let ends = SimTime::from_ns(anchor_moment().nanoseconds() + 4_500 * MILLISECOND);
    let meal = |tag: &[u8], payload| after(2_500 * MILLISECOND, tag, payload);
    let water = Delivered {
        boundary: "gut water",
        amount: Measure::new(300.0)?,
        ends,
    };
    let sodium = Delivered {
        boundary: "gut sodium",
        amount: Measure::new(50.0)?,
        ends,
    };
    let potassium = Delivered {
        boundary: "gut potassium",
        amount: Measure::new(2.0)?,
        ends,
    };
    let chloride = Delivered {
        boundary: "gut chloride",
        amount: Measure::new(52.0)?,
        ends,
    };
    events.push(meal(b"meal water", BoundaryInput::Water(water)));
    events.push(meal(b"meal sodium", BoundaryInput::Sodium(sodium)));
    events.push(meal(b"meal potassium", BoundaryInput::Potassium(potassium)));
    events.push(meal(b"meal chloride", BoundaryInput::Chloride(chloride)));
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{Instant, Kind, Measured, Reading, Trace, Transition, WindowPolicy, over_interval};
    use crate::AccuracyFailure;
    use ng_contract::{Compensated, Dimensionless, Measure, SimTime};

    const MILLISECOND: u64 = 1_000_000;
    const SECOND: u64 = 1_000_000_000;

    /// No compared values, so only modes and transitions decide: on the body a moved transition moves values too,
    /// and the values are compared first.
    fn modes_only(modes: &[usize], transitions: &[((usize, usize), u64)]) -> Trace {
        let instants = (modes.iter().zip(1..))
            .map(|(mode, step)| Instant {
                at: SimTime::from_ns(step * 10 * MILLISECOND),
                modes: vec![*mode],
                values: Vec::new(),
            })
            .collect();
        let transitions = (transitions.iter())
            .map(|&((mode, entered), at_ms)| Transition {
                kind: Kind { mode, entered },
                at: SimTime::from_ns(at_ms * MILLISECOND),
            })
            .collect();
        Trace {
            instants,
            transitions,
        }
    }

    fn measured() -> Measured {
        let target = modes_only(&[0, 1, 1, 0], &[((0, 1), 15), ((0, 0), 35)]);
        let rung = modes_only(&[0, 0, 1, 0], &[((0, 1), 18), ((0, 0), 34)]);
        Measured::new(
            &WindowPolicy::of(SECOND),
            Vec::new(),
            target,
            &[&rung],
            SimTime::from_ns(40 * MILLISECOND),
        )
    }

    #[test]
    fn an_exchange_is_compared_by_its_net_over_each_interval_and_a_level_as_it_stands() {
        let value = |v: f64| Compensated::of(&Measure::<Dimensionless>::new(v).expect("finite"));
        let mut reached = [Compensated::ZERO; 2];
        let read =
            |level: f64, total: f64| [Reading::Level(value(level)), Reading::Total(value(total))].into_iter();
        let instants = [(5.0, 1.0), (7.0, 3.0), (7.0, 6.0)]
            .map(|(level, total)| over_interval(read(level, total), &mut reached));
        let expected =
            [(5.0, 1.0), (7.0, 2.0), (7.0, 3.0)].map(|(level, net)| vec![value(level), value(net)]);
        assert_eq!(instants, expected);
    }

    #[test]
    fn a_transition_moved_past_twice_the_rungs_spread_fails_and_one_inside_it_passes() {
        let window = measured();
        let inside = modes_only(&[0, 0, 1, 0], &[((0, 1), 21), ((0, 0), 35)]);
        assert!(window.verdict(&inside, 0).is_ok());
        let moved = modes_only(&[0, 0, 1, 0], &[((0, 1), 22), ((0, 0), 35)]);
        assert!(matches!(
            window.verdict(&moved, 0),
            Err(AccuracyFailure::TransitionMoved {
                apart_ns: 7_000_000,
                distance_ns: 6_000_000,
                ..
            })
        ));
    }

    #[test]
    fn a_kind_the_reference_never_makes_is_as_far_as_can_be() {
        let window = measured();
        let spurious = modes_only(&[0, 1, 1, 0], &[((0, 1), 15), ((0, 2), 20), ((0, 0), 35)]);
        assert!(matches!(
            window.verdict(&spurious, 0),
            Err(AccuracyFailure::TransitionMoved {
                apart_ns: u64::MAX,
                distance_ns: 0,
                ..
            })
        ));
    }

    #[test]
    fn a_run_may_step_over_what_the_reference_makes_until_its_other_modes_pass_twice_the_rungs() {
        let window = measured();
        let missed = modes_only(&[0, 0, 0, 0], &[]);
        assert!(window.verdict(&missed, 0).is_ok());
        let elsewhere = modes_only(&[1, 0, 0, 1], &[]);
        assert!(matches!(
            window.verdict(&elsewhere, 0),
            Err(AccuracyFailure::ModesApart {
                apart: 4,
                most: 2,
                ..
            })
        ));
    }
}
