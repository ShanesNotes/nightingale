use crate::session::Session;
use ng_contract::{AcceptedStateView, Compensated, Digest, Interval, ObservationLens, SimTime};

/// One accepted revision through a lens.
pub struct ObservationView<'r> {
    lens: &'r ObservationLens,
    accepted: AcceptedStateView<'r>,
    seal: Digest,
    previous: Option<Digest>,
}
impl<'r> ObservationView<'r> {
    pub fn lens(&self) -> &'r ObservationLens {
        self.lens
    }
    pub fn time(&self) -> SimTime {
        self.accepted.time()
    }
    pub fn seal(&self) -> Digest {
        self.seal
    }
    /// The seal this revision follows; `None` at admission.
    pub fn previous(&self) -> Option<Digest> {
        self.previous
    }
    /// Each grid instant this revision holds, oldest first, with one value per channel. A step holds its end and
    /// not its start, so an instant falls in one step, and revision 0 holds the admission instant when it is on
    /// the grid. At the head a channel reads the accepted state, which is what a firing left, so one instant has
    /// one value.
    pub fn samples(&self) -> Vec<(SimTime, Vec<Compensated>)> {
        let head = self.accepted.time();
        let segment = self.accepted.last_segment();
        let period = self.lens.period_ns();
        let first = match segment.interval() {
            Some(step) => step.start().nanoseconds() / period + 1,
            None => head.nanoseconds().div_ceil(period),
        };
        let read = |row, at| {
            if at == head {
                self.accepted.value(row)
            } else {
                segment
                    .sample(row, at)
                    .expect("a grid instant lies inside its step")
            }
        };
        (first..=head.nanoseconds() / period)
            .map(|index| SimTime::from_ns(index * period))
            .map(|at| {
                let channels = self.lens.channels().iter();
                (at, channels.map(|channel| read(channel.row(), at)).collect())
            })
            .collect()
    }
    /// The step this revision accepted and each channel's chord over it, its values at the step's start and at its
    /// end before any firing, which is the curve a time mean integrates; `None` at admission, which holds no step.
    pub fn chords(&self) -> Option<(Interval, Vec<(Compensated, Compensated)>)> {
        let segment = self.accepted.last_segment();
        let step = segment.interval()?;
        let ends = |row| {
            let start = segment.sample(row, step.start()).expect("a step holds its start");
            let end = segment.sample(row, step.end()).expect("a step holds its end");
            (start, end)
        };
        Some((
            step,
            self.lens
                .channels()
                .iter()
                .map(|channel| ends(channel.row()))
                .collect(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRefusal {
    /// A lens compiled for another body, a view through another lens, or a checkpoint of another lens or seed.
    OtherLens,
    /// A view that does not follow the last revision read: skipped, repeated or of another course.
    Gap { expected: Digest, found: Option<Digest> },
    /// Checkpoint bytes that are not an observer's, or that fail their raw digest.
    Malformed,
}

impl<S> Session<'_, S> {
    /// Reads the last accepted image through a lens, also after a fault. The session never sees an observer.
    pub fn observation<'r>(
        &'r self,
        lens: &'r ObservationLens,
    ) -> Result<ObservationView<'r>, ObservationRefusal> {
        let record = self.record();
        if lens.model() != record.identity().model() {
            return Err(ObservationRefusal::OtherLens);
        }
        Ok(ObservationView {
            lens,
            accepted: record.accepted(),
            seal: record.seal().digest(),
            previous: record.history().previous(),
        })
    }
}
