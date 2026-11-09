use super::{
    AdmissionWorkspace, END, INTERIOR, InitializationFailure, InitializationSystem, LegHandle, Located,
    Topology, undefined,
};
use crate::Slot;
use crate::evaluate::{Bounds, Inputs, Jet};

/// The least share of its interval a leg keeps: a crossing closer than this to a knot fires at the knot.
pub(crate) const CLEARANCE: f64 = 1.0 / 4_294_967_296.0;
/// Halvings of a leg the scan may take before it calls a crossing unresolved: a few hundred times rounding on the
/// leg's own clock.
const DEPTH: u32 = 44;
/// A guard's rounding, as a share of the terms its gap is computed from: a gap within it reads as zero however slowly the
/// guard moves.
const ROUNDING: f64 = 64.0 * f64::EPSILON;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relocated {
    Settled,
    Split,
    Merged,
}

/// One leg's dense curve: the quadratic through the state it starts from, its interior stage at a third and its end,
/// in the share of the leg `s` from 0 to 1. Radau IIA is a collocation method, so this is the curve its stages lie on.
struct Curve {
    a: Vec<f64>,
    b: Vec<f64>,
    c: Vec<f64>,
}
impl Curve {
    fn slope(&self, s: f64) -> Vec<f64> {
        (self.b.iter().zip(&self.c))
            .map(|(b, c)| b + 2.0 * s * c)
            .collect()
    }
    fn at(&self, s: f64) -> Vec<f64> {
        (self.a.iter().zip(&self.b).zip(&self.c))
            .map(|((a, b), c)| a + s * (b + s * c))
            .collect()
    }
    fn jet(&self, row: usize, lo: f64, hi: f64) -> Jet {
        let (a, b, c) = (self.a[row], self.b[row], self.c[row]);
        let value = |s: f64| a + s * (b + s * c);
        let mut points = vec![value(lo), value(hi)];
        if c != 0.0 {
            let vertex = -b / (2.0 * c);
            if lo < vertex && vertex < hi {
                points.push(value(vertex));
            }
        }
        let least = points
            .iter()
            .copied()
            .fold(f64::INFINITY, |least, v| if v < least { v } else { least });
        let most = points
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |most, v| if v > most { v } else { most });
        let slopes = [b + 2.0 * c * lo, b + 2.0 * c * hi];
        let (low, high) = if slopes[0] < slopes[1] {
            (slopes[0], slopes[1])
        } else {
            (slopes[1], slopes[0])
        };
        let wide = Bounds {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        };
        Jet {
            value: Bounds::around(least, most).unwrap_or(wide),
            slope: Bounds::around(low, high).unwrap_or(wide),
        }
    }
}

enum Event {
    Clear,
    Cross {
        share: f64,
        guards: Vec<usize>,
    },
    Unarmed,
    /// The crossing that ends the leg stands short of its guard's zero with no room left in its interval to reach it,
    /// or its guard was past zero where the leg starts, so it crossed before. Where the leg reaches the interval's knot
    /// on that side, the crossing goes across it, to the neighbouring interval and the offset its guard's slope there
    /// gives.
    Gone {
        across: Option<(usize, f64)>,
    },
}

/// A guard read at one point of a leg: its gap, the gap's slope per share of the leg, and the size of the terms the gap
/// is computed from.
struct Reading {
    gap: f64,
    rising: f64,
    terms: f64,
}
impl Reading {
    /// Whether the guard stands at its zero: within its rounding, or with its root nearer along its slope than `room`,
    /// the share of the leg a crossing must keep from a knot. Both scale with the guard, and neither reads how far its
    /// value lies from zero elsewhere, so a small swing far from zero keeps the crossings of a large one.
    fn stands(&self, room: f64) -> bool {
        let off = self.gap.abs();
        off <= ROUNDING * self.terms || off <= self.rising.abs() * room
    }
}

enum Found {
    Clear,
    At(f64),
    Unresolved,
}

struct Knot {
    offset: f64,
    guards: Option<Vec<usize>>,
    fired: Vec<usize>,
}

impl InitializationSystem<'_> {
    fn variable(&self, guard: usize) -> Slot {
        self.body.layout.modes[self.body.layout.guards[guard].variable].variable
    }
    fn unresolved(&self, guard: usize) -> InitializationFailure {
        InitializationFailure::Unresolved {
            variable: self.variable(guard),
        }
    }
    fn curve(&self, workspace: &AdmissionWorkspace, leg: usize) -> Curve {
        let start = self.origin(workspace, leg);
        let interior = &workspace.unknowns[self.slots(leg, INTERIOR)];
        let end = &workspace.unknowns[self.slots(leg, END)];
        let mut curve = Curve {
            a: start.clone(),
            b: Vec::with_capacity(start.len()),
            c: Vec::with_capacity(start.len()),
        };
        for ((p, u), v) in start.iter().zip(interior).zip(end) {
            curve.b.push(-4.0 * p + 4.5 * u - 0.5 * v);
            curve.c.push(3.0 * p - 4.5 * u + 1.5 * v);
        }
        curve
    }
    fn gap_in(
        &self,
        guard: usize,
        interval: usize,
        state: &[f64],
        modes: &[usize],
        values: &[f64],
    ) -> Result<f64, InitializationFailure> {
        let inputs = Inputs {
            parameters: values,
            frame: &self.frames[self.before(interval)],
            unknowns: state,
            modes,
            draws: &self.draws,
            factors: &self.factors,
        };
        self.body.guard_gap(guard, &inputs).map_err(undefined)
    }
    /// The least share of `leg` a crossing keeps from either of its knots.
    fn room(&self, workspace: &AdmissionWorkspace, leg: usize) -> f64 {
        CLEARANCE * self.interval.unwrap_or(0.0) / workspace.legs[leg].length
    }
    #[allow(clippy::too_many_arguments)]
    fn reading(
        &self,
        guard: usize,
        interval: usize,
        state: &[f64],
        modes: &[usize],
        values: &[f64],
        direction: &[f64],
    ) -> Result<Reading, InitializationFailure> {
        let gap = self.gap_in(guard, interval, state, modes, values)?;
        let inputs = Inputs {
            parameters: values,
            frame: &self.frames[self.before(interval)],
            unknowns: state,
            modes,
            draws: &self.draws,
            factors: &self.factors,
        };
        let slopes = self.body.guard_slopes(guard, &inputs).map_err(undefined)?;
        let (mut rising, mut terms) = (0.0, 0.0);
        for &(row, slope) in &slopes.unknowns {
            rising += slope * direction[row];
            terms += slope.abs() * state[row].abs();
        }
        Ok(Reading { gap, rising, terms })
    }
    fn earliest(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        curve: &Curve,
        guard: usize,
        ends_here: bool,
    ) -> Result<Found, InitializationFailure> {
        let held = &workspace.legs[leg];
        let gap = |share: f64| self.gap_in(guard, held.interval, &curve.at(share), &held.modes, values);
        let stands_at_end = || -> Result<bool, InitializationFailure> {
            let (end, slope) = (curve.at(1.0), curve.slope(1.0));
            let reading = self.reading(guard, held.interval, &end, &held.modes, values, &slope)?;
            Ok(reading.stands(self.room(workspace, leg)))
        };
        let still = curve.at(0.0);
        let inputs = self.inputs(held, &still, values);
        let mut pieces = vec![(0.0, 1.0, 0)];
        while let Some((lo, hi, depth)) = pieces.pop() {
            let jet = self
                .body
                .guard_enclosure(guard, &inputs, &|row| curve.jet(row, lo, hi));
            let split = |pieces: &mut Vec<(f64, f64, u32)>| {
                let middle = 0.5 * (lo + hi);
                pieces.push((middle, hi, depth + 1));
                pieces.push((lo, middle, depth + 1));
            };
            let Some(jet) = jet else {
                if depth >= DEPTH {
                    return Ok(Found::Unresolved);
                }
                split(&mut pieces);
                continue;
            };
            if jet.value.hi < 0.0 {
                continue;
            }
            // Past zero over the whole piece: from the leg's start a knot fires it, and later it crossed at the piece's
            // start, which a piece before it bounds.
            if jet.value.lo >= 0.0 {
                return Ok(Found::At(lo));
            }
            if ends_here && hi == 1.0 && jet.slope.lo > 0.0 && stands_at_end()? {
                continue;
            }
            if jet.slope.lo > 0.0 {
                if gap(hi)? < 0.0 {
                    continue;
                }
                return Ok(Found::At(self.bisect(&gap, lo, hi)?));
            }
            // A falling piece holds no rising crossing: one at its start was found on the piece before it, or is a knot's.
            if jet.slope.hi < 0.0 {
                continue;
            }
            if depth >= DEPTH {
                return Ok(Found::Unresolved);
            }
            split(&mut pieces);
        }
        Ok(Found::Clear)
    }
    fn bisect(
        &self,
        gap: &dyn Fn(f64) -> Result<f64, InitializationFailure>,
        lo: f64,
        hi: f64,
    ) -> Result<f64, InitializationFailure> {
        if gap(lo)? >= 0.0 {
            return Ok(lo);
        }
        let (mut below, mut above) = (lo, hi);
        for _ in 0..64 {
            let middle = 0.5 * (below + above);
            if middle <= below || middle >= above {
                break;
            }
            if gap(middle)? >= 0.0 {
                above = middle
            } else {
                below = middle
            }
        }
        Ok(above)
    }
    fn same_zero(&self, one: usize, other: usize) -> bool {
        let guards = &self.body.layout.guards;
        let (one, other) = (&guards[one], &guards[other]);
        one.crossing == other.crossing && one.code.ops == other.code.ops && one.inputs == other.inputs
    }
    /// The first crossing inside one leg of any guard its modes arm, but a paced cycle's clock, which crosses only
    /// at the wrap, or the crossing that ends the leg standing off its root, which lies before the next knot.
    fn scan(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
    ) -> Result<Event, InitializationFailure> {
        let held = &workspace.legs[leg];
        let guards = &self.body.layout.guards;
        let ending = held.ends_at.map(|at| &workspace.topology.crossings[at]);
        let curve = self.curve(workspace, leg);
        let room = self.room(workspace, leg);
        if let Some(crossing) = ending {
            let root = crossing.guards[0];
            if !guards[root].armed(&held.modes) {
                return Ok(Event::Unarmed);
            }
            let interval = self.interval.unwrap_or(0.0);
            let edge = 4.0 * CLEARANCE * interval;
            let points = self.frames.len();
            let read = |share: f64| {
                let (state, slope) = (curve.at(share), curve.slope(share));
                self.reading(root, held.interval, &state, &held.modes, values, &slope)
            };
            let reading = read(1.0)?;
            let (gap, rising) = (reading.gap, reading.rising);
            if gap < 0.0 && !reading.stands(room) {
                let end = held.start + held.length;
                let ahead = -gap / rising * held.length;
                // A leg's root moves with its length, so one just past its end keeps the crossing for Newton to place.
                let next = (workspace.legs.get(leg + 1))
                    .filter(|next| next.interval == held.interval)
                    .map_or(interval, |next| next.start + next.length);
                if rising > 0.0 && end + ahead < next - edge {
                    return Ok(Event::Cross {
                        share: 1.0 + ahead / held.length,
                        guards: crossing.guards.clone(),
                    });
                }
                let at_edge = interval - end <= edge;
                let across = (at_edge && rising > 0.0)
                    .then(|| ((held.interval + 1) % points, ahead.clamp(edge, 0.5 * interval)));
                return Ok(Event::Gone { across });
            }
            let reading = read(0.0)?;
            let (gap, rising) = (reading.gap, reading.rising);
            if gap > 0.0 && !reading.stands(room) {
                let across = (held.start == 0.0 && rising > 0.0).then(|| {
                    let behind = gap / rising * held.length;
                    (
                        (held.interval + points - 1) % points,
                        (interval - behind).clamp(0.5 * interval, interval - edge),
                    )
                });
                return Ok(Event::Gone { across });
            }
        }
        let reference = self.period.map(|period| period.guard);
        let mut first: Option<(f64, Vec<usize>)> = None;
        for guard in
            (0..guards.len()).filter(|&guard| guards[guard].armed(&held.modes) && Some(guard) != reference)
        {
            let ends_here = ending.is_some_and(|crossing| crossing.guards.contains(&guard));
            let share = match self.earliest(workspace, values, leg, &curve, guard, ends_here)? {
                Found::Clear => continue,
                Found::Unresolved => return Err(self.unresolved(guard)),
                Found::At(share) => share,
            };
            if share < room || 1.0 - share < room {
                continue;
            }
            match &mut first {
                Some((known, batch)) if *known == share => {
                    if !self.same_zero(batch[0], guard) {
                        return Err(self.unresolved(guard));
                    }
                    batch.push(guard);
                }
                Some((known, _)) if *known < share => {}
                _ => first = Some((share, vec![guard])),
            }
        }
        Ok(match first {
            Some((share, guards)) => Event::Cross { share, guards },
            None => Event::Clear,
        })
    }
    /// What fires at the start of `leg` after modes `before`: the crossing's guards there, or a paced cycle's clock at
    /// the wrap, and every other guard those modes arm whose gap is not below zero at the knot, read in the leg's own
    /// drive frame; then, in turn, each guard armed before the knot that the resets and modes so far leave crossed. A
    /// guard only a firing arms waits for the next knot, as it waits for the next step in a run. Each firing runs
    /// through `Layout::fire`, so its order is the run's. A guard at its own zero there fires only if the curve that
    /// reaches the knot was raising it. A guard that would fire twice at one knot is unresolved.
    fn knot(
        &self,
        workspace: &AdmissionWorkspace,
        values: &[f64],
        leg: usize,
        before: &[usize],
    ) -> Result<(Vec<usize>, Vec<usize>), InitializationFailure> {
        let held = &workspace.legs[leg];
        let guards = &self.body.layout.guards;
        let previous = Self::previous(&workspace.legs, leg);
        let mut state = workspace.unknowns[self.slots(previous, END)].to_vec();
        let reference = self.period.map(|period| period.guard);
        let forced: Vec<usize> = match held.starts_at {
            Some(at) => workspace.topology.crossings[at].guards.clone(),
            None if leg == self.first_leg(&workspace.legs) => reference.into_iter().collect(),
            None => Vec::new(),
        };
        let arriving = self.curve(workspace, previous).slope(1.0);
        let room = self.room(workspace, previous);
        let armed: Vec<bool> = guards.iter().map(|guard| guard.armed(before)).collect();
        let mut modes = before.to_vec();
        let mut fired: Vec<usize> = Vec::new();
        for round in 0..=guards.len() {
            let mut crossed = Vec::with_capacity(guards.len());
            for (at, guard) in guards.iter().enumerate() {
                let forcing = round == 0 && forced.contains(&at);
                let open = Some(at) != reference && armed[at] && guard.armed(&modes);
                crossed.push(
                    forcing
                        || (open
                            && self.crossed_at(
                                at,
                                held.interval,
                                (&state, &arriving),
                                &modes,
                                values,
                                room,
                                round == 0,
                            )?),
                );
            }
            let batch = self.body.layout.fired(&mut modes, &crossed);
            if batch.is_empty() {
                return Ok((fired, modes));
            }
            for &guard in &batch {
                if fired.contains(&guard) {
                    return Err(self.unresolved(guard));
                }
                if let Some(row) = guards[guard].resets {
                    state[row.index()] = 0.0;
                }
            }
            fired.extend(batch);
        }
        Err(self.unresolved(fired[0]))
    }
    /// Whether a guard is crossed at a knot: past its zero and not standing at it, or, when `rising_fires`, standing at
    /// its zero and rising along the curve that reaches the knot, whose slope per share of its leg `arriving` gives. A
    /// guard at its zero and falling, such as the one a firing's complement leaves, does not fire.
    #[allow(clippy::too_many_arguments)]
    fn crossed_at(
        &self,
        guard: usize,
        interval: usize,
        (state, arriving): (&[f64], &[f64]),
        modes: &[usize],
        values: &[f64],
        room: f64,
        rising_fires: bool,
    ) -> Result<bool, InitializationFailure> {
        let reading = self.reading(guard, interval, state, modes, values, arriving)?;
        if !reading.stands(room) {
            return Ok(reading.gap > 0.0);
        }
        Ok(rising_fires && reading.rising > 0.0)
    }
    fn modes_before(&self, workspace: &AdmissionWorkspace, leg: usize) -> Vec<usize> {
        workspace.legs[Self::previous(&workspace.legs, leg)].modes.clone()
    }
    pub(super) fn settled(&self, workspace: &AdmissionWorkspace) -> Result<(), InitializationFailure> {
        let values = self.values(workspace);
        let first = self.first_leg(&workspace.legs);
        for (leg, held) in workspace.legs.iter().enumerate() {
            let point = InitializationFailure::GuardSweep { point: held.interval };
            let (fired, modes) = self.knot(workspace, &values, leg, &self.modes_before(workspace, leg))?;
            if leg == first && modes != held.modes {
                return Err(InitializationFailure::GuardSweep { point: 0 });
            }
            if fired != workspace.topology.knots[leg] || modes != held.modes {
                return Err(point);
            }
            match self.scan(workspace, &values, leg)? {
                Event::Clear => {}
                Event::Cross { .. } | Event::Unarmed | Event::Gone { .. } => return Err(point),
            }
        }
        let Some(period) = self.period else {
            return Ok(());
        };
        let last = Self::previous(&workspace.legs, first);
        for (leg, held) in workspace.legs.iter().enumerate() {
            if !self.body.layout.guards[period.guard].armed(&held.modes) {
                continue;
            }
            let curve = self.curve(workspace, leg);
            match self.earliest(workspace, &values, leg, &curve, period.guard, leg == last)? {
                Found::Clear => {}
                Found::At(_) => return Err(InitializationFailure::SecondWrap),
                Found::Unresolved => return Err(self.unresolved(period.guard)),
            }
        }
        Ok(())
    }
    fn knots(&self, workspace: &AdmissionWorkspace) -> Vec<Vec<Knot>> {
        let mut knots: Vec<Vec<Knot>> = (0..self.frames.len()).map(|_| Vec::new()).collect();
        for (leg, held) in workspace.legs.iter().enumerate() {
            let crossing = held.starts_at.map(|at| &workspace.topology.crossings[at]);
            knots[held.interval].push(Knot {
                offset: held.start,
                guards: crossing.map(|crossing| crossing.guards.clone()),
                fired: workspace.topology.knots[leg].clone(),
            });
        }
        knots
    }
    fn topology_of(&self, opening: Vec<usize>, mut knots: Vec<Vec<Knot>>) -> Topology {
        let clearance = CLEARANCE * self.interval.unwrap_or(0.0);
        let mut topology = Topology {
            opening,
            crossings: Vec::new(),
            knots: Vec::new(),
        };
        for interval in self.chronological() {
            let knots = &mut knots[interval];
            knots.sort_by(|one, other| one.offset.total_cmp(&other.offset));
            let mut last = 0.0;
            for knot in knots.drain(..) {
                if let Some(guards) = knot.guards {
                    let room = self.interval.unwrap_or(0.0) - knot.offset;
                    if knot.offset - last < clearance || room < clearance {
                        continue;
                    }
                    last = knot.offset;
                    topology.crossings.push(Located {
                        interval,
                        guards,
                        offset: knot.offset,
                    });
                }
                topology.knots.push(knot.fired);
            }
        }
        topology
    }
    /// `topology` without each crossing whose guard the modes before it do not arm, in turn until none is left: such a
    /// crossing fires nothing, and its root would only pin a time no guard owns.
    fn armed(&self, mut topology: Topology) -> Topology {
        let guards = &self.body.layout.guards;
        while let Some(legs) = self.derive(&topology) {
            let idle = (legs.iter()).find_map(|leg| {
                let at = leg.ends_at?;
                (!guards[topology.crossings[at].guards[0]].armed(&leg.modes)).then_some(at)
            });
            let Some(at) = idle else {
                break;
            };
            let after = (legs.iter())
                .position(|leg| leg.starts_at == Some(at))
                .expect("a crossing starts a leg");
            topology.crossings.remove(at);
            topology.knots.remove(after);
        }
        topology
    }
    fn retopologize(&self, workspace: &mut AdmissionWorkspace, topology: Topology) {
        let Some(legs) = self.derive(&topology) else {
            return;
        };
        let curves: Vec<Curve> = (0..workspace.legs.len())
            .map(|leg| self.curve(workspace, leg))
            .collect();
        let read = |interval: usize, time: f64, left: bool| {
            let old = &workspace.legs;
            let holds = |leg: &&super::Leg| {
                let end = leg.start + leg.length;
                leg.interval == interval
                    && if left {
                        leg.start < time && time <= end
                    } else {
                        leg.start <= time && time < end
                    }
            };
            let at = (old.iter().position(|leg| holds(&leg)))
                .or_else(|| (old.iter()).rposition(|leg| leg.interval == interval && leg.start < time))
                .or_else(|| old.iter().position(|leg| leg.interval == interval))
                .expect("every interval has a leg");
            let share = (time - old[at].start) / old[at].length;
            curves[at].at(share)
        };
        let fits = workspace.unknowns[workspace.fitted_slots()].to_vec();
        let mut unknowns = Vec::with_capacity(legs.len() * self.width() + fits.len());
        for leg in &legs {
            unknowns.extend(read(leg.interval, leg.start + leg.length, true));
            unknowns.extend(read(leg.interval, leg.start + leg.length / 3.0, false));
        }
        unknowns.extend(fits);
        workspace.unknowns = unknowns;
        workspace.topology = topology;
        workspace.legs = legs;
    }
    /// Reads every knot's firings and every leg's first crossing off the workspace, and puts the topology they give in
    /// place: a crossing a scan finds inside a leg splits it, one whose root passed a knot moves across it, one whose
    /// guard is not armed or not reached goes. It moves no crossing within its knots: Newton's root rows do. Returns
    /// whether anything changed, so a solver alternates it with Newton until it returns `false`. It reads no tolerance;
    /// `admit` refuses a workspace it would change or whose crossing stands off its root.
    pub fn sweep(&self, workspace: &mut AdmissionWorkspace) -> Result<bool, InitializationFailure> {
        self.check(workspace)?;
        if self.interval.is_none() {
            return Ok(false);
        }
        let values = self.values(workspace);
        let mut knots = self.knots(workspace);
        let mut fired = Vec::with_capacity(workspace.legs.len());
        let mut events = Vec::with_capacity(workspace.legs.len());
        let first = self.first_leg(&workspace.legs);
        let mut opening = workspace.topology.opening.clone();
        for leg in 0..workspace.legs.len() {
            let (knot, modes) = self.knot(workspace, &values, leg, &self.modes_before(workspace, leg))?;
            if leg == first {
                opening = modes;
            }
            fired.push(knot);
            events.push(self.scan(workspace, &values, leg)?);
        }
        let mut place = vec![0; self.frames.len()];
        let mut ends: Vec<(usize, usize)> = Vec::with_capacity(workspace.legs.len());
        for held in &workspace.legs {
            ends.push((held.interval, place[held.interval]));
            place[held.interval] += 1;
        }
        let mut gone = Vec::new();
        for (leg, (event, fired)) in events.into_iter().zip(fired).enumerate() {
            let held = &workspace.legs[leg];
            let (interval, at) = ends[leg];
            knots[interval][at].fired = fired;
            match event {
                Event::Clear => {}
                Event::Cross { share, guards } => {
                    let ending = held
                        .ends_at
                        .filter(|&end| workspace.topology.crossings[end].guards == guards);
                    // A crossing whose root moved but stays between its knots keeps the topology: its root row, not the
                    // sweep, places it.
                    if ending.is_none() {
                        knots[interval].push(Knot {
                            offset: held.start + share * held.length,
                            fired: guards.clone(),
                            guards: Some(guards),
                        });
                    }
                }
                Event::Unarmed | Event::Gone { across: None } => gone.push((interval, at + 1)),
                Event::Gone {
                    across: Some((next, offset)),
                } => {
                    gone.push((interval, at + 1));
                    let guards = workspace.topology.crossings[held.ends_at.expect("an end")]
                        .guards
                        .clone();
                    knots[next].push(Knot {
                        offset,
                        fired: guards.clone(),
                        guards: Some(guards),
                    });
                }
            }
        }
        for &(interval, at) in gone.iter().rev() {
            knots[interval].remove(at);
        }
        let topology = self.armed(self.topology_of(opening, knots));
        if topology == workspace.topology {
            return Ok(false);
        }
        self.retopologize(workspace, topology);
        Ok(true)
    }
    /// A march's step after it solves `leg`'s stages: the knot at its start again, then its first crossing. A crossing
    /// inside it splits it, and one whose guard is unarmed or unreached goes. One whose root moved within its knots stays
    /// where it is, for Newton's root row to place.
    pub fn relocate(
        &self,
        workspace: &mut AdmissionWorkspace,
        leg: LegHandle,
    ) -> Result<Relocated, InitializationFailure> {
        self.check(workspace)?;
        if self.interval.is_none() {
            return Ok(Relocated::Settled);
        }
        let values = self.values(workspace);
        let index = leg.index;
        let held = workspace.legs[index].clone();
        let mut knots = self.knots(workspace);
        let at = (workspace.legs[..index].iter())
            .filter(|other| other.interval == held.interval)
            .count();
        let outcome = match self.scan(workspace, &values, index)? {
            Event::Clear => return Ok(Relocated::Settled),
            Event::Cross { share, guards } => {
                let ending = held
                    .ends_at
                    .filter(|&end| workspace.topology.crossings[end].guards == guards);
                if ending.is_some() {
                    return Ok(Relocated::Settled);
                }
                knots[held.interval].push(Knot {
                    offset: held.start + share * held.length,
                    fired: guards.clone(),
                    guards: Some(guards),
                });
                Relocated::Split
            }
            Event::Unarmed | Event::Gone { .. } => {
                knots[held.interval].remove(at + 1);
                Relocated::Merged
            }
        };
        let topology = self.armed(self.topology_of(workspace.topology.opening.clone(), knots));
        if topology == workspace.topology {
            return Ok(Relocated::Settled);
        }
        self.retopologize(workspace, topology);
        Ok(outcome)
    }
    /// Reads again what fires at the start of `leg` from the end of the leg before it, and returns whether that
    /// changed its modes. A crossing the new modes leave unarmed goes. A march calls it before it solves the leg.
    pub fn settle(
        &self,
        workspace: &mut AdmissionWorkspace,
        leg: LegHandle,
    ) -> Result<bool, InitializationFailure> {
        self.check(workspace)?;
        if self.interval.is_none() {
            return Ok(false);
        }
        let values = self.values(workspace);
        let before = self.modes_before(workspace, leg.index);
        let (fired, modes) = self.knot(workspace, &values, leg.index, &before)?;
        let first = leg.index == self.first_leg(&workspace.legs);
        if fired == workspace.topology.knots[leg.index] && (!first || modes == workspace.topology.opening) {
            return Ok(false);
        }
        let mut topology = workspace.topology.clone();
        topology.knots[leg.index] = fired;
        if first {
            topology.opening = modes;
        }
        let topology = self.armed(topology);
        if topology.crossings.len() == workspace.topology.crossings.len() {
            workspace.topology = topology;
            if let Some(legs) = self.derive(&workspace.topology) {
                workspace.legs = legs;
            }
        } else {
            self.retopologize(workspace, topology);
        }
        Ok(true)
    }
}
