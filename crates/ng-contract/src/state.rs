use crate::body::RowKind;
use crate::identity::{Canonical, Seal};
use crate::{Compensated, CompiledBody, Digest, Interval, RowId, SimTime};

/// A full state in one compiled layout: amounts stay compensated, memory and algebraic values are
/// coordinates. `fired` says this head ended at a firing, so a fresh session here reaches the ticket a
/// running one would. No public constructor: admission makes the first, `verify` every other, and
/// `ImageParts::parse` one a checkpoint saved.
pub struct Candidate {
    pub(crate) amounts: Vec<Compensated>,
    pub(crate) memory: Vec<f64>,
    pub(crate) algebraic: Vec<f64>,
    pub(crate) modes: Vec<usize>,
    pub(crate) ordinals: Vec<u64>,
    pub(crate) fired: bool,
}
impl Candidate {
    pub(crate) fn of_coordinates(body: &CompiledBody, coordinates: &[f64], modes: Vec<usize>) -> Self {
        let mut state = Self {
            amounts: Vec::new(),
            memory: Vec::new(),
            algebraic: Vec::new(),
            modes,
            ordinals: vec![0; body.layout.streams.len()],
            fired: false,
        };
        for (row, &value) in body.rows().iter().zip(coordinates) {
            match row.kind() {
                RowKind::Amount { .. } => state.amounts.push(Compensated::exact(value)),
                RowKind::Memory(_) => state.memory.push(value),
                RowKind::Algebraic(_) => state.algebraic.push(value),
            }
        }
        state
    }
    pub(crate) fn value(&self, row: RowId) -> Compensated {
        let Self {
            amounts,
            memory,
            algebraic,
            ..
        } = self;
        let at = row.index();
        if at < amounts.len() {
            return amounts[at];
        }
        let at = at - amounts.len();
        if at < memory.len() {
            return Compensated::exact(memory[at]);
        }
        Compensated::exact(algebraic[at - memory.len()])
    }
    pub(crate) fn values(&self) -> Vec<Compensated> {
        let Self {
            amounts,
            memory,
            algebraic,
            ..
        } = self;
        let coordinates = memory
            .iter()
            .chain(algebraic)
            .map(|&value| Compensated::exact(value));
        amounts.iter().copied().chain(coordinates).collect()
    }
    pub(crate) fn coordinates(&self) -> Vec<f64> {
        let Self {
            amounts,
            memory,
            algebraic,
            ..
        } = self;
        (amounts.iter().map(|amount| amount.rounded()))
            .chain(memory.iter().copied())
            .chain(algebraic.iter().copied())
            .collect()
    }
    pub fn digest(&self) -> Digest {
        let Self {
            amounts,
            memory,
            algebraic,
            modes,
            ordinals,
            fired,
        } = self;
        let mut out = Canonical::new("ng-contract state 1");
        amounts.seal(&mut out);
        memory.seal(&mut out);
        algebraic.seal(&mut out);
        modes.seal(&mut out);
        ordinals.seal(&mut out);
        out.number(u64::from(*fired));
        out.digest()
    }
    pub fn modes_digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract modes 1");
        self.modes.seal(&mut out);
        out.digest()
    }
    pub fn streams_digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract streams 1");
        self.ordinals.seal(&mut out);
        out.digest()
    }
}

/// One accepted interval of every row, degree 1: the chord between the base and the published values,
/// derived, so no core can publish a curve its stages did not make. The admission segment has no interval.
pub struct DenseSegment {
    pub(crate) interval: Option<Interval>,
    pub(crate) start: Vec<Compensated>,
    pub(crate) end: Vec<Compensated>,
}
impl DenseSegment {
    pub(crate) fn empty() -> Self {
        Self {
            interval: None,
            start: Vec::new(),
            end: Vec::new(),
        }
    }
    pub fn interval(&self) -> Option<Interval> {
        self.interval
    }
    /// One row at `at`; `None` outside the interval.
    pub fn sample(&self, row: RowId, at: SimTime) -> Option<Compensated> {
        let interval = self
            .interval
            .filter(|interval| interval.start() <= at && at <= interval.end())?;
        let reached = at.nanoseconds() - interval.start().nanoseconds();
        Some(Compensated::between(
            self.start[row.index()],
            self.end[row.index()],
            reached,
            interval.length_ns(),
        ))
    }
    pub fn digest(&self) -> Digest {
        let Self { interval, start, end } = self;
        let mut out = Canonical::new("ng-contract dense segment 1");
        let ends = interval.map(|interval| (interval.start(), interval.end()));
        ends.seal(&mut out);
        start.seal(&mut out);
        end.seal(&mut out);
        out.digest()
    }
}

/// The polynomial family and degree of every dense curve: the chord, degree 1, in Bernstein form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseBasis {
    family: BasisFamily,
    degree: u8,
}
impl DenseBasis {
    pub const CHORD: Self = Self {
        family: BasisFamily::Bernstein,
        degree: 1,
    };
    pub fn family(&self) -> BasisFamily {
        self.family
    }
    pub fn degree(&self) -> u8 {
        self.degree
    }
}
/// Bernstein form: the coefficient hull bounds every interior value, which makes the certificate's
/// interior-extremum check conservative without sampling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasisFamily {
    Bernstein,
}

/// What a solver core needs to continue exactly, tagged by its schema. Integers only: a float travels as
/// its `to_bits`.
#[derive(Clone)]
pub struct MethodHistory {
    pub(crate) schema: Digest,
    pub(crate) started_at: SimTime,
    pub(crate) counters: Vec<u64>,
}
impl MethodHistory {
    /// `started_at` is where this history began; after a breakpoint it must be that breakpoint.
    pub fn new(schema: Digest, started_at: SimTime, counters: Vec<u64>) -> Self {
        Self {
            schema,
            started_at,
            counters,
        }
    }
    pub fn schema(&self) -> Digest {
        self.schema
    }
    pub fn started_at(&self) -> SimTime {
        self.started_at
    }
    pub fn counters(&self) -> &[u64] {
        &self.counters
    }
    pub fn digest(&self) -> Digest {
        let Self {
            schema,
            started_at,
            counters,
        } = self;
        let mut out = Canonical::new("ng-contract method history 1");
        schema.seal(&mut out);
        started_at.seal(&mut out);
        counters.seal(&mut out);
        out.digest()
    }
}

/// What a problem reads of the accepted image. The record builds one per attempt; a core reads the head's
/// time, values and continuation here.
#[derive(Clone, Copy)]
pub struct AcceptedStateView<'r> {
    time: SimTime,
    state: &'r Candidate,
    history: &'r MethodHistory,
    segment: &'r DenseSegment,
}
impl<'r> AcceptedStateView<'r> {
    pub fn new(
        time: SimTime,
        state: &'r Candidate,
        history: &'r MethodHistory,
        segment: &'r DenseSegment,
    ) -> Self {
        Self {
            time,
            state,
            history,
            segment,
        }
    }
    pub fn time(self) -> SimTime {
        self.time
    }
    /// The accepted value of one row, compensated for an amount and exact for memory and algebraic values.
    pub fn value(self, row: RowId) -> Compensated {
        self.state.value(row)
    }
    pub(crate) fn state(self) -> &'r Candidate {
        self.state
    }
    /// Each mode variable's value, in compiled order.
    pub fn modes(self) -> &'r [usize] {
        &self.state.modes
    }
    pub fn method_history(self) -> &'r MethodHistory {
        self.history
    }
    pub fn last_segment(self) -> &'r DenseSegment {
        self.segment
    }
}
