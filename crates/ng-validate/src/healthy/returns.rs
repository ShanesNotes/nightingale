use super::{BareReading, CheckVerdict, DAY_NS, ReadingCheckError};
use core::num::NonZeroU32;
use ng_contract::{
    AnchorId, AnchorReading, AnchorStatistic, Compensated, Digest, Dimensionless, HealthyRangeSet, Measure,
    PacketKind, RowKind, SimTime,
};

/// The reference change value without its analytical part, in within-subject variations (record 0004).
const RETURN_FACTOR: f64 = 1.96 * core::f64::consts::SQRT_2;
const MORNINGS: [NonZeroU32; 2] = [NonZeroU32::MIN, NonZeroU32::MIN.saturating_add(1)];

/// What the return check reads of one anchor, as its packet's view states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchorPlan {
    pub(crate) anchor: AnchorId,
    pub(crate) row: RowKind,
    pub(crate) method: &'static str,
    pub(crate) statistic: AnchorStatistic,
    pub(crate) window_ns: u64,
}
impl From<&AnchorReading> for AnchorPlan {
    fn from(reading: &AnchorReading) -> Self {
        Self {
            anchor: reading.id(),
            row: reading.row(),
            method: reading.method(),
            statistic: reading.statistic(),
            window_ns: reading.window_ns(),
        }
    }
}

/// The return check of one run against one set (record 0004). Only `ReadingChecks::finish` makes one.
#[derive(Clone, Debug, PartialEq)]
pub struct ReturnReport {
    pub(super) run: Digest,
    pub(super) seal: Digest,
    pub(super) set: Digest,
    pub(super) lens: Digest,
    pub(super) results: Vec<AnchorReturn>,
    pub(super) gaps: Vec<ReturnGap>,
}
impl ReturnReport {
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
    /// Each anchor's mornings, in anchor id order.
    pub fn results(&self) -> &[AnchorReturn] {
        &self.results
    }
    pub fn gaps(&self) -> &[ReturnGap] {
        &self.gaps
    }
    pub fn verdict(&self) -> CheckVerdict {
        verdict(&self.results, &self.gaps)
    }
    /// The verdict over one later morning alone. A morning this check does not read is blocked.
    pub fn morning(&self, morning: NonZeroU32) -> CheckVerdict {
        let failed = (self.results.iter()).any(|result| result.morning == morning && !result.passed());
        let blocked = !MORNINGS.contains(&morning) || self.gaps.iter().any(|gap| gap.blocks(morning));
        CheckVerdict::of(failed, blocked)
    }
}

fn verdict(results: &[AnchorReturn], gaps: &[ReturnGap]) -> CheckVerdict {
    CheckVerdict::of(results.iter().any(|result| !result.passed()), !gaps.is_empty())
}

/// One anchor on one later morning, read by its method over its window from `at`, as admission read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorReturn {
    anchor: AnchorId,
    morning: NonZeroU32,
    at: SimTime,
    admission: Option<Compensated>,
    reading: Option<Compensated>,
    allowed: Compensated,
}
impl AnchorReturn {
    pub fn anchor(&self) -> AnchorId {
        self.anchor
    }
    /// 1 or 2: the days since admission.
    pub fn morning(&self) -> NonZeroU32 {
        self.morning
    }
    /// Admission's instant and the morning's whole days.
    pub fn at(&self) -> SimTime {
        self.at
    }
    /// The anchor as the run read it at admission; `None` when a value in the window was not present.
    pub fn admission(&self) -> Option<Compensated> {
        self.admission
    }
    /// The anchor as the run read it this morning; `None` when a value in the window was not present.
    pub fn reading(&self) -> Option<Compensated> {
        self.reading
    }
    /// How far the reading may move from admission's.
    pub fn allowed(&self) -> Compensated {
        self.allowed
    }
    /// A window with a value that was not present fails.
    pub fn passed(&self) -> bool {
        match (self.admission, self.reading) {
            (Some(admission), Some(reading)) => reading.minus(admission).abs().at_most(self.allowed),
            _ => false,
        }
    }
}

/// An anchor the check cannot hold to its return. Each blocks the verdict and none counts as a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnGap {
    /// The set holds no within-subject variation for the anchor's reading.
    NoVariation { anchor: AnchorId },
    /// No channel of the bare lens reads the anchor's row by its method with its kind of mean.
    NoReading { anchor: AnchorId },
    /// The run ended before this morning's window did.
    NotReached { anchor: AnchorId, morning: NonZeroU32 },
}
impl ReturnGap {
    fn blocks(&self, morning: NonZeroU32) -> bool {
        match *self {
            Self::NoVariation { .. } | Self::NoReading { .. } => true,
            Self::NotReached { morning: missed, .. } => missed == morning,
        }
    }
}

/// One anchor window's values so far, from the instant it is read from.
#[derive(Clone, Copy)]
struct Window {
    at: SimTime,
    sum: Compensated,
    seen: u64,
    absent: bool,
}

/// One anchor the check can hold: its reading, its window's shape and its allowance, then its window from
/// admission and from each later morning.
struct Held {
    anchor: AnchorId,
    reading: usize,
    first_ns: u64,
    count: u64,
    allowed: Compensated,
    windows: [Window; 3],
}

/// The return check as a fold over each bare reading's values in time order.
pub(crate) struct ReturnFold {
    period_ns: u64,
    held: Vec<Held>,
    gaps: Vec<ReturnGap>,
}
impl ReturnFold {
    /// Reads each anchor as admission does: an instant from the sample at its moment, a cycle mean from the cycle's
    /// samples after it, and a cycle time mean from the cycle's grid periods after it, each through the bare
    /// reading of its own kind.
    pub(crate) fn new<K: PacketKind>(
        readings: &[BareReading],
        period_ns: u64,
        set: &HealthyRangeSet<K>,
        plans: &[AnchorPlan],
        admission: SimTime,
    ) -> Result<Self, ReadingCheckError> {
        if !DAY_NS.is_multiple_of(period_ns) {
            return Err(ReadingCheckError::UnevenGrid);
        }
        let factor = Measure::<Dimensionless>::new(RETURN_FACTOR)?;
        let (mut held, mut gaps) = (Vec::new(), Vec::new());
        for plan in plans {
            let anchor = plan.anchor;
            let integrated = plan.statistic == AnchorStatistic::CycleTimeMean;
            let reading = (readings.iter()).position(|bare| {
                (bare.row, bare.method, bare.integrated) == (plan.row, plan.method, integrated)
            });
            let variation = (set.variations().iter())
                .find(|variation| (variation.row(), variation.method()) == (plan.row, plan.method));
            if reading.is_none() {
                gaps.push(ReturnGap::NoReading { anchor });
            }
            if variation.is_none() {
                gaps.push(ReturnGap::NoVariation { anchor });
            }
            let (Some(reading), Some(variation)) = (reading, variation) else {
                continue;
            };
            let (first_ns, count) = match plan.statistic {
                AnchorStatistic::Instant => (0, 1),
                AnchorStatistic::CycleMean | AnchorStatistic::CycleTimeMean => {
                    if plan.window_ns == 0 || !plan.window_ns.is_multiple_of(period_ns) {
                        return Err(ReadingCheckError::AnchorWindow { anchor });
                    }
                    (period_ns, plan.window_ns / period_ns)
                }
            };
            let windows = [0, 1, 2].map(|days: u64| Window {
                at: SimTime::from_ns(admission.nanoseconds().saturating_add(days * DAY_NS)),
                sum: Compensated::ZERO,
                seen: 0,
                absent: false,
            });
            held.push(Held {
                anchor,
                reading,
                first_ns,
                count,
                allowed: variation.sd().times(&factor),
                windows,
            });
        }
        Ok(Self {
            period_ns,
            held,
            gaps,
        })
    }
    pub(crate) fn feed(&mut self, reading: usize, at: SimTime, value: Option<Compensated>) {
        let period_ns = self.period_ns;
        for held in self.held.iter_mut().filter(|held| held.reading == reading) {
            for window in &mut held.windows {
                let first = window.at.nanoseconds().saturating_add(held.first_ns);
                let last = first.saturating_add((held.count - 1).saturating_mul(period_ns));
                if (first..=last).contains(&at.nanoseconds()) && window.seen < held.count {
                    window.sum = window.sum.plus(value.unwrap_or(Compensated::ZERO));
                    window.absent |= value.is_none();
                    window.seen += 1;
                }
            }
        }
    }
    /// A morning whose window the run did not finish is not reached.
    pub(crate) fn finish(self) -> (Vec<AnchorReturn>, Vec<ReturnGap>) {
        let Self { held, mut gaps, .. } = self;
        let mut results = Vec::new();
        for anchor in &held {
            let read = |window: &Window| {
                (window.seen == anchor.count).then(|| {
                    (!window.absent)
                        .then(|| Compensated::between(Compensated::ZERO, window.sum, 1, anchor.count))
                })
            };
            let [admission, later @ ..] = &anchor.windows;
            for (morning, window) in MORNINGS.into_iter().zip(later) {
                match (read(admission), read(window)) {
                    (Some(admission), Some(reading)) => results.push(AnchorReturn {
                        anchor: anchor.anchor,
                        morning,
                        at: window.at,
                        admission,
                        reading,
                        allowed: anchor.allowed,
                    }),
                    _ => gaps.push(ReturnGap::NotReached {
                        anchor: anchor.anchor,
                        morning,
                    }),
                }
            }
        }
        (results, gaps)
    }
}
