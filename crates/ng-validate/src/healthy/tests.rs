use super::range::RangeFold;
use super::returns::{AnchorPlan, ReturnFold};
use super::*;
use crate::{ReplayCase, TwoReservoirs};
use core::num::NonZeroU32;
use ng_contract::{
    AnchorStatistic, Averaging, Compensated, HealthyRange, LawName, Measure, ParameterPacket, Pressure,
    Scope, SyntheticFixture, Water, WithinSubjectVariation,
};

const SECOND: u64 = 1_000_000_000;
const HOUR: u64 = 3_600 * SECOND;
const METHOD: &str = "a synthetic gauge";

fn row(local: &'static str) -> RowKind {
    RowKind::Algebraic(LawName { owner: "XX", local }.into())
}

fn reading() -> RowKind {
    row("reading")
}

fn sampled(row: RowKind) -> BareReading {
    BareReading {
        row,
        method: METHOD,
        taken: Taken::Sampled,
    }
}

fn time_mean(row: RowKind) -> BareReading {
    BareReading {
        row,
        method: METHOD,
        taken: Taken::TimeMean,
    }
}

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn pressure(value: f64) -> Measure<Pressure> {
    Measure::new(value).expect("finite")
}

fn value(value: f64) -> Compensated {
    Compensated::of(&pressure(value))
}

fn sliding(length_ns: u64) -> Averaging {
    Averaging::Sliding(NonZeroU64::new(length_ns).expect("not zero"))
}

fn range(
    id: &'static str,
    scope: Scope,
    averaging: Averaging,
    low: f64,
    high: f64,
) -> HealthyRange<SyntheticFixture> {
    let (low, high) = (pressure(low), pressure(high));
    HealthyRange::synthetic(id, reading(), METHOD, scope, averaging, low, high, "synthetic").expect("a range")
}

fn variation(sd: f64) -> WithinSubjectVariation<SyntheticFixture> {
    WithinSubjectVariation::synthetic("xx.variation", reading(), METHOD, pressure(sd), "synthetic")
        .expect("a variation")
}

fn set(
    ranges: Vec<HealthyRange<SyntheticFixture>>,
    variations: Vec<WithinSubjectVariation<SyntheticFixture>>,
) -> HealthyRangeSet<SyntheticFixture> {
    HealthyRangeSet::synthetic(ranges, variations).expect("a set")
}

fn day(spans: &[(u64, &'static str)]) -> DayActivities {
    let spans = spans
        .iter()
        .map(|&(start, name)| (at(start), Activity::new(name)));
    DayActivities::new(spans.collect()).expect("a day")
}

fn morning(days: u32) -> NonZeroU32 {
    NonZeroU32::new(days).expect("not zero")
}

#[derive(Clone, Copy)]
struct Read {
    reading: usize,
    at: u64,
    value: Option<f64>,
}

fn each_period(from: u64, period_ns: u64, values: &[f64]) -> Vec<Read> {
    (0u64..)
        .zip(values)
        .map(|(index, &read)| Read {
            reading: 0,
            at: from + index * period_ns,
            value: Some(read),
        })
        .collect()
}

fn unread() -> Digest {
    Digest::of_canonical(b"a fold read with no run")
}

fn ranged(
    readings: &[BareReading],
    period_ns: u64,
    set: &HealthyRangeSet<SyntheticFixture>,
    day: &DayActivities,
    admission: u64,
    stream: &[Read],
) -> RangeReport {
    let mut fold = RangeFold::new(readings, period_ns, set, day, at(admission)).expect("a fold");
    for read in stream {
        fold.feed(read.reading, at(read.at), read.value.map(value));
    }
    let (results, gaps) = fold.finish();
    RangeReport {
        run: unread(),
        seal: unread(),
        set: unread(),
        lens: unread(),
        day: unread(),
        results,
        gaps,
    }
}

fn each_second(set: &HealthyRangeSet<SyntheticFixture>, day: &DayActivities, values: &[f64]) -> RangeReport {
    ranged(
        &[sampled(reading())],
        SECOND,
        set,
        day,
        0,
        &each_period(0, SECOND, values),
    )
}

fn result(report: &RangeReport, id: &str) -> RangeResult {
    *(report.results().iter())
        .find(|result| result.range() == id)
        .expect("a result for the range")
}

fn failure(report: &RangeReport, id: &str) -> (SimTime, u64, Option<Compensated>) {
    let failure = result(report, id).failure().expect("a window failed");
    (failure.end(), failure.length_ns(), failure.mean())
}

fn returned(
    readings: &[BareReading],
    set: &HealthyRangeSet<SyntheticFixture>,
    plans: &[AnchorPlan],
    stream: &[Read],
) -> ReturnReport {
    let mut fold = ReturnFold::new(readings, HOUR, set, plans, at(0)).expect("a fold");
    for read in stream {
        fold.feed(read.reading, at(read.at), read.value.map(value));
    }
    let (results, gaps) = fold.finish();
    ReturnReport {
        run: unread(),
        seal: unread(),
        set: unread(),
        lens: unread(),
        packet: ParameterPacket::<SyntheticFixture>::new().anchors().packet(),
        results,
        gaps,
    }
}

fn anchor(id: &'static str, statistic: AnchorStatistic, window_ns: u64) -> AnchorPlan {
    AnchorPlan {
        anchor: AnchorId::new(id),
        row: reading(),
        method: METHOD,
        statistic,
        window_ns,
    }
}

fn instant() -> AnchorPlan {
    anchor("xx.anchor", AnchorStatistic::Instant, 0)
}

fn hourly(hours: u64, read: impl Fn(u64) -> f64) -> Vec<Read> {
    let values: Vec<f64> = (0..=hours).map(read).collect();
    each_period(0, HOUR, &values)
}

fn returns_of(report: &ReturnReport) -> Vec<(AnchorId, u32, SimTime, Option<Compensated>, bool)> {
    (report.results().iter())
        .map(|result| {
            (
                result.anchor(),
                result.morning().get(),
                result.at(),
                result.reading(),
                result.passed(),
            )
        })
        .collect()
}

#[test]
fn a_window_outside_its_range_fails_naming_the_reading_and_the_window() {
    let zero_to_twenty = set(
        vec![range("xx.range", Scope::Every, sliding(2 * SECOND), 0.0, 20.0)],
        Vec::new(),
    );
    let values = [10.0, 10.0, 10.0, 10.0, 30.0, 30.0, 10.0, 10.0, 10.0];
    let report = each_second(&zero_to_twenty, &day(&[(0, "awake")]), &values);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    assert_eq!(report.gaps(), []);
    let checked = result(&report, "xx.range");
    assert_eq!((checked.row(), checked.method()), (reading(), METHOD));
    assert_eq!(checked.judged(), 8, "each window of two of the nine values");
    let named = (at(5 * SECOND), 2 * SECOND, Some(value(30.0)));
    assert_eq!(
        failure(&report, "xx.range"),
        named,
        "only the values at 4 s and 5 s leave 0 to 20"
    );
    let ten_to_twenty = set(
        vec![range("xx.range", Scope::Every, sliding(2 * SECOND), 10.0, 20.0)],
        Vec::new(),
    );
    let report = each_second(
        &ten_to_twenty,
        &day(&[(0, "awake")]),
        &[15.0, 15.0, 5.0, 5.0, 15.0],
    );
    assert_eq!(
        failure(&report, "xx.range"),
        (at(3 * SECOND), 2 * SECOND, Some(value(5.0))),
        "the values at 2 s and 3 s lie below 10 to 20, and each window holding one 5 and one 15 sits on its low limit"
    );
}

#[test]
fn a_window_holding_a_value_that_was_not_present_fails_with_no_mean() {
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(2 * SECOND), 0.0, 20.0)],
        Vec::new(),
    );
    let mut stream = each_period(0, SECOND, &[10.0, 10.0, 10.0, 10.0]);
    stream[1].value = None;
    let report = ranged(
        &[sampled(reading())],
        SECOND,
        &set,
        &day(&[(0, "awake")]),
        0,
        &stream,
    );
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    assert_eq!(result(&report, "xx.range").judged(), 3);
    assert_eq!(
        failure(&report, "xx.range"),
        (at(SECOND), 2 * SECOND, None),
        "the window of the values at 0 s and 1 s, where a zero would leave a mean of 5 inside 0 to 20"
    );
}

#[test]
fn a_reading_that_leaves_its_range_between_two_tumbling_means_fails_a_sliding_window() {
    let values = [5.0, 5.0, 14.0, 14.0, 14.0, 14.0, 5.0, 5.0];
    let tumbling: Vec<f64> = values
        .chunks(4)
        .map(|four| four.iter().sum::<f64>() / 4.0)
        .collect();
    assert_eq!(
        tumbling,
        [9.5, 9.5],
        "each tumbling mean of four stays inside 0 to 10"
    );
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(4 * SECOND), 0.0, 10.0)],
        Vec::new(),
    );
    let report = each_second(&set, &day(&[(0, "awake")]), &values);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    assert_eq!(
        result(&report, "xx.range").judged(),
        5,
        "a window ends at each value from the fourth on"
    );
    let named = (at(4 * SECOND), 4 * SECOND, Some(value(11.75)));
    assert_eq!(
        failure(&report, "xx.range"),
        named,
        "the window of the values at 1 s to 4 s"
    );
}

fn change(from: &'static str, to: &'static str, lasting_ns: u64) -> Scope {
    Scope::Change {
        from: Activity::new(from),
        to: Activity::new(to),
        lasting: NonZeroU64::new(lasting_ns).expect("not zero"),
    }
}

fn sitting_up_for(lasting_ns: u64) -> Scope {
    change("supine", "seated", lasting_ns)
}

fn sitting_up(change_high: Option<f64>) -> HealthyRangeSet<SyntheticFixture> {
    let (supine, seated) = (Activity::new("supine"), Activity::new("seated"));
    let mut ranges = vec![
        range("xx.supine", Scope::During(supine), sliding(2 * SECOND), 0.0, 10.0),
        range(
            "xx.seated",
            Scope::During(seated),
            sliding(2 * SECOND),
            20.0,
            30.0,
        ),
    ];
    if let Some(high) = change_high {
        let change = sitting_up_for(SECOND);
        ranges.push(range("xx.sitting-up", change, sliding(2 * SECOND), 0.0, high));
    }
    set(ranges, Vec::new())
}

const AROUND_SITTING_UP: [f64; 8] = [5.0, 5.0, 5.0, 5.0, 25.0, 25.0, 25.0, 25.0];

#[test]
fn a_window_across_a_change_takes_the_changes_own_range() {
    let day = day(&[(0, "supine"), (4 * SECOND, "seated")]);
    let report = each_second(&sitting_up(Some(30.0)), &day, &AROUND_SITTING_UP);
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "15 across the change, outside 0 to 10 and 20 to 30"
    );
    assert_eq!(report.gaps(), []);
    assert_eq!(
        result(&report, "xx.sitting-up").judged(),
        2,
        "the window across the change, and the one from the change in the 1 s its shift lasts"
    );
    let report = each_second(&sitting_up(Some(12.0)), &day, &AROUND_SITTING_UP);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    let named = (at(4 * SECOND), 2 * SECOND, Some(value(15.0)));
    assert_eq!(
        failure(&report, "xx.sitting-up"),
        named,
        "the window of the values at 3 s and 4 s"
    );
    assert_eq!(result(&report, "xx.supine").failure(), None);
    assert_eq!(result(&report, "xx.seated").failure(), None);
}

fn sitting_up_dips(mut ranges: Vec<HealthyRange<SyntheticFixture>>) -> HealthyRangeSet<SyntheticFixture> {
    ranges.push(range(
        "xx.every",
        Scope::Every,
        sliding(4 * SECOND),
        4500.0,
        7000.0,
    ));
    let change = sitting_up_for(4 * SECOND);
    ranges.push(range(
        "xx.sitting-up",
        change,
        sliding(4 * SECOND),
        3500.0,
        7000.0,
    ));
    set(ranges, Vec::new())
}

fn dipping(dip: usize, values: usize) -> Vec<f64> {
    let mut read = vec![5000.0; values];
    read[5..5 + dip].fill(3750.0);
    read
}

#[test]
fn a_changes_range_judges_each_window_in_the_time_its_shift_lasts() {
    let day = day(&[(0, "supine"), (4 * SECOND, "seated")]);
    let report = each_second(&sitting_up_dips(Vec::new()), &day, &dipping(3, 12));
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "a dip to 3750 in the 4 s from the change"
    );
    assert_eq!(report.gaps(), []);
    let judged = (
        result(&report, "xx.sitting-up").judged(),
        result(&report, "xx.every").judged(),
    );
    assert_eq!(
        judged,
        (7, 2),
        "the windows ending at 4 s to 10 s cross the change or start in its 4 s"
    );
    let report = each_second(&sitting_up_dips(Vec::new()), &day, &dipping(6, 15));
    assert_eq!(result(&report, "xx.sitting-up").failure(), None);
    assert_eq!(
        failure(&report, "xx.every"),
        (at(11 * SECOND), 4 * SECOND, Some(value(4062.5))),
        "a dip that outlasts the shift fails the first window wholly past it, the values at 8 s to 11 s"
    );
}

#[test]
fn a_change_owns_the_time_its_longest_lasting_range_states() {
    let brief = range(
        "xx.brief",
        sitting_up_for(2 * SECOND),
        sliding(2 * SECOND),
        3500.0,
        7000.0,
    );
    let day = day(&[(0, "supine"), (4 * SECOND, "seated")]);
    let report = each_second(&sitting_up_dips(vec![brief]), &day, &dipping(3, 12));
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "the 4 s windows from 6 s and 7 s lie in the 4 s the longer range states, not the 2 s"
    );
    assert_eq!(result(&report, "xx.every").judged(), 2);
}

#[test]
fn a_window_of_a_length_the_change_has_no_range_for_is_not_judged_in_the_time_its_shift_lasts() {
    let set = set(
        vec![
            range("xx.every", Scope::Every, sliding(4 * SECOND), 0.0, 20.0),
            range(
                "xx.sitting-up",
                sitting_up_for(4 * SECOND),
                sliding(2 * SECOND),
                0.0,
                40.0,
            ),
        ],
        Vec::new(),
    );
    let day = day(&[(0, "supine"), (8 * SECOND, "seated")]);
    let mut values = [10.0; 16];
    values[6..10].fill(30.0);
    let report = each_second(&set, &day, &values);
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "the 4 s windows holding 30s cross the change or start in its 4 s, which its 2 s windows cover"
    );
    assert_eq!(report.gaps(), []);
    let judged = (
        result(&report, "xx.sitting-up").judged(),
        result(&report, "xx.every").judged(),
    );
    assert_eq!(
        judged,
        (5, 6),
        "2 s windows ending at 8 s to 12 s; 4 s windows ending at 3 s to 7 s and at 15 s"
    );
}

#[test]
fn a_window_that_belongs_to_two_changes_blocks_at_the_first_after_its_first_value() {
    let set = set(
        vec![
            range("xx.every", Scope::Every, sliding(4 * SECOND), 0.0, 100.0),
            range(
                "xx.sitting-up",
                change("supine", "seated", 4 * SECOND),
                sliding(4 * SECOND),
                0.0,
                100.0,
            ),
            range(
                "xx.standing-up",
                change("seated", "standing", 4 * SECOND),
                sliding(4 * SECOND),
                0.0,
                100.0,
            ),
        ],
        Vec::new(),
    );
    let day = day(&[(0, "supine"), (4 * SECOND, "seated"), (6 * SECOND, "standing")]);
    let report = each_second(&set, &day, &[50.0; 12]);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = |from, to, seconds| RangeGap::NoChangeRange {
        row: reading(),
        method: METHOD,
        from: Activity::new(from),
        to: Activity::new(to),
        at: at(seconds * SECOND),
    };
    assert_eq!(
        report.gaps(),
        [named("supine", "seated", 4), named("seated", "standing", 6)],
        "the window from 3 s crosses both changes, and those from 4 s and 5 s start in sitting up's 4 s and cross standing up"
    );
}

#[test]
fn a_change_between_activities_with_different_ranges_and_none_of_its_own_blocks_naming_the_change() {
    let day = day(&[(0, "supine"), (4 * SECOND, "seated")]);
    let report = each_second(&sitting_up(None), &day, &AROUND_SITTING_UP);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::NoChangeRange {
        row: reading(),
        method: METHOD,
        from: Activity::new("supine"),
        to: Activity::new("seated"),
        at: at(4 * SECOND),
    };
    assert_eq!(report.gaps(), [named]);
    assert!(report.results().iter().all(|result| result.failure().is_none()));
}

#[test]
fn a_change_between_activities_that_share_the_every_activity_range_is_judged_by_it() {
    let day = day(&[(0, "supine"), (4 * SECOND, "seated")]);
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(2 * SECOND), 0.0, 30.0)],
        Vec::new(),
    );
    let report = each_second(&set, &day, &AROUND_SITTING_UP);
    assert_eq!(report.verdict(), CheckVerdict::Pass);
    assert_eq!(report.gaps(), []);
    assert_eq!(
        result(&report, "xx.range").judged(),
        7,
        "the window across the change among them"
    );
}

#[test]
fn a_window_across_two_shared_changes_is_judged_by_the_every_activity_range() {
    let set = set(
        vec![range("xx.every", Scope::Every, sliding(4 * SECOND), 0.0, 20.0)],
        Vec::new(),
    );
    let day = day(&[(0, "a"), (2 * SECOND, "b"), (4 * SECOND, "c")]);
    let report = each_second(&set, &day, &[0.0, 30.0, 20.0, 20.0, 30.0, 0.0, 0.0, 0.0]);
    assert_eq!(report.gaps(), []);
    assert_eq!(
        failure(&report, "xx.every"),
        (at(4 * SECOND), 4 * SECOND, Some(value(25.0))),
        "the window of the values at 1 s to 4 s, from a through b to c"
    );
}

fn nights(night: [f64; 4]) -> RangeReport {
    let day = day(&[
        (0, "asleep"),
        (4 * SECOND, "awake"),
        (8 * SECOND, "asleep"),
        (12 * SECOND, "awake"),
        (16 * SECOND, "asleep"),
    ]);
    let asleep = Scope::During(Activity::new("asleep"));
    let set = set(
        vec![
            range("xx.awake", Scope::Every, sliding(SECOND), 0.0, 100.0),
            range("xx.night", asleep, Averaging::WholePeriod, 0.0, 10.0),
        ],
        Vec::new(),
    );
    let [first, second, third, fourth] = night;
    let values = [
        40.0, 40.0, 50.0, 50.0, 50.0, 50.0, first, second, third, fourth, 50.0, 50.0, 50.0, 50.0, 40.0, 40.0,
    ];
    ranged(
        &[sampled(reading())],
        SECOND,
        &set,
        &day,
        2 * SECOND,
        &each_period(2 * SECOND, SECOND, &values),
    )
}

#[test]
fn a_whole_period_range_judges_each_whole_periods_mean_and_counts_the_periods_the_run_cuts() {
    let report = nights([2.0, 2.0, 30.0, 2.0]);
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "a brief 30 in a night whose mean is 9"
    );
    assert_eq!(report.gaps(), []);
    let night = result(&report, "xx.night");
    assert_eq!(night.judged(), 1, "the one night the run read whole");
    assert_eq!(
        night.cut(),
        2,
        "the night admission cut and the night the run's end cut, each at 40"
    );
    assert_eq!(result(&report, "xx.awake").judged(), 8, "each awake sample alone");
    let report = nights([12.0, 12.0, 12.0, 12.0]);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    let named = (at(11 * SECOND), 4 * SECOND, Some(value(12.0)));
    assert_eq!(failure(&report, "xx.night"), named, "the night from 8 s to 12 s");
}

fn night_and_day() -> HealthyRangeSet<SyntheticFixture> {
    let asleep = Scope::During(Activity::new("asleep"));
    set(
        vec![
            range("xx.awake", Scope::Every, sliding(SECOND), 0.0, 100.0),
            range("xx.night", asleep, Averaging::WholePeriod, 0.0, 10.0),
        ],
        Vec::new(),
    )
}

#[test]
fn a_period_that_starts_at_admission_is_whole() {
    let day = day(&[(2 * SECOND, "asleep"), (6 * SECOND, "awake")]);
    let stream = each_period(2 * SECOND, SECOND, &[5.0, 5.0, 5.0, 5.0, 50.0, 50.0]);
    let report = ranged(
        &[sampled(reading())],
        SECOND,
        &night_and_day(),
        &day,
        2 * SECOND,
        &stream,
    );
    let night = result(&report, "xx.night");
    assert_eq!(
        (night.judged(), night.cut()),
        (1, 0),
        "the night from admission at 2 s to 6 s"
    );
}

#[test]
fn a_time_mean_whole_period_holds_every_mean_closing_in_its_span() {
    let day = day(&[(0, "awake"), (4 * SECOND, "asleep"), (8 * SECOND, "awake")]);
    let values = [50.0, 50.0, 50.0, 50.0, 12.0, 12.0, 12.0, 2.0, 50.0, 50.0];
    let stream = each_period(SECOND, SECOND, &values);
    let report = ranged(
        &[time_mean(reading())],
        SECOND,
        &night_and_day(),
        &day,
        0,
        &stream,
    );
    let night = result(&report, "xx.night");
    assert_eq!(
        (night.judged(), night.cut(), night.failure()),
        (1, 0, None),
        "mean 9.5 over the four means closing in (4 s, 8 s]"
    );
}

#[test]
fn a_whole_period_holding_a_value_that_was_not_present_fails_with_no_mean() {
    let day = day(&[(0, "awake"), (4 * SECOND, "asleep"), (8 * SECOND, "awake")]);
    let values = [50.0, 50.0, 50.0, 50.0, 2.0, 2.0, 2.0, 2.0, 50.0, 50.0];
    let mut stream = each_period(0, SECOND, &values);
    stream[5].value = None;
    let report = ranged(&[sampled(reading())], SECOND, &night_and_day(), &day, 0, &stream);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    let night = result(&report, "xx.night");
    assert_eq!((night.judged(), night.cut()), (1, 0));
    assert_eq!(
        failure(&report, "xx.night"),
        (at(7 * SECOND), 4 * SECOND, None),
        "the night from 4 s to 8 s, where a zero would leave a mean of 1.5 inside 0 to 10"
    );
}

#[test]
fn a_time_mean_takes_the_activity_of_the_period_it_covers() {
    let (a, b) = (Activity::new("a"), Activity::new("b"));
    let set = set(
        vec![
            range("xx.a", Scope::During(a), sliding(SECOND), 0.0, 10.0),
            range("xx.b", Scope::During(b), sliding(SECOND), 20.0, 30.0),
        ],
        Vec::new(),
    );
    let day = day(&[(0, "a"), (4 * SECOND, "b")]);
    let stream = each_period(SECOND, SECOND, &[5.0, 5.0, 5.0, 5.0, 25.0, 25.0, 25.0, 25.0]);
    let report = ranged(&[time_mean(reading())], SECOND, &set, &day, 0, &stream);
    assert_eq!(
        report.verdict(),
        CheckVerdict::Pass,
        "the mean closing at 4 s covers (3 s, 4 s], in a"
    );
    let judged = (result(&report, "xx.a").judged(), result(&report, "xx.b").judged());
    assert_eq!(judged, (4, 4));
}

#[test]
fn a_lens_reading_with_no_range_in_the_set_blocks_and_is_named() {
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(SECOND), 0.0, 100.0)],
        Vec::new(),
    );
    let readings = [sampled(reading()), sampled(row("unranged"))];
    let stream: Vec<Read> = (0..4)
        .flat_map(|second| {
            [0, 1].map(|reading| Read {
                reading,
                at: second * SECOND,
                value: Some(50.0),
            })
        })
        .collect();
    let report = ranged(&readings, SECOND, &set, &day(&[(0, "awake")]), 0, &stream);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::NoRange {
        row: row("unranged"),
        method: METHOD,
    };
    assert_eq!(report.gaps(), [named]);
}

#[test]
fn a_reading_with_no_range_is_named_once_however_the_lens_reads_it() {
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(SECOND), 0.0, 100.0)],
        Vec::new(),
    );
    let readings = [sampled(row("unranged")), time_mean(row("unranged"))];
    let report = ranged(&readings, SECOND, &set, &day(&[(0, "awake")]), 0, &[]);
    let named = RangeGap::NoRange {
        row: row("unranged"),
        method: METHOD,
    };
    assert_eq!(
        report.gaps(),
        [named],
        "one row by one method, sampled and time-averaged"
    );
}

#[test]
fn an_activity_with_no_range_blocks_and_is_named() {
    let awake = Scope::During(Activity::new("awake"));
    let set = set(
        vec![range("xx.range", awake, sliding(SECOND), 0.0, 100.0)],
        Vec::new(),
    );
    let report = each_second(&set, &day(&[(0, "awake"), (2 * SECOND, "asleep")]), &[50.0; 4]);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::NoActivityRange {
        row: reading(),
        method: METHOD,
        activity: Activity::new("asleep"),
    };
    assert_eq!(report.gaps(), [named], "named once for its two values");
}

#[test]
fn a_reading_the_run_holds_no_whole_window_of_blocks_and_is_named() {
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(8 * SECOND), 0.0, 100.0)],
        Vec::new(),
    );
    let report = each_second(&set, &day(&[(0, "awake")]), &[50.0; 4]);
    assert_eq!(result(&report, "xx.range").judged(), 0);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::Unjudged {
        range: "xx.range",
        row: reading(),
        method: METHOD,
    };
    assert_eq!(report.gaps(), [named], "four seconds hold no window of eight");
}

#[test]
fn a_range_whose_activity_the_run_holds_but_never_judges_blocks_and_is_named() {
    let day = day(&[(0, "awake"), (4 * SECOND, "asleep"), (20 * SECOND, "awake")]);
    let (asleep, exercising) = (Activity::new("asleep"), Activity::new("exercising"));
    let set = set(
        vec![
            range("xx.awake", Scope::Every, sliding(SECOND), 0.0, 100.0),
            range(
                "xx.exercise",
                Scope::During(exercising),
                sliding(SECOND),
                0.0,
                100.0,
            ),
            range(
                "xx.night",
                Scope::During(asleep),
                Averaging::WholePeriod,
                0.0,
                10.0,
            ),
            range(
                "xx.waking",
                change("asleep", "awake", SECOND),
                sliding(SECOND),
                0.0,
                100.0,
            ),
        ],
        Vec::new(),
    );
    let values = [
        50.0, 50.0, 50.0, 50.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0, 1000.0,
    ];
    let report = each_second(&set, &day, &values);
    let night = result(&report, "xx.night");
    assert_eq!((night.judged(), night.cut()), (0, 1), "the run ends in the night");
    assert_eq!(
        report.verdict(),
        CheckVerdict::Blocked,
        "every night value is 1000, against a night range of 0 to 10"
    );
    let named = RangeGap::Unjudged {
        range: "xx.night",
        row: reading(),
        method: METHOD,
    };
    assert_eq!(
        report.gaps(),
        [named],
        "the run never exercises and never wakes, so neither of those ranges is a gap"
    );
}

#[test]
fn a_change_the_run_crosses_whose_range_judged_nothing_blocks_and_is_named() {
    let (asleep, awake) = (Activity::new("asleep"), Activity::new("awake"));
    let set = set(
        vec![
            range("xx.day", Scope::During(awake), sliding(SECOND), 0.0, 100.0),
            range("xx.every", Scope::Every, sliding(16 * SECOND), 0.0, 100.0),
            range(
                "xx.falling-asleep",
                change("awake", "asleep", SECOND),
                sliding(16 * SECOND),
                0.0,
                100.0,
            ),
            range("xx.night", Scope::During(asleep), sliding(SECOND), 0.0, 100.0),
            range(
                "xx.waking",
                change("asleep", "awake", SECOND),
                sliding(16 * SECOND),
                0.0,
                100.0,
            ),
        ],
        Vec::new(),
    );
    let day = day(&[(0, "awake"), (2 * SECOND, "asleep"), (6 * SECOND, "awake")]);
    let stream = each_period(4 * SECOND, SECOND, &[50.0; 8]);
    let report = ranged(&[sampled(reading())], SECOND, &set, &day, 4 * SECOND, &stream);
    let judged = (
        result(&report, "xx.night").judged(),
        result(&report, "xx.day").judged(),
    );
    assert_eq!(judged, (2, 5), "the value at 6 s lies in waking's 1 s");
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::Unjudged {
        range: "xx.waking",
        row: reading(),
        method: METHOD,
    };
    assert_eq!(
        report.gaps(),
        [named],
        "the run starts after falling asleep, and each activity has a range of its own, so neither the falling-asleep \
         range nor the every-activity range is a gap"
    );
}

#[test]
fn a_range_check_that_judged_nothing_blocks() {
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(SECOND), 0.0, 100.0)],
        Vec::new(),
    );
    let report = ranged(&[], SECOND, &set, &day(&[(0, "awake")]), 0, &[]);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    assert_eq!(report.gaps(), [RangeGap::NothingJudged]);
}

#[test]
fn a_reading_that_gave_no_value_blocks_and_is_named_beside_one_that_was_judged() {
    let other = row("other");
    let every = |id, row| {
        let (low, high) = (pressure(0.0), pressure(100.0));
        HealthyRange::synthetic(
            id,
            row,
            METHOD,
            Scope::Every,
            sliding(SECOND),
            low,
            high,
            "synthetic",
        )
        .expect("a range")
    };
    let set = set(
        vec![every("xx.other", other), every("xx.range", reading())],
        Vec::new(),
    );
    let readings = [sampled(reading()), time_mean(other)];
    let stream = each_period(0, SECOND, &[50.0; 4]);
    let report = ranged(&readings, SECOND, &set, &day(&[(0, "awake")]), 0, &stream);
    assert_eq!(result(&report, "xx.range").judged(), 4);
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let named = RangeGap::Unread {
        row: other,
        method: METHOD,
    };
    assert_eq!(
        report.gaps(),
        [named],
        "the time mean gave no value in a run shorter than its grid period"
    );
}

#[test]
fn a_packet_with_no_anchors_blocks_the_return_check() {
    let set = set(Vec::new(), vec![variation(10.0)]);
    let report = returned(&[sampled(reading())], &set, &[], &hourly(48, |_| 100.0));
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    assert_eq!(report.gaps(), [ReturnGap::NoAnchors]);
    assert_eq!(report.morning(morning(1)), CheckVerdict::Blocked);
}

#[test]
fn a_run_inside_its_range_but_away_from_its_admission_reading_at_24_hours_fails_only_the_return_check() {
    let stream = hourly(48, |hour| if hour == 24 { 140.0 } else { 100.0 });
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(HOUR), 0.0, 200.0)],
        vec![variation(10.0)],
    );
    let checked = ranged(
        &[sampled(reading())],
        HOUR,
        &set,
        &day(&[(0, "awake")]),
        0,
        &stream,
    );
    assert_eq!(
        checked.verdict(),
        CheckVerdict::Pass,
        "every value stays inside 0 to 200"
    );
    let report = returned(&[sampled(reading())], &set, &[instant()], &stream);
    assert_eq!(report.verdict(), CheckVerdict::Fail);
    assert_eq!(
        report.morning(morning(1)),
        CheckVerdict::Fail,
        "140 is 40 from admission's 100"
    );
    assert_eq!(report.morning(morning(2)), CheckVerdict::Pass);
    let first = report.results()[0];
    assert_eq!(
        (first.at(), first.admission()),
        (at(24 * HOUR), Some(value(100.0)))
    );
}

#[test]
fn a_run_back_at_its_admission_reading_that_left_its_range_between_fails_only_the_range_check() {
    let stream = hourly(48, |hour| if hour == 12 { 300.0 } else { 100.0 });
    let set = set(
        vec![range("xx.range", Scope::Every, sliding(HOUR), 0.0, 200.0)],
        vec![variation(10.0)],
    );
    let checked = ranged(
        &[sampled(reading())],
        HOUR,
        &set,
        &day(&[(0, "awake")]),
        0,
        &stream,
    );
    assert_eq!(checked.verdict(), CheckVerdict::Fail);
    assert_eq!(
        failure(&checked, "xx.range"),
        (at(12 * HOUR), HOUR, Some(value(300.0)))
    );
    let report = returned(&[sampled(reading())], &set, &[instant()], &stream);
    assert_eq!(report.verdict(), CheckVerdict::Pass);
    let anchor = AnchorId::new("xx.anchor");
    let back = Some(value(100.0));
    assert_eq!(
        returns_of(&report),
        [
            (anchor, 1, at(24 * HOUR), back, true),
            (anchor, 2, at(48 * HOUR), back, true)
        ]
    );
}

#[test]
fn a_morning_may_move_from_admission_by_1_96_root_2_within_subject_variations() {
    let stream = hourly(48, |hour| match hour {
        24 => 127.7,
        48 => 72.2,
        _ => 100.0,
    });
    let set = set(Vec::new(), vec![variation(10.0)]);
    let report = returned(&[sampled(reading())], &set, &[instant()], &stream);
    let anchor = AnchorId::new("xx.anchor");
    let expected = [
        (anchor, 1, at(24 * HOUR), Some(value(127.7)), true),
        (anchor, 2, at(48 * HOUR), Some(value(72.2)), false),
    ];
    assert_eq!(
        returns_of(&report),
        expected,
        "27.7 up passes and 27.8 down fails"
    );
    let allowed = report.results()[0].allowed();
    assert!(
        value(27.71).at_most(allowed) && allowed.at_most(value(27.72)),
        "{allowed:?}"
    );
}

#[test]
fn a_morning_whose_window_holds_a_value_that_was_not_present_fails() {
    let mut stream = hourly(48, |_| 10.0);
    stream[24].value = None;
    let set = set(Vec::new(), vec![variation(10.0)]);
    let report = returned(&[sampled(reading())], &set, &[instant()], &stream);
    let anchor = AnchorId::new("xx.anchor");
    let expected = [
        (anchor, 1, at(24 * HOUR), None, false),
        (anchor, 2, at(48 * HOUR), Some(value(10.0)), true),
    ];
    assert_eq!(
        returns_of(&report),
        expected,
        "a zero would sit 10 from admission's 10, inside the 27.7 a variation of 10 allows"
    );
    assert_eq!(report.gaps(), []);
    assert_eq!(report.morning(morning(1)), CheckVerdict::Fail);
}

#[test]
fn an_anchor_with_no_within_subject_variation_blocks_and_is_named() {
    let stream = hourly(48, |_| 100.0);
    let report = returned(
        &[sampled(reading())],
        &set(Vec::new(), Vec::new()),
        &[instant()],
        &stream,
    );
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    assert_eq!(
        report.gaps(),
        [ReturnGap::NoVariation {
            anchor: AnchorId::new("xx.anchor")
        }]
    );
    assert_eq!(report.results(), [], "no morning passes without a variation");
}

#[test]
fn a_morning_the_run_does_not_reach_blocks_and_is_named() {
    let stream = hourly(30, |_| 100.0);
    let report = returned(
        &[sampled(reading())],
        &set(Vec::new(), vec![variation(10.0)]),
        &[instant()],
        &stream,
    );
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
    let missed = ReturnGap::NotReached {
        anchor: AnchorId::new("xx.anchor"),
        morning: morning(2),
    };
    assert_eq!(report.gaps(), [missed]);
    assert_eq!(report.morning(morning(1)), CheckVerdict::Pass);
    assert_eq!(report.morning(morning(2)), CheckVerdict::Blocked);
    assert_eq!(
        report.morning(morning(3)),
        CheckVerdict::Blocked,
        "a morning the check never reads"
    );
}

#[test]
fn a_cycle_anchor_reads_the_cycle_after_each_morning_by_its_own_kind_of_mean() {
    let read = |hour| match hour {
        24 => 0.0,
        49 | 50 => 140.0,
        _ => 100.0,
    };
    let at_hour = |reading, hour| Read {
        reading,
        at: hour * HOUR,
        value: Some(read(hour)),
    };
    let mut stream: Vec<Read> = (0..=50).map(|hour| at_hour(0, hour)).collect();
    stream.extend((1..=50).map(|hour| at_hour(1, hour)));
    let plans = [
        anchor("xx.sampled", AnchorStatistic::CycleMean, 2 * HOUR),
        anchor("xx.timed", AnchorStatistic::CycleTimeMean, 2 * HOUR),
    ];
    let readings = [sampled(reading()), time_mean(reading())];
    let report = returned(
        &readings,
        &set(Vec::new(), vec![variation(10.0)]),
        &plans,
        &stream,
    );
    let (by_samples, by_time) = (AnchorId::new("xx.sampled"), AnchorId::new("xx.timed"));
    let (back, away) = (Some(value(100.0)), Some(value(140.0)));
    let expected = [
        (by_samples, 1, at(24 * HOUR), back, true),
        (by_samples, 2, at(48 * HOUR), away, false),
        (by_time, 1, at(24 * HOUR), back, true),
        (by_time, 2, at(48 * HOUR), away, false),
    ];
    assert_eq!(
        returns_of(&report),
        expected,
        "the 0 at 24 h lies before each morning's cycle"
    );
}

#[test]
fn an_anchor_no_bare_reading_reads_blocks_and_is_named() {
    let stream = hourly(48, |_| 100.0);
    let plans = [anchor("xx.anchor", AnchorStatistic::CycleTimeMean, HOUR)];
    let report = returned(
        &[sampled(reading())],
        &set(Vec::new(), vec![variation(10.0)]),
        &plans,
        &stream,
    );
    assert_eq!(
        report.gaps(),
        [ReturnGap::NoReading {
            anchor: AnchorId::new("xx.anchor")
        }]
    );
    assert_eq!(report.verdict(), CheckVerdict::Blocked);
}

#[test]
fn a_set_entry_on_a_row_the_body_holds_in_another_dimension_is_refused() {
    let case = TwoReservoirs::closed().expect("a body");
    let body = case.body();
    let [a, _] = case.rows();
    let amount = body.rows()[a.index()].kind();
    let every = |id| (id, amount, METHOD, Scope::Every, sliding(SECOND));
    let (id, row, method, scope, averaging) = every("xx.water");
    let (low, high) = (
        Measure::<Water>::new(0.0).expect("finite"),
        Measure::new(1e6).expect("finite"),
    );
    let water =
        HealthyRange::synthetic(id, row, method, scope, averaging, low, high, "synthetic").expect("a range");
    assert_eq!(body.check_set(&set(vec![water], Vec::new())), Ok(()));
    let (id, row, method, scope, averaging) = every("xx.pressure");
    let (low, high) = (pressure(0.0), pressure(1e6));
    let other =
        HealthyRange::synthetic(id, row, method, scope, averaging, low, high, "synthetic").expect("a range");
    let refused = Err(AdmissionError::RangeDimension { id: "xx.pressure" });
    assert_eq!(body.check_set(&set(vec![other], Vec::new())), refused);
    let variation =
        WithinSubjectVariation::synthetic("xx.variation", amount, METHOD, pressure(1.0), "synthetic")
            .expect("a variation");
    let refused = Err(AdmissionError::RangeDimension { id: "xx.variation" });
    assert_eq!(body.check_set(&set(Vec::new(), vec![variation])), refused);
    let elsewhere = range("xx.elsewhere", Scope::Every, sliding(SECOND), 0.0, 1.0);
    assert_eq!(
        body.check_set(&set(vec![elsewhere], Vec::new())),
        Ok(()),
        "one set serves every body"
    );
}

#[test]
fn the_checks_refuse_a_length_a_change_or_a_window_off_the_grid_and_a_grid_off_the_day() {
    let readings = [sampled(reading())];
    let off = set(
        vec![range("xx.range", Scope::Every, sliding(3 * SECOND / 2), 0.0, 1.0)],
        Vec::new(),
    );
    let refused = Some(ReadingCheckError::SlidingLength { range: "xx.range" });
    assert_eq!(
        RangeFold::new(&readings, SECOND, &off, &day(&[(0, "awake")]), at(0)).err(),
        refused
    );
    let on = set(
        vec![range("xx.range", Scope::Every, sliding(2 * SECOND), 0.0, 1.0)],
        vec![variation(1.0)],
    );
    let changed = day(&[(0, "awake"), (3 * SECOND / 2, "asleep")]);
    let refused = Some(ReadingCheckError::ChangeOffGrid {
        at: at(3 * SECOND / 2),
    });
    assert_eq!(
        RangeFold::new(&readings, SECOND, &on, &changed, at(0)).err(),
        refused
    );
    let refused = Some(ReadingCheckError::AnchorWindow {
        anchor: AnchorId::new("xx.anchor"),
    });
    for window_ns in [3 * HOUR / 2, 0] {
        let plans = [anchor("xx.anchor", AnchorStatistic::CycleMean, window_ns)];
        assert_eq!(
            ReturnFold::new(&readings, HOUR, &on, &plans, at(0)).err(),
            refused
        );
    }
    let refused = Some(ReadingCheckError::UnevenGrid);
    assert_eq!(
        ReturnFold::new(&readings, 7 * 60 * SECOND, &on, &[], at(0)).err(),
        refused
    );
}

#[test]
fn a_day_refuses_no_span_a_start_out_of_order_and_one_activity_twice_running() {
    let (awake, asleep) = (Activity::new("awake"), Activity::new("asleep"));
    assert_eq!(
        DayActivities::new(Vec::new()).err(),
        Some(ReadingCheckError::EmptyDay)
    );
    let unordered = DayActivities::new(vec![(at(2), awake), (at(2), asleep)]);
    assert_eq!(
        unordered.err(),
        Some(ReadingCheckError::UnorderedDay { at: at(2) })
    );
    let repeated = DayActivities::new(vec![(at(2), awake), (at(4), awake)]);
    assert_eq!(
        repeated.err(),
        Some(ReadingCheckError::RepeatedActivity { at: at(4) })
    );
    let base = day(&[(0, "awake"), (4, "asleep")]).digest();
    assert_eq!(day(&[(0, "awake"), (4, "asleep")]).digest(), base);
    assert_ne!(day(&[(0, "awake"), (5, "asleep")]).digest(), base, "a start");
    assert_ne!(day(&[(0, "awake"), (4, "seated")]).digest(), base, "an activity");
    assert_ne!(day(&[(0, "awake")]).digest(), base, "a span");
}

#[test]
fn a_day_that_starts_after_admission_is_refused() {
    let asleep = Scope::During(Activity::new("asleep"));
    let set = set(
        vec![
            range("xx.awake", Scope::Every, sliding(SECOND), 0.0, 100.0),
            range("xx.night", asleep, Averaging::WholePeriod, 0.0, 10.0),
        ],
        Vec::new(),
    );
    let readings = [sampled(reading())];
    let night = day(&[(10 * SECOND, "asleep"), (20 * SECOND, "awake")]);
    let refused = Some(ReadingCheckError::DayAfterAdmission { at: at(10 * SECOND) });
    assert_eq!(
        RangeFold::new(&readings, SECOND, &set, &night, at(0)).err(),
        refused,
        "no activity holds the values from admission to 10 s"
    );
    assert!(
        RangeFold::new(&readings, SECOND, &set, &night, at(10 * SECOND)).is_ok(),
        "a day may start at admission"
    );
}

#[test]
fn an_observation_not_collected_or_invalid_is_fed_as_an_absent_value_and_a_pending_one_waits() {
    assert_eq!(fed(ObservedValue::Present(value(10.0))), Some(Some(value(10.0))));
    assert_eq!(fed(ObservedValue::NotCollected), Some(None));
    assert_eq!(fed(ObservedValue::InvalidSignal), Some(None));
    assert_eq!(fed(ObservedValue::Pending), None);
}
