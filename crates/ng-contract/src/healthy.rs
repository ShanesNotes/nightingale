use crate::identity::{Canonical, Seal};
use crate::provenance::seal_trace;
use crate::quantity::dimension;
use crate::{
    AdmissionError, Compensated, CompiledBody, Digest, Dimension, Measure, PacketKind, RowKind,
    SyntheticFixture,
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
        let variations =
            (set.variations.iter()).map(|variation| (variation.id, variation.row, &variation.dimension));
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
    use crate::{Context, LawName, Pressure, Reviewed, Slot, Volume};

    const SECOND: u64 = 1_000_000_000;
    const NIGHT: Activity = Activity::new("asleep");
    const DAY: Activity = Activity::new("seated");

    const fn reading(local: &'static str) -> RowKind {
        RowKind::Algebraic(Slot {
            law: LawName { owner: "XX", local },
            at: Context::Body,
        })
    }

    fn limit(value: f64) -> Measure<Pressure> {
        Measure::new(value).expect("finite")
    }

    fn sliding(seconds: u64) -> Averaging {
        Averaging::Sliding(NonZeroU64::new(seconds * SECOND).expect("not zero"))
    }

    /// One field changed, by name.
    type Change<T> = (&'static str, fn(&mut T));

    /// A synthetic range's arguments, changed one at a time.
    #[derive(Clone, Copy)]
    struct Range {
        id: &'static str,
        row: RowKind,
        method: &'static str,
        scope: Scope,
        averaging: Averaging,
        low: f64,
        high: f64,
        note: &'static str,
    }
    impl Range {
        fn of(id: &'static str) -> Self {
            Self {
                id,
                row: reading("reading"),
                method: "a synthetic gauge",
                scope: Scope::Every,
                averaging: sliding(4),
                low: 0.0,
                high: 20.0,
                note: "synthetic",
            }
        }
        fn built(self) -> Result<HealthyRange<SyntheticFixture>, AdmissionError> {
            let Self {
                id,
                row,
                method,
                scope,
                averaging,
                low,
                high,
                note,
            } = self;
            HealthyRange::synthetic(id, row, method, scope, averaging, limit(low), limit(high), note)
        }
        fn range(self) -> HealthyRange<SyntheticFixture> {
            self.built().expect("a range")
        }
    }

    /// A synthetic variation's arguments, changed one at a time.
    #[derive(Clone, Copy)]
    struct Variation {
        id: &'static str,
        row: RowKind,
        method: &'static str,
        sd: f64,
        note: &'static str,
    }
    impl Variation {
        fn of(id: &'static str) -> Self {
            Self {
                id,
                row: reading("reading"),
                method: "a synthetic gauge",
                sd: 1.0,
                note: "synthetic",
            }
        }
        fn built(self) -> Result<WithinSubjectVariation<SyntheticFixture>, AdmissionError> {
            let Self {
                id,
                row,
                method,
                sd,
                note,
            } = self;
            WithinSubjectVariation::synthetic(id, row, method, limit(sd), note)
        }
        fn variation(self) -> WithinSubjectVariation<SyntheticFixture> {
            self.built().expect("a variation")
        }
    }

    fn set(
        ranges: Vec<HealthyRange<SyntheticFixture>>,
        variations: Vec<WithinSubjectVariation<SyntheticFixture>>,
    ) -> Result<HealthyRangeSet<SyntheticFixture>, AdmissionError> {
        HealthyRangeSet::synthetic(ranges, variations)
    }

    #[test]
    fn a_range_whose_high_limit_is_below_its_low_one_is_refused() {
        let reversed = Range {
            low: 20.0,
            high: 10.0,
            ..Range::of("xx.range")
        };
        let refused = Some(AdmissionError::ReversedRange { id: "xx.range" });
        assert_eq!(reversed.built().err(), refused);
        let point = Range {
            low: 10.0,
            high: 10.0,
            ..Range::of("xx.range")
        };
        assert!(point.built().is_ok());
    }

    #[test]
    fn a_whole_period_holds_only_during_one_activity() {
        let whole = |scope| Range {
            scope,
            averaging: Averaging::WholePeriod,
            ..Range::of("xx.range")
        };
        let refused = Some(AdmissionError::WholePeriodScope { id: "xx.range" });
        assert_eq!(whole(Scope::Every).built().err(), refused);
        let change = Scope::Change { from: NIGHT, to: DAY };
        assert_eq!(whole(change).built().err(), refused);
        assert!(whole(Scope::During(NIGHT)).built().is_ok());
    }

    #[test]
    fn a_negative_within_subject_variation_is_refused() {
        let negative = Variation {
            sd: -1.0,
            ..Variation::of("xx.variation")
        };
        let refused = Some(AdmissionError::NegativeVariation { id: "xx.variation" });
        assert_eq!(negative.built().err(), refused);
        let none = Variation {
            sd: 0.0,
            ..Variation::of("xx.variation")
        };
        assert!(none.built().is_ok());
    }

    #[test]
    fn a_set_holds_one_entry_per_id_per_reading_scope_and_averaging_and_one_variation_per_reading() {
        let repeated = |id| Some(AdmissionError::DuplicateRangeEntry { id });
        let twice = vec![Range::of("xx.a").range(), Range::of("xx.a").range()];
        assert_eq!(set(twice, Vec::new()).err(), repeated("xx.a"));
        let same = vec![Range::of("xx.a").range(), Range::of("xx.b").range()];
        assert_eq!(set(same, Vec::new()).err(), repeated("xx.b"));
        let longer = Range {
            averaging: sliding(8),
            ..Range::of("xx.b")
        };
        assert!(set(vec![Range::of("xx.a").range(), longer.range()], Vec::new()).is_ok());
        let one_reading = vec![
            Variation::of("xx.v").variation(),
            Variation::of("xx.w").variation(),
        ];
        assert_eq!(set(Vec::new(), one_reading).err(), repeated("xx.w"));
        let elsewhere = Variation {
            row: reading("other"),
            ..Variation::of("xx.a")
        };
        let shared_id = set(vec![Range::of("xx.a").range()], vec![elsewhere.variation()]);
        assert_eq!(shared_id.err(), repeated("xx.a"));
    }

    #[test]
    fn a_set_keeps_its_entries_in_id_order() {
        let later = Range {
            averaging: sliding(8),
            ..Range::of("xx.a")
        };
        let set = set(vec![Range::of("xx.b").range(), later.range()], Vec::new()).expect("a set");
        let ids: Vec<&str> = set.ranges().iter().map(HealthyRange::id).collect();
        assert_eq!(ids, ["xx.a", "xx.b"]);
    }

    #[test]
    fn a_set_digest_covers_every_field_of_every_entry() {
        let mut range = Range::of("xx.a");
        range.scope = Scope::During(NIGHT);
        let variation = Variation::of("xx.v");
        let digest = |ranges, variations| set(ranges, variations).expect("a set").digest();
        let first = digest(vec![range.range()], vec![variation.variation()]);
        let again = digest(vec![range.range()], vec![variation.variation()]);
        assert_eq!(again, first, "one set, one digest");
        let ranges: [Change<Range>; 11] = [
            ("id", |range| range.id = "xx.b"),
            ("row", |range| range.row = reading("other")),
            ("method", |range| range.method = "another gauge"),
            ("activity", |range| range.scope = Scope::During(DAY)),
            ("scope", |range| range.scope = Scope::Every),
            ("change", |range| {
                range.scope = Scope::Change { from: NIGHT, to: DAY }
            }),
            ("length", |range| range.averaging = sliding(8)),
            ("averaging", |range| range.averaging = Averaging::WholePeriod),
            ("low", |range| range.low = 1.0),
            ("high", |range| range.high = 21.0),
            ("note", |range| range.note = "another note"),
        ];
        for (field, change) in ranges {
            let mut changed = range;
            change(&mut changed);
            let changed = digest(vec![changed.range()], vec![variation.variation()]);
            assert_ne!(changed, first, "a range's {field}");
        }
        let (low, high) = (Measure::<Volume>::new(0.0), Measure::<Volume>::new(20.0));
        let (low, high) = (low.expect("finite"), high.expect("finite"));
        let Range {
            id,
            row,
            method,
            scope,
            averaging,
            note,
            ..
        } = range;
        let volume = HealthyRange::synthetic(id, row, method, scope, averaging, low, high, note);
        let changed = digest(vec![volume.expect("a range")], vec![variation.variation()]);
        assert_ne!(changed, first, "a range's dimension");
        let variations: [Change<Variation>; 5] = [
            ("id", |variation| variation.id = "xx.w"),
            ("row", |variation| variation.row = reading("other")),
            ("method", |variation| variation.method = "another gauge"),
            ("sd", |variation| variation.sd = 2.0),
            ("note", |variation| variation.note = "another note"),
        ];
        for (field, change) in variations {
            let mut changed = variation;
            change(&mut changed);
            let changed = digest(vec![range.range()], vec![changed.variation()]);
            assert_ne!(changed, first, "a variation's {field}");
        }
        let sd = Measure::<Volume>::new(1.0).expect("finite");
        let volume =
            WithinSubjectVariation::synthetic("xx.v", variation.row, variation.method, sd, "synthetic");
        let changed = digest(vec![range.range()], vec![volume.expect("a variation")]);
        assert_ne!(changed, first, "a variation's dimension");
        let none = digest(vec![range.range()], Vec::new());
        assert_ne!(none, first, "the variations it holds");
    }

    #[test]
    fn a_synthetic_set_and_a_reviewed_one_seal_their_kind() {
        let reviewed = HealthyRangeSet::<Reviewed> {
            ranges: Vec::new(),
            variations: Vec::new(),
        };
        let synthetic = set(Vec::new(), Vec::new()).expect("a set");
        assert_ne!(reviewed.digest(), synthetic.digest());
    }
}
