use crate::identity::{Canonical, Seal};
use crate::{
    AdmissionRow, Binding, Compensated, ErrorScales, LocatedCrossing, Measure, PacketHash, PacketKind,
    Reviewed, RowKind, Slot, SyntheticFixture,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterKey {
    key: &'static str,
}
impl ParameterKey {
    pub const fn new(key: &'static str) -> Self {
        Self { key }
    }
    pub(crate) fn key(&self) -> &'static str {
        self.key
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Basis {
    Fixture(&'static str),
}

/// One anchor's stable name in its packet, the id its source record carries in `data/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnchorId {
    id: &'static str,
}
impl AnchorId {
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }
    pub(crate) fn id(&self) -> &'static str {
        self.id
    }
}

/// What an anchor's value summarizes. A cycle mean names no length: the cycle is the packet's, so one number has
/// one home, and a cycle a heart sets for itself moves no anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnchorStatistic {
    Instant,
    /// The mean of the cycle's grid samples, as an observer counts them.
    CycleMean,
    /// The row's integral over the cycle over its period, as an observer integrates it.
    CycleTimeMean,
}
impl Seal for AnchorStatistic {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}

/// What admission reads of one anchor: its row, by its method, over its window. Its value never leaves it (I14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorReading {
    pub(crate) id: AnchorId,
    pub(crate) row: RowKind,
    pub(crate) method: &'static str,
    pub(crate) statistic: AnchorStatistic,
    pub(crate) window_ns: u64,
}
impl AnchorReading {
    pub fn id(&self) -> AnchorId {
        self.id
    }
    pub fn row(&self) -> RowKind {
        self.row
    }
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn statistic(&self) -> AnchorStatistic {
        self.statistic
    }
    /// The cycle's period for a cycle statistic, zero for an instant.
    pub fn window_ns(&self) -> u64 {
        self.window_ns
    }
}
/// Every anchor of one packet, in id order. Only its packet makes one, so no caller can leave an anchor out.
#[derive(Debug)]
pub struct Anchors {
    pub(crate) readings: Vec<AnchorReading>,
    pub(crate) packet: PacketHash,
}
impl Anchors {
    pub fn iter(&self) -> core::slice::Iter<'_, AnchorReading> {
        self.readings.iter()
    }
    /// The hash of the packet these anchors came from, as `bind` gives it.
    pub fn packet(&self) -> PacketHash {
        self.packet
    }
}
impl<'a> IntoIterator for &'a Anchors {
    type Item = &'a AnchorReading;
    type IntoIter = core::slice::Iter<'a, AnchorReading>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// What an anchor states (rule 17): the row it reads, the method that reads it, the window and the value. The
/// value leaves only as an admission row's target, so no packet entry can copy it.
pub struct Snapshot<U> {
    pub(crate) row: RowKind,
    pub(crate) method: &'static str,
    pub(crate) statistic: AnchorStatistic,
    pub(crate) value: Measure<U>,
}
impl<U> Snapshot<U> {
    pub fn new(row: RowKind, method: &'static str, statistic: AnchorStatistic, value: Measure<U>) -> Self {
        Self {
            row,
            method,
            statistic,
            value,
        }
    }
}

/// A source and the page or table in it. It cannot prove anyone read the source; `data/`'s gate holds the rest.
pub struct Citation {
    pub(crate) source: &'static str,
    pub(crate) locator: &'static str,
}
impl Citation {
    pub fn new(source: &'static str, locator: &'static str) -> Self {
        Self { source, locator }
    }
}

/// A target a produced quantity must meet at admission. Its trace is what its packet's kind takes: a note in a
/// fixture, a citation in a reviewed packet, with no conversion between the two.
pub struct Anchor<K: PacketKind, U> {
    pub(crate) id: AnchorId,
    pub(crate) snapshot: Snapshot<U>,
    pub(crate) trace: K::Trace,
}
impl<K: PacketKind, U> Anchor<K, U> {
    pub fn new(id: AnchorId, snapshot: Snapshot<U>, trace: K::Trace) -> Self {
        Self { id, snapshot, trace }
    }
}
/// A cited anchor, the one kind a reviewed packet takes.
pub type Specified<U> = Anchor<Reviewed, U>;
/// A nonclinical anchor with a note where a citation would be.
pub type Synthetic<U> = Anchor<SyntheticFixture, U>;

/// How a value was set. A fixture's records are synthetic, so every kind can stand in one, and `Fixture` is a
/// synthetic value set by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueProvenance {
    Fixture,
    /// A named identity: one other entry of this packet times a stated scale. `bind` computes it, so it moves with
    /// its source.
    Derived {
        identity: &'static str,
        source: ParameterKey,
        binding: Binding,
    },
    /// Drawn from its key's population record with the packet seed.
    Sampled,
    /// Chosen inside its key's population record, with a reason.
    Authored {
        reason: &'static str,
    },
    /// Fitted at admission so this anchor holds.
    Calibrated {
        anchor: AnchorId,
    },
    /// A start value admission moved off the packet's guess. It is computed, so its trace is the packet's
    /// anchors, totals and laws.
    Solved,
}
impl Seal for ValueProvenance {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Self::Fixture => out.number(0),
            Self::Derived {
                identity,
                source,
                binding,
            } => {
                out.number(1);
                out.text(identity);
                out.text(source.key());
                binding.seal(out);
            }
            Self::Sampled => out.number(2),
            Self::Authored { reason } => {
                out.number(3);
                out.text(reason);
            }
            Self::Calibrated { anchor } => {
                out.number(4);
                out.text(anchor.id());
            }
            Self::Solved => out.number(5),
        }
    }
}

/// Which packet value a record speaks for: an entry as the packet wrote it, a row's start or a first mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueName {
    Parameter { key: ParameterKey, binding: Binding },
    Start(RowKind),
    Mode { variable: Slot, value: &'static str },
}

/// One value as the run starts from it. A first mode's number is its place in the compiled order of its values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueRecord {
    pub name: ValueName,
    pub provenance: ValueProvenance,
    pub value: Compensated,
}

/// Where every value of an admitted start came from, the tolerance it was held to and the row nearest that
/// tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct InitializationProvenance {
    pub(crate) values: Vec<ValueRecord>,
    pub(crate) tolerance: ErrorScales,
    pub(crate) worst: Option<(AdmissionRow, Compensated)>,
    pub(crate) crossings: Vec<LocatedCrossing>,
}
impl InitializationProvenance {
    /// One per packet parameter entry, then one per row, then one per mode variable.
    pub fn values(&self) -> &[ValueRecord] {
        &self.values
    }
    pub fn tolerance(&self) -> &ErrorScales {
        &self.tolerance
    }
    /// The row with the largest defect over its scale, at most one. `None` for a body with no row.
    pub fn worst(&self) -> Option<(AdmissionRow, Compensated)> {
        self.worst
    }
    pub fn crossings(&self) -> &[LocatedCrossing] {
        &self.crossings
    }
}

pub(crate) fn seal_trace(trace: &[&'static str; 2], out: &mut Canonical) {
    let [first, second] = trace;
    out.text(first);
    out.text(second);
}

#[cfg(test)]
mod tests {
    use crate::{
        Anchor, AnchorId, AnchorStatistic, Cycle, LawName, Measure, ParameterPacket, Pressure, RowKind,
        Snapshot, SyntheticFixture,
    };
    use core::num::{NonZeroU32, NonZeroU64};

    fn row(local: &'static str) -> RowKind {
        RowKind::Algebraic(LawName { owner: "XX", local }.into())
    }

    fn anchor(
        id: &'static str,
        local: &'static str,
        statistic: AnchorStatistic,
    ) -> Anchor<SyntheticFixture, Pressure> {
        let value = Measure::new(10.0).expect("finite");
        Anchor::new(
            AnchorId::new(id),
            Snapshot::new(row(local), "a synthetic gauge", statistic, value),
            "synthetic",
        )
    }

    #[test]
    fn a_packet_lists_what_admission_reads_of_each_anchor_in_id_order() {
        let period = NonZeroU64::new(4_000_000_000).expect("not zero");
        let cycle = Cycle::new(period, NonZeroU32::new(4).expect("not zero")).expect("a cycle");
        let mut packet = ParameterPacket::<SyntheticFixture>::periodic(cycle);
        packet
            .anchor(anchor("xx.c", "third", AnchorStatistic::CycleTimeMean))
            .expect("an anchor");
        packet
            .anchor(anchor("xx.a", "first", AnchorStatistic::Instant))
            .expect("an anchor");
        packet
            .anchor(anchor("xx.b", "second", AnchorStatistic::CycleMean))
            .expect("an anchor");
        let read: Vec<_> = (packet.anchors().iter())
            .map(|reading| {
                (
                    reading.id(),
                    reading.row(),
                    reading.method(),
                    reading.statistic(),
                    reading.window_ns(),
                )
            })
            .collect();
        let method = "a synthetic gauge";
        assert_eq!(
            read,
            [
                (
                    AnchorId::new("xx.a"),
                    row("first"),
                    method,
                    AnchorStatistic::Instant,
                    0
                ),
                (
                    AnchorId::new("xx.b"),
                    row("second"),
                    method,
                    AnchorStatistic::CycleMean,
                    period.get()
                ),
                (
                    AnchorId::new("xx.c"),
                    row("third"),
                    method,
                    AnchorStatistic::CycleTimeMean,
                    period.get()
                ),
            ]
        );
    }

    #[test]
    fn an_anchor_view_names_its_packet_apart_from_another_with_the_same_anchors() {
        let anchored = || {
            let mut packet = ParameterPacket::<SyntheticFixture>::new();
            packet
                .anchor(anchor("xx.a", "first", AnchorStatistic::Instant))
                .expect("an anchor");
            packet
        };
        let mut started = anchored();
        let value = Measure::<Pressure>::new(10.0).expect("finite");
        started.start(row("first"), value, "synthetic").expect("a start");
        let (view, again, other) = (anchored().anchors(), anchored().anchors(), started.anchors());
        assert!(view.iter().eq(other.iter()), "one anchor, read alike");
        assert_eq!(view.packet(), again.packet(), "one packet built twice");
        assert_ne!(view.packet(), other.packet(), "a packet with one more start");
    }
}
