use crate::identity::{Canonical, Seal};
use crate::provenance::seal_trace;
use crate::quantity::dimension;
use crate::{
    AdmissionError, Compensated, CompiledBody, Digest, Dimension, Measure, PacketKind, RowKind, SyntheticFixture,
};
use core::num::NonZeroU64;

/// What the patient is doing as his day names it, such as asleep or seated. Compared only for equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Activity {
    name: &'static str,
}
impl Activity {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
}
impl Seal for Activity {
    fn seal(&self, out: &mut Canonical) {
        let Self { name } = self;
        out.text(name);
    }
}

/// When a healthy range holds: during every activity, during one, or over a window across one change of
/// activity, where a source shows the change shifts the reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Every,
    During(Activity),
    Change { from: Activity, to: Activity },
}
impl Seal for Scope {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Self::Every => out.number(0),
            Self::During(activity) => {
                out.number(1);
                activity.seal(out);
            }
            Self::Change { from, to } => {
                out.number(2);
                from.seal(out);
                to.seal(out);
            }
        }
    }
}

/// How a range's study averaged its reading: over a sliding window of this many nanoseconds, or over the
/// whole period of one activity, such as the night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Averaging {
    Sliding(NonZeroU64),
    WholePeriod,
}
impl Seal for Averaging {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Self::Sliding(length_ns) => {
                out.number(0);
                out.number(length_ns.get());
            }
            Self::WholePeriod => out.number(1),
        }
    }
}

/// The central 95% of healthy adults for one reading, its row by its method, over its study's averaging, in
/// the row's dimension (record 0004).
pub struct HealthyRange<K: PacketKind> {
    id: &'static str,
    row: RowKind,
    method: &'static str,
    scope: Scope,
    averaging: Averaging,
    dimension: String,
    low: Compensated,
    high: Compensated,
    trace: K::Trace,
}
impl HealthyRange<SyntheticFixture> {
    /// Refuses a high limit below the low one, and a whole period that is not during one activity.
    #[allow(clippy::too_many_arguments)]
    pub fn synthetic<U: Dimension>(
        id: &'static str,
        row: RowKind,
        method: &'static str,
        scope: Scope,
        averaging: Averaging,
        low: Measure<U>,
        high: Measure<U>,
        note: &'static str,
    ) -> Result<Self, AdmissionError> {
        let (low, high) = (Compensated::of(&low), Compensated::of(&high));
        if !low.at_most(high) {
            return Err(AdmissionError::ReversedRange { id });
        }
        if averaging == Averaging::WholePeriod && !matches!(scope, Scope::During(_)) {
            return Err(AdmissionError::WholePeriodScope { id });
        }
        Ok(Self {
            id,
            row,
            method,
            scope,
            averaging,
            dimension: dimension::<U>(),
            low,
            high,
            trace: note,
        })
    }
}
impl<K: PacketKind> HealthyRange<K> {
    pub fn id(&self) -> &'static str {
        self.id
    }
    pub fn row(&self) -> RowKind {
        self.row
    }
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn averaging(&self) -> Averaging {
        self.averaging
    }
    pub fn low(&self) -> Compensated {
        self.low
    }
    pub fn high(&self) -> Compensated {
        self.high
    }
    pub fn trace(&self) -> &K::Trace {
        &self.trace
    }
}

/// One healthy adult's own day-to-day SD of one reading, its row by its method, in the reading's own unit.
pub struct WithinSubjectVariation<K: PacketKind> {
    id: &'static str,
    row: RowKind,
    method: &'static str,
    dimension: String,
    sd: Compensated,
    trace: K::Trace,
}
impl WithinSubjectVariation<SyntheticFixture> {
    /// Refuses a negative SD.
    pub fn synthetic<U: Dimension>(
        id: &'static str,
        row: RowKind,
        method: &'static str,
        sd: Measure<U>,
        note: &'static str,
    ) -> Result<Self, AdmissionError> {
        let sd = Compensated::of(&sd);
        if !Compensated::ZERO.at_most(sd) {
            return Err(AdmissionError::NegativeVariation { id });
        }
        Ok(Self {
            id,
            row,
            method,
            dimension: dimension::<U>(),
            sd,
            trace: note,
        })
    }
}
impl<K: PacketKind> WithinSubjectVariation<K> {
    pub fn id(&self) -> &'static str {
        self.id
    }
    pub fn row(&self) -> RowKind {
        self.row
    }
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn sd(&self) -> Compensated {
        self.sd
    }
    pub fn trace(&self) -> &K::Trace {
        &self.trace
    }
}

/// Every healthy range and within-subject variation the checks hold a body to. It only judges, so no law
/// reads it and it stays out of the run's identity: correcting it rechecks a replay as a new lens does (record
/// 0005). One set serves every body.
pub struct HealthyRangeSet<K: PacketKind> {
    ranges: Vec<HealthyRange<K>>,
    variations: Vec<WithinSubjectVariation<K>>,
}
impl HealthyRangeSet<SyntheticFixture> {
    /// Refuses a repeated id, two ranges for one reading with one scope and averaging, and two variations for
    /// one reading.
    pub fn synthetic(
        ranges: Vec<HealthyRange<SyntheticFixture>>,
        variations: Vec<WithinSubjectVariation<SyntheticFixture>>,
    ) -> Result<Self, AdmissionError> {
        Self::of(ranges, variations)
    }
}
impl<K: PacketKind> HealthyRangeSet<K> {
    /// In id order.
    pub fn ranges(&self) -> &[HealthyRange<K>] {
        &self.ranges
    }
    /// In id order.
    pub fn variations(&self) -> &[WithinSubjectVariation<K>] {
        &self.variations
    }
    /// Every field of every entry, in id order, with the set's kind and each entry's trace.
    pub fn digest(&self) -> Digest {
        let Self { ranges, variations } = self;
        let mut out = Canonical::new("ng-contract healthy-range set 1");
        out.number(K::TAG);
        ranges.seal(&mut out);
        variations.seal(&mut out);
        out.digest()
    }
    fn of(
        mut ranges: Vec<HealthyRange<K>>,
        mut variations: Vec<WithinSubjectVariation<K>>,
    ) -> Result<Self, AdmissionError> {
        let mut ids: Vec<&'static str> = ranges.iter().map(|range| range.id).collect();
        ids.extend(variations.iter().map(|variation| variation.id));
        ids.sort_unstable();
        if let Some(pair) = ids.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(AdmissionError::DuplicateRangeEntry { id: pair[0] });
        }
        ranges.sort_by_key(|range| range.id);
        variations.sort_by_key(|variation| variation.id);
        for (at, range) in ranges.iter().enumerate() {
            let same = |known: &HealthyRange<K>| {
                (known.row, known.method, known.scope, known.averaging)
                    == (range.row, range.method, range.scope, range.averaging)
            };
            if ranges[..at].iter().any(same) {
                return Err(AdmissionError::DuplicateRangeEntry { id: range.id });
            }
        }
        for (at, variation) in variations.iter().enumerate() {
            let same = |known: &WithinSubjectVariation<K>| {
                (known.row, known.method) == (variation.row, variation.method)
            };
            if variations[..at].iter().any(same) {
                return Err(AdmissionError::DuplicateRangeEntry { id: variation.id });
            }
        }
        Ok(Self { ranges, variations })
    }
}
impl<K: PacketKind> Seal for HealthyRange<K> {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            id,
            row,
            method,
            scope,
            averaging,
            dimension,
            low,
            high,
            trace,
        } = self;
        out.text(id);
        row.seal(out);
        out.text(method);
        scope.seal(out);
        averaging.seal(out);
        out.text(dimension);
        low.seal(out);
        high.seal(out);
        seal_trace(&K::words(trace), out);
    }
}
impl<K: PacketKind> Seal for WithinSubjectVariation<K> {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            id,
            row,
            method,
            dimension,
            sd,
            trace,
        } = self;
        out.text(id);
        row.seal(out);
        out.text(method);
        out.text(dimension);
        sd.seal(out);
        seal_trace(&K::words(trace), out);
    }
}

impl CompiledBody {
    /// Refuses an entry on a row this body holds in another dimension. An entry on a row this body lacks is no
    /// error, because one set serves every body.
    pub fn check_set<K: PacketKind>(&self, set: &HealthyRangeSet<K>) -> Result<(), AdmissionError> {
        let ranges = (set.ranges.iter()).map(|range| (range.id, range.row, &range.dimension));
        let variations = (set.variations.iter()).map(|variation| (variation.id, variation.row, &variation.dimension));
        for (id, row, dimension) in ranges.chain(variations) {
            if (self.rows().iter()).any(|known| known.kind == row && known.dimension != *dimension) {
                return Err(AdmissionError::RangeDimension { id });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LawName, Pressure, Reviewed};

    const SECOND: u64 = 1_000_000_000;

    fn reading() -> RowKind {
        RowKind::Algebraic(
            LawName {
                owner: "XX",
                local: "reading",
            }
            .into(),
        )
    }

    fn other_reading() -> RowKind {
        RowKind::Algebraic(
            LawName {
                owner: "XX",
                local: "other",
            }
            .into(),
        )
    }

    fn limit(value: f64) -> Measure<Pressure> {
        Measure::new(value).expect("finite")
    }

    fn sliding(seconds: u64) -> Averaging {
        Averaging::Sliding(NonZeroU64::new(seconds * SECOND).expect("not zero"))
    }

    fn range(
        id: &'static str,
        scope: Scope,
        averaging: Averaging,
        low: f64,
        high: f64,
    ) -> Result<HealthyRange<SyntheticFixture>, AdmissionError> {
        HealthyRange::synthetic(id, reading(), "a synthetic gauge", scope, averaging, limit(low), limit(high), "synthetic")
    }

    fn variation(id: &'static str, row: RowKind, sd: f64) -> Result<WithinSubjectVariation<SyntheticFixture>, AdmissionError> {
        WithinSubjectVariation::synthetic(id, row, "a synthetic gauge", limit(sd), "synthetic")
    }

    fn every(id: &'static str) -> HealthyRange<SyntheticFixture> {
        range(id, Scope::Every, sliding(4), 0.0, 20.0).expect("a range")
    }

    #[test]
    fn a_range_whose_high_limit_is_below_its_low_one_is_refused() {
        let reversed = range("xx.range", Scope::Every, sliding(4), 20.0, 10.0);
        assert_eq!(reversed.err(), Some(AdmissionError::ReversedRange { id: "xx.range" }));
        assert!(range("xx.range", Scope::Every, sliding(4), 10.0, 10.0).is_ok());
    }

    #[test]
    fn a_whole_period_holds_only_during_one_activity() {
        let night = Activity::new("asleep");
        let day = Activity::new("seated");
        let refused = Some(AdmissionError::WholePeriodScope { id: "xx.range" });
        assert_eq!(range("xx.range", Scope::Every, Averaging::WholePeriod, 0.0, 20.0).err(), refused);
        let change = Scope::Change { from: night, to: day };
        assert_eq!(range("xx.range", change, Averaging::WholePeriod, 0.0, 20.0).err(), refused);
        assert!(range("xx.range", Scope::During(night), Averaging::WholePeriod, 0.0, 20.0).is_ok());
    }

    #[test]
    fn a_negative_within_subject_variation_is_refused() {
        let refused = variation("xx.variation", reading(), -1.0);
        assert_eq!(refused.err(), Some(AdmissionError::NegativeVariation { id: "xx.variation" }));
        assert!(variation("xx.variation", reading(), 0.0).is_ok());
    }

    #[test]
    fn a_set_holds_one_entry_per_id_per_reading_scope_and_averaging_and_one_variation_per_reading() {
        let repeated = |id| Some(AdmissionError::DuplicateRangeEntry { id });
        let set = HealthyRangeSet::synthetic(vec![every("xx.a"), every("xx.a")], Vec::new());
        assert_eq!(set.err(), repeated("xx.a"));
        let set = HealthyRangeSet::synthetic(vec![every("xx.a"), every("xx.b")], Vec::new());
        assert_eq!(set.err(), repeated("xx.b"));
        let longer = range("xx.b", Scope::Every, sliding(8), 0.0, 20.0).expect("a range");
        assert!(HealthyRangeSet::synthetic(vec![every("xx.a"), longer], Vec::new()).is_ok());
        let variations = vec![
            variation("xx.v", reading(), 1.0).expect("a variation"),
            variation("xx.w", reading(), 2.0).expect("a variation"),
        ];
        assert_eq!(HealthyRangeSet::synthetic(Vec::new(), variations).err(), repeated("xx.w"));
        let variations = vec![variation("xx.a", other_reading(), 1.0).expect("a variation")];
        assert_eq!(HealthyRangeSet::synthetic(vec![every("xx.a")], variations).err(), repeated("xx.a"));
    }

    #[test]
    fn a_set_keeps_its_entries_in_id_order() {
        let set = HealthyRangeSet::synthetic(vec![every("xx.b"), every_longer("xx.a")], Vec::new()).expect("a set");
        let ids: Vec<&str> = set.ranges().iter().map(HealthyRange::id).collect();
        assert_eq!(ids, ["xx.a", "xx.b"]);
    }

    fn every_longer(id: &'static str) -> HealthyRange<SyntheticFixture> {
        range(id, Scope::Every, sliding(8), 0.0, 20.0).expect("a range")
    }

    #[test]
    fn a_set_digest_covers_every_field_of_every_entry() {
        let night = Activity::new("asleep");
        let day = Activity::new("seated");
        let digest = |ranges: Vec<HealthyRange<SyntheticFixture>>, variations| {
            HealthyRangeSet::synthetic(ranges, variations).expect("a set").digest()
        };
        let base = || {
            (
                vec![range("xx.a", Scope::During(night), sliding(4), 0.0, 20.0).expect("a range")],
                vec![variation("xx.v", reading(), 1.0).expect("a variation")],
            )
        };
        let (ranges, variations) = base();
        let first = digest(ranges, variations);
        let (ranges, variations) = base();
        assert_eq!(digest(ranges, variations), first, "one set, one digest");
        let changed: Vec<(&str, HealthyRange<SyntheticFixture>)> = vec![
            ("id", range("xx.b", Scope::During(night), sliding(4), 0.0, 20.0).expect("a range")),
            ("scope", range("xx.a", Scope::During(day), sliding(4), 0.0, 20.0).expect("a range")),
            ("scope kind", range("xx.a", Scope::Every, sliding(4), 0.0, 20.0).expect("a range")),
            (
                "change",
                range("xx.a", Scope::Change { from: night, to: day }, sliding(4), 0.0, 20.0).expect("a range"),
            ),
            ("averaging", range("xx.a", Scope::During(night), sliding(8), 0.0, 20.0).expect("a range")),
            ("whole period", range("xx.a", Scope::During(night), Averaging::WholePeriod, 0.0, 20.0).expect("a range")),
            ("low", range("xx.a", Scope::During(night), sliding(4), 1.0, 20.0).expect("a range")),
            ("high", range("xx.a", Scope::During(night), sliding(4), 0.0, 21.0).expect("a range")),
            (
                "row",
                HealthyRange::synthetic("xx.a", other_reading(), "a synthetic gauge", Scope::During(night), sliding(4), limit(0.0), limit(20.0), "synthetic")
                    .expect("a range"),
            ),
            (
                "method",
                HealthyRange::synthetic("xx.a", reading(), "another gauge", Scope::During(night), sliding(4), limit(0.0), limit(20.0), "synthetic")
                    .expect("a range"),
            ),
            (
                "dimension",
                HealthyRange::synthetic("xx.a", reading(), "a synthetic gauge", Scope::During(night), sliding(4), Measure::<crate::Volume>::new(0.0).expect("finite"), Measure::new(20.0).expect("finite"), "synthetic")
                    .expect("a range"),
            ),
            (
                "note",
                HealthyRange::synthetic("xx.a", reading(), "a synthetic gauge", Scope::During(night), sliding(4), limit(0.0), limit(20.0), "another note")
                    .expect("a range"),
            ),
        ];
        for (field, changed) in changed {
            let (_, variations) = base();
            assert_ne!(digest(vec![changed], variations), first, "a range's {field}");
        }
        let changed = vec![
            ("id", variation("xx.w", reading(), 1.0).expect("a variation")),
            ("row", variation("xx.v", other_reading(), 1.0).expect("a variation")),
            ("sd", variation("xx.v", reading(), 2.0).expect("a variation")),
            (
                "method",
                WithinSubjectVariation::synthetic("xx.v", reading(), "another gauge", limit(1.0), "synthetic")
                    .expect("a variation"),
            ),
            (
                "dimension",
                WithinSubjectVariation::synthetic("xx.v", reading(), "a synthetic gauge", Measure::<crate::Volume>::new(1.0).expect("finite"), "synthetic")
                    .expect("a variation"),
            ),
            (
                "note",
                WithinSubjectVariation::synthetic("xx.v", reading(), "a synthetic gauge", limit(1.0), "another note")
                    .expect("a variation"),
            ),
        ];
        for (field, changed) in changed {
            let (ranges, _) = base();
            assert_ne!(digest(ranges, vec![changed]), first, "a variation's {field}");
        }
        let (ranges, _) = base();
        assert_ne!(digest(ranges, Vec::new()), first, "the variations it holds");
    }

    #[test]
    fn a_synthetic_set_and_a_reviewed_one_seal_their_kind() {
        let empty = HealthyRangeSet::<Reviewed> {
            ranges: Vec::new(),
            variations: Vec::new(),
        };
        let synthetic = HealthyRangeSet::synthetic(Vec::new(), Vec::new()).expect("a set");
        assert_ne!(empty.digest(), synthetic.digest());
    }
}
