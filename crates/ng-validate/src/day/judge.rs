use super::table::{Bound, Course, Direction, Lean, Measured, ROWS, Row, Scoring, Span};
use super::trace::{Beat, DayTrace};
use super::{
    DayCheck, DayPlan, DayQuantity, EventKind, HOUR, MINUTE, Miss, Occurrence, SECOND, Unavailable, Verdict,
};
use ng_contract::SimTime;

const BASELINE: u64 = 10 * MINUTE;
const FIRST_PHASE: u64 = 30 * SECOND;
const NIGHT_HOUR: u64 = HOUR;
const FAST_STAY: usize = 3;
const SLOW_STAY: u64 = 5 * MINUTE;

/// A comparison mean less its baseline, with the floors it must clear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Change {
    pub(super) delta: f64,
    /// Three of the baseline's standard deviations.
    pub(super) noise: f64,
    pub(super) tolerance: f64,
}
impl Change {
    fn of(delta: f64, sd: f64, tolerance: f64) -> Self {
        Self {
            delta,
            noise: 3.0 * sd,
            tolerance,
        }
    }
    /// A response is more than three noise floors and at least the declared tolerance. A NaN is no response.
    pub(super) fn sign(self) -> Option<Direction> {
        let size = self.delta.abs();
        if !(size > self.noise && size >= self.tolerance) {
            return None;
        }
        Some(if self.delta > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Uncovered {
    EndsBefore(SimTime),
    StartsAfter(SimTime),
    /// The plan holds no event to bound the window.
    Unbounded,
}

/// What a check's report line shows. Each instant is measured from the event's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Shown {
    pub(super) change: Option<Change>,
    pub(super) dip: Option<Change>,
    pub(super) first: Option<u64>,
    pub(super) second: Option<u64>,
    pub(super) uncovered: Option<Uncovered>,
}

struct Pending {
    why: Unavailable,
    uncovered: Option<Uncovered>,
}
impl From<Unavailable> for Pending {
    fn from(why: Unavailable) -> Self {
        Self { why, uncovered: None }
    }
}
fn uncovered(uncovered: Uncovered) -> Pending {
    Pending {
        why: Unavailable::NotCovered,
        uncovered: Some(uncovered),
    }
}

type Judged = (Verdict, Shown);

#[derive(Clone, Copy, Debug)]
struct Stats {
    mean: f64,
    sd: f64,
    tolerance: f64,
}

/// NaN when either is.
fn larger(first: f64, second: f64) -> f64 {
    if first > second {
        first
    } else if second >= first {
        second
    } else {
        f64::NAN
    }
}

fn plus(at: SimTime, ns: u64) -> SimTime {
    SimTime::from_ns(at.nanoseconds().saturating_add(ns))
}
fn minus(at: SimTime, ns: u64) -> SimTime {
    SimTime::from_ns(at.nanoseconds().saturating_sub(ns))
}
fn elapsed(from: SimTime, to: SimTime) -> u64 {
    to.nanoseconds().saturating_sub(from.nanoseconds())
}

/// The beats that start and end inside `[start, end)`.
fn within(trace: &DayTrace, start: SimTime, end: SimTime) -> &[Beat] {
    let first = trace.beats.partition_point(|beat| beat.start < start);
    let last = trace.beats.partition_point(|beat| beat.end <= end);
    trace.beats.get(first..last).unwrap_or(&[])
}

/// The beat whose span holds `at`.
fn holding(trace: &DayTrace, at: SimTime) -> Option<&Beat> {
    let index = trace.beats.partition_point(|beat| beat.end < at);
    trace.beats.get(index).filter(|beat| beat.start < at)
}

/// Each beat counts once, and the standard deviation is the sample one, over two passes in time order.
fn stats(beats: &[Beat], quantity: Measured) -> Result<Stats, Pending> {
    if beats.len() < 2 {
        return Err(Unavailable::TooFewBeats.into());
    }
    let count = beats.len() as f64;
    let value = |beat: &Beat| beat.reading(quantity).value;
    let mean = beats.iter().map(value).sum::<f64>() / count;
    let spread = (beats.iter().map(value))
        .map(|value| (value - mean) * (value - mean))
        .sum::<f64>();
    let tolerance = beats.iter().map(|beat| beat.reading(quantity).floor).sum::<f64>() / count;
    Ok(Stats {
        mean,
        sd: (spread / (count - 1.0)).sqrt(),
        tolerance,
    })
}

fn response(seen: Stats, against: Stats) -> Change {
    Change::of(
        seen.mean - against.mean,
        against.sd,
        larger(against.tolerance, seen.tolerance),
    )
}

fn beyond(value: f64, from: f64, margin: f64, direction: Direction) -> bool {
    match direction {
        Direction::Up => value > from + margin,
        Direction::Down => value < from - margin,
    }
}

fn marks(beats: &[Beat], mut out: impl FnMut(&Beat) -> bool) -> Vec<(SimTime, bool)> {
    beats.iter().map(|beat| (beat.end, out(beat))).collect()
}

/// The end of the first marked beat that stays marked: with the next two for a fast check, through the first beat
/// ending 5 min or more after it for a slow one. A stay never runs past the beats it is given.
fn onset(marks: &[(SimTime, bool)], fast: bool) -> Option<SimTime> {
    for (first, &(end, out)) in marks.iter().enumerate() {
        if !out {
            continue;
        }
        let stay = if fast {
            marks.get(first..first + FAST_STAY)?
        } else {
            let until = plus(end, SLOW_STAY);
            let last = first + marks[first..].iter().position(|&(end, _)| end >= until)?;
            &marks[first..=last]
        };
        if stay.iter().all(|&(_, out)| out) {
            return Some(end);
        }
    }
    None
}

fn covered(trace: &DayTrace, end: Option<SimTime>) -> Result<SimTime, Pending> {
    match end {
        Some(end) if trace.end >= end => Ok(end),
        Some(end) => Err(uncovered(Uncovered::EndsBefore(end))),
        None => Err(uncovered(Uncovered::Unbounded)),
    }
}

fn one_way_verdict(change: Change, direction: Direction, began: Option<u64>, bound: Bound) -> Verdict {
    if change.sign() != Some(direction) {
        Verdict::Fail(Miss::Direction)
    } else if began.is_some_and(|ns| bound.meets(ns)) {
        Verdict::Pass
    } else {
        Verdict::Fail(Miss::Onset)
    }
}

struct Scene<'a> {
    plan: &'a DayPlan,
    index: usize,
    occurrence: &'a Occurrence,
    fed: &'a DayTrace,
}
impl Scene<'_> {
    fn at(&self) -> SimTime {
        self.occurrence.at
    }
    fn next(&self) -> Option<SimTime> {
        self.plan.next_after(self.at())
    }
    /// The 10 min before the event, cut at the trace's start.
    fn baseline(&self) -> SimTime {
        minus(self.at(), BASELINE).max(self.fed.start)
    }
    fn value(quantity: Measured) -> impl Fn(&Beat) -> f64 {
        move |beat: &Beat| beat.reading(quantity).value
    }

    fn one_way(&self, quantity: Measured, direction: Direction, bound: Bound) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let end = covered(fed, self.next())?;
        let base = stats(within(fed, self.baseline(), at), quantity)?;
        let window = within(fed, at, end);
        let change = response(stats(window, quantity)?, base);
        let value = Self::value(quantity);
        let out = |beat: &Beat| beyond(value(beat), base.mean, 3.0 * base.sd, direction);
        let began = onset(&marks(window, out), bound.span().fast()).map(|end| elapsed(at, end));
        let shown = Shown {
            change: Some(change),
            first: began,
            ..Shown::default()
        };
        Ok((one_way_verdict(change, direction, began, bound), shown))
    }

    fn dip_then_back(
        &self,
        quantity: Measured,
        dip: Bound,
        back: Bound,
        held_from: Span,
    ) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let end = covered(fed, self.next())?;
        let base = stats(within(fed, self.baseline(), at), quantity)?;
        let change = response(stats(within(fed, plus(at, held_from.ns()), end), quantity)?, base);
        let (band, value) = (larger(3.0 * base.sd, base.tolerance), Self::value(quantity));
        let early = within(fed, at, plus(at, FIRST_PHASE));
        let below = |beat: &Beat| beyond(value(beat), base.mean, band, Direction::Down);
        let dipped = onset(&marks(early, below), dip.span().fast());
        let window = within(fed, at, end);
        let after = dipped.map_or(&[][..], |dipped| {
            &window[window.partition_point(|beat| beat.start < dipped)..]
        });
        let returned = onset(
            &marks(after, |beat| value(beat) >= base.mean - band),
            back.span().fast(),
        );
        let shown = Shown {
            change: Some(change),
            first: dipped.map(|end| elapsed(at, end)),
            second: returned.map(|end| elapsed(at, end)),
            ..Shown::default()
        };
        let verdict = if !shown.first.is_some_and(|ns| dip.meets(ns)) {
            Verdict::Fail(Miss::Dip)
        } else if !shown.second.is_some_and(|ns| back.meets(ns)) {
            Verdict::Fail(Miss::Recovery)
        } else if change.sign() == Some(Direction::Down) {
            Verdict::Fail(Miss::Held)
        } else {
            Verdict::Pass
        };
        Ok((verdict, shown))
    }

    fn rise_then_fall(
        &self,
        quantity: Measured,
        rise: Bound,
        fall_from: Span,
        fall: Bound,
    ) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let end = covered(fed, self.next())?;
        let base = stats(within(fed, self.baseline(), at), quantity)?;
        let change = response(stats(within(fed, plus(at, fall_from.ns()), end), quantity)?, base);
        let (band, value) = (larger(3.0 * base.sd, base.tolerance), Self::value(quantity));
        let early = within(fed, at, plus(at, FIRST_PHASE));
        let above = |beat: &Beat| beyond(value(beat), base.mean, band, Direction::Up);
        let below = |beat: &Beat| beyond(value(beat), base.mean, band, Direction::Down);
        let rose = onset(&marks(early, above), rise.span().fast());
        let fell = onset(&marks(within(fed, at, end), below), fall.span().fast());
        let shown = Shown {
            change: Some(change),
            first: rose.map(|end| elapsed(at, end)),
            second: fell.map(|end| elapsed(at, end)),
            ..Shown::default()
        };
        let verdict = if !shown.first.is_some_and(|ns| rise.meets(ns)) {
            Verdict::Fail(Miss::Rise)
        } else if change.sign() != Some(Direction::Down) || !shown.second.is_some_and(|ns| fall.meets(ns)) {
            Verdict::Fail(Miss::Fall)
        } else {
            Verdict::Pass
        };
        Ok((verdict, shown))
    }

    /// The control comes from the fed run's own start, so beats pair by the instant a fed beat ends, never by count.
    fn against_control(
        &self,
        quantity: Measured,
        direction: Direction,
        bound: Bound,
        controls: &[(usize, DayTrace)],
    ) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let end = covered(fed, Some(self.plan.meal_end(at)))?;
        let start = self.baseline();
        let fed_base = stats(within(fed, start, at), quantity)?;
        let window = within(fed, at, end);
        let fed_seen = stats(window, quantity)?;
        let found = controls.iter().find(|(meal, _)| *meal == self.index);
        let (_, control) = found.ok_or_else(|| Pending::from(Unavailable::NoCounterfactual))?;
        covered(control, Some(end))?;
        let control_base = stats(within(control, start, at), quantity)?;
        let control_seen = stats(within(control, at, end), quantity)?;
        let offset = fed_base.mean - control_base.mean;
        let change = Change::of(
            (fed_seen.mean - control_seen.mean) - offset,
            fed_base.sd,
            larger(fed_seen.tolerance, control_seen.tolerance),
        );
        let value = Self::value(quantity);
        let out = |beat: &Beat| {
            let paired = holding(control, beat.end);
            paired.is_some_and(|paired| {
                beyond(
                    value(beat) - value(paired) - offset,
                    0.0,
                    3.0 * fed_base.sd,
                    direction,
                )
            })
        };
        let began = onset(&marks(window, out), bound.span().fast()).map(|end| elapsed(at, end));
        let shown = Shown {
            change: Some(change),
            first: began,
            ..Shown::default()
        };
        Ok((one_way_verdict(change, direction, began, bound), shown))
    }

    fn direction_only(&self, quantity: Measured, direction: Direction) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let end = covered(fed, self.next())?;
        let base = stats(within(fed, self.baseline(), at), quantity)?;
        let change = response(stats(within(fed, at, end), quantity)?, base);
        let verdict = if change.sign() == Some(direction) {
            Verdict::Pass
        } else {
            Verdict::Fail(Miss::Direction)
        };
        let shown = Shown {
            change: Some(change),
            ..Shown::default()
        };
        Ok((verdict, shown))
    }

    /// The awake reference is the hour before the previous night's lie-down, the asleep baseline the hour before the
    /// wake.
    fn after_night_dip(&self, quantity: Measured) -> Result<Judged, Pending> {
        let (fed, at) = (self.fed, self.at());
        let nights = self.plan.occurrences.iter();
        let lying =
            nights.filter(|occurrence| occurrence.kind == EventKind::LieDownToSleep && occurrence.at < at);
        let lie_down = lying.map(|occurrence| occurrence.at).max();
        let lie_down = lie_down.ok_or_else(|| uncovered(Uncovered::Unbounded))?;
        let awake_start = minus(lie_down, NIGHT_HOUR);
        if fed.start > awake_start {
            return Err(uncovered(Uncovered::StartsAfter(awake_start)));
        }
        let end = covered(fed, self.next())?;
        let awake = stats(within(fed, awake_start, lie_down), quantity)?;
        let asleep = stats(within(fed, minus(at, NIGHT_HOUR), at), quantity)?;
        let change = response(stats(within(fed, at, end), quantity)?, asleep);
        let dip = response(asleep, awake);
        let verdict = if dip.sign() != Some(Direction::Down) {
            Verdict::Fail(Miss::NoDip)
        } else if change.sign() != Some(Direction::Up) {
            Verdict::Fail(Miss::Direction)
        } else {
            Verdict::Pass
        };
        let shown = Shown {
            change: Some(change),
            dip: Some(dip),
            ..Shown::default()
        };
        Ok((verdict, shown))
    }

    fn assess(
        &self,
        row: &Row,
        quantity: Measured,
        controls: &[(usize, DayTrace)],
    ) -> Result<Judged, Pending> {
        if self.occurrence.missing_payload {
            return Err(Unavailable::NoPayload.into());
        }
        if !self.fed.applied.get(self.index).copied().unwrap_or(false) {
            return Err(Unavailable::NotReached.into());
        }
        match row.scoring {
            Scoring::Scored(_, Course::OneWay(direction, bound)) => self.one_way(quantity, direction, bound),
            Scoring::Scored(_, Course::DipThenBack { dip, back, held_from }) => {
                self.dip_then_back(quantity, dip, back, held_from)
            }
            Scoring::Scored(
                _,
                Course::RiseThenFall {
                    rise,
                    fall_from,
                    fall,
                },
            ) => self.rise_then_fall(quantity, rise, fall_from, fall),
            Scoring::Scored(_, Course::AgainstControl(direction, bound)) => {
                self.against_control(quantity, direction, bound, controls)
            }
            Scoring::DirectionOnly(_, Lean::Down) => self.direction_only(quantity, Direction::Down),
            Scoring::DirectionOnly(_, Lean::UpAfterNightDip) => self.after_night_dip(quantity),
            Scoring::Unscored(..) => Err(Unavailable::Unscored.into()),
        }
    }
}

/// Every row of the table, in order: one check per quantity per occurrence of its event in the fed run, or one per
/// quantity when the run holds none. `controls` pairs each meal occurrence's index with the run that withholds it.
pub(super) fn judge(plan: &DayPlan, fed: &DayTrace, controls: &[(usize, DayTrace)]) -> Vec<DayCheck> {
    let mut checks = Vec::new();
    for row in &ROWS {
        let quantities: Vec<(DayQuantity, Option<Measured>)> = match row.scoring {
            Scoring::Scored(measured, _) => vec![(measured.quantity(), Some(measured))],
            Scoring::DirectionOnly(measured, _) => measured
                .iter()
                .map(|&measured| (measured.quantity(), Some(measured)))
                .collect(),
            Scoring::Unscored(quantities, _) => quantities.iter().map(|&quantity| (quantity, None)).collect(),
        };
        let of_row = |occurrence: &Occurrence| occurrence.kind == row.event;
        let present: Vec<usize> = (plan.occurrences.iter().enumerate())
            .filter(|(_, occurrence)| {
                of_row(occurrence) && fed.start <= occurrence.at && occurrence.at < fed.end
            })
            .map(|(index, _)| index)
            .collect();
        if present.is_empty() {
            let why = if matches!(row.scoring, Scoring::Unscored(..)) {
                Unavailable::Unscored
            } else if plan
                .occurrences
                .iter()
                .any(|occurrence| of_row(occurrence) && occurrence.missing_payload)
            {
                Unavailable::NoPayload
            } else {
                Unavailable::NotReached
            };
            let absent = |&(quantity, _): &(DayQuantity, Option<Measured>)| DayCheck {
                row,
                quantity,
                occurrence: None,
                verdict: Verdict::NotAvailableYet(why),
                shown: Shown::default(),
            };
            checks.extend(quantities.iter().map(absent));
        }
        for index in present {
            let occurrence = &plan.occurrences[index];
            let scene = Scene {
                plan,
                index,
                occurrence,
                fed,
            };
            for &(quantity, measured) in &quantities {
                let judged = match measured {
                    Some(measured) => scene.assess(row, measured, controls),
                    None => Err(Unavailable::Unscored.into()),
                };
                let (verdict, shown) = judged.unwrap_or_else(|pending| {
                    let shown = Shown {
                        uncovered: pending.uncovered,
                        ..Shown::default()
                    };
                    (Verdict::NotAvailableYet(pending.why), shown)
                });
                checks.push(DayCheck {
                    row,
                    quantity,
                    occurrence: Some((occurrence.day, occurrence.at)),
                    verdict,
                    shown,
                });
            }
        }
    }
    checks
}
