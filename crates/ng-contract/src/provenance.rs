use crate::identity::{Canonical, Seal};
use crate::{
    AdmissionRow, Binding, Compensated, ErrorScales, Measure, PacketKind, Reviewed, RowKind, SyntheticFixture,
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
    CycleMean,
}
impl Seal for AnchorStatistic {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
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
    /// By a named identity over other packet values. `30-data` recomputes a clinical identity; this keeps the
    /// trace.
    Derived {
        identity: &'static str,
        inputs: &'static [ParameterKey],
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
            Self::Derived { identity, inputs } => {
                out.number(1);
                out.text(identity);
                out.length(inputs.len());
                for input in *inputs {
                    out.text(input.key());
                }
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

/// Which packet value a record speaks for: an entry as the packet wrote it, or a row's start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueName {
    Parameter { key: ParameterKey, binding: Binding },
    Start(RowKind),
}

/// One value as the run starts from it.
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
}
impl InitializationProvenance {
    /// One per packet parameter entry, then one per row.
    pub fn values(&self) -> &[ValueRecord] {
        &self.values
    }
    pub fn tolerance(&self) -> ErrorScales {
        self.tolerance
    }
    /// The row with the largest defect over its scale, at most one. `None` for a body with no row.
    pub fn worst(&self) -> Option<(AdmissionRow, Compensated)> {
        self.worst
    }
}

pub(crate) fn seal_trace(trace: &[&'static str; 2], out: &mut Canonical) {
    let [first, second] = trace;
    out.text(first);
    out.text(second);
}
