use super::{BareReading, CheckVerdict, DayActivities, ReadingCheckError};
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
    /// The healthy-range set's digest.
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
    /// A window across this change, whose activities' ranges differ, and no range of the change's own.
    NoChangeRange {
        row: RowKind,
        method: &'static str,
        from: Activity,
        to: Activity,
        at: SimTime,
    },
    /// The run held no whole window or whole period of any of this reading's ranges, so a check that tested
    /// nothing never passes.
    Unjudged { row: RowKind, method: &'static str },
}

#[derive(Clone, Copy)]
struct Limits {
    low: Compensated,
    high: Compensated,
}

/// One span of the day, where the next begins, and whether it began before admission, so the run's start cut it.
#[derive(Clone, Copy)]
struct Span {
    start: SimTime,
    activity: Activity,
    end: Option<SimTime>,
    before_admission: bool,
}

/// How a window across the change into a span is judged: by the change's own range, by the every-activity range
/// both its activities fall back to, or by nothing, which blocks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Across {
    Own(usize),
    Shared,
    Unranged,
}

/// The running sum and count of values not present before one value, and that value's span.
#[derive(Clone, Copy)]
struct Before {
    sum: Compensated,
    absent: u64,
    span: usize,
}

/// One window's values: their sum, how many were not present, and the first one's span.
struct Window {
    sum: Compensated,
    absent: u64,
    first: usize,
}

/// Every window of one length over one reading, one ending at each value, each from two running sums, so memory
/// is the window.
struct Slider {
    length_ns: u64,
    count: u64,
    received: u64,
    sum: Compensated,
    absent: u64,
    held: VecDeque<Before>,
    /// Per span, the range of a window inside it.
    within: Vec<Option<usize>>,
    /// Per change of activity, from the second span on.
    across: Vec<Across>,
    every: Option<usize>,
}
impl Slider {
    fn push(&mut self, span: usize, value: Option<Compensated>) -> Option<Window> {
        self.held.push_back(Before {
            sum: self.sum,
            absent: self.absent,
            span,
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
        })
    }
    /// The range that judges a window from span `first` to span `last`, or the change into a span that no range
    /// covers. A window across more than one change falls back to the every-activity range only where each of its
    /// changes would.
    fn judging(&self, first: usize, last: usize) -> Result<Option<usize>, usize> {
        if first == last {
            return Ok(self.within[last]);
        }
        if first + 1 == last {
            return match self.across[last - 1] {
                Across::Own(range) => Ok(Some(range)),
                Across::Shared => Ok(self.every),
                Across::Unranged => Err(last),
            };
        }
        match (first + 1..=last).find(|&change| self.across[change - 1] != Across::Shared) {
            Some(change) => Err(change),
            None => Ok(self.every),
        }
    }
}

/// One span's values so far, for its activity's whole-period range.
#[derive(Clone, Copy)]
struct Period {
    span: usize,
    sum: Compensated,
    absent: u64,
    seen: u64,
}

/// One reading of the bare lens, and every range of the set for its row and method.
struct Reading {
    row: RowKind,
    method: &'static str,
    integrated: bool,
    ranged: bool,
    /// Per span, whether its activity has no range for this reading.
    unranged: Vec<bool>,
    /// Per span, its activity's whole-period range.
    whole: Vec<Option<usize>>,
    sliders: Vec<Slider>,
    period: Option<Period>,
    span: usize,
}
impl Reading {
    /// An activity's ranges are its own if it has any, else the every-activity ones.
    fn new(reading: BareReading, own: &[(usize, Scope, Averaging)], spans: &[Span], period_ns: u64) -> Self {
        let find = |scope: Scope, averaging: Averaging| {
            (own.iter())
                .find(|&&(_, held, by)| (held, by) == (scope, averaging))
                .map(|&(range, _, _)| range)
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
                let across = (spans.windows(2))
                    .map(|pair| {
                        let (from, to) = (pair[0].activity, pair[1].activity);
                        match find(Scope::Change { from, to }, sliding) {
                            Some(range) => Across::Own(range),
                            None if !during(from) && !during(to) => Across::Shared,
                            None => Across::Unranged,
                        }
                    })
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
            integrated: reading.integrated,
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
        }
    }
}

/// The healthy-range check as a fold over each bare reading's values in time order. Each window is judged when
/// its last value arrives.
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
                gaps.push(RangeGap::NoRange {
                    row: reading.row,
                    method: reading.method,
                });
            }
            folds.push(Reading::new(reading, &own, &spans, period_ns));
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
    /// One value of one reading: a sample read at `at`, or the time mean of the grid period that ends there.
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
        let covers = if fold.integrated {
            at.nanoseconds().saturating_sub(*period_ns)
        } else {
            at.nanoseconds()
        };
        while (spans.get(fold.span + 1)).is_some_and(|next| next.start.nanoseconds() <= covers) {
            fold.span += 1;
        }
        let span = fold.span;
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
            let Some(window) = slider.push(span, value) else {
                continue;
            };
            match slider.judging(window.first, span) {
                Ok(Some(range)) => {
                    let mean = mean(window.sum, window.absent, slider.count);
                    judge(&limits[range], &mut results[range], mean, at, slider.length_ns);
                }
                Ok(None) => {}
                Err(change) => {
                    let (from, to) = (spans[change - 1], spans[change]);
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
        // A sampled span holds the samples in [start, end), a time-mean span the means that close in (start, end].
        let last = if fold.integrated {
            end
        } else {
            SimTime::from_ns(end.nanoseconds().saturating_sub(*period_ns))
        };
        if at != last {
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
    /// A period still open when the run ends is cut: counted, not judged.
    pub(crate) fn finish(self) -> (Vec<RangeResult>, Vec<RangeGap>) {
        let Self {
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
        for reading in readings.iter().filter(|reading| reading.ranged) {
            let own = |result: &&RangeResult| (result.row, result.method) == (reading.row, reading.method);
            if results.iter().filter(own).all(|result| result.judged == 0) {
                let (row, method) = (reading.row, reading.method);
                note(&mut gaps, RangeGap::Unjudged { row, method });
            }
        }
        (results, gaps)
    }
}

/// `None` when a value was not present, which fails its window.
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
