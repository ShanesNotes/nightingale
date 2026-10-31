mod chance_cores;

use chance_cores::{
    CountsEvaluations, DelaysEveryEvent, DifferencesItsJacobian, DrawsPerEvaluation, DrawsPerStep,
    JittersItsSpan, KeepsItsHistoryAcrossAFiring, LosesItsWayIfResumedAt, ReadingClock, RefusesByHash,
    RefusesEveryLanding, RefusesItsFirstTrial, TracesItsTickets, VerifiesDry,
};
use core::cell::{Cell, RefCell};
use ng_contract::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
/// A model default: the landing tolerance plus what a quadrature books wrong on this case, about a quarter of
/// the coarser step times the rate's relative swing.
const EVENT_TOLERANCE_NS: u64 = 500 * MILLISECOND;
const ONSET_PER_SECOND: f64 = 0.05;
const ONSET_INTEGRAL: &str = "onset-hazard";
const RETURN_INTEGRAL: &str = "return-hazard";
const SECOND_INTEGRAL: &str = "second-hazard";
const ONSET: &str = "ectopic-beat";
const RETURN: &str = "sinus-return";
const SECOND_STREAM: &str = "second-stream";

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    ReferenceIntegrator::new(interval_ns)
}

fn clock() -> ReadingClock {
    ReadingClock {
        readings: vec![0, 40, 90, 170, 900, 90_000],
        next: Cell::new(0),
    }
}

fn plan<'c>(clock: &'c ReadingClock, second: &'c dyn CoreFactory) -> ReplayPlan<'c> {
    ReplayPlan {
        clock,
        simulated_per_wall: SECOND,
        chunks: vec![7 * SECOND, 13 * SECOND + 1],
        resume_every: 60,
        chance: Some(ChancePlan {
            second_policy: second,
            reference: &|| ReferenceIntegrator::new(SECOND / 16),
            event_tolerance_ns: EVENT_TOLERANCE_NS,
        }),
    }
}

fn finished<'m>(session: Session<'m, Running>, end: SimTime) -> (Session<'m, Running>, Vec<Fired>) {
    let mut session = session;
    let mut fired = Vec::new();
    while session.record().accepted().time() < end {
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                Step::Accepted(session, receipt) => {
                    fired.extend_from_slice(receipt.fired());
                    break session;
                }
                Step::Refused(session) => session.retry(),
                Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
            };
        };
    }
    (session, fired)
}

fn events(case: &dyn ReplayCase, core: &dyn CoupledSolver) -> Vec<Fired> {
    finished(case.admit(core).expect("admitted"), case.end()).1
}

fn one_step<'m>(session: Session<'m, Running>) -> (Session<'m, Running>, StepReceipt) {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, receipt) => return (session, receipt),
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        };
    }
}

fn row(case: &ChanceCase, local: &'static str) -> RowId {
    case.memory(local).expect("the body declares it")
}

#[test]
fn a_streams_ordinals_climb_by_one_per_event_from_zero() {
    let case = ChanceCase::ectopy().expect("a body");
    let fired = events(&case, &reference(SECOND));
    let ordinals = |local: &'static str| -> Vec<u64> {
        (fired.iter())
            .filter(|event| event.stream().local == local)
            .map(|event| event.ordinal())
            .collect()
    };
    let (onset, recovery) = (ordinals(ONSET), ordinals(RETURN));
    assert!(onset.len() >= 2);
    assert_eq!(onset, (0..onset.len() as u64).collect::<Vec<u64>>());
    assert_eq!(recovery, (0..recovery.len() as u64).collect::<Vec<u64>>());
}

#[test]
fn each_stream_and_each_ordinal_has_its_own_draw() {
    let case = ChanceCase::ectopy().expect("a body");
    let fired = events(&case, &reference(SECOND));
    let mut draws: Vec<[u64; 2]> = fired.iter().map(|event| event.threshold().bits()).collect();
    let count = draws.len();
    assert!(count >= 4);
    draws.sort_unstable();
    draws.dedup();
    assert_eq!(draws.len(), count);
}

#[test]
fn a_rate_runs_only_in_the_mode_its_law_holds_in() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(SECOND);
    let (onset, recovery) = (row(&case, ONSET_INTEGRAL), row(&case, RETURN_INTEGRAL));
    let bits = |session: &Session<'_, Running>, row| session.record().accepted().value(row).bits();
    let mut session = case.admit(&core).expect("admitted");
    let mut idle = recovery;
    let (mut moved, mut held) = (0, 0);
    while session.record().accepted().time() < case.end() {
        let before = bits(&session, idle);
        let (next, receipt) = one_step(session);
        session = next;
        match receipt.fired().first() {
            None => {
                assert_eq!(bits(&session, idle), before, "the zero law holds here");
                held += 1;
            }
            Some(event) => {
                idle = if event.entered().value == "sinus" {
                    recovery
                } else {
                    onset
                };
                moved += 1;
            }
        }
    }
    assert!(moved >= 2 && held >= 20, "{moved} firings and {held} held steps");
}

#[test]
fn an_integral_past_its_draw_when_its_guard_arms_fires_within_a_floor() {
    let case = ChanceCase::racing().expect("a body").until(at(2_400));
    let fired = events(&case, &reference(SECOND));
    let overdue = fired.windows(2).filter(|pair| {
        pair[0].entered().value == "sinus"
            && pair[1].stream().local == SECOND_STREAM
            && pair[1].at().nanoseconds() - pair[0].at().nanoseconds() <= MILLISECOND
    });
    assert!(
        overdue.count() > 0,
        "no guard armed past its draw in {} events",
        fired.len()
    );
}

/// A rate that holds in every mode climbs past its draw while its guard is not armed, and only an armed guard ends
/// a step.
#[test]
fn an_integral_past_its_draw_stops_no_step_while_its_guard_is_disarmed() {
    let case = ChanceCase::racing()
        .expect("a body")
        .delivering(Vec::new())
        .until(at(2_400));
    let core = reference(SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut fired = Vec::new();
    while session.record().accepted().time() < case.end() {
        let before = session.record().accepted().time().nanoseconds();
        let (next, receipt) = one_step(session);
        session = next;
        let span = session.record().accepted().time().nanoseconds() - before;
        if receipt.fired().is_empty() {
            assert_eq!(span, SECOND, "a step from {before} ns stopped for no firing");
        }
        fired.extend_from_slice(receipt.fired());
    }
    let overdue = fired.windows(2).filter(|pair| {
        pair[0].entered().value == "sinus"
            && pair[1].stream().local == SECOND_STREAM
            && pair[1].at().nanoseconds() - pair[0].at().nanoseconds() <= MILLISECOND
    });
    assert!(
        overdue.count() > 0,
        "no guard stood past its draw, so this test pressed nothing"
    );
}

/// Only a guard armed before any guard fires can fire, so a guard a firing arms waits for the next step.
#[test]
fn a_guard_a_firing_arms_fires_on_the_next_step_within_a_floor() {
    let case = ChanceCase::echoing().expect("a body").until(at(2_400));
    let fired = events(&case, &reference(SECOND));
    let mut echoes = 0;
    for pair in fired.windows(2) {
        let apart = pair[1].at().nanoseconds() - pair[0].at().nanoseconds();
        if pair[0].stream().local == ONSET && pair[1].stream().local == SECOND_STREAM && apart <= MILLISECOND
        {
            assert!(apart > 0, "a firing armed a guard that fired in its own step");
            echoes += 1;
        }
    }
    assert!(
        echoes > 0,
        "no firing armed a guard past its draw in {} events",
        fired.len()
    );
}

const PROBE: LawName = LawName {
    owner: "CV",
    local: "probe",
};

#[derive(Clone, Copy, PartialEq)]
enum Reads {
    LogarithmOfTheIntegral,
    TheIntegral,
    TheDraw,
    TheClimb,
}

/// An integral that climbs in the first mode, and a law that holds only in the second, so the law first reads what a
/// firing leaves. Its range lies above any value it can read there. A law on the climb holds only in the first mode,
/// under a range no climb stays in, so what a firing leaves shows no excursion.
fn reads_a_reset(reads: Reads) -> Result<(CompiledBody, ParameterPacket<SyntheticFixture>), AdmissionError> {
    let body = assemble(two_vessels()?, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let class = RateClass::new(RateBand::Beat, SECOND)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer_along(&route, flow);
        let integral = laws.state::<Dimensionless>("integral", Admissible::nonnegative());
        let probe = laws.state::<Dimensionless>(PROBE.local, Admissible::finite());
        let pace = laws.parameter(ParameterKey::new("fixture.pace"), Admissible::positive());
        let dice = laws.stream("dice");
        let [before, after] = laws.modes("rhythm", ["before", "after"]);
        let still = |holds| {
            laws.kernel(|mut k| {
                let rate = k.zero::<Rate<Dimensionless>>();
                k.finish(rate, holds, class)
            })
        };
        let (holds, rests, lower, upper) = match reads {
            Reads::TheClimb => (before.only(), after.only(), 0.0, 0.0),
            _ => (after.only(), before.only(), 1.0, 2.0),
        };
        let (idle_integral, idle_probe) = (still(after.only()), still(rests));
        let climbing = laws.kernel(|mut k| {
            let rate = k.parameter(&pace);
            k.finish(rate, before.only(), class)
        });
        let reading = laws.kernel(|mut k| {
            let value = k.state(&integral);
            let factor = match reads {
                Reads::LogarithmOfTheIntegral => k.ln(value),
                Reads::TheIntegral | Reads::TheClimb => value,
                Reads::TheDraw => k.draw(&dice),
            };
            let pace = k.parameter(&pace);
            let rate = k.scale(pace, factor);
            k.finish(rate, holds, class)
        });
        if reads != Reads::LogarithmOfTheIntegral {
            let range = ValidityRange::new(
                Measure::new(lower)?,
                Measure::new(upper)?,
                Basis::Fixture("synthetic"),
            )?;
            laws.validity(&reading, range);
        }
        laws.memory_rate(&integral, climbing);
        laws.memory_rate(&integral, idle_integral);
        laws.memory_rate(&probe, reading);
        laws.memory_rate(&probe, idle_probe);
        laws.hazard(HazardGuard::new(&integral, &dice, before.to(&after)?));
        body.add(laws.finish());
        body.compile()
    })?;
    let mut packet = ParameterPacket::new();
    let pace = Measure::<Rate<Dimensionless>>::new(1.0)?;
    packet.synthetic(
        ParameterKey::new("fixture.pace"),
        Binding::Body,
        pace,
        "synthetic",
    )?;
    for row in body.rows() {
        match row.kind() {
            RowKind::Memory(_) => {
                packet.start(row.kind(), Measure::<Dimensionless>::new(0.0)?, "synthetic")?
            }
            _ => packet.start(row.kind(), Measure::<Water>::new(100.0)?, "synthetic")?,
        }
    }
    let rhythm = LawName {
        owner: "CV",
        local: "rhythm",
    };
    packet.start_mode(rhythm, "before", "synthetic")?;
    Ok((body, packet))
}

/// A crossing nothing can move, into a state one law cannot read: the run must stop short of it and say why.
#[test]
fn a_firing_no_law_can_read_faults_short_of_its_crossing_and_names_the_law()
-> Result<(), InitializationFailure> {
    let (body, packet) = reads_a_reset(Reads::LogarithmOfTheIntegral)?;
    let core = reference(SECOND);
    let mut session = admit_fixture(&body, packet, BoundaryPort::new(Vec::new())?, &core)?;
    loop {
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                Step::Accepted(session, receipt) => {
                    assert!(receipt.fired().is_empty(), "a state no law can read published");
                    break session;
                }
                Step::Refused(session) => session.retry(),
                Step::Faulted(stopped) => {
                    let fault = stopped.fault();
                    assert_eq!(fault.kind(), EngineFaultKind::NoAdmissibleStep);
                    let Some(NumericalRefusal::TrialDomain { source }) = fault.last_refusal() else {
                        panic!("{fault:?}");
                    };
                    assert_eq!(
                        body.refusal_sources()[source.index()],
                        RefusalSource::LogarithmArgument { law: PROBE }
                    );
                    return Ok(());
                }
            };
        };
    }
}

fn first_excursion_after_a_firing(reads: Reads) -> Result<(ValidityExcursion, Fired), InitializationFailure> {
    let (body, packet) = reads_a_reset(reads)?;
    let core = reference(SECOND);
    let mut session = admit_fixture(&body, packet, BoundaryPort::new(Vec::new())?, &core)?;
    loop {
        let (next, receipt) = one_step(session);
        session = next;
        let Some(event) = receipt.fired().first() else {
            assert!(receipt.excursions().is_empty(), "the law does not hold yet");
            continue;
        };
        let excursion = (receipt.excursions().first()).expect("what it left is below its range");
        assert_eq!((excursion.law(), excursion.side()), (PROBE, ExcursionSide::Below));
        assert_eq!(excursion.at(), event.at());
        return Ok((*excursion, *event));
    }
}

#[test]
fn a_range_is_judged_on_what_a_firing_leaves() -> Result<(), InitializationFailure> {
    let (excursion, _) = first_excursion_after_a_firing(Reads::TheIntegral)?;
    assert_eq!(excursion.value().bits(), Compensated::ZERO.bits());
    Ok(())
}

/// The law stops holding at the firing, so only the end of the climb shows how far it went.
#[test]
fn a_firing_keeps_the_excursion_its_step_ended_on() -> Result<(), InitializationFailure> {
    let (body, packet) = reads_a_reset(Reads::TheClimb)?;
    let core = reference(SECOND);
    let mut session = admit_fixture(&body, packet, BoundaryPort::new(Vec::new())?, &core)?;
    loop {
        let (next, receipt) = one_step(session);
        session = next;
        let Some(event) = receipt.fired().first() else {
            continue;
        };
        let [excursion] = receipt.excursions() else {
            panic!("the climb ended past its range: {:?}", receipt.excursions());
        };
        assert_eq!((excursion.law(), excursion.side()), (PROBE, ExcursionSide::Above));
        assert_eq!(excursion.at(), event.at());
        assert!(
            event.threshold().at_most(excursion.value()),
            "it stopped short of the draw"
        );
        return Ok(());
    }
}

/// The firing spent its draw, so a law read on what it leaves reads the stream's next one.
#[test]
fn what_a_firing_leaves_reads_the_next_draw() -> Result<(), InitializationFailure> {
    let (excursion, event) = first_excursion_after_a_firing(Reads::TheDraw)?;
    let [threshold, _] = event.threshold().bits();
    let spent = exp(-f64::from_bits(threshold));
    let [read, _] = excursion.value().bits();
    let read = f64::from_bits(read);
    assert!((read - spent).abs() > 1e-9, "{read} is the draw that fired");
    Ok(())
}

#[test]
fn a_rate_that_falls_below_zero_never_lowers_its_integral() {
    let case = ChanceCase::relaxing().expect("a body");
    let core = reference(SECOND / 4);
    let integral = row(&case, ONSET_INTEGRAL);
    let mut session = case.admit(&core).expect("admitted");
    let mut last = Compensated::ZERO;
    let mut held = 0;
    while session.record().accepted().time() < case.end() {
        let (next, receipt) = one_step(session);
        session = next;
        let now = session.record().accepted().value(integral);
        if receipt.fired().is_empty() {
            assert!(last.at_most(now), "the integral fell with no firing");
            held += u32::from(now.bits() == last.bits() && now.bits() != Compensated::ZERO.bits());
        }
        last = now;
    }
    assert!(
        held > 0,
        "the rate never went negative, so this test pressed nothing"
    );
}

#[test]
fn a_segment_ends_at_the_left_limit_of_a_reset() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(SECOND);
    let integral = row(&case, ONSET_INTEGRAL);
    let mut session = case.admit(&core).expect("admitted");
    loop {
        let (next, receipt) = one_step(session);
        session = next;
        if receipt.fired().is_empty() {
            continue;
        }
        let accepted = session.record().accepted();
        assert_eq!(
            accepted.value(integral).bits(),
            Compensated::ZERO.bits(),
            "the reset"
        );
        let segment = accepted.last_segment();
        let reached = segment.sample(integral, accepted.time()).expect("its end");
        assert!(
            !reached.at_most(Compensated::ZERO),
            "the chord does not run through the reset"
        );
        return;
    }
}

/// A floor past the span lands a guard at the first ticket that crosses, so two can cross in one landing.
#[test]
fn two_guards_racing_for_one_mode_fire_once_and_the_loser_keeps_its_draw() {
    let mut raced = 0;
    for seed in 0..64 {
        let case = ChanceCase::racing()
            .expect("a body")
            .delivering(Vec::new())
            .seeded(Seed::new([seed, 1, 2, 3]))
            .until(at(120));
        let core = reference(16 * SECOND).with_floor(300 * SECOND);
        let thresholds = events(&case, &reference(SECOND));
        let Some(loser) = (thresholds.iter()).find(|event| event.stream().local == SECOND_STREAM) else {
            continue;
        };
        let (draw, ordinal) = (loser.threshold(), loser.ordinal());
        if ordinal != 0 {
            continue;
        }
        let second = row(&case, SECOND_INTEGRAL);
        let mut session = case.admit(&core).expect("admitted");
        let mut lost = false;
        while session.record().accepted().time() < case.end() {
            let (next, receipt) = one_step(session);
            session = next;
            assert!(receipt.fired().len() <= 1, "one mode variable moves once");
            let Some(event) = receipt.fired().first() else {
                continue;
            };
            if lost && event.stream().local == SECOND_STREAM {
                let fired = (event.ordinal(), event.threshold().bits());
                assert_eq!(fired, (0, draw.bits()), "the loser kept its draw");
                raced += 1;
                break;
            }
            let kept = session.record().accepted().value(second);
            lost = lost || (event.stream().local == ONSET && draw.at_most(kept));
            if !lost {
                break;
            }
        }
    }
    assert!(
        raced > 0,
        "no two guards raced in 64 seeds, so this test pressed nothing"
    );
}

/// The landing stops short of an input the core has been shown, so a late stamp between them would give two
/// schedules of one input set two heads: the same reason a refused cut leaves a fence.
#[test]
fn a_cut_that_crosses_lands_short_of_its_input_and_leaves_a_fence() {
    let mut found = 0;
    for seed in 0..400 {
        let base = ChanceCase::ectopy().expect("a body");
        let tone = base.tone_slot();
        let stamp = at(5 + seed % 40);
        let delivery = Delivery {
            id: b"shown input",
            from: stamp,
            to: at(100),
            amount: Measure::new(10.0).expect("finite"),
        };
        let case = base.delivering(vec![delivery]).seeded(Seed::new([seed, 1, 2, 3]));
        let core = DrawsPerStep {
            interval_ns: 8 * SECOND,
            tone,
            reach: 1.5,
        };
        let mut session = case.admit(&core).expect("admitted");
        while session.record().accepted().time() < stamp {
            let (next, receipt) = one_step(session);
            session = next;
            let (head, earliest) = (
                session.record().accepted().time(),
                session.record().earliest_stamp(),
            );
            if head < stamp && earliest == stamp {
                assert!(
                    !receipt.fired().is_empty(),
                    "a landing fires on the step it stops"
                );
                found += 1;
                let between = SimTime::from_ns(head.nanoseconds() + 1);
                let late = BoundaryEvent::new(
                    InputId::new(Digest::of_canonical(b"between")),
                    between,
                    BoundaryInput::Water(WaterDelivery {
                        boundary: "exterior",
                        amount: Measure::new(1.0).expect("finite"),
                        ends: at(100),
                    }),
                );
                let command = Digest::of_canonical(b"a command");
                let input = AuthorizedInput::from_command_receipt(command, late);
                assert!(matches!(
                    session.enqueue(input),
                    Err(AdmissionError::EventBeforeFence { .. })
                ));
                break;
            }
        }
    }
    assert!(
        found > 0,
        "no cut crossed in 400 seeds, so this test pressed nothing"
    );
}

#[test]
fn a_guard_inside_a_span_ends_the_step_short_of_the_span() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(64 * SECOND);
    let landed = events(&case, &core);
    let fine = events(&case, &reference(SECOND / 8));
    assert!(!landed.is_empty());
    for (a, b) in landed.iter().zip(&fine) {
        assert_eq!((a.stream(), a.ordinal()), (b.stream(), b.ordinal()));
        assert_ne!(
            a.at().nanoseconds() % (64 * SECOND),
            0,
            "a landing lies inside a span"
        );
    }
}

/// One seed is not enough: a guard that crossed nothing moved another stream's landing on about one seed in six
/// while a search read every armed guard.
#[test]
fn adding_a_second_stream_leaves_the_first_streams_events_unchanged() {
    let core = reference(SECOND);
    let mut compared = 0;
    for word in 0..24 {
        let seed = Seed::new([word, 5, 6, 7]);
        let one = ChanceCase::ectopy().expect("a body").seeded(seed);
        let both = ChanceCase::ectopy_and_bystander().expect("a body").seeded(seed);
        let (lone, shared) = (events(&one, &core), events(&both, &core));
        let first = (shared.iter()).position(|event| event.stream().local == "bystander-stream");
        let upto = first.unwrap_or(shared.len());
        assert_eq!(
            lone[..upto],
            shared[..upto],
            "every bit of each event, seed {word}"
        );
        compared += upto;
    }
    assert!(compared >= 24, "{compared} events compared");
}

#[test]
fn a_hazard_rate_outside_its_range_is_reported_though_it_holds_flat() {
    let case = ChanceCase::relaxing_within(
        ValidityRange::new(
            Measure::new(0.0).expect("finite"),
            Measure::new(1.0).expect("finite"),
            Basis::Fixture("synthetic"),
        )
        .expect("a range"),
    )
    .expect("a body");
    let core = reference(SECOND / 4);
    let mut session = case.admit(&core).expect("admitted");
    let mut below = 0;
    while session.record().accepted().time() < case.end() {
        let (next, receipt) = one_step(session);
        session = next;
        below += (receipt.excursions().iter())
            .filter(|excursion| excursion.side() == ExcursionSide::Below)
            .count();
    }
    assert!(
        below > 0,
        "the rate never left its range, so this test pressed nothing"
    );
}

#[test]
fn a_slope_is_the_slope_of_what_a_run_integrates_in_each_mode_and_past_a_rates_zero() {
    let cases = [ChanceCase::ectopy, ChanceCase::relaxing];
    for case in cases.map(|case| case().expect("a body")) {
        let core = DifferencesItsJacobian {
            inner: reference(SECOND / 4),
            compared: Cell::new(0),
        };
        let fired = events(&case, &core);
        assert!(fired.len() >= 2, "both modes ran");
        assert!(core.compared.get() > 100);
    }
}

/// Ordinals keep climbing where a mode comes back, so one list cannot stand in for the other.
#[test]
fn the_modes_digest_reads_modes_and_the_streams_digest_reads_ordinals() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let start = session.record().digests();
    let mut fired = 0;
    while fired < 2 {
        let (next, receipt) = one_step(session);
        session = next;
        fired += receipt.fired().len();
        if fired == 1 {
            assert_ne!(session.record().digests().modes, start.modes);
        }
    }
    let back = session.record().digests();
    assert_eq!(back.modes, start.modes);
    assert_ne!(back.streams, start.streams);
}

#[test]
fn a_draw_is_a_function_of_seed_key_and_ordinal_whatever_else_fires() {
    let plain = ChanceCase::ectopy().expect("a body").until(at(1_200));
    let raced = ChanceCase::racing().expect("a body").until(at(1_200));
    let core = reference(SECOND);
    let (lone, shared) = (events(&plain, &core), events(&raced, &core));
    let mut compared = 0;
    for event in shared
        .iter()
        .filter(|event| event.stream().local != SECOND_STREAM)
    {
        let same = (lone.iter())
            .find(|known| known.stream() == event.stream() && known.ordinal() == event.ordinal());
        if let Some(known) = same {
            assert_eq!(known.threshold().bits(), event.threshold().bits());
            compared += 1;
        }
    }
    assert!(compared >= 4, "{compared} draws compared");
    assert_ne!(lone, shared, "the second stream changed the course");
}

#[test]
fn a_new_seed_gives_a_different_course_through_the_same_body() {
    let first = ChanceCase::ectopy().expect("a body");
    let second = ChanceCase::ectopy()
        .expect("a body")
        .seeded(Seed::new([9, 9, 9, 9]));
    let core = reference(SECOND);
    assert_ne!(events(&first, &core), events(&second, &core));
}

#[test]
fn a_forced_extra_evaluation_leaves_every_draw_and_ordinal_unchanged() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = || CountsEvaluations {
        inner: reference(SECOND),
        evaluations: Cell::new(0),
    };
    let evidence = replay_gate(ReplayGate::ExtraEvaluation, &case, &cores, &plan).expect("the gate");
    assert!(!evidence.fired().is_empty());
    assert_eq!(evidence.runs()[0].streams, evidence.runs()[1].streams);
}

#[test]
fn a_core_that_draws_per_evaluation_fails_the_extra_evaluation_gate() {
    let case = ChanceCase::ectopy().expect("a body");
    let tone = case.tone_slot();
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = move || DrawsPerEvaluation {
        interval_ns: SECOND,
        tone,
        reach: 0.5,
        solves: Cell::new(0),
    };
    let failure = replay_gate(ReplayGate::ExtraEvaluation, &case, &cores, &plan);
    assert!(matches!(
        failure,
        Err(ReplayFailure::Diverged {
            gate: ReplayGate::ExtraEvaluation,
            ..
        })
    ));
}

#[test]
fn two_step_policies_fire_the_same_events_within_the_tolerance() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let coarse = || reference(SECOND);
    let evidence = replay_gate(ReplayGate::StepPolicy, &case, &coarse, &plan).expect("the gate");
    assert!(evidence.fired().len() >= 2, "more than one event, so a reset ran");
}

#[test]
fn a_step_length_from_private_dice_is_a_legal_step_policy() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let jitters = || JittersItsSpan {
        inner: reference(SECOND / 2),
        spread_ns: SECOND / 4,
    };
    assert!(replay_gates(&case, &jitters, &plan).is_ok());
}

#[test]
fn a_core_that_draws_per_step_passes_every_bit_gate_and_fails_the_step_policy_gate() {
    let case = ChanceCase::ectopy().expect("a body");
    let tone = case.tone_slot();
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = move || DrawsPerStep {
        interval_ns: SECOND / 4,
        tone,
        reach: 1.5,
    };
    for gate in [
        ReplayGate::Pace,
        ReplayGate::Resume,
        ReplayGate::LateAdmission,
        ReplayGate::Rewind,
        ReplayGate::ExtraEvaluation,
        ReplayGate::NonPerturbation,
    ] {
        assert!(replay_gate(gate, &case, &cores, &plan).is_ok(), "{gate:?}");
    }
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert!(matches!(failure, Err(ReplayFailure::EventMoved { .. })));
}

#[test]
fn a_core_that_delays_every_event_alike_fails_against_a_reference_policy() {
    let case = ChanceCase::ectopy().expect("a body");
    let tone = case.tone_slot();
    let clock = clock();
    let delayed = |interval_ns| DelaysEveryEvent {
        interval_ns,
        tone,
        by: 0.5,
        from: at(0),
    };
    let second = move || delayed(SECOND / 16);
    let plan = self::plan(&clock, &second);
    let cores = move || delayed(SECOND / 4);
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::EventMoved { .. })),
        "{failure:?}"
    );
    let honest = || reference(SECOND);
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &honest, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::EventMoved { .. })),
        "the second policy answers too: {failure:?}"
    );
}

/// It stalls just before the reference's last event, so its own falls past the case's end, every pair agrees
/// and the list is one short.
#[test]
fn a_core_that_stops_firing_differs_from_the_reference_by_its_list_of_events() {
    let whole = ChanceCase::ectopy().expect("a body");
    let tone = whole.tone_slot();
    let fired = events(&whole, &reference(SECOND / 16));
    let last = fired.last().expect("an event").at().nanoseconds();
    let case = ChanceCase::ectopy()
        .expect("a body")
        .until(SimTime::from_ns(last + 5 * SECOND));
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = move || DelaysEveryEvent {
        interval_ns: SECOND / 8,
        tone,
        by: 1.0,
        from: SimTime::from_ns(last - SECOND),
    };
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::EventsDiffer { .. })),
        "{failure:?}"
    );
}

/// A negative delay hurries, past the reference's last event, so every pair agrees and the policy's list is one
/// long, with its extra event far enough from the end to be no straddle.
#[test]
fn a_core_that_fires_an_event_too_many_differs_from_the_reference_by_its_list_of_events() {
    let long = ChanceCase::ectopy().expect("a body").until(at(2_400));
    let tone = long.tone_slot();
    let fired = events(&long, &reference(SECOND / 16));
    let pair = fired.windows(2).find(|pair| {
        pair[0].entered().value == "sinus"
            && pair[1].at().nanoseconds() - pair[0].at().nanoseconds() >= 16 * SECOND
    });
    let pair = pair.expect("a long wait for an event");
    let case = ChanceCase::ectopy()
        .expect("a body")
        .until(SimTime::from_ns(pair[1].at().nanoseconds() - 2 * SECOND));
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = move || DelaysEveryEvent {
        interval_ns: SECOND / 8,
        tone,
        by: -1.0,
        from: SimTime::from_ns(pair[0].at().nanoseconds() + SECOND),
    };
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::EventsDiffer { .. })),
        "{failure:?}"
    );
}

/// Two rhythms fire independently, so two unrelated events can share a landing under a coarser policy and come
/// out in another order, and each still pairs by its stream and ordinal. One seed in 48 shows this.
#[test]
fn two_step_policies_may_fire_unrelated_events_in_another_order() {
    let case = ChanceCase::two_rhythms()
        .expect("a body")
        .delivering(Vec::new())
        .seeded(Seed::new([37, 8, 9, 10]));
    let order = |list: &[Fired]| -> Vec<(LawName, u64)> {
        (list.iter())
            .map(|event| (event.stream(), event.ordinal()))
            .collect()
    };
    assert_ne!(
        order(&events(&case, &reference(SECOND))),
        order(&events(&case, &reference(SECOND / 16))),
        "no two events swapped, so this test pressed nothing"
    );
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = || reference(SECOND);
    let verdict = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert!(verdict.is_ok(), "{:?}", verdict.err());
}

/// A landing reads one guard's gap, and another guard can cross first in the same span. With the forcing flat a
/// first crossing is a threshold over a rate, so this needs no run to compare with.
#[test]
fn a_landing_stops_at_the_first_crossing_whichever_guard_it_reads() {
    let core = reference(64 * SECOND);
    let floor = core.resolution().minimum_interval_ns() as f64;
    let mut first_of_a_shared_span = 0;
    for word in 0..64 {
        let case = ChanceCase::two_rhythms()
            .expect("a body")
            .steady()
            .delivering(Vec::new())
            .until(at(64))
            .seeded(Seed::new([word, 11, 12, 13]));
        let fired = events(&case, &core);
        let first = |stream: &str| {
            (fired.iter()).find(|event| event.stream().local == stream && event.ordinal() == 0)
        };
        let (read, other) = (first("ectopic-beat"), first("second-there"));
        for event in [read, other].into_iter().flatten() {
            let [threshold, _] = event.threshold().bits();
            let truth = f64::from_bits(threshold) / ONSET_PER_SECOND * SECOND as f64;
            let apart = event.at().nanoseconds() as f64 - truth;
            assert!(
                apart.abs() <= floor + 1.0,
                "seed {word}: {apart} ns from its crossing"
            );
        }
        if let (Some(read), Some(other)) = (read, other) {
            first_of_a_shared_span += usize::from(other.at() < read.at());
        }
    }
    assert!(
        first_of_a_shared_span >= 8,
        "{first_of_a_shared_span} shared spans"
    );
}

/// A run stops at its first head past the end. Just short of an event, a coarse step overshoots and fires it and a
/// fine run stops before it, so one list is short and nothing differs.
#[test]
fn an_event_only_a_coarse_step_reaches_past_the_end_is_no_difference() {
    let whole = ChanceCase::ectopy().expect("a body");
    let last = events(&whole, &reference(SECOND / 16));
    let last = last.last().expect("an event").at().nanoseconds();
    let case = ChanceCase::ectopy()
        .expect("a body")
        .until(SimTime::from_ns(last - SECOND / 8));
    assert_eq!(
        events(&case, &reference(SECOND)).len(),
        events(&case, &reference(SECOND / 16)).len() + 1,
        "one list is short, or this test presses nothing"
    );
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let cores = || reference(SECOND);
    for distance in [EVENT_TOLERANCE_NS, u64::MAX] {
        let mut plan = plan(&clock, &fine);
        plan.chance
            .as_mut()
            .expect("a plan for chance")
            .event_tolerance_ns = distance;
        let gate = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
        assert!(gate.is_ok(), "{distance}: {:?}", gate.err());
    }
}

#[test]
fn step_policy_evidence_names_the_distance_it_held() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = || reference(SECOND);
    let evidence = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan).expect("the gate");
    assert_eq!(evidence.tolerance_ns(), Some(EVENT_TOLERANCE_NS));
}

#[test]
fn two_policies_on_one_grid_are_missing_evidence() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let same = || reference(SECOND);
    let plan = plan(&clock, &same);
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &same, &plan);
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::StepPolicy
        })
    );
}

#[test]
fn a_body_with_a_hazard_and_no_second_policy_is_missing_evidence() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = ReplayPlan {
        chance: None,
        ..plan(&clock, &fine)
    };
    let cores = || reference(SECOND);
    let failure = replay_gate(ReplayGate::StepPolicy, &case, &cores, &plan);
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::StepPolicy
        })
    );
}

#[test]
fn a_run_that_fired_nothing_is_missing_evidence_for_every_gate() {
    let case = ChanceCase::ectopy().expect("a body").until(at(1));
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let cores = || reference(SECOND / 4);
    for gate in [
        ReplayGate::Pace,
        ReplayGate::Resume,
        ReplayGate::LateAdmission,
        ReplayGate::Rewind,
        ReplayGate::ExtraEvaluation,
        ReplayGate::StepPolicy,
        ReplayGate::NonPerturbation,
    ] {
        let failure = replay_gate(gate, &case, &cores, &plan);
        assert_eq!(failure.err(), Some(ReplayFailure::MissingEvidence { gate }));
    }
}

/// The first firing with no input in play tells where the crossing lies, and each input then sits inside one 64 s
/// span with it, on one side. The late run takes its input at the last head it can, so every ticket either run
/// solved, refusals included, must be the same.
#[test]
fn a_late_input_on_either_side_of_a_crossing_takes_the_known_runs_tickets_under_refusals() {
    let core = || TracesItsTickets {
        inner: RefusesByHash {
            inner: reference(64 * SECOND),
            salt: 7,
        },
        trace: RefCell::new(Vec::new()),
    };
    let seeded = |word| {
        ChanceCase::ectopy()
            .expect("a body")
            .seeded(Seed::new([word, 2, 3, 4]))
    };
    let first = |word| {
        let quiet = seeded(word).delivering(Vec::new());
        events(&quiet, &core())[0].at().nanoseconds()
    };
    let word = (0..64).find(|&word| (20 * SECOND..60 * SECOND).contains(&first(word)));
    let word = word.expect("a seed whose first crossing lies past the short spans");
    let crossing = first(word);
    let mut refusals = 0;
    for stamp in [crossing - 5 * SECOND, crossing + 5 * SECOND] {
        let delivery = Delivery {
            id: b"on one side",
            from: SimTime::from_ns(stamp),
            to: at(100),
            amount: Measure::new(10.0).expect("finite"),
        };
        let case = seeded(word).delivering(vec![delivery]).until(at(120));
        let withheld = core();
        let mut heads = vec![case.admit_withheld(&withheld).expect("admitted")];
        let mut late = None;
        let mut stepped = heads.pop().expect("a session");
        while stepped.record().accepted().time() < case.end() {
            let (next, _) = one_step(stepped);
            stepped = next;
            if stepped.record().earliest_stamp() <= SimTime::from_ns(stamp) {
                late = Some(stepped.record().revision());
            }
        }
        let arrival = late.expect("a head before the input");
        let run = |core: &TracesItsTickets, known: bool| {
            let mut session = if known {
                case.admit(core)
            } else {
                case.admit_withheld(core)
            }
            .expect("admitted");
            let mut rows = Vec::new();
            while session.record().accepted().time() < case.end() {
                if !known && session.record().revision() == arrival {
                    let event = case.events().expect("events").remove(0);
                    let command = Digest::of_canonical(b"a command");
                    let input = AuthorizedInput::from_command_receipt(command, event);
                    session.enqueue(input).expect("it can still arrive");
                }
                let (next, _) = one_step(session);
                session = next;
                rows.push((session.record().accepted().time(), session.record().seal()));
            }
            (rows, session.record().digests())
        };
        let (known_core, late_core) = (core(), core());
        assert_eq!(run(&known_core, true), run(&late_core, false));
        assert_eq!(*known_core.trace.borrow(), *late_core.trace.borrow());
        let trace = known_core.trace.borrow();
        let attempts = trace.iter().map(|ticket| ticket.2).filter(|&attempt| attempt > 0);
        refusals += attempts.count();
    }
    assert!(
        refusals > 0,
        "no ticket was refused, so this test pressed nothing"
    );
}

/// The step into a firing is a landing, so its head is where a resume has the most to lose, and no spacing of
/// `resume_every` is what picks it here.
#[test]
fn the_resume_gate_presses_the_head_just_before_a_firing() {
    let case = ChanceCase::ectopy().expect("a body").delivering(Vec::new());
    let honest = reference(SECOND);
    let mut session = case.admit(&honest).expect("admitted");
    let head = loop {
        let head = session.record().accepted().time();
        let (next, receipt) = one_step(session);
        session = next;
        if !receipt.fired().is_empty() {
            break head;
        }
    };
    assert!(head > at(1), "the gate's first revisions would pick this head");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = ReplayPlan {
        resume_every: u64::MAX,
        ..plan(&clock, &fine)
    };
    let cores = move || LosesItsWayIfResumedAt {
        inner: reference(SECOND),
        head,
        first: Cell::new(true),
    };
    let failure = replay_gate(ReplayGate::Resume, &case, &cores, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::Faulted { .. })),
        "{failure:?}"
    );
}

#[derive(Default)]
struct Store(Vec<(Digest, Vec<u8>)>);
impl HistoryStore for Store {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let link = self.0.iter().find(|known| known.0 == *seal)?;
        Some(link.1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        self.0.push((seal, link));
        Ok(())
    }
}

const HEADER: usize = 128;
const RAW_DIGEST: core::ops::Range<usize> = 8..40;

fn chance_fields(body: &CompiledBody) -> (usize, usize, usize) {
    let rows = body.rows();
    let count = |wanted: fn(&RowKind) -> bool| rows.iter().filter(|row| wanted(&row.kind())).count();
    let amounts = count(|kind| matches!(kind, RowKind::Amount { .. }));
    let memory = count(|kind| matches!(kind, RowKind::Memory(_)));
    let algebraic = rows.len() - amounts - memory;
    let tag = 8 + "ng-contract image 1".len();
    let lists = (8 + 16 * amounts) + (8 + 8 * memory) + (8 + 8 * algebraic);
    let modes = HEADER + 8 + tag + 8 + lists + 8;
    let ordinals = modes + 8 + 8;
    (modes, ordinals, ordinals + 2 * 8)
}

/// The image closes on the bound first modes, and the fixture's one variable is its last word.
fn bound_mode(saved: &Checkpoint) -> usize {
    let bytes = saved.as_bytes();
    let stated = bytes[HEADER..].first_chunk().expect("the image's length");
    HEADER + u64::from_be_bytes(*stated) as usize
}

fn resealed(mut bytes: Vec<u8>) -> Checkpoint {
    let raw = Digest::of_canonical(&bytes[RAW_DIGEST.end..]);
    bytes[RAW_DIGEST].copy_from_slice(raw.as_bytes());
    Checkpoint::from_bytes(bytes)
}

fn forged(saved: &Checkpoint, at: usize, value: u64) -> Checkpoint {
    let mut bytes = saved.as_bytes().to_vec();
    bytes[at..at + 8].copy_from_slice(&value.to_be_bytes());
    resealed(bytes)
}

/// One ordinal gone and every length around it true, so only the list's length is wrong.
fn short_of_an_ordinal(saved: &Checkpoint, ordinals: usize) -> Checkpoint {
    let mut bytes = saved.as_bytes().to_vec();
    bytes.drain(ordinals..ordinals + 8);
    let mut lower = |at: usize| {
        let stated = bytes[at..at + 8].first_chunk().expect("eight bytes");
        let shorter = u64::from_be_bytes(*stated) - 8;
        bytes[at..at + 8].copy_from_slice(&shorter.to_be_bytes());
    };
    lower(RAW_DIGEST.end);
    lower(HEADER);
    let count = ordinals - 8;
    bytes[count..ordinals].copy_from_slice(&1_u64.to_be_bytes());
    resealed(bytes)
}

#[test]
fn a_head_after_a_firing_resumes_and_rewinds_exactly_and_refuses_forged_chance_state() {
    let case = ChanceCase::ectopy().expect("a body").delivering(Vec::new());
    let (core, fresh) = (reference(SECOND), reference(SECOND));
    let mut session = case.admit(&core).expect("admitted");
    loop {
        let (next, receipt) = one_step(session);
        session = next;
        if !receipt.fired().is_empty() {
            break;
        }
    }
    let mut store = Store::default();
    let saved = session.checkpoint(&mut store).expect("saved");
    let identity = session.record().identity().clone();
    let body = case.body();
    let (modes, ordinals, flag) = chance_fields(body);
    for (place, value, expected) in [
        (modes, 99, CheckpointError::Malformed),
        (modes, 1, CheckpointError::SealMismatch),
        (ordinals, 55, CheckpointError::SealMismatch),
        (flag, 0, CheckpointError::SealMismatch),
        (flag, 2, CheckpointError::Malformed),
    ] {
        let refused = Session::resume(&forged(&saved, place, value), &identity, body, &fresh, &store);
        assert_eq!(refused.err(), Some(expected), "at {place}, {value}");
    }
    let moved = forged(&saved, bound_mode(&saved), 0);
    assert!(matches!(
        Session::resume(&moved, &identity, body, &fresh, &store).err(),
        Some(CheckpointError::HistoryAltered { .. })
    ));
    let short = short_of_an_ordinal(&saved, ordinals);
    assert_eq!(
        Session::resume(&short, &identity, body, &fresh, &store).err(),
        Some(CheckpointError::Malformed)
    );
    let resumed = Session::resume(&saved, &identity, body, &fresh, &store).expect("resumed");
    let (continued, receipt) = one_step(session);
    let (resumed, on_resume) = one_step(resumed);
    let branch = BranchId::new(Digest::of_canonical(b"a fork at a firing"));
    let forked = continued.rewind(&saved, branch).expect("a fork");
    let (forked, on_fork) = one_step(forked);
    assert_eq!(continued.record().digests(), resumed.record().digests());
    assert_eq!(continued.record().digests(), forked.record().digests());
    assert!(receipt.after_breakpoint() && on_resume.after_breakpoint() && on_fork.after_breakpoint());
}

/// A trial shorter than a span that passed can still be refused, because a stage's error over a forcing is not
/// monotone in its length, so this run must reach its end at spans that alias the knots.
#[test]
fn a_long_span_over_a_forcing_lands_its_events_and_never_crawls() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(64 * SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let (mut steps, mut fired) = (0, 0);
    while session.record().accepted().time() < case.end() {
        let next = session.record().accepted().time().nanoseconds() + 1;
        let (reached, events) = finished(session, SimTime::from_ns(next));
        session = reached;
        fired += events.len();
        steps += 1;
        assert!(
            steps < 400,
            "a run of 240 s at 64 s spans crawled at {steps} steps"
        );
    }
    assert!(fired >= 8, "{fired} events in {steps} steps");
}

#[test]
fn a_core_that_refuses_every_landing_faults_and_publishes_nothing_past_the_crossing() {
    let case = ChanceCase::ectopy().expect("a body");
    let honest = events(&case, &reference(SECOND));
    let core = RefusesEveryLanding {
        inner: reference(SECOND),
        interval_ns: SECOND,
    };
    let mut session = case.admit(&core).expect("admitted");
    let stopped = loop {
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                Step::Accepted(session, _) => break session,
                Step::Refused(session) => session.retry(),
                Step::Faulted(stopped) => {
                    assert_eq!(stopped.fault().kind(), EngineFaultKind::NoAdmissibleStep);
                    let head = stopped.record().accepted().time();
                    assert!(head < honest[0].at());
                    return;
                }
            };
        };
        if session.record().accepted().time() >= case.end() {
            break session;
        }
    };
    panic!("{:?}", stopped.record().accepted().time());
}

#[test]
fn a_refused_landing_trial_retries_at_a_halved_span_and_loses_nothing() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = RefusesItsFirstTrial {
        inner: reference(SECOND),
        interval_ns: SECOND,
        seen: Cell::new(None),
    };
    let events = events(&case, &core);
    let honest = self::events(&case, &reference(SECOND));
    assert_eq!(events.len(), honest.len());
    for (a, b) in events.iter().zip(&honest) {
        assert_eq!((a.stream(), a.ordinal()), (b.stream(), b.ordinal()));
        assert!(a.at().nanoseconds().abs_diff(b.at().nanoseconds()) <= EVENT_TOLERANCE_NS);
    }
}

#[test]
fn a_core_that_keeps_its_history_across_a_firing_is_a_method_mismatch() {
    // No delivery, so a firing is the only breakpoint that can catch it.
    let case = ChanceCase::ectopy().expect("a body").delivering(Vec::new());
    let core = KeepsItsHistoryAcrossAFiring { interval_ns: SECOND };
    let mut session = case.admit(&core).expect("admitted");
    let fault = loop {
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                Step::Accepted(session, _) => break session,
                Step::Refused(session) => session.retry(),
                Step::Faulted(stopped) => {
                    assert_eq!(stopped.fault().kind(), EngineFaultKind::MethodMismatch);
                    return;
                }
            };
        };
        if session.record().accepted().time() >= case.end() {
            break session.record().revision();
        }
    };
    panic!("{fault}");
}

#[test]
fn a_firing_is_a_breakpoint_a_fresh_session_reaches_from_sealed_state() {
    let case = ChanceCase::ectopy().expect("a body").delivering(Vec::new());
    let core = reference(SECOND);
    let session = case.admit(&core).expect("admitted");
    let (session, fired) = finished(session, at(1));
    let first = events(&case, &core)[0];
    assert!(fired.is_empty(), "no event yet");
    let mut session = session;
    let mut last = None;
    while session.record().accepted().time() < first.at() {
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                Step::Accepted(session, receipt) => {
                    last = Some(receipt);
                    break session;
                }
                Step::Refused(session) => session.retry(),
                Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
            };
        };
    }
    assert_eq!(last.expect("a step").fired()[0].at(), first.at());
    let Step::Accepted(session, receipt) = session.step() else {
        panic!("the step after a firing");
    };
    assert!(receipt.after_breakpoint());
    let Step::Accepted(_, receipt) = session.step() else {
        panic!("the step after that");
    };
    assert!(!receipt.after_breakpoint(), "one firing is one breakpoint");
}

#[test]
fn a_refusal_discards_the_reset_the_advance_and_the_mode_together() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = RefusesItsFirstTrial {
        inner: reference(SECOND),
        interval_ns: SECOND,
        seen: Cell::new(None),
    };
    let mut session = case.admit(&core).expect("admitted");
    loop {
        let before = session.record().digests();
        match session.step() {
            Step::Accepted(next, _) => session = next,
            Step::Refused(refused) => {
                assert_eq!(refused.record().digests(), before);
                return;
            }
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        }
    }
}

#[test]
fn a_forcing_period_the_clock_cannot_hold_is_refused_at_binding() {
    let (body, _) = holds_a_forcing().expect("a body");
    let key = ParameterKey::new("fixture.period");
    for seconds in [1.5e-9, 1e-12, 1e100, 0.1 + 0.2] {
        let mut packet = ParameterPacket::<SyntheticFixture>::new();
        let note = "synthetic";
        let scale = Measure::<Dimensionless>::new(3.0).expect("finite");
        packet
            .synthetic(ParameterKey::new("fixture.scale"), Binding::Body, scale, note)
            .expect("one entry");
        let amplitude = Measure::<Dimensionless>::new(0.5).expect("finite");
        packet
            .synthetic(
                ParameterKey::new("fixture.amplitude"),
                Binding::Body,
                amplitude,
                note,
            )
            .expect("one entry");
        let period = Measure::<Time>::new(seconds).expect("finite");
        packet
            .synthetic(key, Binding::Body, period, note)
            .expect("one entry");
        assert_eq!(
            body.bind(packet).err(),
            Some(AdmissionError::ForcingPeriod { key }),
            "{seconds}"
        );
    }
}

#[test]
fn a_forcing_period_the_clock_cannot_hold_is_malformed_under_a_good_digest() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(SECOND);
    let session = case.admit(&core).expect("admitted");
    let mut store = Store::default();
    let saved = session.checkpoint(&mut store).expect("saved");
    let identity = session.record().identity().clone();
    let held = 10.0_f64.to_bits().to_be_bytes();
    let bytes = saved.as_bytes();
    let at = (0..bytes.len() - 8)
        .find(|&at| bytes[at..at + 8] == held)
        .expect("the bound period");
    for seconds in [1.5e-9_f64, 0.0, 1e100] {
        let forged = forged(&saved, at, seconds.to_bits());
        assert_eq!(
            Session::resume(&forged, &identity, case.body(), &core, &store).err(),
            Some(CheckpointError::Malformed),
            "{seconds}"
        );
    }
}

/// Only forged bytes can carry such an identity. The refusal comes before any byte is read, so honest bytes press
/// it: they would come back as another identity's.
#[test]
fn resume_refuses_a_floor_of_zero_as_admission_does() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = reference(SECOND);
    let session = case.admit(&core).expect("admitted");
    let mut store = Store::default();
    let saved = session.checkpoint(&mut store).expect("saved");
    let known = session.record().identity();
    let zero = reference(SECOND).with_floor(0);
    let identity = RunIdentity::new(
        known.model(),
        known.packet(),
        Seed::new([1, 2, 3, 4]),
        zero.resolution(),
    );
    assert_eq!(
        Session::resume(&saved, &identity, case.body(), &zero, &store).err(),
        Some(CheckpointError::ZeroMinimumInterval)
    );
}

#[test]
fn every_replay_gate_passes_with_a_stream_and_a_forcing_on() {
    let case = ChanceCase::ectopy().expect("a body");
    let clock = clock();
    let fine = || reference(SECOND / 4);
    let plan = plan(&clock, &fine);
    let evidence = replay_gates(&case, &|| reference(SECOND), &plan).expect("every gate");
    assert!(!evidence.of(ReplayGate::Resume).fired().is_empty());
    assert!(!evidence.of(ReplayGate::NonPerturbation).fired().is_empty());
}

struct Random(u64);
impl Random {
    fn below(&mut self, limit: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % limit
    }
}

fn random_case(random: &mut Random) -> ChanceCase {
    let names: [&'static [u8]; 3] = [b"a", b"b", b"c"];
    let deliveries = (0..1 + random.below(3))
        .map(|name| {
            let from = SECOND + random.below(40 * SECOND);
            let to = from + (1 + random.below(30)) * SECOND;
            Delivery {
                id: names[name as usize],
                from: SimTime::from_ns(from),
                to: SimTime::from_ns(to),
                amount: Measure::new((10 * (1 + from % 50)) as f64).expect("finite"),
            }
        })
        .collect();
    let words = [random.below(1 << 40), random.below(1 << 40), 3, 4];
    ChanceCase::ectopy()
        .expect("a body")
        .delivering(deliveries)
        .seeded(Seed::new(words))
        .until(at(60))
}

/// A fault, an input inside the first step or a case that fired nothing says nothing about the claim; a
/// divergence breaks it.
#[test]
fn late_inputs_match_the_known_timeline_with_a_guard_among_them() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let (mut same, mut said_nothing) = (0, 0);
    for round in 0..120 {
        let case = random_case(&mut random);
        let interval_ns = (1 + random.below(40)) * SECOND / 2;
        let salt = random.below(u64::MAX);
        let clock = clock();
        let fine = || reference(SECOND);
        let plan = plan(&clock, &fine);
        let tone = case.tone_slot();
        let honest = move || reference(interval_ns);
        let refusing = move || RefusesByHash {
            inner: reference(interval_ns),
            salt,
        };
        let jagged = move || DrawsPerStep {
            interval_ns,
            tone,
            reach: 1.5,
        };
        let cores: [&dyn CoreFactory; 3] = [&honest, &refusing, &jagged];
        let cores = cores[round % 3];
        match replay_gate(ReplayGate::LateAdmission, &case, cores, &plan) {
            Ok(_) => same += 1,
            Err(
                ReplayFailure::NoLateBoundary { .. }
                | ReplayFailure::Faulted { .. }
                | ReplayFailure::MissingEvidence { .. },
            ) => said_nothing += 1,
            Err(failure) => panic!("round {round}: {failure:?}"),
        }
    }
    assert!(same > 60, "{same} compared and {said_nothing} said nothing");
}

fn two_vessels() -> Result<Schema, AdmissionError> {
    let origin = || -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?);
    let a = map.site::<VesselSegment>("a", origin()?);
    let b = map.site::<VesselSegment>("b", origin()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    Schema::new(map.finish()?)
}

fn holds_a_forcing() -> Result<(CompiledBody, RowId), AdmissionError> {
    let body = assemble(two_vessels()?, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        let scale = laws.parameter(ParameterKey::new("fixture.scale"), Admissible::finite());
        let wobble = laws.forcing(
            "wobble",
            ParameterKey::new("fixture.period"),
            ParameterKey::new("fixture.amplitude"),
        );
        let held = laws.algebraic::<Dimensionless>("held", Admissible::finite());
        let closure = laws.kernel(|mut k| {
            let seen = k.algebraic(&held);
            let scale = k.parameter(&scale);
            let factor = k.forcing(&wobble);
            let target = k.scale(scale, factor);
            let residual = k.subtract(seen, target);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&held, closure);
        body.add(laws.finish());
        body.compile()
    })?;
    let row = (body.rows().iter())
        .find(|row| matches!(row.kind(), RowKind::Algebraic(_)))
        .map(Row::id)
        .expect("the slot");
    Ok((body, row))
}

/// `verify` is public, so a core can run it on its own proposal. It says nothing of a guard there: only an
/// attempt reads a crossing, and only the step an attempt selects fires.
#[test]
fn a_dry_verify_fires_nothing_and_shows_no_threshold() {
    let case = ChanceCase::ectopy().expect("a body");
    let core = VerifiesDry {
        inner: reference(SECOND),
        checked: Cell::new(0),
    };
    let fired = events(&case, &core);
    assert!(!fired.is_empty());
    assert!(core.checked.get() > fired.len() as u64);
}

/// Two spans to one instant: a forcing read at a ticket's start would differ between them, and one read at
/// the stage's instant agrees to the solve's own rounding.
#[test]
fn a_forcing_is_a_function_of_the_instant_and_not_of_the_step() -> Result<(), InitializationFailure> {
    let (body, held) = holds_a_forcing()?;
    let packet = || -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let mut packet = ParameterPacket::new();
        let note = "synthetic";
        let body_binding = Binding::Body;
        let scale = Measure::<Dimensionless>::new(3.0)?;
        packet.synthetic(ParameterKey::new("fixture.scale"), body_binding, scale, note)?;
        let amplitude = Measure::<Dimensionless>::new(0.5)?;
        packet.synthetic(
            ParameterKey::new("fixture.amplitude"),
            body_binding,
            amplitude,
            note,
        )?;
        let period = Measure::<Time>::new(1.0)?;
        packet.synthetic(ParameterKey::new("fixture.period"), body_binding, period, note)?;
        for row in body.rows() {
            match row.kind() {
                RowKind::Algebraic(_) => {
                    packet.start(row.kind(), Measure::<Dimensionless>::new(3.0)?, note)?
                }
                _ => packet.start(row.kind(), Measure::<Water>::new(100.0)?, note)?,
            }
        }
        Ok(packet)
    };
    let value_at_one_second = |interval: u64| -> Result<Compensated, InitializationFailure> {
        let core = reference(interval);
        let session = admit_fixture(&body, packet()?, BoundaryPort::new(Vec::new())?, &core)?;
        let (session, _) = finished(session, at(1));
        Ok(session.record().accepted().value(held))
    };
    let (long, short) = (value_at_one_second(SECOND)?, value_at_one_second(SECOND / 4)?);
    let apart = long.minus(short);
    let small = Compensated::of(&Measure::<Dimensionless>::new(1e-6)?);
    assert!(apart.at_most(small) && Compensated::ZERO.minus(apart).at_most(small));
    let unmoved = Compensated::of(&Measure::<Dimensionless>::new(3.0)?);
    assert_ne!(long.bits(), unmoved.bits(), "the forcing moved it");
    Ok(())
}

/// A body with a stream holds no algebraic row yet, so a law's own domain shows the value: `ln(draw - p)` has
/// a value at admission only for `p` below one half.
#[test]
fn admission_holds_a_draw_at_its_mean() -> Result<(), InitializationFailure> {
    let body = assemble(two_vessels()?, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        let offset = laws.parameter(ParameterKey::new("fixture.offset"), Admissible::finite());
        let pace = laws.parameter(ParameterKey::new("fixture.pace"), Admissible::finite());
        let dice = laws.stream("dice");
        let integral = laws.state::<Dimensionless>("integral", Admissible::nonnegative());
        let [a, b] = laws.modes("rhythm", ["a", "b"]);
        let rate = laws.kernel(|mut k| {
            let draw = k.draw(&dice);
            let offset = k.parameter(&offset);
            let above = k.subtract(draw, offset);
            let logarithm = k.ln(above);
            let pace = k.parameter(&pace);
            let rate = k.scale(pace, logarithm);
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.memory_rate(&integral, rate);
        laws.hazard(HazardGuard::new(&integral, &dice, a.to(&b)?));
        body.add(laws.finish());
        body.compile()
    })?;
    let start = |offset: f64| -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let mut packet = ParameterPacket::new();
        let note = "synthetic";
        let number = Measure::<Dimensionless>::new(offset)?;
        packet.synthetic(ParameterKey::new("fixture.offset"), Binding::Body, number, note)?;
        let pace = Measure::<Rate<Dimensionless>>::new(1.0)?;
        packet.synthetic(ParameterKey::new("fixture.pace"), Binding::Body, pace, note)?;
        let variable = LawName {
            owner: "CV",
            local: "rhythm",
        };
        packet.start_mode(variable, "a", note)?;
        for row in body.rows() {
            match row.kind() {
                RowKind::Memory(_) => packet.start(row.kind(), Measure::<Dimensionless>::new(0.0)?, note)?,
                _ => packet.start(row.kind(), Measure::<Water>::new(100.0)?, note)?,
            }
        }
        Ok(packet)
    };
    let core = reference(SECOND);
    let port = || BoundaryPort::new(Vec::new());
    assert!(admit_fixture(&body, start(0.4999)?, port()?, &core).is_ok());
    let past = admit_fixture(&body, start(0.5001)?, port()?, &core).err();
    assert!(matches!(past, Some(InitializationFailure::Undefined { .. })));
    Ok(())
}

/// A law's algebraic row equals the forcing's factor times a constant, so admission's exact-zero check passes
/// only if it read exactly the mean.
#[test]
fn admission_holds_the_forcing_at_its_mean() -> Result<(), InitializationFailure> {
    let (body, _) = holds_a_forcing()?;
    let start = |held: f64| -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let mut packet = ParameterPacket::new();
        let default = Binding::Body;
        packet.synthetic(
            ParameterKey::new("fixture.scale"),
            default,
            Measure::<Dimensionless>::new(3.0)?,
            "synthetic",
        )?;
        packet.synthetic(
            ParameterKey::new("fixture.amplitude"),
            default,
            Measure::<Dimensionless>::new(0.5)?,
            "synthetic",
        )?;
        packet.synthetic(
            ParameterKey::new("fixture.period"),
            default,
            Measure::<Time>::new(1.0)?,
            "synthetic",
        )?;
        for row in body.rows() {
            match row.kind() {
                RowKind::Algebraic(_) => {
                    packet.start(row.kind(), Measure::<Dimensionless>::new(held)?, "synthetic")?;
                }
                _ => packet.start(row.kind(), Measure::<Water>::new(100.0)?, "synthetic")?,
            }
        }
        Ok(packet)
    };
    let core = reference(SECOND);
    let port = || BoundaryPort::new(Vec::new());
    assert!(admit_fixture(&body, start(3.0)?, port()?, &core).is_ok());
    let off = admit_fixture(&body, start(3.0001)?, port()?, &core).err();
    assert!(matches!(off, Some(InitializationFailure::Inconsistent { .. })));
    Ok(())
}
