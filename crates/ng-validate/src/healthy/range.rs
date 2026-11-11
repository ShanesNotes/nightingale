use super::{BareReading, CheckVerdict, DayActivities, ReadingCheckError, Taken};
use alloc::collections::VecDeque;
use core::num::NonZeroU64;
use ng_contract::{
    Activity, Averaging, Compensated, Digest, HealthyRange, HealthyRangeSet, PacketKind, RowKind, Scope,
    SimTime,
};

/// The healthy-range check of one run against one set (record 0004). Only `ReadingChecks::finish` makes one.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeReport {
    pub(super) run: Digest,
    pub(super) seal: Digest,
    pub(super) set: Digest,
    pub(super) lens: Digest,
    pub(super) day: Digest,
    pub(super) results: Vec<RangeResult>,
    pub(super) gaps: Vec<RangeGap>,
}
impl RangeReport {
    /// The run identity's digest.
    pub fn run(&self) -> Digest {
        self.run
    }
    /// The seal of the last revision read.
    pub fn seal(&self) -> Digest {
        self.seal
    }
    pub fn set(&self) -> Digest {
        self.set
    }
    /// The learner's lens, whose readings the check read bare.
    pub fn lens(&self) -> Digest {
        self.lens
    }
    pub fn day(&self) -> Digest {
        self.day
    }
    /// One per set range that matches a reading of the lens, in id order.
    pub fn results(&self) -> &[RangeResult] {
        &self.results
    }
    /// In the order the check met them, each once.
    pub fn gaps(&self) -> &[RangeGap] {
        &self.gaps
    }
    pub fn verdict(&self) -> CheckVerdict {
        verdict(&self.results, &self.gaps)
    }
}

fn verdict(results: &[RangeResult], gaps: &[RangeGap]) -> CheckVerdict {
    CheckVerdict::of(
        results.iter().any(|result| result.failure.is_some()),
        !gaps.is_empty(),
    )
}

/// What one set range judged: windows or whole periods, the earliest that failed, and the periods the run's start
/// or end cut, which it counted and did not judge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeResult {
    range: &'static str,
    row: RowKind,
    method: &'static str,
    judged: u64,
    failure: Option<RangeFailure>,
    cut: u64,
}
impl RangeResult {
    /// The range's id in its set.
    pub fn range(&self) -> &'static str {
        self.range
    }
    pub fn row(&self) -> RowKind {
        self.row
    }
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn judged(&self) -> u64 {
        self.judged
    }
    pub fn failure(&self) -> Option<RangeFailure> {
        self.failure
    }
    pub fn cut(&self) -> u64 {
        self.cut
    }
}

/// A window that failed its range: the values read in `(end - length, end]`, as an observation's window holds
/// them, so one that began before the clock's zero is still named.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeFailure {
    end: SimTime,
    length_ns: u64,
    mean: Option<Compensated>,
}
impl RangeFailure {
    /// When the window's last value was read.
    pub fn end(&self) -> SimTime {
        self.end
    }
    /// The range's sliding length, or the whole period's.
    pub fn length_ns(&self) -> u64 {
        self.length_ns
    }
    /// `None` when a value in the window was not present.
    pub fn mean(&self) -> Option<Compensated> {
        self.mean
    }
}

/// Where the set holds no range for a reading of the lens. Each blocks the verdict, since a study could close it,
/// and none leaves its reading unchecked silently (record 0004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeGap {
    NoRange {
        row: RowKind,
        method: &'static str,
    },
    NoActivityRange {
        row: RowKind,
        method: &'static str,
        activity: Activity,
    },
    /// A window that no one range judges at this change: it crosses the change, whose activities' ranges differ
    /// and which has no range of its own, or it belongs to this change and to another whose shift it lies in.
    NoChangeRange {
        row: RowKind,
        method: &'static str,
        from: Activity,
        to: Activity,
        at: SimTime,
    },
    /// A range whose activity or change the run holds that judged no whole window or whole period, so no range
    /// passes untested. The run holds an activity with a value in it and a change with a value on each side, and
    /// the every-activity range holds for each activity with no range of its own. A range whose activity or change
    /// the run never holds is no gap, since one set serves every run.
    Unjudged {
        range: &'static str,
        row: RowKind,
        method: &'static str,
    },
    /// The check judged no window or whole period at all and named nothing else, as with a lens of no reading or a
    /// run that gave no value, so a check that tested nothing never passes.
    NothingJudged,
}

#[derive(Clone, Copy)]
struct Limits {
    low: Compensated,
    high: Compensated,
}

#[derive(Clone, Copy)]
struct Span {
    start: SimTime,
    activity: Activity,
    end: Option<SimTime>,
    before_admission: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Across {
    Own {
        range: Option<usize>,
        shift_ends_ns: u64,
    },
    EveryActivity,
    Unranged,
}

enum Judging {
    By(usize),
    NoRangeOfThisLength,
    Blocked { change_into: usize },
}
impl Judging {
    fn of(range: Option<usize>) -> Self {
        range.map_or(Self::NoRangeOfThisLength, Self::By)
    }
}

#[derive(Clone, Copy)]
struct Before {
    sum: Compensated,
    absent: u64,
    span: usize,
    covers: u64,
}

struct Window {
    sum: Compensated,
    absent: u64,
    first: usize,
    covers: u64,
}

struct Slider {
    length_ns: u64,
    count: u64,
    received: u64,
    sum: Compensated,
    absent: u64,
    held: VecDeque<Before>,
    within: Vec<Option<usize>>,
    across: Vec<Across>,
    every: Option<usize>,
}
impl Slider {
    fn push(&mut self, span: usize, covers: u64, value: Option<Compensated>) -> Option<Window> {
        self.held.push_back(Before {
            sum: self.sum,
            absent: self.absent,
            span,
            covers,
        });
        self.sum = self.sum.plus(value.unwrap_or(Compensated::ZERO));
        self.absent += u64::from(value.is_none());
        self.received += 1;
        if self.received < self.count {
            return None;
        }
        while self.held.len() as u64 > self.count {
            self.held.pop_front();
        }
        let first = *self.held.front()?;
        Some(Window {
            sum: self.sum.minus(first.sum),
            absent: self.absent - first.absent,
            first: first.span,
            covers: first.covers,
        })
    }
    fn judging(&self, first: usize, covers: u64, last: usize) -> Judging {
        let into_first = (first > 0).then(|| self.across[first]);
        let shift = into_first.and_then(|across| match across {
            Across::Own { range, shift_ends_ns } if covers < shift_ends_ns => Some(range),
            Across::Own { .. } | Across::EveryActivity | Across::Unranged => None,
        });
        let mut needing_own = (first + 1..=last).filter(|&span| self.across[span] != Across::EveryActivity);
        let Some(change_into) = needing_own.next() else {
            return Judging::of(match shift {
                Some(range) => range,
                None if first == last => self.within[last],
                None => self.every,
            });
        };
        match (shift, needing_own.next(), self.across[change_into]) {
            (None, None, Across::Own { range, .. }) => Judging::of(range),
            _ => Judging::Blocked { change_into },
        }
    }
}

#[derive(Clone, Copy)]
struct Period {
    span: usize,
    sum: Compensated,
    absent: u64,
    seen: u64,
}

struct Reading {
    row: RowKind,
    method: &'static str,
    taken: Taken,
    ranged: bool,
    unranged: Vec<bool>,
    whole: Vec<Option<usize>>,
    sliders: Vec<Slider>,
    period: Option<Period>,
    span: usize,
    own: Vec<(usize, Scope, Averaging)>,
    read: Vec<bool>,
}
impl Reading {
    fn new(
        reading: BareReading,
        own: Vec<(usize, Scope, Averaging)>,
        spans: &[Span],
        period_ns: u64,
    ) -> Self {
        let find = |scope: Scope, averaging: Averaging| {
            (own.iter())
                .find(|&&(_, held, by)| (held, by) == (scope, averaging))
                .map(|&(range, _, _)| range)
        };
        let changes = |from: Activity, to: Activity| {
            own.iter()
                .filter_map(move |&(range, scope, averaging)| match scope {
                    Scope::Change {
                        from: left,
                        to: entered,
                        lasting,
                    } if (left, entered) == (from, to) => Some((range, averaging, lasting.get())),
                    Scope::Every | Scope::During(_) | Scope::Change { .. } => None,
                })
        };
        let during = |activity: Activity| own.iter().any(|&(_, scope, _)| scope == Scope::During(activity));
        let every = own.iter().any(|&(_, scope, _)| scope == Scope::Every);
        let mut lengths: Vec<NonZeroU64> = (own.iter())
            .filter_map(|&(_, _, averaging)| match averaging {
                Averaging::Sliding(length) => Some(length),
                Averaging::WholePeriod => None,
            })
            .collect();
        lengths.sort_unstable();
        lengths.dedup();
        let sliders = (lengths.into_iter())
            .map(|length| {
                let sliding = Averaging::Sliding(length);
                let family = |activity| {
                    if during(activity) {
                        find(Scope::During(activity), sliding)
                    } else {
                        find(Scope::Every, sliding)
                    }
                };
                let into_later_spans = (spans.windows(2)).map(|pair| {
                    let (from, to) = (pair[0].activity, pair[1].activity);
                    match changes(from, to).map(|(_, _, lasting)| lasting).max() {
                        Some(lasting) => Across::Own {
                            range: (changes(from, to).find(|&(_, by, _)| by == sliding))
                                .map(|(range, _, _)| range),
                            shift_ends_ns: pair[1].start.nanoseconds().saturating_add(lasting),
                        },
                        None if !during(from) && !during(to) => Across::EveryActivity,
                        None => Across::Unranged,
                    }
                });
                let across = core::iter::once(Across::Unranged)
                    .chain(into_later_spans)
                    .collect();
                Slider {
                    length_ns: length.get(),
                    count: length.get() / period_ns,
                    received: 0,
                    sum: Compensated::ZERO,
                    absent: 0,
                    held: VecDeque::new(),
                    within: spans.iter().map(|span| family(span.activity)).collect(),
                    across,
                    every: find(Scope::Every, sliding),
                }
            })
            .collect();
        Self {
            row: reading.row,
            method: reading.method,
            taken: reading.taken,
            ranged: !own.is_empty(),
            unranged: spans
                .iter()
                .map(|span| !during(span.activity) && !every)
                .collect(),
            whole: (spans.iter())
                .map(|span| find(Scope::During(span.activity), Averaging::WholePeriod))
                .collect(),
            sliders,
            period: None,
            span: 0,
            read: vec![false; spans.len()],
            own,
        }
    }
    fn holds(&self, range: usize, spans: &[Span]) -> bool {
        let Some(&(_, scope, _)) = self.own.iter().find(|&&(own, _, _)| own == range) else {
            return false;
        };
        let read = |span: &usize| self.read[*span];
        let activity = |span: usize| spans[span].activity;
        match scope {
            Scope::Every => (0..spans.len()).filter(read).any(|span| {
                let its_own = Scope::During(activity(span));
                !self.own.iter().any(|&(_, held, _)| held == its_own)
            }),
            Scope::During(during) => (0..spans.len()).filter(read).any(|span| activity(span) == during),
            Scope::Change { from, to, .. } => (1..spans.len()).any(|span| {
                let crossed = self.read[span - 1] && self.read[span];
                crossed && (activity(span - 1), activity(span)) == (from, to)
            }),
        }
    }
}

pub(crate) struct RangeFold {
    period_ns: u64,
    spans: Vec<Span>,
    limits: Vec<Limits>,
    results: Vec<RangeResult>,
    readings: Vec<Reading>,
    gaps: Vec<RangeGap>,
}
impl RangeFold {
    pub(crate) fn new<K: PacketKind>(
        readings: &[BareReading],
        period_ns: u64,
        set: &HealthyRangeSet<K>,
        day: &DayActivities,
        admission: SimTime,
    ) -> Result<Self, ReadingCheckError> {
        let day = day.spans();
        if let Some(&(at, _)) = day.first()
            && at > admission
        {
            return Err(ReadingCheckError::DayAfterAdmission { at });
        }
        let changes = day.iter().skip(1).map(|&(at, _)| at);
        if let Some(at) = changes
            .clone()
            .find(|at| !at.nanoseconds().is_multiple_of(period_ns))
        {
            return Err(ReadingCheckError::ChangeOffGrid { at });
        }
        let ends = changes.map(Some).chain([None]);
        let spans: Vec<Span> = (day.iter().zip(ends))
            .map(|(&(start, activity), end)| Span {
                start,
                activity,
                end,
                before_admission: start < admission,
            })
            .collect();
        let matched: Vec<&HealthyRange<K>> = (set.ranges().iter())
            .filter(|range| {
                let reading = (range.row(), range.method());
                readings.iter().any(|bare| (bare.row, bare.method) == reading)
            })
            .collect();
        for range in &matched {
            if let Averaging::Sliding(length) = range.averaging()
                && !length.get().is_multiple_of(period_ns)
            {
                return Err(ReadingCheckError::SlidingLength { range: range.id() });
            }
        }
        let mut folds = Vec::with_capacity(readings.len());
        let mut gaps = Vec::new();
        for &reading in readings {
            let own: Vec<(usize, Scope, Averaging)> = (matched.iter().enumerate())
                .filter(|(_, range)| (range.row(), range.method()) == (reading.row, reading.method))
                .map(|(at, range)| (at, range.scope(), range.averaging()))
                .collect();
            if own.is_empty() {
                let (row, method) = (reading.row, reading.method);
                note(&mut gaps, RangeGap::NoRange { row, method });
            }
            folds.push(Reading::new(reading, own, &spans, period_ns));
        }
        Ok(Self {
            period_ns,
            spans,
            limits: (matched.iter())
                .map(|range| Limits {
                    low: range.low(),
                    high: range.high(),
                })
                .collect(),
            results: (matched.iter())
                .map(|range| RangeResult {
                    range: range.id(),
                    row: range.row(),
                    method: range.method(),
                    judged: 0,
                    failure: None,
                    cut: 0,
                })
                .collect(),
            readings: folds,
            gaps,
        })
    }
    pub(crate) fn feed(&mut self, reading: usize, at: SimTime, value: Option<Compensated>) {
        let Self {
            period_ns,
            spans,
            limits,
            results,
            readings,
            gaps,
        } = self;
        let Some(fold) = readings.get_mut(reading).filter(|fold| fold.ranged) else {
            return;
        };
        let covers = fold.taken.covers(at, *period_ns);
        while (spans.get(fold.span + 1)).is_some_and(|next| next.start.nanoseconds() <= covers) {
            fold.span += 1;
        }
        let span = fold.span;
        fold.read[span] = true;
        if fold.unranged[span] {
            let activity = spans[span].activity;
            note(
                gaps,
                RangeGap::NoActivityRange {
                    row: fold.row,
                    method: fold.method,
                    activity,
                },
            );
        }
        for slider in &mut fold.sliders {
            let Some(window) = slider.push(span, covers, value) else {
                continue;
            };
            match slider.judging(window.first, window.covers, span) {
                Judging::By(range) => {
                    let mean = mean(window.sum, window.absent, slider.count);
                    judge(&limits[range], &mut results[range], mean, at, slider.length_ns);
                }
                Judging::NoRangeOfThisLength => {}
                Judging::Blocked { change_into } => {
                    let (from, to) = (spans[change_into - 1], spans[change_into]);
                    note(
                        gaps,
                        RangeGap::NoChangeRange {
                            row: fold.row,
                            method: fold.method,
                            from: from.activity,
                            to: to.activity,
                            at: to.start,
                        },
                    );
                }
            }
        }
        let Some(range) = fold.whole[span] else {
            return;
        };
        let open = (fold.period.filter(|open| open.span == span)).unwrap_or(Period {
            span,
            sum: Compensated::ZERO,
            absent: 0,
            seen: 0,
        });
        let period = Period {
            sum: open.sum.plus(value.unwrap_or(Compensated::ZERO)),
            absent: open.absent + u64::from(value.is_none()),
            seen: open.seen + 1,
            ..open
        };
        let Span {
            start,
            end,
            before_admission,
            ..
        } = spans[span];
        let Some(end) = end else {
            fold.period = Some(period);
            return;
        };
        if !fold.taken.closes(at, end, *period_ns) {
            fold.period = Some(period);
            return;
        }
        fold.period = None;
        let result = &mut results[range];
        if before_admission {
            result.cut += 1;
        } else {
            let length_ns = end.nanoseconds() - start.nanoseconds();
            judge(
                &limits[range],
                result,
                mean(period.sum, period.absent, period.seen),
                at,
                length_ns,
            );
        }
    }
    pub(crate) fn finish(self) -> (Vec<RangeResult>, Vec<RangeGap>) {
        let Self {
            spans,
            mut results,
            readings,
            mut gaps,
            ..
        } = self;
        for reading in &readings {
            if let Some(range) = (reading.period).and_then(|open| reading.whole[open.span]) {
                results[range].cut += 1;
            }
        }
        for (at, result) in results.iter().enumerate() {
            if result.judged == 0 && readings.iter().any(|reading| reading.holds(at, &spans)) {
                gaps.push(RangeGap::Unjudged {
                    range: result.range,
                    row: result.row,
                    method: result.method,
                });
            }
        }
        if gaps.is_empty() && results.iter().all(|result| result.judged == 0) {
            gaps.push(RangeGap::NothingJudged);
        }
        (results, gaps)
    }
}

fn mean(sum: Compensated, absent: u64, count: u64) -> Option<Compensated> {
    (absent == 0).then(|| Compensated::between(Compensated::ZERO, sum, 1, count))
}

fn judge(limits: &Limits, result: &mut RangeResult, mean: Option<Compensated>, end: SimTime, length_ns: u64) {
    result.judged += 1;
    if mean.is_some_and(|mean| limits.low.at_most(mean) && mean.at_most(limits.high)) {
        return;
    }
    if result.failure.is_none_or(|known| end < known.end) {
        result.failure = Some(RangeFailure { end, length_ns, mean });
    }
}

fn note(gaps: &mut Vec<RangeGap>, gap: RangeGap) {
    if !gaps.contains(&gap) {
        gaps.push(gap);
    }
}
