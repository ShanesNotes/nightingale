use super::judge::judge;
use super::table::{Course, Direction, Lean, Measured, ROWS, Row, Scoring};
use super::trace::{
    Beat, DayTrace, Fold, READS, Read, Reading, Readings, SERIES, Sample, Series, blood_spaces,
};
use super::{DAY, DayCheck, DayPlan, DayQuantity, DayReport, EventKind, HOUR, MINUTE, Miss, SECOND};
use super::{Unavailable, Verdict};
use core::num::NonZeroU32;
use ng_contract::{
    AnatomyBuilder, CapillaryBed, Digest, Dimensionless, ErrorScales, InputId, Measure, Position, SimTime,
    SiteClass, SiteId, Space, VesselSegment,
};

const ORDER: [Series; SERIES] = [
    Series::HeartRate,
    Series::ArterialPressure,
    Series::AorticFlow,
    Series::PlasmaVolume,
    Series::PlasmaAlbumin,
    Series::PlasmaSodium,
    Series::PlasmaOsmolality,
    Series::SodiumExcretion,
    Series::UrineFlow,
];
const FLOOR: f64 = 0.01;
const S: i64 = SECOND as i64;
const M: i64 = MINUTE as i64;
const H: i64 = HOUR as i64;

fn clock(day: u64, hours: u64, minutes: u64) -> SimTime {
    SimTime::from_ns((day - 1) * DAY + hours * HOUR + minutes * MINUTE)
}

fn since(event: SimTime, at: SimTime) -> i64 {
    at.nanoseconds() as i64 - event.nanoseconds() as i64
}

fn two_days() -> NonZeroU32 {
    NonZeroU32::new(2).expect("two")
}

fn plan_with_every_payload() -> DayPlan {
    let mut plan = DayPlan::reference(two_days());
    for occurrence in &mut plan.occurrences {
        occurrence.missing_payload = false;
    }
    plan
}

fn wobble(index: usize) -> f64 {
    if index.is_multiple_of(2) { 1.0 } else { -1.0 }
}

fn beats(
    start: SimTime,
    end: SimTime,
    period: u64,
    value: impl Fn(Series, SimTime, usize) -> Sample,
) -> Vec<Beat> {
    let starts = (start.nanoseconds()..).step_by(period as usize);
    let starts = starts.take_while(|from| from + period <= end.nanoseconds());
    let beat = |(index, from): (usize, u64)| {
        let begins = SimTime::from_ns(from);
        Beat {
            start: begins,
            end: SimTime::from_ns(from + period),
            means: ORDER.map(|series| value(series, begins, index)),
        }
    };
    starts.enumerate().map(beat).collect()
}

fn traced(plan: &DayPlan, start: SimTime, end: SimTime, beats: Vec<Beat>) -> DayTrace {
    let synthetic = Digest::of_canonical(b"a synthetic trace");
    DayTrace {
        run: synthetic,
        seal: synthetic,
        start,
        end,
        beats,
        applied: vec![true; plan.occurrences.len()],
    }
}

fn steady(index: usize, effect: f64) -> Sample {
    Sample {
        value: 100.0 + wobble(index) + effect,
        floor: FLOOR,
    }
}

#[derive(Clone, Copy)]
struct Stage {
    event: SimTime,
    start: SimTime,
    end: SimTime,
    period: u64,
}
fn stage(kind: EventKind) -> Stage {
    match kind {
        EventKind::SitUp => Stage {
            event: clock(1, 7, 0),
            start: clock(1, 6, 55),
            end: clock(1, 7, 31),
            period: SECOND,
        },
        EventKind::Meal => Stage {
            event: clock(1, 7, 30),
            start: clock(1, 7, 15),
            end: clock(1, 11, 31),
            period: 30 * SECOND,
        },
        EventKind::LieDownToSleep => Stage {
            event: clock(1, 23, 0),
            start: clock(1, 22, 45),
            end: clock(2, 6, 31),
            period: 30 * SECOND,
        },
        EventKind::Wake => Stage {
            event: clock(2, 6, 30),
            start: clock(1, 21, 55),
            end: clock(2, 7, 1),
            period: 30 * SECOND,
        },
    }
}

fn moving(plan: &DayPlan, stage: Stage, effect: impl Fn(Series, i64) -> f64) -> DayTrace {
    let value = |series, at, index| steady(index, effect(series, since(stage.event, at)));
    traced(
        plan,
        stage.start,
        stage.end,
        beats(stage.start, stage.end, stage.period, value),
    )
}

fn meal_index(plan: &DayPlan, at: SimTime) -> usize {
    let found = plan.occurrences.iter().position(|occurrence| occurrence.at == at);
    found.expect("a meal at that instant")
}

fn verdicts(row: u8, checks: &[DayCheck], at: SimTime) -> Vec<Verdict> {
    let ours = checks
        .iter()
        .filter(|check| check.row() == row && check.at() == Some(at));
    ours.map(DayCheck::verdict).collect()
}

fn judged(row: &Row, effect: impl Fn(Series, i64) -> f64) -> Vec<Verdict> {
    let plan = plan_with_every_payload();
    let stage = stage(row.event);
    let fed = moving(&plan, stage, effect);
    let control = moving(&plan, stage, |_, _| 0.0);
    let checks = judge(&plan, &fed, &[(meal_index(&plan, stage.event), control)]);
    verdicts(row.number, &checks, stage.event)
}

fn row(number: u8) -> &'static Row {
    &ROWS[usize::from(number) - 1]
}

fn signed(direction: Direction, size: f64) -> f64 {
    match direction {
        Direction::Up => size,
        Direction::Down => -size,
    }
}

type Effect = Box<dyn Fn(Series, i64) -> f64>;

fn courses(row: &Row) -> Vec<(Effect, Vec<Verdict>)> {
    let fail = Verdict::Fail;
    match row.scoring {
        Scoring::Scored(measured, Course::OneWay(direction, _) | Course::AgainstControl(direction, _)) => {
            let moved = move |size: f64| -> Effect {
                Box::new(move |series: Series, since: i64| {
                    if series == measured.series() && since >= 0 {
                        signed(direction, size)
                    } else {
                        0.0
                    }
                })
            };
            vec![
                (moved(50.0), vec![Verdict::Pass]),
                (moved(-50.0), vec![fail(Miss::Direction)]),
            ]
        }
        Scoring::Scored(measured, Course::DipThenBack { .. }) => {
            let phased = move |dip: f64, between: f64, held: f64| -> Effect {
                Box::new(move |series: Series, since: i64| match since {
                    _ if series != measured.series() || since < 0 => 0.0,
                    _ if since < 5 * S => dip,
                    _ if since < 30 * S => between,
                    _ => held,
                })
            };
            vec![
                (phased(-50.0, 0.0, 0.0), vec![Verdict::Pass]),
                (phased(50.0, 0.0, 0.0), vec![fail(Miss::Dip)]),
                (phased(-50.0, -50.0, -50.0), vec![fail(Miss::Recovery)]),
                (phased(-50.0, 0.0, -50.0), vec![fail(Miss::Held)]),
            ]
        }
        Scoring::Scored(measured, Course::RiseThenFall { .. }) => {
            let phased = move |rise: f64, after: f64| -> Effect {
                Box::new(move |series: Series, since: i64| match since {
                    _ if series != measured.series() || since < 0 => 0.0,
                    _ if since < 10 * S => rise,
                    _ if since < M => 0.0,
                    _ => after,
                })
            };
            vec![
                (phased(50.0, -50.0), vec![Verdict::Pass]),
                (phased(-50.0, -50.0), vec![fail(Miss::Rise)]),
                (phased(50.0, 50.0), vec![fail(Miss::Fall)]),
            ]
        }
        Scoring::DirectionOnly(measured, lean) => {
            let night = -(7 * H + 30 * M);
            let moved = move |reversed: Option<Measured>| -> Effect {
                Box::new(move |series: Series, since: i64| {
                    let flipped = reversed.is_some_and(|reversed| reversed.series() == series);
                    let read = measured.iter().any(|measured| measured.series() == series);
                    match lean {
                        _ if !read => 0.0,
                        Lean::Down if since < 0 => 0.0,
                        Lean::Down => {
                            if flipped {
                                50.0
                            } else {
                                -50.0
                            }
                        }
                        Lean::UpAfterNightDip if since < night => 0.0,
                        Lean::UpAfterNightDip if since < 0 => -50.0,
                        Lean::UpAfterNightDip => {
                            if flipped {
                                -100.0
                            } else {
                                0.0
                            }
                        }
                    }
                })
            };
            let mut courses = vec![(moved(None), vec![Verdict::Pass; measured.len()])];
            for (index, &alone) in measured.iter().enumerate() {
                let mut expected = vec![Verdict::Pass; measured.len()];
                expected[index] = fail(Miss::Direction);
                courses.push((moved(Some(alone)), expected));
            }
            courses
        }
        Scoring::Unscored(..) => Vec::new(),
    }
}

#[test]
fn the_registry_holds_the_ticket_rows_in_order() {
    let numbers: Vec<u8> = ROWS.iter().map(|row| row.number).collect();
    assert_eq!(numbers, (1..=17).collect::<Vec<u8>>());
    let which = |kind: fn(&Scoring) -> bool| -> Vec<u8> {
        (ROWS.iter().filter(|row| kind(&row.scoring)))
            .map(|row| row.number)
            .collect()
    };
    assert_eq!(
        which(|scoring| matches!(scoring, Scoring::Scored(..))),
        [1, 2, 3, 4, 5, 6, 7, 9, 10, 14]
    );
    assert_eq!(
        which(|scoring| matches!(scoring, Scoring::DirectionOnly(..))),
        [15, 16, 17]
    );
    assert_eq!(
        which(|scoring| matches!(scoring, Scoring::Unscored(..))),
        [8, 11, 12, 13]
    );
    let against = which(|scoring| matches!(scoring, Scoring::Scored(_, Course::AgainstControl(..))));
    let scored_meals = (ROWS.iter())
        .filter(|row| row.event == EventKind::Meal && matches!(row.scoring, Scoring::Scored(..)))
        .map(|row| row.number);
    assert_eq!(against, [9, 10]);
    assert_eq!(against, scored_meals.collect::<Vec<u8>>());
}

#[test]
fn the_registry_states_the_tickets_directions_and_bounds() {
    let stated = [
        "SitUp Scored(HeartRate, OneWay(Up, Under(Seconds(15))))",
        "SitUp Scored(ArterialPressure, DipThenBack { dip: By(Seconds(10)), back: Under(Seconds(30)), held_from: Seconds(30) })",
        "SitUp Scored(StrokeVolume, OneWay(Down, By(Seconds(30))))",
        "SitUp Scored(CardiacOutput, RiseThenFall { rise: By(Seconds(30)), fall_from: Minutes(5), fall: By(Minutes(20)) })",
        "SitUp Scored(PlasmaVolume, OneWay(Down, By(Minutes(20))))",
        "SitUp Scored(PlasmaAlbumin, OneWay(Up, By(Minutes(20))))",
        "SitUp Scored(SodiumExcretion, OneWay(Down, By(Minutes(30))))",
        "SitUp Unscored([Filtration], \"direction disputed\")",
        "Meal Scored(PlasmaOsmolality, AgainstControl(Up, By(Hours(4))))",
        "Meal Scored(PlasmaSodium, AgainstControl(Up, By(Hours(4))))",
        "Meal Unscored([Vasopressin], \"unsourced\")",
        "Meal Unscored([SodiumExcretion, PotassiumExcretion, ChlorideExcretion], \"unsourced\")",
        "Meal Unscored([PlasmaPotassium], \"unsourced\")",
        "LieDownToSleep Scored(PlasmaVolume, OneWay(Up, By(Minutes(35))))",
        "LieDownToSleep DirectionOnly([ArterialPressure, HeartRate], Down)",
        "LieDownToSleep DirectionOnly([UrineFlow, SodiumExcretion], Down)",
        "Wake DirectionOnly([ArterialPressure, HeartRate], UpAfterNightDip)",
    ];
    for (row, stated) in ROWS.iter().zip(stated) {
        let found = format!("{:?} {:?}", row.event, row.scoring);
        assert_eq!(found, stated, "row {}", row.number);
    }
}

#[test]
fn each_scored_row_passes_its_way_and_fails_with_the_move_reversed() {
    let mut exercised = Vec::new();
    for row in &ROWS {
        let courses = courses(row);
        if courses.is_empty() {
            continue;
        }
        exercised.push(row.number);
        for (course, (effect, expected)) in courses.into_iter().enumerate() {
            assert_eq!(
                judged(row, effect),
                expected,
                "row {} course {course}",
                row.number
            );
        }
    }
    assert_eq!(exercised, [1, 2, 3, 4, 5, 6, 7, 9, 10, 14, 15, 16, 17]);
}

fn heart_rate_step(before: impl Fn(usize) -> f64, after: f64, floor: f64) -> Verdict {
    let (plan, stage) = (plan_with_every_payload(), stage(EventKind::SitUp));
    let value = |_, at, index| Sample {
        value: if at < stage.event { before(index) } else { after },
        floor,
    };
    let fed = traced(
        &plan,
        stage.start,
        stage.end,
        beats(stage.start, stage.end, stage.period, value),
    );
    let checks = judge(&plan, &fed, &[]);
    verdicts(1, &checks, stage.event)[0]
}

#[test]
fn a_response_is_over_three_noise_floors_and_at_least_the_declared_tolerance() {
    let baseline_beats: f64 = 300.0;
    let sd = (baseline_beats / (baseline_beats - 1.0)).sqrt();
    let wobbled = |index| 100.0 + wobble(index);
    assert_eq!(
        heart_rate_step(wobbled, 100.0 + 3.0 * sd * 1.001, FLOOR),
        Verdict::Pass
    );
    assert_eq!(
        heart_rate_step(wobbled, 100.0 + 3.0 * sd * 0.999, FLOOR),
        Verdict::Fail(Miss::Direction)
    );
    let flat = |_| 100.0;
    assert_eq!(heart_rate_step(flat, 100.5, 0.5), Verdict::Pass);
    assert_eq!(
        heart_rate_step(flat, 100.4999, 0.5),
        Verdict::Fail(Miss::Direction)
    );
}

fn sit_up_row(number: u8, effect: impl Fn(i64) -> f64) -> Verdict {
    let series = match row(number).scoring {
        Scoring::Scored(measured, _) => measured.series(),
        _ => panic!("row {number} is scored"),
    };
    judged(
        row(number),
        |read, since| if read == series { effect(since) } else { 0.0 },
    )[0]
}

#[test]
fn a_fast_onset_must_meet_its_bound_and_stay_out_three_beats() {
    let rises_at = |start: i64| move |since: i64| if since >= start { 50.0 } else { 0.0 };
    assert_eq!(sit_up_row(1, rises_at(13 * S)), Verdict::Pass);
    assert_eq!(sit_up_row(1, rises_at(16 * S)), Verdict::Fail(Miss::Onset));
    let flicker = |since: i64| match since {
        _ if since < 0 => 0.0,
        _ if since < 2 * S => 50.0,
        _ if since < 20 * S => 0.0,
        _ => 50.0,
    };
    assert_eq!(sit_up_row(1, flicker), Verdict::Fail(Miss::Onset));
}

#[test]
fn a_slow_onset_stays_out_five_minutes_inside_its_window() {
    let excursion = |since: i64| match since {
        _ if since < 0 => 0.0,
        _ if since < 4 * M + 30 * S => -50.0,
        _ if since < 21 * M => 0.0,
        _ => -50.0,
    };
    assert_eq!(sit_up_row(5, excursion), Verdict::Fail(Miss::Onset));
    let falls_at = |start: i64| move |since: i64| if since >= start { -50.0 } else { 0.0 };
    assert_eq!(sit_up_row(7, falls_at(24 * M)), Verdict::Pass);
    assert_eq!(sit_up_row(7, falls_at(26 * M)), Verdict::Fail(Miss::Onset));
}

#[test]
fn day_one_baseline_is_cut_at_the_trace_start_and_day_two_holds_ten_minutes() {
    let plan = plan_with_every_payload();
    let (start, end) = (clock(1, 6, 55), clock(2, 7, 31));
    let planted = |from: SimTime, to: SimTime| {
        move |_, at: SimTime, index| {
            let morning = at.nanoseconds() % DAY;
            let step = if morning >= 7 * HOUR { 50.0 } else { 0.0 };
            let wide = if (from..to).contains(&at) { 60.0 } else { 1.0 };
            Sample {
                value: 100.0 + wide * wobble(index) + step,
                floor: FLOOR,
            }
        }
    };
    let day_two = |from, to| {
        let mut both = beats(start, clock(1, 7, 31), SECOND, planted(from, to));
        both.extend(beats(clock(2, 6, 39), end, SECOND, planted(from, to)));
        let checks = judge(&plan, &traced(&plan, start, end, both), &[]);
        (
            verdicts(1, &checks, clock(1, 7, 0)),
            verdicts(1, &checks, clock(2, 7, 0)),
        )
    };
    let (none, never) = (clock(1, 0, 0), clock(1, 0, 0));
    assert_eq!(day_two(none, never), (vec![Verdict::Pass], vec![Verdict::Pass]));
    let inside = day_two(clock(2, 6, 50), clock(2, 6, 55));
    assert_eq!(
        inside,
        (vec![Verdict::Pass], vec![Verdict::Fail(Miss::Direction)])
    );
    let before = day_two(clock(2, 6, 40), clock(2, 6, 50));
    assert_eq!(before, (vec![Verdict::Pass], vec![Verdict::Pass]));
}

#[test]
fn a_window_ends_at_the_next_occurrence() {
    let (plan, at) = (plan_with_every_payload(), clock(1, 7, 0));
    let stage = Stage {
        end: clock(1, 8, 1),
        ..stage(EventKind::SitUp)
    };
    let effect = |series, since: i64| match since {
        _ if series != Series::HeartRate || since < 0 => 0.0,
        _ if since < 30 * M => 50.0,
        _ => -500.0,
    };
    let checks = judge(&plan, &moving(&plan, stage, effect), &[]);
    assert_eq!(verdicts(1, &checks, at), [Verdict::Pass]);
}

fn meal_rows(fed: &DayTrace, control: Option<DayTrace>) -> Vec<Verdict> {
    let (plan, at) = (plan_with_every_payload(), clock(1, 7, 30));
    let controls: Vec<(usize, DayTrace)> = control
        .into_iter()
        .map(|control| (meal_index(&plan, at), control))
        .collect();
    let checks = judge(&plan, fed, &controls);
    [verdicts(9, &checks, at), verdicts(10, &checks, at)].concat()
}

#[test]
fn a_meal_reads_against_its_counterfactual() {
    let (plan, stage) = (plan_with_every_payload(), stage(EventKind::Meal));
    let drift = |since: i64| 10.0 * since as f64 / H as f64;
    let shared = moving(&plan, stage, |_, since| drift(since));
    let fed = moving(&plan, stage, |_, since| {
        drift(since) + if since >= 0 { 50.0 } else { 0.0 }
    });
    let both = |verdict| vec![verdict; 2];
    assert_eq!(
        meal_rows(&shared, Some(shared.clone())),
        both(Verdict::Fail(Miss::Direction))
    );
    assert_eq!(meal_rows(&fed, Some(shared.clone())), both(Verdict::Pass));
    for apart in [-50.0, 50.0] {
        let parted = moving(&plan, stage, |_, since| drift(since) + apart);
        assert_eq!(
            meal_rows(&shared, Some(parted.clone())),
            both(Verdict::Fail(Miss::Direction))
        );
        assert_eq!(meal_rows(&fed, Some(parted)), both(Verdict::Pass));
    }
    let none = Verdict::NotAvailableYet(Unavailable::NoCounterfactual);
    assert_eq!(meal_rows(&fed, None), both(none));
    let short = DayTrace {
        end: clock(1, 11, 0),
        beats: (shared.beats.iter().copied())
            .filter(|beat| beat.end <= clock(1, 11, 0))
            .collect(),
        ..shared
    };
    let uncovered = Verdict::NotAvailableYet(Unavailable::NotCovered);
    assert_eq!(meal_rows(&fed, Some(short)), both(uncovered));
}

#[test]
fn fed_beats_shorter_than_the_controls_pair_by_time_and_find_the_planted_step() {
    let (plan, stage) = (plan_with_every_payload(), stage(EventKind::Meal));
    let dip = |since: i64| {
        if (30 * M..40 * M).contains(&since) {
            -60.0
        } else {
            0.0
        }
    };
    let fed = moving(
        &plan,
        Stage {
            period: 20 * SECOND,
            ..stage
        },
        |_, since| dip(since) + if since >= H { 50.0 } else { 0.0 },
    );
    let control = moving(&plan, stage, |_, since| dip(since));
    let at = clock(1, 7, 30);
    let checks = judge(&plan, &fed, &[(meal_index(&plan, at), control)]);
    for number in [9, 10] {
        let check = checks
            .iter()
            .find(|check| check.row() == number && check.at() == Some(at));
        let check = check.expect("a meal check");
        assert_eq!(check.verdict(), Verdict::Pass);
        let onset = check.shown.first.expect("an onset");
        assert!(
            (HOUR..=HOUR + 20 * SECOND).contains(&onset),
            "row {number}: onset {onset} ns after the meal"
        );
    }
}

fn phases(number: u8, floor: f64, effect: impl Fn(i64) -> f64) -> Verdict {
    let (plan, stage) = (plan_with_every_payload(), stage(EventKind::SitUp));
    let series = match row(number).scoring {
        Scoring::Scored(measured, _) => measured.series(),
        _ => panic!("row {number} is scored"),
    };
    let value = |read, at, index| Sample {
        value: 100.0
            + wobble(index)
            + if read == series {
                effect(since(stage.event, at))
            } else {
                0.0
            },
        floor,
    };
    let fed = traced(
        &plan,
        stage.start,
        stage.end,
        beats(stage.start, stage.end, stage.period, value),
    );
    verdicts(number, &judge(&plan, &fed, &[]), stage.event)[0]
}

#[test]
fn rows_two_and_four_fail_with_the_phase_that_is_missing() {
    let after = |at: i64, size: f64| move |since: i64| if since >= at { size } else { 0.0 };
    let span = |from: i64, to: i64, size: f64| {
        move |since: i64| if (from..to).contains(&since) { size } else { 0.0 }
    };
    assert_eq!(phases(2, FLOOR, |_| 0.0), Verdict::Fail(Miss::Dip));
    assert_eq!(phases(2, FLOOR, after(0, -50.0)), Verdict::Fail(Miss::Recovery));
    let late_fall = |since| span(0, 5 * S, -50.0)(since) + after(30 * S, -50.0)(since);
    assert_eq!(phases(2, FLOOR, late_fall), Verdict::Fail(Miss::Held));
    assert_eq!(phases(2, 10.0, span(0, 5 * S, -5.0)), Verdict::Fail(Miss::Dip));
    assert_eq!(phases(2, FLOOR, span(0, 5 * S, -5.0)), Verdict::Pass);
    assert_eq!(phases(4, FLOOR, after(M, -50.0)), Verdict::Fail(Miss::Rise));
    assert_eq!(phases(4, FLOOR, span(0, 10 * S, 50.0)), Verdict::Fail(Miss::Fall));
    let small_rise = |since| span(0, 10 * S, 5.0)(since) + after(M, -50.0)(since);
    assert_eq!(phases(4, 10.0, small_rise), Verdict::Fail(Miss::Rise));
    assert_eq!(phases(4, FLOOR, small_rise), Verdict::Pass);
}

#[test]
fn a_phase_bound_holds_its_onset_and_its_stay_may_run_past_it() {
    let rises_at_twenty_eight = |since: i64| match since {
        _ if (28 * S..33 * S).contains(&since) => 50.0,
        _ if since >= M => -50.0,
        _ => 0.0,
    };
    assert_eq!(sit_up_row(4, rises_at_twenty_eight), Verdict::Pass);
}

#[test]
fn a_phase_at_the_declared_tolerance_is_a_response() {
    let flat =
        |effect: fn(i64) -> f64| move |since: i64| 100.0 + if since >= 0 { effect(since) } else { 0.0 };
    let at_tolerance = |number, effect: fn(i64) -> f64| {
        let (plan, stage) = (plan_with_every_payload(), stage(EventKind::SitUp));
        let series = match row(number).scoring {
            Scoring::Scored(measured, _) => measured.series(),
            _ => panic!("row {number} is scored"),
        };
        let moved = flat(effect);
        let value = |read, at, _| Sample {
            value: if read == series {
                moved(since(stage.event, at))
            } else {
                100.0
            },
            floor: 10.0,
        };
        let fed = traced(
            &plan,
            stage.start,
            stage.end,
            beats(stage.start, stage.end, stage.period, value),
        );
        verdicts(number, &judge(&plan, &fed, &[]), stage.event)[0]
    };
    let dip = |since: i64| if since < 5 * S { -10.0 } else { 0.0 };
    let rise_then_fall = |since: i64| match since {
        _ if since < 10 * S => 10.0,
        _ if since < M => 0.0,
        _ => -50.0,
    };
    assert_eq!(at_tolerance(2, dip), Verdict::Pass);
    assert_eq!(at_tolerance(4, rise_then_fall), Verdict::Pass);
}

#[test]
fn the_wake_rises_only_after_a_night_dip() {
    let night = -(7 * H + 30 * M);
    let wake = |asleep: f64, morning: f64| {
        let effect = move |series, since: i64| match since {
            _ if series != Series::HeartRate && series != Series::ArterialPressure => 0.0,
            _ if since < night => 0.0,
            _ if since < 0 => asleep,
            _ => morning,
        };
        judged(row(17), effect)
    };
    assert_eq!(wake(0.0, 50.0), vec![Verdict::Fail(Miss::NoDip); 2]);
    assert_eq!(wake(-50.0, -50.0), vec![Verdict::Fail(Miss::Direction); 2]);
    assert_eq!(wake(-50.0, 0.0), vec![Verdict::Pass; 2]);
}

#[test]
fn unscored_rows_never_pass_or_fail() {
    let plan = plan_with_every_payload();
    for kind in [EventKind::SitUp, EventKind::Meal] {
        let stage = stage(kind);
        for size in [50.0, -50.0, 0.0] {
            let fed = moving(&plan, stage, |_, since| if since >= 0 { size } else { 0.0 });
            let control = moving(&plan, stage, |_, _| 0.0);
            let controls = [(meal_index(&plan, clock(1, 7, 30)), control)];
            let checks = judge(&plan, &fed, &controls);
            let unscored = checks
                .iter()
                .filter(|check| [8, 11, 12, 13].contains(&check.row()));
            let verdicts: Vec<Verdict> = unscored.map(DayCheck::verdict).collect();
            assert!(!verdicts.is_empty());
            assert!(
                verdicts
                    .iter()
                    .all(|verdict| *verdict == Verdict::NotAvailableYet(Unavailable::Unscored)),
                "{verdicts:?}"
            );
        }
    }
}

#[test]
fn a_direction_only_row_passes_with_a_late_change() {
    let late = |series, since: i64| match series {
        Series::HeartRate | Series::ArterialPressure if since >= 4 * H => -50.0,
        _ => 0.0,
    };
    assert_eq!(judged(row(15), late), [Verdict::Pass, Verdict::Pass]);
}

#[test]
fn a_check_says_why_it_is_not_available_yet() {
    let reference = DayPlan::reference(two_days());
    let falls = |_, since: i64| if since >= 0 { -50.0 } else { 0.0 };
    let night = moving(&reference, stage(EventKind::LieDownToSleep), falls);
    let checks = judge(&reference, &night, &[]);
    let no_payload = Verdict::NotAvailableYet(Unavailable::NoPayload);
    for number in 14..=16 {
        assert!(
            verdicts(number, &checks, clock(1, 23, 0))
                .iter()
                .all(|verdict| *verdict == no_payload)
        );
    }
    assert_eq!(verdicts(17, &checks, clock(2, 6, 30)), [no_payload; 2]);

    let plan = plan_with_every_payload();
    let at = clock(1, 7, 0);
    let early = Stage {
        end: clock(1, 7, 20),
        ..stage(EventKind::SitUp)
    };
    let short = moving(&plan, early, falls);
    let checks = judge(&plan, &short, &[]);
    let uncovered = Verdict::NotAvailableYet(Unavailable::NotCovered);
    for number in 1..=7 {
        assert_eq!(verdicts(number, &checks, at), [uncovered]);
    }
    let absent = |number| {
        checks
            .iter()
            .filter(move |check: &&DayCheck| check.row() == number)
    };
    let not_reached = Verdict::NotAvailableYet(Unavailable::NotReached);
    assert!(absent(9).all(|check| check.at().is_none() && check.verdict() == not_reached));

    let mut unapplied = moving(&plan, stage(EventKind::SitUp), falls);
    unapplied.applied[0] = false;
    let checks = judge(&plan, &unapplied, &[]);
    for number in 1..=7 {
        assert_eq!(verdicts(number, &checks, at), [not_reached]);
    }

    let sparse = moving(
        &plan,
        Stage {
            period: 4 * MINUTE,
            ..stage(EventKind::SitUp)
        },
        falls,
    );
    let checks = judge(&plan, &sparse, &[]);
    assert_eq!(
        verdicts(1, &checks, at),
        [Verdict::NotAvailableYet(Unavailable::TooFewBeats)]
    );
}

#[test]
fn the_report_shows_a_line_per_check_with_its_reason() {
    let plan = plan_with_every_payload();
    let early = Stage {
        end: clock(1, 7, 20),
        ..stage(EventKind::SitUp)
    };
    let trace = moving(&plan, early, |_, _| 0.0);
    let checks = judge(&plan, &trace, &[]);
    let report = DayReport {
        run: trace.run,
        seal: trace.seal,
        start: trace.start,
        end: trace.end,
        beats: trace.beats.len(),
        checks,
    };
    let shown = format!("{report}");
    let lines: Vec<&str> = shown.lines().collect();
    assert_eq!(lines.len(), 1 + report.checks().len());
    assert!(lines[0].starts_with("day checks run "), "{}", lines[0]);
    assert!(
        lines[0].ends_with("from 06:55:00 to 07:20:00, 1500 beats"),
        "{}",
        lines[0]
    );
    let first = "row  1 heart rate, sit up day 1 07:00:00: not available yet: the run ends before 07:30:00 (Smith 1994)";
    assert_eq!(lines[1], first);
    assert!(
        lines.iter().any(
            |line| line.starts_with("row  8 filtration") && line.contains("unscored, direction disputed")
        )
    );
    let quantities = report
        .checks()
        .iter()
        .filter(|check| check.quantity() == DayQuantity::Vasopressin);
    assert_eq!(quantities.count(), 1);
}

fn scales() -> ErrorScales {
    let unit = |value| Measure::<Dimensionless>::new(value).expect("finite");
    ErrorScales::uniform(unit(0.5), unit(0.25)).expect("positive")
}

fn by_hand(reading: &Reading, sizes: &Reading, sodium_per_second: f64) -> [Sample; SERIES] {
    let value = |read: Read| reading[read as usize];
    let allowed = |read: Read| 0.5 * sizes[read as usize] + 0.25 * value(read).abs();
    let own = |read: Read| Sample {
        value: value(read),
        floor: allowed(read),
    };
    let (water, water_floor) = (value(Read::Water), allowed(Read::Water));
    let per_water = |top: f64, top_floor: f64| Sample {
        value: top / water,
        floor: top_floor / water.abs() + top.abs() * water_floor / (water * water),
    };
    let ions = [Read::Sodium, Read::Potassium, Read::Chloride];
    let (flow, sodium, duct_water) = (
        value(Read::Tubular),
        value(Read::DuctSodium),
        value(Read::DuctWater),
    );
    let concentration = sodium / duct_water;
    let concentration_floor = allowed(Read::DuctSodium) / duct_water.abs()
        + sodium.abs() * allowed(Read::DuctWater) / (duct_water * duct_water);
    [
        own(Read::Rate),
        own(Read::Pressure),
        own(Read::Aortic),
        own(Read::Water),
        per_water(value(Read::Albumin), allowed(Read::Albumin)),
        per_water(value(Read::Sodium), allowed(Read::Sodium)),
        per_water(
            ions.iter().map(|&read| value(read)).sum(),
            ions.iter().map(|&read| allowed(read)).sum(),
        ),
        Sample {
            value: sodium_per_second,
            floor: concentration.abs() * allowed(Read::Tubular) + flow.abs() * concentration_floor,
        },
        own(Read::Tubular),
    ]
}

fn close(found: f64, expected: f64) -> bool {
    (found - expected).abs() <= 1e-12 * expected.abs()
}

#[test]
fn the_fold_keeps_whole_beats_from_edge_to_edge_with_step_length_weighted_means() {
    let lengths: [u64; 7] = [100, 300, 200, 100, 400, 200, 300].map(|ms| ms * 1_000_000);
    let phases = [0.7, 0.8, 0.1, 0.4, 0.9, 0.3, 0.6];
    let reading =
        |step: usize| -> Reading { core::array::from_fn(|read| (step + 1) as f64 * (read + 2) as f64) };
    let sodium_per_second = |step: usize| 0.3 * (step + 1) as f64;
    let sizes: Reading = core::array::from_fn(|read| (read + 1) as f64);
    let start = clock(1, 6, 55);
    let ends: Vec<SimTime> = (lengths.iter())
        .scan(start.nanoseconds(), |at, length| {
            *at += length;
            Some(SimTime::from_ns(*at))
        })
        .collect();
    let mut fold = Fold::new(&plan_with_every_payload(), scales(), sizes, start, 0.9);
    for step in 0..lengths.len() {
        fold.step(
            ends[step],
            lengths[step],
            phases[step],
            &reading(step),
            sodium_per_second(step),
        );
    }
    let synthetic = Digest::of_canonical(b"a synthetic run");
    let trace = fold.finish(synthetic, synthetic, ends[6]);
    let spans: Vec<(SimTime, SimTime)> = trace.beats.iter().map(|beat| (beat.start, beat.end)).collect();
    assert_eq!(spans, [(ends[0], ends[2]), (ends[2], ends[5])]);
    for (beat, steps) in trace.beats.iter().zip([1..3, 3..6]) {
        let over = |series: usize, pick: fn(Sample) -> f64| -> f64 {
            let each = steps.clone().map(|step| {
                pick(by_hand(&reading(step), &sizes, sodium_per_second(step))[series]) * lengths[step] as f64
            });
            each.sum()
        };
        let weight: f64 = steps.clone().map(|step| lengths[step] as f64).sum();
        for (series, found) in beat.means.iter().enumerate() {
            let (value, floor) = (
                over(series, |sample| sample.value),
                over(series, |sample| sample.floor),
            );
            assert!(
                close(found.value, value / weight) && close(found.floor, floor / weight),
                "{:?}: {found:?}",
                ORDER[series]
            );
        }
        let aortic = Series::AorticFlow as usize;
        let stroke = beat.reading(Measured::StrokeVolume);
        let integral = |pick| over(aortic, pick) / SECOND as f64;
        assert!(close(stroke.value, integral(|sample| sample.value)), "{stroke:?}");
        assert!(close(stroke.floor, integral(|sample| sample.floor)), "{stroke:?}");
        assert_eq!(beat.reading(Measured::CardiacOutput), beat.means[aortic]);
    }
}

#[test]
fn a_reversed_urinary_flow_excretes_what_the_ledger_moved_with_the_bladders_floor() {
    let mut reading: Reading = [1.0; READS];
    let set = [
        (Read::Tubular, -2.0),
        (Read::DuctSodium, 50.0),
        (Read::DuctWater, 10.0),
        (Read::BladderSodium, 20.0),
        (Read::BladderWater, 20.0),
    ];
    for (read, value) in set {
        reading[read as usize] = value;
    }
    let start = clock(1, 6, 55);
    let mut fold = Fold::new(&plan_with_every_payload(), scales(), [1.0; READS], start, 0.9);
    for (second, phase) in [(1, 0.1), (2, 0.5), (3, 0.2)] {
        let end = SimTime::from_ns(start.nanoseconds() + second * SECOND);
        fold.step(end, SECOND, phase, &reading, -2.0);
    }
    let synthetic = Digest::of_canonical(b"a synthetic run");
    let trace = fold.finish(
        synthetic,
        synthetic,
        SimTime::from_ns(start.nanoseconds() + 3 * SECOND),
    );
    let [beat] = trace.beats[..] else {
        panic!("one beat: {:?}", trace.beats);
    };
    let excretion = beat.reading(Measured::SodiumExcretion);
    assert_eq!(excretion.value, -2.0, "{excretion:?}");
    assert!(close(excretion.floor, 2.1), "{excretion:?}");
}

#[test]
fn the_trace_reads_the_sodium_the_engine_carried_from_the_duct_to_the_bladder() {
    let body = crate::arena_body().expect("compiled");
    let core = ng_solver::Core::new(10_000_000);
    let store = crate::MemoryStore::default();
    let (packet, lens) = (
        crate::arena_packet(&body).expect("a packet"),
        crate::arena_lens(&body).expect("a lens"),
    );
    let port = ng_contract::BoundaryPort::new(Vec::new()).expect("an empty port");
    let start = crate::initialize::solved_start(&body, packet, port, &core, &lens, crate::anchor_moment());
    let seed = ng_contract::Seed::new([1, 2, 3, 4]);
    let admitted = start.and_then(|start| crate::initialize::admit_start(&body, &core, start, seed, &store));
    let mut session = admitted.unwrap_or_else(|_| panic!("admitted"));
    let readings = Readings::bind(&body).unwrap_or_else(|_| panic!("bound"));
    for _ in 0..20 {
        session = crate::run::advance(session)
            .unwrap_or_else(|_| panic!("stepped"))
            .0;
        let ledger = session.conservation();
        let moved = readings.sodium_excreted_per_second(&ledger.head().expect("a head"));
        let (_, reading) = readings.read(session.record().accepted());
        let [flow, sodium, water] =
            [Read::Tubular, Read::DuctSodium, Read::DuctWater].map(|read| reading[read as usize]);
        let carried = flow * sodium / water;
        assert!(carried > 0.0, "{carried}");
        assert!(
            (moved - carried).abs() <= 1e-6 * carried,
            "{moved} against {carried}"
        );
    }
}

#[test]
fn an_occurrence_applies_once_every_input_has() {
    let plan = DayPlan::reference(NonZeroU32::MIN);
    let ids = |index: usize| -> Vec<InputId> {
        plan.occurrences[index]
            .inputs
            .iter()
            .map(|input| input.id)
            .collect()
    };
    let (sit_up, meal) = (ids(0), ids(1));
    let applied = |receipts: &[&[InputId]]| {
        let mut fold = Fold::new(&plan, scales(), [1.0; READS], clock(1, 6, 55), 0.0);
        receipts.iter().for_each(|ids| fold.applied(ids));
        let synthetic = Digest::of_canonical(b"a synthetic run");
        fold.finish(synthetic, synthetic, clock(1, 7, 1)).applied
    };
    let partly = applied(&[&sit_up[..20], &meal[..3]]);
    assert_eq!(partly, [false, false, false, false, false, true, false]);
    let all = applied(&[&sit_up[..20], &[], &meal, &sit_up[20..]]);
    assert_eq!(all, [true, true, false, false, false, true, false]);
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    let at = Position::new(origin(), origin(), origin());
    AnatomyBuilder::new().site::<C>(key, at).id()
}

#[test]
fn the_blood_spaces_are_the_arena_hosts_vessel_lumens_and_bed_plasma() {
    let body = crate::arena_body().expect("compiled");
    let mut found = blood_spaces(&body);
    found.sort_unstable();
    let vessels = [
        "right atrium",
        "right ventricle",
        "pulmonary artery",
        "pulmonary vein",
        "left atrium",
        "left ventricle",
        "systemic arteries",
        "central veins",
        "splanchnic veins",
        "leg veins",
    ];
    let beds = [
        "pulmonary bed",
        "splanchnic bed",
        "leg bed",
        "glomerular bed",
        "peritubular bed",
    ];
    let lumens = vessels.map(|key| (site::<VesselSegment>(key), Space::Lumen));
    let plasma = beds.map(|key| (site::<CapillaryBed>(key), Space::Plasma));
    let mut expected: Vec<(SiteId, Space)> = lumens.into_iter().chain(plasma).collect();
    expected.sort_unstable();
    assert_eq!(found, expected);
    assert!(
        Readings::bind(&body).is_ok(),
        "the arena body has every row the trace reads"
    );
}
