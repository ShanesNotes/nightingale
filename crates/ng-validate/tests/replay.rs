mod replay_cores;

use core::cell::{Cell, RefCell};
use ng_contract::*;
use ng_record::*;
use ng_validate::*;
use replay_cores::{
    ActsOnACut, ClipsAt, CountsInACell, KeepsACache, KeepsACount, KeepsTheDigest, MarksItsFirstSteps,
    ReadsPastTheBreakpoint, ReadsTheHostClock, RefreshesASixth, RefusesByHash, RefusesOne,
    RestartsAfterAJump, ScriptedClock, Seen, ShowsAt,
};

const SECOND: u64 = 1_000_000_000;
const GATES: [ReplayGate; 4] = [
    ReplayGate::Pace,
    ReplayGate::Resume,
    ReplayGate::LateAdmission,
    ReplayGate::Rewind,
];

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

/// What a core and its test both hold, for the length of the test.
fn shared<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn start() -> [Measure<Water>; 2] {
    [measure(500_500.0), measure(499_500.0)]
}

fn at_rest() -> [Measure<Water>; 2] {
    [measure(1_000.0), measure(1_000.0)]
}

fn delivery(id: &'static [u8], from: u64, to: u64) -> Delivery {
    Delivery {
        id,
        from: at(from),
        to: at(to),
        amount: measure(100.0),
    }
}

fn delivering(deliveries: Vec<Delivery>, end: u64) -> TwoReservoirs {
    TwoReservoirs::delivering(start(), deliveries)
        .expect("a body")
        .until(at(end))
}

fn busy() -> TwoReservoirs {
    let deliveries = vec![
        delivery(b"first", 30, 90),
        delivery(b"beside it", 30, 60),
        Delivery {
            id: b"off the grid",
            from: SimTime::from_ns(45 * SECOND + 500_000_000),
            to: at(70),
            amount: measure(40.0),
        },
    ];
    delivering(deliveries, 120)
}

fn water(id: &[u8], from: SimTime, to: SimTime, amount: f64) -> BoundaryEvent {
    let delivery = WaterDelivery {
        boundary: "exterior",
        amount: measure(amount),
        ends: to,
    };
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(id)),
        from,
        BoundaryInput::Water(delivery),
    )
}

fn authorized(id: &[u8], from: u64, to: u64) -> AuthorizedInput {
    let command = Digest::of_canonical(b"a command");
    AuthorizedInput::from_command_receipt(command, water(id, at(from), at(to), 100.0))
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    ReferenceIntegrator::new(interval_ns)
}

fn refused_past_the_input() -> ReferenceIntegrator {
    let scales = ErrorScales::uniform(measure(0.1), measure(0.0)).expect("scales");
    reference(16 * SECOND).with_scales(scales)
}

fn script() -> ScriptedClock {
    ScriptedClock::new(vec![
        0,
        0,
        5 * SECOND,
        5 * SECOND,
        7 * SECOND,
        900 * SECOND,
        90_000 * SECOND,
    ])
}

fn plan(clock: &ScriptedClock) -> ReplayPlan<'_> {
    ReplayPlan {
        clock,
        simulated_per_wall: 1,
        chunks: vec![3 * SECOND, 11 * SECOND + 1, SECOND / 2],
        resume_every: 5,
    }
}

fn advance<'m>(session: Session<'m, Running>) -> Session<'m, Running> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, _) => return session,
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("faulted: {:?}", stopped.fault()),
        };
    }
}

fn to<'m>(mut session: Session<'m, Running>, end: SimTime) -> Session<'m, Running> {
    while session.record().accepted().time() < end {
        session = advance(session);
    }
    session
}

fn seals(mut session: Session<'_, Running>, end: SimTime) -> Vec<(SimTime, Seal)> {
    let mut rows = vec![(session.record().accepted().time(), session.record().seal())];
    while session.record().accepted().time() < end {
        session = advance(session);
        rows.push((session.record().accepted().time(), session.record().seal()));
    }
    rows
}

#[derive(Default, Clone)]
struct Store {
    links: Vec<(Digest, Vec<u8>)>,
}
impl Store {
    fn replace(&mut self, seal: Digest, link: Vec<u8>) {
        self.links.retain(|known| known.0 != seal);
        self.links.push((seal, link));
    }
    fn without(&self, seal: Digest) -> Self {
        let mut store = self.clone();
        store.links.retain(|known| known.0 != seal);
        store
    }
}
impl HistoryStore for Store {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let link = self.links.iter().find(|known| known.0 == *seal)?;
        Some(link.1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        self.replace(seal, link);
        Ok(())
    }
}

struct RefusesToKeep;
impl HistoryStore for RefusesToKeep {
    fn resolve(&self, _seal: &Digest) -> Option<Vec<u8>> {
        None
    }
    fn keep(&mut self, seal: Digest, _link: Vec<u8>) -> Result<(), CheckpointError> {
        Err(CheckpointError::StoreFailed { seal })
    }
}

#[test]
fn every_gate_passes_the_reference_core_with_one_set_of_digests() {
    for case in [delivering(vec![delivery(b"input", 2_000, 6_000)], 7_000), busy()] {
        let clock = script();
        let plan = ReplayPlan {
            resume_every: 100,
            ..plan(&clock)
        };
        let evidence = replay_gates(&case, &|| reference(8 * SECOND), &plan).expect("every gate");
        let core = reference(8 * SECOND);
        let run = case.admit(&core).expect("admitted").record().identity().digest();
        assert_eq!(evidence.run(), run);
        let reference_digests = to(case.admit(&core).expect("admitted"), case.end())
            .record()
            .digests();
        assert_ne!(
            reference_digests.applied,
            case.admit(&core).expect("admitted").record().digests().applied
        );
        for gate in GATES {
            let of = evidence.of(gate);
            assert_eq!((of.gate(), of.run()), (gate, run));
            assert!(of.runs().len() >= 2, "{gate:?} compared {} runs", of.runs().len());
            assert_eq!(of.runs()[0], reference_digests, "{gate:?}");
        }
        for gate in [ReplayGate::Pace, ReplayGate::LateAdmission, ReplayGate::Rewind] {
            assert!(
                evidence
                    .of(gate)
                    .runs()
                    .iter()
                    .all(|run| *run == reference_digests),
                "{gate:?}"
            );
        }
    }
}

#[test]
fn an_honest_core_refused_past_an_input_passes_late_admission() {
    let case = delivering(vec![delivery(b"input", 18, 118)], 60);
    let core = refused_past_the_input();
    let heads: Vec<u64> = (seals(case.admit(&core).expect("admitted"), case.end()).iter())
        .map(|row| row.0.nanoseconds() / SECOND)
        .collect();
    assert_eq!(heads, [0, 8, 16, 18, 34, 50, 66]);
    let clock = script();
    let gate = replay_gate(
        ReplayGate::LateAdmission,
        &case,
        &refused_past_the_input,
        &plan(&clock),
    );
    assert_eq!(gate.expect("the gate").runs().len(), 2);
    let mut late = case.admit_withheld(&core).expect("admitted");
    late = to(late, at(16));
    assert_eq!(
        late.record().accepted().time(),
        at(16),
        "the core's own retry landed here"
    );
    late.enqueue(authorized(b"input", 18, 118))
        .expect("admitted at head 16");
    assert_eq!(
        seals(late, case.end()),
        seals(case.admit(&core).expect("admitted"), case.end())[2..]
    );
}

#[test]
fn an_input_stamped_at_the_head_matches_the_same_input_known_from_the_start() {
    let case = delivering(vec![delivery(b"input", 16, 116)], 60);
    let core = reference(8 * SECOND);
    let mut late = to(case.admit_withheld(&core).expect("admitted"), at(16));
    late.enqueue(authorized(b"input", 16, 116))
        .expect("stamped at the head");
    let known = to(case.admit(&core).expect("admitted"), at(16));
    assert_eq!(late.record().seal(), known.record().seal());
    assert_eq!(seals(late, case.end()), seals(known, case.end()));
}

#[test]
fn inputs_admitted_at_one_head_in_any_order_match_the_timeline_known_from_the_start() {
    let case = delivering(vec![delivery(b"later", 40, 50), delivery(b"sooner", 30, 50)], 60);
    let core = reference(8 * SECOND);
    let mut late = to(case.admit_withheld(&core).expect("admitted"), at(24));
    late.enqueue(authorized(b"later", 40, 50)).expect("admitted");
    late.enqueue(authorized(b"sooner", 30, 50)).expect("admitted");
    let known = to(case.admit(&core).expect("admitted"), at(24));
    assert_eq!(late.record().digests(), known.record().digests());
}

#[test]
fn a_core_that_keeps_a_cache_outside_its_continuation_fails_resume_and_rewind() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let cores = || KeepsACache {
        interval_ns: 12 * SECOND,
        steps: Cell::new(0),
    };
    let clock = script();
    for gate in [ReplayGate::Resume, ReplayGate::Rewind] {
        let failure = replay_gate(gate, &case, &cores, &plan(&clock));
        assert!(
            matches!(failure, Err(ReplayFailure::Diverged { gate: failed, .. }) if failed == gate),
            "{gate:?}: {failure:?}"
        );
    }
}

#[test]
fn the_same_count_in_a_continuation_passes_resume_and_rewind() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let cores = || KeepsACount {
        interval_ns: 12 * SECOND,
    };
    let clock = script();
    for gate in [ReplayGate::Resume, ReplayGate::Rewind] {
        replay_gate(gate, &case, &cores, &plan(&clock)).expect("a legal twin");
    }
}

fn two_callers(first: Vec<SimTime>, later: Vec<SimTime>) -> impl Fn() -> ClipsAt {
    let calls = Cell::new(0);
    move || {
        let outputs = if calls.get() == 0 {
            first.clone()
        } else {
            later.clone()
        };
        calls.set(calls.get() + 1);
        ClipsAt {
            inner: reference(8 * SECOND),
            outputs,
        }
    }
}

#[test]
fn a_core_that_clips_at_caller_output_times_fails_every_gate() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let clock = script();
    let outputs = vec![SimTime::from_ns(19 * SECOND + 7)];
    for gate in GATES {
        let cores = two_callers(Vec::new(), outputs.clone());
        let failure = replay_gate(gate, &case, &cores, &plan(&clock));
        assert!(
            matches!(failure, Err(ReplayFailure::Diverged { gate: failed, first_divergence })
                if failed == gate && first_divergence == outputs[0].min(at(24))),
            "{gate:?}: {failure:?}"
        );
    }
    let same = two_callers(outputs.clone(), outputs);
    let clock = script();
    replay_gate(ReplayGate::Pace, &case, &same, &plan(&clock)).expect("one caller's times in every run");
}

#[test]
fn a_core_told_the_timeline_in_only_one_run_fails_late_admission() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let spread: Vec<SimTime> = vec![at(20), at(30), at(35)];
    let cores = two_callers(spread, Vec::new());
    let clock = script();
    let failure = replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock));
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::NoLateBoundary {
            id: InputId::new(Digest::of_canonical(b"input"))
        }),
        "the late run has passed the head the known run gave the input"
    );
}

#[test]
fn a_core_that_reads_the_host_clock_fails_pace() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let clock = shared(ScriptedClock::new(vec![0, 7, 19, 101, 90_000 * SECOND]));
    let cores = move || ReadsTheHostClock {
        inner: reference(8 * SECOND),
        clock,
    };
    let plan = ReplayPlan {
        clock,
        simulated_per_wall: 1,
        chunks: vec![SECOND],
        resume_every: 5,
    };
    let failure = replay_gate(ReplayGate::Pace, &case, &cores, &plan);
    assert!(
        matches!(failure, Err(ReplayFailure::Diverged { .. })),
        "{failure:?}"
    );
}

#[test]
fn a_live_pace_with_pauses_and_a_fast_forward_gives_the_same_bits() {
    let case = busy();
    let clock = script();
    let evidence = replay_gate(ReplayGate::Pace, &case, &|| reference(8 * SECOND), &plan(&clock));
    let evidence = evidence.expect("the gate");
    assert_eq!(evidence.clock(), Some(ClockSource::Scripted));
    assert_eq!(evidence.runs().len(), 3);
}

#[test]
fn a_clock_that_stops_before_the_end_is_missing_evidence() {
    let case = busy();
    let clock = ScriptedClock::new(vec![0, 5 * SECOND]);
    let failure = replay_gate(ReplayGate::Pace, &case, &|| reference(8 * SECOND), &plan(&clock));
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::Pace
        })
    );
}

#[test]
fn a_plan_that_compares_nothing_is_missing_evidence() {
    let case = busy();
    let cores = || reference(8 * SECOND);
    let clock = script();
    for (gate, chunks, resume_every) in [
        (ReplayGate::Pace, Vec::new(), 5),
        (ReplayGate::Pace, vec![SECOND, 0], 5),
        (ReplayGate::Resume, vec![SECOND], 0),
    ] {
        let plan = ReplayPlan {
            clock: &clock,
            simulated_per_wall: 1,
            chunks,
            resume_every,
        };
        let failure = replay_gate(gate, &case, &cores, &plan);
        assert_eq!(failure.err(), Some(ReplayFailure::MissingEvidence { gate }));
    }
    let silent = delivering(Vec::new(), 100);
    let clock = script();
    let failure = replay_gate(ReplayGate::LateAdmission, &silent, &cores, &plan(&clock));
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::LateAdmission
        })
    );
}

type Seeing = &'static RefCell<Vec<Seen>>;

fn reader(span_ns: u64, refuses_over_ns: u64, seen: Seeing) -> ReadsPastTheBreakpoint {
    ReadsPastTheBreakpoint {
        inner: reference(8 * SECOND),
        span_ns,
        refuses_over_ns,
        seen,
    }
}

fn tickets_until_the_fault(
    case: &TwoReservoirs,
    span_ns: u64,
    refuses_over_ns: u64,
) -> (Vec<Seen>, EngineFault) {
    let seen = shared(RefCell::default());
    let core = reader(span_ns, refuses_over_ns, seen);
    let mut attempt = case.admit(&core).expect("admitted").step();
    loop {
        attempt = match attempt {
            Step::Accepted(..) => panic!("no fault"),
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => return (seen.take(), *stopped.fault()),
        };
    }
}

#[test]
fn a_span_of_any_size_reads_no_breakpoint_while_it_does_not_pass() {
    let known = delivering(vec![delivery(b"input", 2_000, 6_000)], 12_000);
    let quiet = delivering(Vec::new(), 12_000);
    let (span, refuses_over) = (1 << 50, 8 * SECOND);
    let (read, fault) = tickets_until_the_fault(&known, span, refuses_over);
    assert_eq!(
        read,
        tickets_until_the_fault(&quiet, span, refuses_over).0,
        "the same tickets, bit for bit"
    );
    assert_eq!(read.len(), 13, "the core's whole budget");
    for (halvings, seen) in read.iter().enumerate() {
        assert_eq!(seen.interval.length_ns(), span >> halvings);
    }
    assert_eq!(fault.kind(), EngineFaultKind::NoAdmissibleStep);
}

#[test]
fn a_ticket_shows_no_waiting_input_until_a_span_past_it_has_passed() {
    let run = |case: &TwoReservoirs| -> Vec<Seen> {
        let seen = shared(RefCell::default());
        let core = ReadsPastTheBreakpoint {
            inner: refused_past_the_input(),
            span_ns: 16 * SECOND,
            refuses_over_ns: u64::MAX,
            seen,
        };
        to(case.admit(&core).expect("admitted"), at(18));
        seen.take()
    };
    let known = run(&delivering(vec![delivery(b"input", 18, 118)], 60));
    let quiet = run(&delivering(Vec::new(), 60));
    assert_eq!(known[..quiet.len()], quiet, "bit for bit, digests included");
    assert_eq!(known.len(), quiet.len() + 1);
    let (passed, cut) = (quiet[quiet.len() - 1], known[quiet.len()]);
    assert!(
        passed.interval.end() > at(18),
        "the last ticket both runs saw crossed the input"
    );
    assert_eq!(cut.interval.end(), at(18), "the first ticket to show it");
    assert_eq!(
        cut.attempt, passed.attempt,
        "after that span passed, with no refusal between"
    );
}

#[test]
fn a_core_that_tries_to_read_past_the_breakpoint_fails_late_admission() {
    let case = delivering(vec![delivery(b"input", 2_000, 6_000)], 12_000);
    let clock = script();
    let refused = || ReadsPastTheBreakpoint {
        inner: reference(8 * SECOND),
        span_ns: 1 << 50,
        refuses_over_ns: 8 * SECOND,
        seen: shared(RefCell::default()),
    };
    let failure = replay_gate(ReplayGate::LateAdmission, &case, &refused, &plan(&clock));
    assert!(
        matches!(failure, Err(ReplayFailure::Faulted { gate: ReplayGate::LateAdmission, fault })
            if fault.kind() == EngineFaultKind::NoAdmissibleStep),
        "a span it refuses reads nothing and ends the run: {failure:?}"
    );
    let passed = || ReadsPastTheBreakpoint {
        inner: reference(8 * SECOND),
        span_ns: 1 << 50,
        refuses_over_ns: u64::MAX,
        seen: shared(RefCell::default()),
    };
    let failure = replay_gate(ReplayGate::LateAdmission, &case, &passed, &plan(&clock));
    assert!(
        matches!(failure, Err(ReplayFailure::NoLateBoundary { .. })),
        "a span the record would accept leaves no head before the input: {failure:?}"
    );
}

fn at_rest_with(deliveries: Vec<Delivery>) -> TwoReservoirs {
    let case = TwoReservoirs::delivering(at_rest(), deliveries).expect("a body");
    case.until(at(12_000))
}

#[test]
fn a_cut_shows_only_after_its_span_has_passed() {
    let span = 1 << 50;
    let seen = shared(RefCell::default());
    let core = reader(span, u64::MAX, seen);
    let case = at_rest_with(vec![delivery(b"input", 2_000, 6_000)]);
    let session = advance(case.admit(&core).expect("admitted"));
    assert_eq!(session.record().accepted().time(), at(2_000));
    let read = seen.take();
    let lengths: Vec<u64> = read.iter().map(|seen| seen.interval.length_ns()).collect();
    assert_eq!(lengths, [span, 2_000 * SECOND]);
    assert_eq!((read[0].attempt, read[1].attempt), (0, 0));
    let quiet = at_rest_with(Vec::new());
    let session = advance(quiet.admit(&core).expect("admitted"));
    assert_eq!(session.record().accepted().time(), SimTime::from_ns(span));
    assert_eq!(seen.take().len(), 1, "with nothing waiting no second solve");
}

#[test]
fn a_core_that_acts_on_a_cut_it_read_has_no_late_boundary() {
    let case = at_rest_with(vec![delivery(b"input", 2_000, 6_000)]);
    let cores = || ActsOnACut {
        span_ns: 1 << 50,
        read: Cell::new(None),
    };
    let clock = script();
    let failure = replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock));
    assert!(
        matches!(failure, Err(ReplayFailure::NoLateBoundary { .. })),
        "{failure:?}"
    );
}

#[test]
fn a_tickets_digest_is_blind_to_a_waiting_input() {
    let cores = || KeepsTheDigest {
        interval_ns: 8 * SECOND,
    };
    let case = delivering(vec![delivery(b"input", 5_000, 5_100)], 6_000);
    let clock = script();
    replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock)).expect("a digest reads no waiter");
}

fn refusing(cut: u64, seen: Seeing) -> RefusesOne {
    RefusesOne {
        inner: reference(8 * SECOND),
        interval: Interval::new(at(0), at(cut)).expect("an interval"),
        later_ns: 8 * SECOND,
        seen,
    }
}

#[test]
fn the_retry_of_a_cut_halves_the_span_and_not_the_cut() {
    for (cut, retry) in [(5, 4), (7, 4), (3, 2)] {
        let seen = shared(RefCell::default());
        let core = refusing(cut, seen);
        let case = delivering(vec![delivery(b"input", cut, 100)], 200);
        let session = advance(case.admit(&core).expect("admitted"));
        let read = seen.take();
        let lengths: Vec<u64> = read
            .iter()
            .map(|seen| seen.interval.length_ns() / SECOND)
            .collect();
        assert_eq!(lengths, [8, cut, retry], "a cut at {cut} s");
        assert_eq!(read[2].attempt, 1);
        assert_eq!(session.record().accepted().time(), at(retry));
    }
}

#[test]
fn a_refused_cut_leaves_a_fence_and_the_stamp_behind_it_is_refused() {
    let seen = shared(RefCell::default());
    let core = refusing(6, seen);
    let case = delivering(vec![delivery(b"input", 6, 100)], 200);
    let mut session = advance(case.admit(&core).expect("admitted"));
    assert_eq!(session.record().accepted().time(), at(4));
    assert_eq!(session.record().earliest_stamp(), at(6));
    let behind = water(b"behind", at(5), at(100), 1.0);
    let command = Digest::of_canonical(b"a command");
    let behind = AuthorizedInput::from_command_receipt(command, behind);
    assert_eq!(
        session.enqueue(behind).err(),
        Some(AdmissionError::EventBeforeFence {
            id: InputId::new(Digest::of_canonical(b"behind")),
            fence: at(6),
        })
    );
    session
        .enqueue(authorized(b"at the fence", 6, 100))
        .expect("a stamp at the fence");
}

#[test]
fn a_fence_stands_until_the_head_passes_it() {
    let core = RefusesOne {
        later_ns: SECOND,
        ..refusing(6, shared(RefCell::default()))
    };
    let case = delivering(vec![delivery(b"input", 6, 100)], 200);
    let session = advance(advance(case.admit(&core).expect("admitted")));
    assert_eq!(session.record().accepted().time(), at(5));
    assert_eq!(
        session.record().earliest_stamp(),
        at(6),
        "a later step does not lower it"
    );
    let session = advance(session);
    assert_eq!(session.record().earliest_stamp(), at(6));
    let session = advance(session);
    assert_eq!(session.record().accepted().time(), at(7));
    assert_eq!(session.record().earliest_stamp(), at(7), "the head has passed it");
}

#[test]
fn a_late_input_at_the_fence_matches_the_same_input_known_from_the_start() {
    let seen = shared(RefCell::default());
    let core = refusing(6, seen);
    let late = delivering(vec![delivery(b"input", 6, 100)], 40);
    let known = delivering(
        vec![delivery(b"input", 6, 100), delivery(b"at the fence", 6, 100)],
        40,
    );
    let mut session = advance(late.admit(&core).expect("admitted"));
    session
        .enqueue(authorized(b"at the fence", 6, 100))
        .expect("admitted");
    let rows = seals(session, known.end());
    let all = seals(known.admit(&core).expect("admitted"), known.end());
    assert_eq!(rows, all[1..]);
}

#[test]
fn a_cut_that_publishes_leaves_the_head_as_the_earliest_stamp() {
    let case = delivering(vec![delivery(b"input", 5, 100)], 200);
    let core = reference(8 * SECOND);
    let session = advance(case.admit(&core).expect("admitted"));
    assert_eq!(session.record().accepted().time(), at(5));
    assert_eq!(session.record().earliest_stamp(), at(5));
}

#[test]
fn enqueue_refuses_a_stamp_before_the_head_and_moves_nothing_it_takes() {
    let case = delivering(Vec::new(), 100);
    let core = reference(8 * SECOND);
    let mut session = to(case.admit(&core).expect("admitted"), at(16));
    let before = session.record().digests();
    let seal = session.record().seal();
    assert_eq!(
        session.enqueue(authorized(b"back-dated", 15, 50)).err(),
        Some(AdmissionError::EventBeforeHead {
            id: InputId::new(Digest::of_canonical(b"back-dated")),
        })
    );
    assert_eq!(session.record().digests(), before, "a refusal moves nothing");
    session
        .enqueue(authorized(b"at the head", 16, 50))
        .expect("admitted");
    let after = session.record().digests();
    assert_ne!(after.waiting, before.waiting);
    assert_eq!(
        RecordDigests {
            waiting: before.waiting,
            ..after
        },
        before
    );
    assert_eq!(session.record().seal(), seal);
    assert_eq!(session.record().revision(), before.revision);
}

#[test]
fn enqueue_refuses_a_stamp_at_the_admission_instant_whose_drive_was_already_read() {
    let case = delivering(Vec::new(), 100);
    let core = reference(8 * SECOND);
    let mut session = case.admit(&core).expect("admitted");
    assert_eq!(session.record().earliest_stamp(), SimTime::from_ns(1));
    assert_eq!(
        session.enqueue(authorized(b"at admission", 0, 50)).err(),
        Some(AdmissionError::EventBeforeFence {
            id: InputId::new(Digest::of_canonical(b"at admission")),
            fence: SimTime::from_ns(1),
        })
    );
    let just_after = water(b"just after", SimTime::from_ns(1), at(50), 1.0);
    let command = Digest::of_canonical(b"a command");
    let just_after = AuthorizedInput::from_command_receipt(command, just_after);
    session.enqueue(just_after).expect("the first legal stamp");
}

fn total_received(session: &Session<'_, Running>, case: &TwoReservoirs) -> f64 {
    let column = case.exterior().expect("an open body");
    let view = session.record().conservation();
    let segment = view.segments().next_back().expect("a segment");
    let (forward, _) = segment.exchange(column);
    let total = forward
        .on(segment.interval())
        .expect("the segment's own interval")[1];
    f64::from_bits(total.bits()[0])
}

#[test]
fn enqueue_is_idempotent_on_id_and_payload_even_after_the_input_applied() {
    let case = delivering(Vec::new(), 100);
    let core = reference(8 * SECOND);
    let mut session = to(case.admit(&core).expect("admitted"), at(16));
    let receipt = session.enqueue(authorized(b"input", 20, 40)).expect("admitted");
    assert_eq!(receipt.id(), InputId::new(Digest::of_canonical(b"input")));
    assert_eq!(receipt.at(), at(20));
    let again = session
        .enqueue(authorized(b"input", 20, 40))
        .expect("the same request");
    assert_eq!(again, receipt);
    let mut session = to(session, at(48));
    let other_command = Digest::of_canonical(b"another command");
    let event = water(b"input", at(20), at(40), 100.0);
    let repeat = AuthorizedInput::from_command_receipt(other_command, event);
    assert_eq!(repeat.command(), other_command);
    assert_eq!(
        session.enqueue(repeat),
        Ok(receipt),
        "after it applied, under another command"
    );
    let received = total_received(&to(session, at(60)), &case);
    assert!((received - 100.0).abs() < 1e-9, "{received}");
}

#[test]
fn enqueue_takes_the_checks_admission_takes() {
    let case = delivering(vec![delivery(b"known", 40, 50)], 100);
    let core = reference(8 * SECOND);
    let mut session = to(case.admit(&core).expect("admitted"), at(16));
    let id = |name: &[u8]| InputId::new(Digest::of_canonical(name));
    let command = Digest::of_canonical(b"a command");
    let input = |event| AuthorizedInput::from_command_receipt(command, event);
    let unknown = BoundaryEvent::new(
        id(b"unknown"),
        at(20),
        BoundaryInput::Water(WaterDelivery {
            boundary: "nowhere",
            amount: measure(1.0),
            ends: at(30),
        }),
    );
    for (event, refusal) in [
        (
            water(b"known", at(40), at(55), 100.0),
            AdmissionError::InputConflict { id: id(b"known") },
        ),
        (
            water(b"empty", at(30), at(30), 1.0),
            AdmissionError::EmptyDelivery { id: id(b"empty") },
        ),
        (
            water(b"negative", at(30), at(40), -1.0),
            AdmissionError::NegativeDelivery { id: id(b"negative") },
        ),
        (
            water(b"unbounded", at(30), SimTime::from_ns(30 * SECOND + 1), f64::MAX),
            AdmissionError::UnboundedDelivery { id: id(b"unbounded") },
        ),
        (unknown, AdmissionError::UnknownBoundary { name: "nowhere" }),
    ] {
        assert_eq!(session.enqueue(input(event)).err(), Some(refusal));
    }
}

fn saved_at<'m>(
    session: Session<'m, Running>,
    head: SimTime,
    store: &mut Store,
) -> (Session<'m, Running>, Checkpoint) {
    let session = to(session, head);
    let saved = session.checkpoint(store).expect("saved");
    (session, saved)
}

fn stored(saved: &Checkpoint) -> Checkpoint {
    Checkpoint::from_bytes(saved.as_bytes().to_vec())
}

#[test]
fn stored_bytes_resume_into_a_fresh_core_at_the_same_seal() {
    let case = busy();
    let core = reference(8 * SECOND);
    let fresh = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(40), &mut store);
    let identity = session.record().identity();
    let resumed = Session::resume(&stored(&saved), identity, case.body(), &fresh, &store).expect("resumed");
    assert_eq!(resumed.record().seal(), session.record().seal());
    assert_eq!(resumed.record().digests(), session.record().digests());
    assert_eq!(resumed.record().revision(), session.record().revision());
    let view = resumed.record().conservation();
    let parent = session.record().conservation();
    assert_eq!(view.span(), parent.span());
    assert_eq!(view.seal(), parent.seal());
    assert_eq!(seals(resumed, case.end()), seals(session, case.end()));
}

#[test]
fn a_checkpoint_at_admission_resumes_with_no_history() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let session = case.admit(&core).expect("admitted");
    let saved = session.checkpoint(&mut store).expect("saved");
    assert!(store.links.is_empty());
    let identity = session.record().identity();
    let resumed = Session::resume(&saved, identity, case.body(), &core, &store).expect("resumed");
    assert_eq!(resumed.record().digests(), session.record().digests());
}

#[test]
fn a_checkpoint_does_not_depend_on_when_it_was_taken() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, first) = saved_at(case.admit(&core).expect("admitted"), at(40), &mut store);
    let second = session.checkpoint(&mut Store::default()).expect("saved");
    assert_eq!(first.as_bytes(), second.as_bytes());
    let identity = session.record().identity();
    let resumed = Session::resume(&first, identity, case.body(), &core, &store).expect("resumed");
    let again = resumed.checkpoint(&mut Store::default()).expect("saved");
    assert_eq!(
        again.as_bytes(),
        first.as_bytes(),
        "a resumed run saves the bytes it came from"
    );
}

#[test]
fn a_waiting_input_survives_a_resume() {
    let case = delivering(Vec::new(), 100);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let mut session = to(case.admit(&core).expect("admitted"), at(16));
    session.enqueue(authorized(b"input", 40, 60)).expect("admitted");
    let saved = session.checkpoint(&mut store).expect("saved");
    let identity = session.record().identity();
    let resumed = Session::resume(&saved, identity, case.body(), &core, &store).expect("resumed");
    assert_eq!(resumed.record().digests(), session.record().digests());
    assert_eq!(seals(resumed, case.end()), seals(session, case.end()));
}

#[test]
fn a_delivery_ending_at_the_head_survives_a_resume() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 100);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(16), &mut store);
    assert_eq!(
        session.record().accepted().time(),
        at(16),
        "the delivery ended at this head"
    );
    let identity = session.record().identity();
    let resumed = Session::resume(&saved, identity, case.body(), &core, &store).expect("resumed");
    let (resumed, session) = (advance(resumed), advance(session));
    assert_eq!(resumed.record().digests(), session.record().digests());
}

#[test]
fn a_fence_survives_a_resume() {
    let seen = shared(RefCell::default());
    let core = refusing(6, seen);
    let case = delivering(vec![delivery(b"input", 6, 100)], 200);
    let mut store = Store::default();
    let session = advance(case.admit(&core).expect("admitted"));
    let saved = session.checkpoint(&mut store).expect("saved");
    let identity = session.record().identity();
    let resumed = Session::resume(&saved, identity, case.body(), &core, &store).expect("resumed");
    assert_eq!(resumed.record().earliest_stamp(), at(6));
}

const RAW_DIGEST: core::ops::Range<usize> = 8..40;
const HEADER: usize = 128;

fn resealed(mut bytes: Vec<u8>) -> Checkpoint {
    let raw = Digest::of_canonical(&bytes[RAW_DIGEST.end..]);
    bytes[RAW_DIGEST].copy_from_slice(raw.as_bytes());
    Checkpoint::from_bytes(bytes)
}

/// A checkpoint with nothing waiting, past every input, so every accepted part it holds is under its seal.
fn past_every_input<'m>(
    session: Session<'m, Running>,
    case: &TwoReservoirs,
    store: &mut Store,
) -> (Session<'m, Running>, Checkpoint) {
    saved_at(session, case.end(), store)
}

#[test]
fn corrupted_bytes_are_refused_before_any_body_byte_is_decoded() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    for position in 0..saved.as_bytes().len() {
        let mut bytes = saved.as_bytes().to_vec();
        bytes[position] ^= 0x10;
        let bytes = Checkpoint::from_bytes(bytes);
        let refused = Session::resume(&bytes, identity, case.body(), &core, &store).err();
        assert!(
            matches!(
                refused,
                Some(
                    CheckpointError::Corrupt
                        | CheckpointError::UnknownFormat
                        | CheckpointError::Truncated
                        | CheckpointError::TrailingContent
                )
            ),
            "a flip at {position}: {refused:?}"
        );
    }
}

#[test]
fn each_header_refusal_is_named() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let refused = |bytes: Vec<u8>| {
        let saved = Checkpoint::from_bytes(bytes);
        Session::resume(&saved, identity, case.body(), &core, &store).err()
    };
    let bytes = saved.as_bytes();
    assert_eq!(
        refused(bytes[..HEADER - 1].to_vec()),
        Some(CheckpointError::Truncated)
    );
    assert_eq!(
        refused(bytes[..bytes.len() - 1].to_vec()),
        Some(CheckpointError::Truncated)
    );
    let mut long = bytes.to_vec();
    long.push(0);
    assert_eq!(refused(long), Some(CheckpointError::TrailingContent));
    let mut foreign = bytes.to_vec();
    foreign[0] ^= 1;
    assert_eq!(refused(foreign), Some(CheckpointError::UnknownFormat));
    let mut flipped = bytes.to_vec();
    flipped[HEADER] ^= 1;
    assert_eq!(refused(flipped), Some(CheckpointError::Corrupt));
    let mut unstated = bytes.to_vec();
    unstated[HEADER - 1] = 2;
    assert_eq!(
        Session::resume(&resealed(unstated), identity, case.body(), &core, &store).err(),
        Some(CheckpointError::UnknownFormat)
    );
    let huge = u64::MAX.to_be_bytes();
    let mut stated = bytes.to_vec();
    stated[40..48].copy_from_slice(&huge);
    assert_eq!(refused(stated), Some(CheckpointError::Truncated));
}

#[test]
fn bytes_of_another_run_body_or_core_are_refused_before_their_body() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let other_interval = reference(4 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let mismatch = Some(CheckpointError::IdentityMismatch);
    let other = other_interval.resolution();
    let other_run = RunIdentity::new(
        identity.model(),
        identity.packet(),
        Seed::new([9, 9, 9, 9]),
        other,
    );
    let wrong_core = Session::resume(&saved, identity, case.body(), &other_interval, &store);
    assert_eq!(wrong_core.err(), mismatch);
    let wrong_run = Session::resume(&saved, &other_run, case.body(), &other_interval, &store);
    assert_eq!(wrong_run.err(), mismatch);
    let closed = TwoReservoirs::closed().expect("a body");
    let wrong_body = Session::resume(&saved, identity, closed.body(), &core, &store);
    assert_eq!(wrong_body.err(), mismatch);
    let seeded = RunIdentity::new(
        identity.model(),
        identity.packet(),
        Seed::new([9, 9, 9, 9]),
        core.resolution(),
    );
    let wrong_seed = Session::resume(&saved, &seeded, case.body(), &core, &store);
    assert_eq!(wrong_seed.err(), mismatch);
}

#[test]
fn a_well_sealed_malformed_body_is_refused_and_never_panics() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let (digests, earliest) = (session.record().digests(), session.record().earliest_stamp());
    let bytes = saved.as_bytes();
    let image = u64::from_be_bytes(*bytes[HEADER..].first_chunk().expect("a length")) as usize;
    let branch = HEADER + 8 + image + 8;
    for position in HEADER..bytes.len() {
        for flip in [0x01, 0x80] {
            let mut flipped = bytes.to_vec();
            flipped[position] ^= flip;
            match Session::resume(&resealed(flipped), identity, case.body(), &core, &store) {
                Ok(session) => {
                    assert!(
                        (branch..branch + 40).contains(&position),
                        "a flip at {position} loaded, and only a branch's name is under no seal"
                    );
                    assert_eq!(session.record().digests(), digests);
                    assert_eq!(session.record().earliest_stamp(), earliest);
                }
                Err(
                    CheckpointError::Malformed
                    | CheckpointError::SealMismatch
                    | CheckpointError::HistoryMissing { .. }
                    | CheckpointError::HistoryAltered { .. },
                ) => {}
                Err(other) => panic!("a flip at {position}: {other:?}"),
            }
        }
    }
}

#[test]
fn a_bound_value_changed_under_a_good_digest_is_refused() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let mut bytes = saved.as_bytes().to_vec();
    let image = u64::from_be_bytes(*bytes[HEADER..].first_chunk().expect("a length")) as usize;
    let value = HEADER + 8 + image - 8;
    bytes[value] ^= 1;
    let refused = Session::resume(&resealed(bytes), identity, case.body(), &core, &store).err();
    assert!(
        matches!(refused, Some(CheckpointError::HistoryAltered { .. })),
        "{refused:?}"
    );
}

#[test]
fn arbitrary_bodies_are_refused_and_never_panic() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let header = &saved.as_bytes()[..HEADER];
    let mut word = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = || {
        word ^= word << 13;
        word ^= word >> 7;
        word ^= word << 17;
        word
    };
    for length in (0..4_000).step_by(37) {
        let mut bytes = header.to_vec();
        bytes[40..48].copy_from_slice(&(length as u64).to_be_bytes());
        for _ in 0..length {
            bytes.push(next() as u8);
        }
        let refused = Session::resume(&resealed(bytes), identity, case.body(), &core, &store).err();
        assert_eq!(refused, Some(CheckpointError::Malformed));
    }
}

fn with_stream_ordinals(saved: &Checkpoint, ordinals: [(u8, u64); 2]) -> Checkpoint {
    let bytes = saved.as_bytes();
    let image = u64::from_be_bytes(*bytes[HEADER..].first_chunk().expect("a length")) as usize;
    let count_at = HEADER + 8 + image;
    let mut out = bytes[..count_at].to_vec();
    out.extend_from_slice(&2_u64.to_be_bytes());
    for (key, ordinal) in ordinals {
        out.extend_from_slice(&[key; 32]);
        out.extend_from_slice(&ordinal.to_be_bytes());
    }
    out.extend_from_slice(&bytes[count_at + 8..]);
    let body = (out.len() - HEADER) as u64;
    out[40..48].copy_from_slice(&body.to_be_bytes());
    resealed(out)
}

#[test]
fn stream_ordinals_read_back_under_the_seal_and_only_in_order() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let body = case.body();
    let ordered = with_stream_ordinals(&saved, [(1, 5), (2, 6)]);
    assert_eq!(
        Session::resume(&ordered, identity, body, &core, &store).err(),
        Some(CheckpointError::SealMismatch),
        "well formed, and the seal covers it"
    );
    let unordered = with_stream_ordinals(&saved, [(2, 6), (1, 5)]);
    assert_eq!(
        Session::resume(&unordered, identity, body, &core, &store).err(),
        Some(CheckpointError::Malformed)
    );
}

#[test]
fn a_missing_or_altered_link_is_refused_by_its_seal() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let identity = session.record().identity();
    let body = case.body();
    let (seal, link) = store.links[0].clone();
    assert_eq!(
        Session::resume(&saved, identity, body, &core, &store.without(seal)).err(),
        Some(CheckpointError::HistoryMissing { seal })
    );
    let altered = Some(CheckpointError::HistoryAltered { seal });
    for (position, length) in [(70, link.len()), (40, link.len()), (0, link.len()), (70, 70)] {
        let mut changed = store.clone();
        let mut bytes = link.clone();
        bytes[position] ^= 1;
        bytes.truncate(length);
        changed.replace(seal, bytes);
        assert_eq!(
            Session::resume(&saved, identity, body, &core, &changed).err(),
            altered
        );
    }
    for short in [31, 63] {
        let mut changed = store.clone();
        changed.replace(seal, vec![0; short]);
        assert_eq!(
            Session::resume(&saved, identity, body, &core, &changed).err(),
            altered
        );
    }
}

fn relinked(store: &mut Store, saved: &Checkpoint, image: &[u8], chunk: &[u8]) -> (Digest, Checkpoint) {
    let (_, link) = store.links[0].clone();
    let revision = u64::from_be_bytes(*saved.as_bytes()[80..].first_chunk().expect("a revision"));
    let mut chained = link[..32].to_vec();
    chained.extend_from_slice(&revision.to_be_bytes());
    chained.extend_from_slice(image);
    chained.extend_from_slice(Digest::of_canonical(chunk).as_bytes());
    let seal = Digest::of_canonical(&chained);
    let stored = [&link[..32], image, chunk].concat();
    store.replace(seal, stored);
    let mut named = saved.as_bytes().to_vec();
    named[88..120].copy_from_slice(seal.as_bytes());
    (seal, resealed(named))
}

#[test]
fn a_link_that_chains_true_and_holds_no_chunk_is_altered() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 16);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(8), &mut store);
    let identity = session.record().identity();
    let (image, good) = (store.links[0].1[32..64].to_vec(), store.links[0].1[64..].to_vec());
    let mut long = good.clone();
    long.push(0);
    let empty_interval: Vec<u8> = [&good[8..16], &good[8..16], &good[16..]].concat();
    let mut not_finite = good.clone();
    not_finite[16..24].copy_from_slice(&f64::INFINITY.to_bits().to_be_bytes());
    for chunk in [Vec::new(), long, empty_interval, not_finite] {
        let (seal, named) = relinked(&mut store, &saved, &image, &chunk);
        assert_eq!(
            Session::resume(&named, identity, case.body(), &core, &store).err(),
            Some(CheckpointError::HistoryAltered { seal })
        );
    }
}

#[test]
fn a_chain_that_records_another_image_for_the_head_is_a_seal_mismatch() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 16);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(8), &mut store);
    let identity = session.record().identity();
    let good = store.links[0].1[64..].to_vec();
    let (_, named) = relinked(&mut store, &saved, &[7; 32], &good);
    assert_eq!(
        Session::resume(&named, identity, case.body(), &core, &store).err(),
        Some(CheckpointError::SealMismatch)
    );
}

#[test]
fn a_chain_that_misses_the_opening_is_refused() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 16);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(8), &mut store);
    let identity = session.record().identity();
    let mut bytes = saved.as_bytes().to_vec();
    let opening_end = bytes.len() - 32;
    bytes[opening_end - 1] ^= 1;
    let refused = Session::resume(&resealed(bytes), identity, case.body(), &core, &store).err();
    assert!(
        matches!(refused, Some(CheckpointError::HistoryAltered { .. })),
        "{refused:?}"
    );
}

#[test]
fn a_store_that_cannot_keep_a_link_leaves_no_checkpoint() {
    let case = busy();
    let core = reference(8 * SECOND);
    let session = to(case.admit(&core).expect("admitted"), at(8));
    let refused = session.checkpoint(&mut RefusesToKeep);
    assert!(matches!(refused.err(), Some(CheckpointError::StoreFailed { .. })));
}

#[test]
fn a_store_keeps_each_link_once_oldest_first() {
    struct Counting {
        kept: Vec<Digest>,
    }
    impl HistoryStore for Counting {
        fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
            self.kept.contains(seal).then(Vec::new)
        }
        fn keep(&mut self, seal: Digest, _link: Vec<u8>) -> Result<(), CheckpointError> {
            self.kept.push(seal);
            Ok(())
        }
    }
    let case = delivering(Vec::new(), 40);
    let core = reference(8 * SECOND);
    let mut store = Counting { kept: Vec::new() };
    let session = to(case.admit(&core).expect("admitted"), at(16));
    session.checkpoint(&mut store).expect("saved");
    assert_eq!(store.kept.len(), 2);
    let session = to(session, at(40));
    session.checkpoint(&mut store).expect("saved");
    assert_eq!(store.kept.len(), 5, "the first two were not kept again");
    let mut fresh = Store::default();
    session.checkpoint(&mut fresh).expect("saved");
    for pair in store.kept.windows(2) {
        let link = fresh.resolve(&pair[1]).expect("a link");
        assert_eq!(
            link[..32],
            *pair[0].as_bytes(),
            "each link follows the one before"
        );
    }
}

#[test]
fn a_rewind_after_the_parent_ran_ahead_leaves_the_parent_and_shares_its_seals() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(40), &mut store);
    let (fork_revision, fork_at) = (session.record().revision(), session.record().accepted().time());
    let parent = to(session, at(80));
    let before = parent.record().digests();
    let branch = BranchId::new(Digest::of_canonical(b"try again"));
    let fork = parent.rewind(&saved, branch).expect("a fork");
    assert_eq!(fork.record().accepted().time(), fork_at);
    assert_eq!(fork.record().revision(), fork_revision);
    assert_eq!(parent.record().digests(), before, "the parent stays as it is");
    let ancestry = fork.record().ancestry();
    assert_eq!(ancestry.len(), 1);
    assert_eq!(ancestry[0].parent(), parent.record().conservation().seal().branch);
    assert_eq!(
        (ancestry[0].revision(), ancestry[0].at()),
        (fork_revision, fork_at)
    );
    assert_eq!(
        fork.record().conservation().seal().branch,
        Digest::of_canonical(b"try again")
    );
    let fork = to(fork, at(80));
    assert_eq!(
        fork.record().seal(),
        parent.record().seal(),
        "one course, one seal"
    );
    assert_eq!(fork.record().digests(), before);
    let saved_fork = fork.checkpoint(&mut store).expect("saved");
    let identity = fork.record().identity();
    let loaded = Session::resume(&saved_fork, identity, case.body(), &core, &store).expect("loaded");
    assert_eq!(loaded.record().ancestry(), fork.record().ancestry());
}

#[test]
fn a_rewind_refuses_a_branch_on_its_line_another_run_and_a_checkpoint_off_its_chain() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(40), &mut store);
    let own = session.record().conservation().seal().branch;
    assert_eq!(
        session.rewind(&saved, BranchId::new(own)).err(),
        Some(CheckpointError::BranchInUse)
    );
    let first = BranchId::new(Digest::of_canonical(b"first"));
    let fork = session.rewind(&saved, first).expect("a fork");
    assert_eq!(
        fork.rewind(&saved, first).err(),
        Some(CheckpointError::BranchInUse)
    );
    assert_eq!(
        fork.rewind(&saved, BranchId::new(own)).err(),
        Some(CheckpointError::BranchInUse),
        "an ancestor's id"
    );
    let other = delivering(vec![delivery(b"first", 30, 90)], 120);
    let foreign = reference(16 * SECOND);
    let mut foreign_store = Store::default();
    let (run, bytes) = saved_at(
        other.admit(&foreign).expect("admitted"),
        at(32),
        &mut foreign_store,
    );
    assert_eq!(
        session.rewind(&bytes, first).err(),
        Some(CheckpointError::IdentityMismatch)
    );
    assert_eq!(
        run.rewind(&saved, first).err(),
        Some(CheckpointError::IdentityMismatch)
    );
    let parted = delivering(vec![delivery(b"sibling", 16, 90)], 120);
    let (_, off_the_chain) = saved_at(parted.admit(&core).expect("admitted"), at(40), &mut store);
    assert_eq!(
        session.rewind(&off_the_chain, first).err(),
        Some(CheckpointError::NotOnThisBranch)
    );
}

#[test]
fn a_rewind_refuses_a_sibling_with_one_ledger_and_another_accepted_image() {
    let case = delivering(Vec::new(), 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = saved_at(case.admit(&core).expect("admitted"), at(8), &mut store);
    let branch = BranchId::new(Digest::of_canonical(b"sibling"));
    let mut sibling = session.rewind(&saved, branch).expect("a fork");
    let nothing = water(b"nothing", at(8), at(24), 0.0);
    let command = Digest::of_canonical(b"a command");
    sibling
        .enqueue(AuthorizedInput::from_command_receipt(command, nothing))
        .expect("admitted");
    let (sibling, parent) = (to(sibling, at(16)), to(session, at(16)));
    assert_eq!(
        sibling.record().digests().ledger,
        parent.record().digests().ledger
    );
    assert_ne!(sibling.record().seal(), parent.record().seal());
    let off_the_chain = sibling.checkpoint(&mut store).expect("saved");
    let other = BranchId::new(Digest::of_canonical(b"other"));
    assert_eq!(
        parent.rewind(&off_the_chain, other).err(),
        Some(CheckpointError::NotOnThisBranch)
    );
}

#[test]
fn a_fork_may_not_take_the_name_of_a_branch_it_came_from() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let session = to(case.admit(&core).expect("admitted"), at(16));
    let saved = session.checkpoint(&mut store).expect("saved");
    let first = BranchId::new(Digest::of_canonical(b"first"));
    let second = BranchId::new(Digest::of_canonical(b"second"));
    let middle = session.rewind(&saved, first).expect("a fork");
    let on_middle = middle.checkpoint(&mut store).expect("saved");
    let last = middle.rewind(&on_middle, second).expect("a fork of a fork");
    assert_eq!(last.record().ancestry().len(), 2);
    let on_last = last.checkpoint(&mut store).expect("saved");
    assert_eq!(
        last.rewind(&on_last, first).err(),
        Some(CheckpointError::BranchInUse)
    );
}

#[test]
fn an_opening_of_another_length_is_malformed_whatever_a_forged_chain_says() {
    let case = delivering(Vec::new(), 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let session = case.admit(&core).expect("admitted");
    let saved = session.checkpoint(&mut store).expect("saved");
    let identity = session.record().identity();
    let values = case.body().conserved_basis().combinations().len();
    let mut bytes = saved.as_bytes().to_vec();
    let count_at = bytes.len() - 32 - 16 * values - 8;
    bytes[count_at..count_at + 8].copy_from_slice(&(values as u64 - 1).to_be_bytes());
    bytes.drain(count_at + 8..count_at + 24);
    let body = (bytes.len() - HEADER) as u64;
    bytes[40..48].copy_from_slice(&body.to_be_bytes());
    assert_eq!(
        Session::resume(&resealed(bytes), identity, case.body(), &core, &store).err(),
        Some(CheckpointError::Malformed)
    );
}

#[test]
fn a_byte_left_over_after_the_parts_is_malformed() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let (session, saved) = past_every_input(case.admit(&core).expect("admitted"), &case, &mut store);
    let mut bytes = saved.as_bytes().to_vec();
    bytes.push(0);
    let body = (bytes.len() - HEADER) as u64;
    bytes[40..48].copy_from_slice(&body.to_be_bytes());
    let identity = session.record().identity();
    assert_eq!(
        Session::resume(&resealed(bytes), identity, case.body(), &core, &store).err(),
        Some(CheckpointError::Malformed)
    );
}

#[test]
fn a_fork_named_after_the_branch_it_saved_from_is_refused() {
    let case = busy();
    let core = reference(8 * SECOND);
    let mut store = Store::default();
    let session = to(case.admit(&core).expect("admitted"), at(16));
    let (fork_id, saved) = {
        let start = session.checkpoint(&mut store).expect("saved");
        let id = BranchId::new(Digest::of_canonical(b"twin"));
        let twin = to(session.rewind(&start, id).expect("a fork"), at(24));
        (id, twin.checkpoint(&mut store).expect("saved"))
    };
    let parent = to(session, at(24));
    assert_eq!(
        parent.rewind(&saved, fork_id).err(),
        Some(CheckpointError::BranchInUse),
        "the twin's course is on this chain and its id is taken"
    );
}

struct FaultsAt {
    inner: ReferenceIntegrator,
    at: SimTime,
}
impl CoupledSolver for FaultsAt {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() == self.at {
            return Err(ticket.mismatch().into());
        }
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn a_latched_session_exports_an_image_only_recover_continues() {
    let case = busy();
    let core = FaultsAt {
        inner: reference(8 * SECOND),
        at: at(16),
    };
    let mut store = Store::default();
    let session = to(case.admit(&core).expect("admitted"), at(16));
    let earlier = {
        let parked = session.checkpoint(&mut store).expect("saved");
        Checkpoint::from_bytes(parked.as_bytes().to_vec())
    };
    let Step::Faulted(stopped) = session.step() else {
        panic!("not faulted");
    };
    let report = stopped.export(&mut store).expect("exported");
    assert_eq!(report.fault().kind(), EngineFaultKind::BindingMismatch);
    assert_eq!(report.refusals(), stopped.refusals());
    let identity = stopped.record().identity();
    let body = case.body();
    assert_eq!(
        Session::resume(report.last_accepted(), identity, body, &core, &store).err(),
        Some(CheckpointError::Latched)
    );
    let branch = BranchId::new(Digest::of_canonical(b"recovered"));
    let recovered = stopped
        .recover(report.last_accepted(), branch)
        .expect("a new branch");
    assert_eq!(recovered.record().accepted().time(), at(16));
    assert_eq!(recovered.record().ancestry().len(), 1);
    let another = BranchId::new(Digest::of_canonical(b"another"));
    assert_eq!(
        recovered.rewind(report.last_accepted(), another).err(),
        Some(CheckpointError::Latched),
        "a running session on that chain goes back to the latch only through a fault"
    );
    assert_eq!(recovered.record().digests(), stopped.record().digests());
    let from_earlier = stopped.recover(&earlier, branch).expect("a new branch");
    assert_eq!(from_earlier.record().digests(), stopped.record().digests());
    assert_eq!(
        stopped
            .recover(
                &earlier,
                BranchId::new(stopped.record().conservation().seal().branch)
            )
            .err(),
        Some(CheckpointError::BranchInUse)
    );
}

fn save_resume_rewind<'m>(
    session: &Session<'m, Running>,
    body: &'m CompiledBody,
    solver: &'m dyn CoupledSolver,
    store: &mut dyn HistoryStore,
    fork: BranchId,
) -> Result<(Session<'m, Running>, Session<'m, Running>), CheckpointError> {
    let saved = session.checkpoint(store)?;
    let stored: Vec<u8> = saved.as_bytes().to_vec();
    let bytes = Checkpoint::from_bytes(stored);
    let resumed = Session::resume(&bytes, session.record().identity(), body, solver, store)?;
    let forked = session.rewind(&saved, fork)?;
    Ok((resumed, forked))
}

#[test]
fn the_contracts_call_site_saves_resumes_and_rewinds() {
    let case = busy();
    let core = reference(8 * SECOND);
    let fresh = reference(8 * SECOND);
    let mut store = Store::default();
    let session = to(case.admit(&core).expect("admitted"), at(40));
    let fork = BranchId::new(Digest::of_canonical(b"fork"));
    let (resumed, forked) =
        save_resume_rewind(&session, case.body(), &fresh, &mut store, fork).expect("the call site");
    assert_eq!(resumed.record().digests(), session.record().digests());
    assert_eq!(forked.record().digests(), session.record().digests());
}

#[test]
fn a_run_on_the_same_grid_diverges_at_the_first_seal_it_changed() {
    let case = delivering(Vec::new(), 40);
    let cores = || CountsInACell {
        interval_ns: 8 * SECOND,
        steps: Cell::new(0),
    };
    let clock = script();
    let plan = ReplayPlan {
        resume_every: 3,
        ..plan(&clock)
    };
    assert_eq!(
        replay_gate(ReplayGate::Resume, &case, &cores, &plan).err(),
        Some(ReplayFailure::Diverged {
            gate: ReplayGate::Resume,
            first_divergence: at(16),
        })
    );
}

#[test]
fn a_core_that_needs_a_mark_a_fresh_core_lacks_fails_rewind_at_the_load() {
    let case = delivering(Vec::new(), 40);
    let cores = || ShowsAt {
        inner: reference(8 * SECOND),
        at: at(16),
        primed: Cell::new(false),
    };
    let clock = script();
    assert_eq!(
        replay_gate(ReplayGate::Rewind, &case, &cores, &plan(&clock)).err(),
        Some(ReplayFailure::Diverged {
            gate: ReplayGate::Rewind,
            first_divergence: at(20),
        })
    );
}

#[test]
fn a_core_that_restarts_after_a_jump_fails_rewind_though_a_load_and_a_fork_agree() {
    let case = delivering(Vec::new(), 40);
    let cores = || RestartsAfterAJump {
        inner: reference(8 * SECOND),
        last: Cell::new(None),
        restarted: Cell::new(false),
    };
    let clock = script();
    assert_eq!(
        replay_gate(ReplayGate::Rewind, &case, &cores, &plan(&clock)).err(),
        Some(ReplayFailure::Diverged {
            gate: ReplayGate::Rewind,
            first_divergence: at(20),
        })
    );
}

fn counting(interval: u64) -> impl Fn() -> KeepsACache {
    move || KeepsACache {
        interval_ns: interval * SECOND,
        steps: Cell::new(0),
    }
}

#[test]
fn a_hidden_period_cannot_land_on_every_resume() {
    let case = delivering(Vec::new(), 100);
    for resume_every in [3, 1_000_000] {
        let clock = script();
        let plan = ReplayPlan {
            resume_every,
            ..plan(&clock)
        };
        let failure = replay_gate(ReplayGate::Resume, &case, &counting(12), &plan);
        assert!(
            matches!(failure, Err(ReplayFailure::Diverged { .. })),
            "{resume_every}"
        );
    }
}

#[test]
fn a_hidden_period_cannot_land_on_every_fork() {
    for end in (50..=200).step_by(2) {
        let case = delivering(vec![delivery(b"input", 40, 45)], end);
        let clock = script();
        let failure = replay_gate(ReplayGate::Rewind, &case, &counting(12), &plan(&clock));
        assert!(matches!(failure, Err(ReplayFailure::Diverged { .. })), "{end}");
    }
    let case = delivering(Vec::new(), 88);
    let clock = script();
    let failure = replay_gate(ReplayGate::Rewind, &case, &counting(12), &plan(&clock));
    assert!(matches!(failure, Err(ReplayFailure::Diverged { .. })));
}

#[test]
fn an_input_no_step_applies_has_no_late_boundary() {
    let id = InputId::new(Digest::of_canonical(b"input"));
    for stamp in [40, 200] {
        let case = delivering(vec![delivery(b"input", stamp, 300)], 40);
        let clock = script();
        let cores = || reference(8 * SECOND);
        let gate = replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock));
        assert_eq!(gate.err(), Some(ReplayFailure::NoLateBoundary { id }), "{stamp}");
    }
}

#[test]
fn a_pace_that_never_stops_a_run_short_of_its_end_is_missing_evidence() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let cores = || reference(8 * SECOND);
    let gate = ReplayGate::Pace;
    let missing = Some(ReplayFailure::MissingEvidence { gate });
    let clock = script();
    let straight_through = ReplayPlan {
        chunks: vec![3_000 * SECOND],
        ..plan(&clock)
    };
    assert_eq!(replay_gate(gate, &case, &cores, &straight_through).err(), missing);
    let clock = ScriptedClock::new(vec![3_000 * SECOND]);
    let one_reading = plan(&clock);
    assert_eq!(replay_gate(gate, &case, &cores, &one_reading).err(), missing);
}

#[test]
fn a_pace_that_moves_no_simulated_time_is_missing_evidence_before_it_reads_a_clock() {
    struct NeverRead;
    impl WallClock for NeverRead {
        fn elapsed_ns(&self) -> Option<u64> {
            panic!("this plan spins on its clock")
        }
        fn source(&self) -> ClockSource {
            ClockSource::Host
        }
    }
    let case = delivering(Vec::new(), 40);
    let plan = ReplayPlan {
        clock: &NeverRead,
        simulated_per_wall: 0,
        chunks: vec![SECOND],
        resume_every: 5,
    };
    let gate = ReplayGate::Pace;
    assert_eq!(
        replay_gate(gate, &case, &|| reference(8 * SECOND), &plan).err(),
        Some(ReplayFailure::MissingEvidence { gate })
    );
}

#[test]
fn every_gate_in_one_evidence_measured_one_run() {
    let case = delivering(vec![delivery(b"input", 40, 60)], 100);
    let calls = Cell::new(0);
    let cores = || {
        calls.set(calls.get() + 1);
        reference(if calls.get() <= 3 { 8 } else { 4 } * SECOND)
    };
    let clock = script();
    assert_eq!(
        replay_gates(&case, &cores, &plan(&clock)).err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::Resume
        })
    );
}

#[test]
fn a_lost_count_that_shows_steps_after_the_load_fails_resume() {
    let case = delivering(Vec::new(), 24);
    let cores = || RefreshesASixth {
        inner: reference(SECOND),
        calls: Cell::new(0),
    };
    let clock = script();
    let plan = ReplayPlan {
        resume_every: 1,
        ..plan(&clock)
    };
    let failure = replay_gate(ReplayGate::Resume, &case, &cores, &plan);
    assert_eq!(
        failure.err(),
        Some(ReplayFailure::Diverged {
            gate: ReplayGate::Resume,
            first_divergence: SimTime::from_ns(6 * SECOND + SECOND / 2),
        })
    );
}

#[test]
fn every_gate_in_one_evidence_measured_one_course() {
    let case = delivering(vec![delivery(b"nothing", 4, 6)], 10);
    let (one, two) = (MarksItsFirstSteps { marker: 1 }, MarksItsFirstSteps { marker: 2 });
    let (one, two) = (
        to(case.admit(&one).expect("admitted"), case.end()),
        to(case.admit(&two).expect("admitted"), case.end()),
    );
    assert_eq!(one.record().digests(), two.record().digests());
    assert_ne!(one.record().seal(), two.record().seal());
    let calls = Cell::new(0);
    let cores = || {
        calls.set(calls.get() + 1);
        MarksItsFirstSteps {
            marker: if calls.get() <= 3 { 1 } else { 2 },
        }
    };
    let clock = script();
    assert_eq!(
        replay_gates(&case, &cores, &plan(&clock)).err(),
        Some(ReplayFailure::MissingEvidence {
            gate: ReplayGate::Resume
        })
    );
}

/// `case`'s own run, admitted again at `instant`: the same identity from another start.
fn admitted_at<'m>(
    case: &'m TwoReservoirs,
    core: &'m dyn CoupledSolver,
    instant: SimTime,
    identity: &RunIdentity,
) -> Session<'m, Running> {
    let default = Binding::Network(Network::Vascular);
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    (packet.synthetic(
        ParameterKey::new("cv.e"),
        default,
        measure::<Elastance>(1.0),
        "synthetic",
    ))
    .expect("a value");
    (packet.synthetic(
        ParameterKey::new("cv.g"),
        default,
        measure::<Conductance<Water>>(0.001),
        "synthetic",
    ))
    .expect("a value");
    for (row, amount) in case.body().rows().iter().zip([500_500.0, 499_500.0]) {
        (packet.start(row.kind(), measure::<Water>(amount), "synthetic")).expect("a start");
    }
    let parameters = case.body().bind(packet).expect("bound");
    let port = BoundaryPort::new(Vec::new()).expect("a port");
    let request = InitializationRequest::consistent_instant(instant);
    let system = (case.body().initialization(parameters, port, request)).expect("a system");
    let solved = system.workspace();
    let start = MethodHistory::new(core.resolution().policy(), instant, Vec::new());
    let origin = system.admit(solved, start).expect("admitted");
    Session::admit(case.body(), core, origin, identity.clone()).expect("the same identity")
}

#[test]
fn a_rewind_refuses_the_admission_checkpoint_of_another_start() {
    let case = TwoReservoirs::closed().expect("a body");
    let core = reference(SECOND);
    let mut store = Store::default();
    let parent = case.admit(&core).expect("admitted");
    let own = parent.checkpoint(&mut store).expect("saved");
    let identity = parent.record().identity().clone();
    let foreign = admitted_at(&case, &core, at(100), &identity);
    assert_ne!(foreign.record().seal(), parent.record().seal());
    let foreign = foreign.checkpoint(&mut store).expect("saved");
    let parent = to(parent, at(2));
    let first = BranchId::new(Digest::of_canonical(b"own origin"));
    let second = BranchId::new(Digest::of_canonical(b"another origin"));
    assert_eq!(
        (parent.rewind(&own, first).expect("a fork").record().accepted()).time(),
        at(0)
    );
    assert_eq!(
        parent.rewind(&foreign, second).err(),
        Some(CheckpointError::NotOnThisBranch)
    );
}

#[test]
fn a_case_with_no_step_is_missing_evidence_for_every_gate() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 0);
    for gate in GATES {
        let clock = script();
        let failure = replay_gate(gate, &case, &|| reference(8 * SECOND), &plan(&clock));
        assert_eq!(failure.err(), Some(ReplayFailure::MissingEvidence { gate }));
    }
}

#[test]
fn a_rewind_on_a_case_of_one_step_still_compares_what_ran() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 8);
    let cores = || KeepsACache {
        interval_ns: 8 * SECOND,
        steps: Cell::new(0),
    };
    let clock = script();
    let failure = replay_gate(ReplayGate::Rewind, &case, &cores, &plan(&clock));
    assert!(
        matches!(failure, Err(ReplayFailure::Diverged { .. })),
        "{failure:?}"
    );
}

#[test]
fn a_cache_that_shows_only_at_a_deliverys_end_fails_resume() {
    let deliveries = vec![Delivery {
        id: b"nothing",
        from: at(12),
        to: at(48),
        amount: measure(0.0),
    }];
    let case = TwoReservoirs::delivering(at_rest(), deliveries).expect("a body");
    let case = case.until(at(80));
    let cores = || ShowsAt {
        inner: reference(4 * SECOND),
        at: at(48),
        primed: Cell::new(false),
    };
    let clock = script();
    let plan = ReplayPlan {
        resume_every: 100,
        ..plan(&clock)
    };
    let failure = replay_gate(ReplayGate::Resume, &case, &cores, &plan);
    assert!(
        matches!(
            failure,
            Err(ReplayFailure::Diverged {
                gate: ReplayGate::Resume,
                ..
            })
        ),
        "{failure:?}"
    );
}

#[test]
fn a_receipt_says_when_its_step_started_at_a_breakpoint() {
    let case = delivering(vec![delivery(b"input", 8, 16)], 40);
    let core = reference(8 * SECOND);
    let mut session = case.admit(&core).expect("admitted");
    let mut started_at_one = Vec::new();
    while session.record().accepted().time() < at(32) {
        let Step::Accepted(next, receipt) = session.step() else {
            panic!("not accepted");
        };
        started_at_one.push(receipt.after_breakpoint());
        session = next;
    }
    assert_eq!(started_at_one, [false, true, true, false]);
}

#[test]
fn an_input_stamped_at_the_first_head_arrives_there() {
    let case = delivering(vec![delivery(b"input", 8, 24)], 40);
    let clock = script();
    let cores = || reference(8 * SECOND);
    let gate = replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock));
    assert_eq!(gate.expect("a late boundary").runs().len(), 2);
}

#[test]
fn an_input_inside_the_first_step_has_no_late_boundary() {
    let case = delivering(vec![delivery(b"input", 5, 24)], 40);
    let clock = script();
    let cores = || reference(8 * SECOND);
    let gate = replay_gate(ReplayGate::LateAdmission, &case, &cores, &plan(&clock));
    assert_eq!(
        gate.err(),
        Some(ReplayFailure::NoLateBoundary {
            id: InputId::new(Digest::of_canonical(b"input"))
        })
    );
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

/// Returns `None` when the known run faults, which says nothing about a late one.
fn late_against_known(core: &dyn CoupledSolver, random: &mut Random) -> Option<u64> {
    let names: [&[u8]; 4] = [b"a", b"b", b"c", b"d"];
    let inputs: Vec<(&[u8], SimTime, SimTime)> = (0..1 + random.below(4))
        .map(|name| {
            let from = at(1).nanoseconds() + random.below(100 * SECOND);
            let to = from + (1 + random.below(30)) * SECOND;
            (names[name as usize], SimTime::from_ns(from), SimTime::from_ns(to))
        })
        .collect();
    let deliveries = (inputs.iter())
        .map(|&(id, from, to)| Delivery {
            id: Box::leak(id.to_vec().into_boxed_slice()),
            from,
            to,
            amount: measure(10.0 * (1 + from.nanoseconds() % 50) as f64),
        })
        .collect();
    let case = delivering(deliveries, 120);
    let mut known = case.admit(core).expect("admitted");
    let mut rows = vec![(
        known.record().accepted().time(),
        known.record().seal(),
        known.record().earliest_stamp(),
    )];
    while known.record().accepted().time() < case.end() {
        let mut attempt = known.step();
        known = loop {
            attempt = match attempt {
                Step::Accepted(session, _) => break session,
                Step::Refused(session) => session.retry(),
                Step::Faulted(_) => return None,
            };
        };
        let record = known.record();
        rows.push((record.accepted().time(), record.seal(), record.earliest_stamp()));
    }
    let mut arrivals: Vec<(u64, BoundaryEvent)> = Vec::new();
    for event in case.events().expect("events") {
        let last = rows.iter().rposition(|row| row.2 < event.at()).unwrap_or(0) as u64;
        arrivals.push((random.below(last + 1), event));
    }
    arrivals.sort_by_key(|arrival| arrival.0);
    let mut late = case.admit_withheld(core).expect("admitted");
    let mut seen = vec![(late.record().accepted().time(), late.record().seal())];
    for (revision, event) in arrivals {
        while late.record().revision() < revision {
            late = advance(late);
            seen.push((late.record().accepted().time(), late.record().seal()));
        }
        let command = Digest::of_canonical(b"a command");
        let input = AuthorizedInput::from_command_receipt(command, event);
        late.enqueue(input).expect("a head that can still take it");
    }
    while late.record().accepted().time() < case.end() {
        late = advance(late);
        seen.push((late.record().accepted().time(), late.record().seal()));
    }
    let known_rows: Vec<(SimTime, Seal)> = rows.iter().map(|row| (row.0, row.1)).collect();
    assert_eq!(seen, known_rows, "every revision, by its seal");
    assert_eq!(late.record().digests(), known.record().digests());
    Some(rows.len() as u64)
}

#[test]
fn late_inputs_match_the_known_timeline_on_random_timelines() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let (mut honest, mut hashed) = (0, 0);
    for _ in 0..40 {
        let scale = [0.02, 0.05, 0.1, 0.3, 1.0][random.below(5) as usize];
        let scales = ErrorScales::uniform(measure(scale), measure(0.0)).expect("scales");
        let interval_ns = (1 + random.below(40)) * SECOND / 2;
        let core = reference(interval_ns).with_scales(scales);
        honest += late_against_known(&core, &mut random).expect("an honest core finishes");
        let refuses = RefusesByHash {
            inner: reference(interval_ns),
            salt: random.below(u64::MAX),
        };
        hashed += late_against_known(&refuses, &mut random).unwrap_or(0);
    }
    assert!(
        honest > 1_000 && hashed > 500,
        "{honest} and {hashed} rows compared"
    );
}
