use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const METHOD: &str = "a synthetic method";

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn water(value: f64) -> Measure<Water> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

/// Reservoir `a` starts near 500 500 and takes 100 000 more from 40 s to 60 s, so it rises past 575 000 while the
/// delivery runs and falls back below it as the two reservoirs settle toward 550 000 each.
fn excursion() -> TwoReservoirs {
    let delivery = Delivery {
        id: b"input",
        from: at(40),
        to: at(60),
        amount: water(100_000.0),
    };
    let case = TwoReservoirs::delivering([water(500_500.0), water(499_500.0)], vec![delivery]).expect("a body");
    case.until(at(600))
}

fn amount(case: &TwoReservoirs) -> RowKind {
    let [a, _] = case.rows();
    case.body().rows()[a.index()].kind()
}

fn set(case: &TwoReservoirs, high: f64) -> HealthyRangeSet<SyntheticFixture> {
    let averaging = Averaging::Sliding(count(4 * SECOND));
    let (low, high) = (water(490_000.0), water(high));
    let range = HealthyRange::synthetic("xx.reservoir", amount(case), METHOD, Scope::Every, averaging, low, high, "synthetic")
        .expect("a range");
    HealthyRangeSet::synthetic(vec![range], Vec::new()).expect("a set")
}

fn lens(case: &TwoReservoirs) -> ObservationLens {
    let channel = ChannelSpec::new(METHOD, amount(case)).averaged(count(4));
    case.body().lens(vec![channel], count(SECOND)).expect("a lens")
}

fn day() -> DayActivities {
    DayActivities::new(vec![(at(0), Activity::new("awake"))]).expect("a day")
}

fn step<'m>(session: Session<'m, Running>) -> Session<'m, Running> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, _) => return session,
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        };
    }
}

struct Checked {
    run: Digest,
    range: RangeReport,
    returns: ReturnReport,
}

/// Admits the case, reads every revision through the bare lens to the case's end, and finishes both checks.
fn checked(case: &TwoReservoirs, set: &HealthyRangeSet<SyntheticFixture>) -> Checked {
    let core = ReferenceIntegrator::new(8 * SECOND);
    let store = MemoryStore::default();
    let body = case.body();
    let bare = BareLens::new(body, &lens(case)).expect("this body's lens");
    let anchors = case.packet().expect("a packet").anchors();
    let mut session = case.admit(&core, &store).expect("admitted");
    let mut checks = ReadingChecks::new(body, &bare, set, &day(), anchors, &session).expect("the checks");
    while session.record().accepted().time() < case.end() {
        session = step(session);
        checks.read(&session).expect("the next revision");
    }
    let run = session.record().identity().digest();
    let (range, returns) = checks.finish();
    Checked { run, range, returns }
}

#[test]
fn a_run_pushed_past_its_healthy_range_for_a_while_fails_naming_the_reading_and_a_window_in_the_excursion() {
    let case = excursion();
    let checked = checked(&case, &set(&case, 575_000.0));
    let range = &checked.range;
    assert_eq!(range.verdict(), CheckVerdict::Fail);
    assert_eq!(range.gaps(), []);
    let [result] = range.results() else {
        panic!("one result for the one range: {:?}", range.results());
    };
    assert_eq!((result.range(), result.row(), result.method()), ("xx.reservoir", amount(&case), METHOD));
    let failure = result.failure().expect("a window above 575 000");
    let opens = SimTime::from_ns(failure.end().nanoseconds() - failure.length_ns());
    assert!(at(40) <= opens && failure.end() <= at(60), "{failure:?} lies inside the delivery that drives the excursion");
    let mean = failure.mean().expect("every value present");
    assert!(!mean.at_most(Compensated::of(&water(575_000.0))), "{mean:?}");
    assert_eq!(range.lens(), lens(&case).digest(), "the report names the learner's lens");
    assert_eq!(checked.returns.verdict(), CheckVerdict::Pass, "the fixture has no anchor to return to");
}

#[test]
fn a_corrected_set_rechecks_a_finished_replay_and_gives_a_new_result_for_the_same_run() {
    let case = excursion();
    let first = checked(&case, &set(&case, 575_000.0));
    let corrected = checked(&case, &set(&case, 700_000.0));
    assert_eq!(first.run, corrected.run, "one run identity before and after");
    assert_eq!((first.range.run(), corrected.range.run()), (first.run, first.run));
    assert_eq!(first.range.seal(), corrected.range.seal(), "the replay reached the same last revision");
    assert_ne!(first.range.set(), corrected.range.set());
    assert_eq!((first.range.verdict(), corrected.range.verdict()), (CheckVerdict::Fail, CheckVerdict::Pass));
    assert_eq!((first.returns.run(), first.returns.set()), (first.run, first.range.set()));
}

#[test]
fn the_checks_read_this_bodys_lens_from_admission() {
    let case = excursion();
    let closed = TwoReservoirs::closed().expect("another body");
    let elsewhere = closed.body().lens(Vec::new(), count(SECOND)).expect("a lens");
    assert_eq!(BareLens::new(case.body(), &elsewhere).err(), Some(ReadingCheckError::OtherBody));
    let core = ReferenceIntegrator::new(8 * SECOND);
    let store = MemoryStore::default();
    let bare = BareLens::new(case.body(), &lens(&case)).expect("this body's lens");
    let set = set(&case, 575_000.0);
    let session = step(case.admit(&core, &store).expect("admitted"));
    let anchors = case.packet().expect("a packet").anchors();
    let late = ReadingChecks::new(case.body(), &bare, &set, &day(), anchors, &session);
    assert_eq!(late.err(), Some(ReadingCheckError::AfterAdmission));
}
