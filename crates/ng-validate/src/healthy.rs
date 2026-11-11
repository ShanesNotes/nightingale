use core::num::NonZeroU64;
use ng_contract::{
    Activity, AdmissionError, AnchorId, Anchors, ChannelSpec, CompiledBody, Digest, HealthyRangeSet,
    ObservationLens, ObservationSeed, PacketHash, PacketKind, RowKind, SimTime,
};
use ng_observe::{Observation, ObservedValue, Observer};
use ng_record::{ObservationRefusal, Session};

mod range;
mod returns;
#[cfg(test)]
mod tests;

use range::RangeFold;
pub use range::{RangeFailure, RangeGap, RangeReport, RangeResult};
use returns::{AnchorPlan, ReturnFold};
pub use returns::{AnchorReturn, ReturnGap, ReturnReport};

const DAY_NS: u64 = 24 * 3_600 * 1_000_000_000;

/// What one check found: a window that failed, else a reading or anchor it could not judge, else nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckVerdict {
    Pass,
    Fail,
    /// Nothing failed, but something the check could not judge stops the claim, as a study could close it
    /// (record 0004).
    Blocked,
}
impl CheckVerdict {
    fn of(failed: bool, blocked: bool) -> Self {
        match (failed, blocked) {
            (true, _) => Self::Fail,
            (false, true) => Self::Blocked,
            (false, false) => Self::Pass,
        }
    }
}

/// Why the checks cannot read a run as asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadingCheckError {
    /// A set entry on one of the body's rows in another dimension, or a bare lens the body refused.
    Admission(AdmissionError),
    /// The admission revision's view through the bare lens.
    Observation(ObservationRefusal),
    /// A lens or bare lens of another body.
    OtherBody,
    EmptyDay,
    /// A span that starts no later than the one before it.
    UnorderedDay {
        at: SimTime,
    },
    /// A span with its neighbour's activity, which changes nothing.
    RepeatedActivity {
        at: SimTime,
    },
    /// A day whose first span starts after admission, so no activity holds the values read before it.
    DayAfterAdmission {
        at: SimTime,
    },
    /// A change of activity between two grid instants, which would split one value between two activities.
    ChangeOffGrid {
        at: SimTime,
    },
    /// A grid that does not divide 24 hours, so a later morning falls between its instants.
    UnevenGrid,
    /// A range whose sliding length is no whole number of grid periods.
    SlidingLength {
        range: &'static str,
    },
    /// A cycle anchor whose window is no whole number of grid periods.
    AnchorWindow {
        anchor: AnchorId,
    },
    /// A first view that is not admission's, so no baseline is left to return to.
    AfterAdmission,
    /// An admission between two grid instants, which no value reads.
    AdmissionOffGrid {
        at: SimTime,
    },
}
impl From<AdmissionError> for ReadingCheckError {
    fn from(error: AdmissionError) -> Self {
        Self::Admission(error)
    }
}
impl From<ObservationRefusal> for ReadingCheckError {
    fn from(refusal: ObservationRefusal) -> Self {
        Self::Observation(refusal)
    }
}

/// What the patient does from admission on: spans in time order, each the start of one activity. The first covers
/// admission and the last runs on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayActivities {
    spans: Vec<(SimTime, Activity)>,
}
impl DayActivities {
    /// Refuses no span, a start no later than the one before it, and two neighbours with one activity.
    pub fn new(spans: Vec<(SimTime, Activity)>) -> Result<Self, ReadingCheckError> {
        if spans.is_empty() {
            return Err(ReadingCheckError::EmptyDay);
        }
        for pair in spans.windows(2) {
            let [(before, earlier), (at, activity)] = [pair[0], pair[1]];
            if at <= before {
                return Err(ReadingCheckError::UnorderedDay { at });
            }
            if activity == earlier {
                return Err(ReadingCheckError::RepeatedActivity { at });
            }
        }
        Ok(Self { spans })
    }
    pub fn digest(&self) -> Digest {
        let Self { spans } = self;
        let mut bytes = b"ng-validate day activities 1".to_vec();
        bytes.extend_from_slice(&(spans.len() as u64).to_be_bytes());
        for (start, activity) in spans {
            bytes.extend_from_slice(&start.nanoseconds().to_be_bytes());
            bytes.extend_from_slice(&(activity.name().len() as u64).to_be_bytes());
            bytes.extend_from_slice(activity.name().as_bytes());
        }
        Digest::of_canonical(&bytes)
    }
    pub(crate) fn spans(&self) -> &[(SimTime, Activity)] {
        &self.spans
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BareReading {
    pub(crate) row: RowKind,
    pub(crate) method: &'static str,
    pub(crate) integrated: bool,
}

/// A learner's lens with nothing a reading adds to its row: one channel for each row, method and kind of mean the
/// lens reads, on its grid, with no window, delay or error, as record 0004 has the checks read.
pub struct BareLens {
    lens: ObservationLens,
    learner: Digest,
    readings: Vec<BareReading>,
}
impl BareLens {
    /// Refuses a lens of another body.
    pub fn new(body: &CompiledBody, lens: &ObservationLens) -> Result<Self, ReadingCheckError> {
        if lens.model() != body.model_hash() {
            return Err(ReadingCheckError::OtherBody);
        }
        let mut readings: Vec<BareReading> = Vec::new();
        for channel in lens.channels() {
            let reading = BareReading {
                row: body.rows()[channel.row().index()].kind(),
                method: channel.method(),
                integrated: channel.integrated(),
            };
            if !readings.contains(&reading) {
                readings.push(reading);
            }
        }
        let period = NonZeroU64::new(lens.period_ns()).expect("a lens's grid period is not zero");
        let channels = readings.iter().map(|reading| {
            let channel = ChannelSpec::new(reading.method, reading.row);
            if reading.integrated {
                channel.time_averaged(period)
            } else {
                channel
            }
        });
        Ok(Self {
            lens: body.lens(channels.collect(), period)?,
            learner: lens.digest(),
            readings,
        })
    }
    /// The lens the checks read through.
    pub fn lens(&self) -> &ObservationLens {
        &self.lens
    }
    /// The learner's lens, whose readings these are.
    pub fn learner(&self) -> Digest {
        self.learner
    }
}

/// The healthy-range check and the return check, read together from admission through a bare lens. No channel
/// errs, so neither verdict moves with the observation seed (record 0004).
pub struct ReadingChecks<'b> {
    lens: &'b BareLens,
    observer: Observer<'b>,
    run: Digest,
    seal: Digest,
    set: Digest,
    day: Digest,
    packet: PacketHash,
    range: RangeFold,
    returns: ReturnFold,
}
impl<'b> ReadingChecks<'b> {
    /// Reads `admitted`'s admission revision, which every later read follows. The checks take the packet's every
    /// anchor, so none can be left out. Refuses a lens or set entry the body cannot hold, a day or range or anchor
    /// window off the grid, a grid that does not divide a day, and a view after admission.
    pub fn new<K: PacketKind, S>(
        body: &CompiledBody,
        bare: &'b BareLens,
        set: &HealthyRangeSet<K>,
        day: &DayActivities,
        anchors: Anchors,
        admitted: &Session<'_, S>,
    ) -> Result<Self, ReadingCheckError> {
        if bare.lens.model() != body.model_hash() {
            return Err(ReadingCheckError::OtherBody);
        }
        body.check_set(set)?;
        let view = admitted.observation(&bare.lens)?;
        if view.previous().is_some() {
            return Err(ReadingCheckError::AfterAdmission);
        }
        let (admission, period_ns) = (view.time(), bare.lens.period_ns());
        if !admission.nanoseconds().is_multiple_of(period_ns) {
            return Err(ReadingCheckError::AdmissionOffGrid { at: admission });
        }
        let plans: Vec<AnchorPlan> = anchors.iter().map(AnchorPlan::from).collect();
        let mut checks = Self {
            lens: bare,
            observer: Observer::new(&bare.lens, ObservationSeed::new([0; 4])),
            run: admitted.record().identity().digest(),
            seal: view.seal(),
            set: set.digest(),
            day: day.digest(),
            packet: anchors.packet(),
            range: RangeFold::new(&bare.readings, period_ns, set, day, admission)?,
            returns: ReturnFold::new(&bare.readings, period_ns, set, &plans, admission)?,
        };
        let observed = checks.observer.advance(&view)?;
        checks.take(&observed);
        Ok(checks)
    }
    /// Reads the next accepted revision. One that does not follow the last read is refused and changes nothing.
    pub fn read<S>(&mut self, session: &Session<'_, S>) -> Result<(), ObservationRefusal> {
        let view = session.observation(&self.lens.lens)?;
        let observed = self.observer.advance(&view)?;
        self.seal = view.seal();
        self.take(&observed);
        Ok(())
    }
    pub fn finish(self) -> (RangeReport, ReturnReport) {
        let Self {
            lens,
            observer: _,
            run,
            seal,
            set,
            day,
            packet,
            range,
            returns,
        } = self;
        let (results, gaps) = range.finish();
        let range = RangeReport {
            run,
            seal,
            set,
            lens: lens.learner,
            day,
            results,
            gaps,
        };
        let (results, gaps) = returns.finish();
        let returns = ReturnReport {
            run,
            seal,
            set,
            lens: lens.learner,
            packet,
            results,
            gaps,
        };
        (range, returns)
    }
    fn take(&mut self, observed: &[Observation]) {
        for observation in observed {
            let value = match observation.value() {
                ObservedValue::Present(value) => Some(value),
                ObservedValue::NotCollected | ObservedValue::InvalidSignal => None,
                ObservedValue::Pending => continue,
            };
            let (reading, at) = (observation.channel(), observation.collected());
            self.range.feed(reading, at, value);
            self.returns.feed(reading, at, value);
        }
    }
}
