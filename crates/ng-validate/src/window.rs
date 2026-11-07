use crate::accuracy::{faulted, net_at};
use crate::initialize::{admit_start, solution, start_from};
use crate::run::advance;
use crate::{
    AccuracyFailure, MemoryStore, ReferenceIntegrator, anchor_moment, arena_body, arena_lens, arena_packet,
};
use core::num::NonZeroU64;
use ng_contract::{
    AdmissionError, BoundaryEvent, BoundaryInput, BoundaryPort, ColumnId, Compensated, CompiledBody,
    CoupledSolver, Delivered, Digest, Dimensionless, ErrorScales, InputId, Legs, Measure, Posture, RowId,
    Seed, SimTime, Target,
};

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const REFINEMENT: u64 = 4;
/// At 1 µs the reference's Newton could not settle on a landing's span and the run faulted.
const LANDING_NS: NonZeroU64 = NonZeroU64::MIN.saturating_add(100_000 - 1);
const GRID_NS: u64 = 10 * MILLISECOND;
const LENGTH_NS: u64 = 6 * SECOND;
/// So a transition just before the end finds its counterpart in a run that makes it just after.
const TRANSITIONS_PAST_END_NS: u64 = 500 * MILLISECOND;
/// Model default.
const DISTANCE_FLOOR_SHARE: f64 = 1e-6;
/// Model default.
const MARGIN: u64 = 2;

/// One quantity the window compares at each instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compared {
    /// An amount, a memory or a produced quantity.
    Row(RowId),
    /// A column's exchange since admission.
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
    fn new(compared: Vec<Compared>, target: Trace, rungs: &[&Trace], end: SimTime) -> Self {
        let floor = Measure::<Dimensionless>::new(DISTANCE_FLOOR_SHARE).expect("a finite constant");
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
                distance.scaled(MARGIN as i64)
            })
            .collect();
        let transition_distances = (target.kinds().into_iter())
            .map(|kind| {
                let most = (rungs.iter())
                    .filter_map(|rung| rung.farthest_from(&target, kind, end))
                    .map(|(_, apart)| apart)
                    .fold(LANDING_NS.get(), u64::max);
                (kind, MARGIN * most)
            })
            .collect();
        let most_apart = rungs.iter().map(|rung| rung.apart(&target)).max().unwrap_or(0);
        Self {
            compared,
            distances,
            transition_distances,
            most_apart: MARGIN as usize * most_apart,
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
    solution: Vec<f64>,
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
        let solution =
            solution(&body, packet, boundary, &lens, anchor_moment()).map_err(AccuracyFailure::Admission)?;
        let longest = body.longest_step_ns();
        let [coarse, middle, finest] =
            [longest, longest / REFINEMENT, longest / (REFINEMENT * REFINEMENT)].map(fixed);
        let [coarse, middle, finest] =
            [&coarse, &middle, &finest].map(|reference| trace(&body, &solution, reference));
        let measured = Measured::new(compared(&body), finest?, &[&coarse?, &middle?], end());
        Ok(Self {
            body,
            solution,
            measured,
        })
    }
    pub(crate) fn judge(
        &self,
        core: &dyn CoupledSolver,
        case: usize,
    ) -> Result<Compensated, AccuracyFailure> {
        let run = trace(&self.body, &self.solution, core)?;
        self.measured.verdict(&run, case)
    }
    /// Everything the window judges by, so two windows that would judge a core apart never share an identity.
    pub fn digest(&self) -> Digest {
        let Self {
            body,
            solution,
            measured,
        } = self;
        let mut bytes = body.model_hash().digest().as_bytes().to_vec();
        let mut number = |value: u64| bytes.extend_from_slice(&value.to_be_bytes());
        solution.iter().for_each(|value| number(value.to_bits()));
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

fn trace(body: &CompiledBody, solution: &[f64], core: &dyn CoupledSolver) -> Result<Trace, AccuracyFailure> {
    let compared = compared(body);
    let boundary = BoundaryPort::new(timeline().map_err(admission)?).map_err(admission)?;
    let (packet, lens) = (
        arena_packet(body).map_err(admission)?,
        arena_lens(body).map_err(admission)?,
    );
    let start = start_from(body, packet, boundary, core, &lens, anchor_moment(), solution);
    let store = MemoryStore::default();
    let seed = Seed::new([1, 2, 3, 4]);
    let mut session = start
        .and_then(|start| admit_start(body, core, start, seed, &store))
        .map_err(AccuracyFailure::Admission)?;
    let end = end().nanoseconds();
    let mut trace = Trace {
        instants: Vec::new(),
        transitions: Vec::new(),
    };
    let mut next = anchor_moment().nanoseconds() + GRID_NS;
    while session.record().accepted().time().nanoseconds() < end + TRANSITIONS_PAST_END_NS {
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
            let read = |compared: &Compared| match *compared {
                Compared::Row(row) => dense
                    .sample(row, at)
                    .expect("a compared instant is inside the segment"),
                Compared::Exchange(column) => net_at(&segment, column, at),
            };
            trace.instants.push(Instant {
                at,
                modes: if at == now { after.to_vec() } else { before.clone() },
                values: compared.iter().map(read).collect(),
            });
            next += GRID_NS;
        }
    }
    Ok(trace)
}

/// Fixed step: scales this loose never refuse on the estimate, so each refinement takes the step it names.
fn fixed(interval_ns: u64) -> ReferenceIntegrator {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("a finite constant");
    let loose = ErrorScales::uniform(scale(1e9), scale(1.0)).expect("positive constants");
    ReferenceIntegrator::new(interval_ns)
        .with_scales(loose)
        .with_floor(LANDING_NS)
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
    use super::{Instant, Kind, Measured, Trace, Transition};
    use crate::AccuracyFailure;
    use ng_contract::SimTime;

    const MILLISECOND: u64 = 1_000_000;

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
        Measured::new(Vec::new(), target, &[&rung], SimTime::from_ns(40 * MILLISECOND))
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
