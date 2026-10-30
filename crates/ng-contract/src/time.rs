/// Simulated time, in integer nanoseconds from the run's origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimTime {
    ns: u64,
}
impl SimTime {
    pub const fn from_ns(ns: u64) -> Self {
        Self { ns }
    }
    pub const fn nanoseconds(self) -> u64 {
        self.ns
    }
    pub fn plus_ns(self, ns: u64) -> Option<Self> {
        self.ns.checked_add(ns).map(Self::from_ns)
    }
}

/// `[start, end]`, with `start` before `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interval {
    start: SimTime,
    end: SimTime,
}
impl Interval {
    /// `None` unless `start` is before `end`.
    pub fn new(start: SimTime, end: SimTime) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }
    pub fn start(self) -> SimTime {
        self.start
    }
    pub fn end(self) -> SimTime {
        self.end
    }
    pub fn length_ns(self) -> u64 {
        self.end.nanoseconds() - self.start.nanoseconds()
    }
}
