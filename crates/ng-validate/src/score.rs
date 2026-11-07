use crate::{AccuracyEvidence, ArenaFailure, Certificate, DayReport, Protocol, ReplayEvidence, RunEvidence};
use ng_contract::{DeclaredResolution, Digest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScorePart {
    Certificate,
    Accuracy,
    Replay,
    Day,
}

/// One core's score under one protocol: each part its own result, held against the timed run. A part that failed
/// keeps its failure and leaves every other part standing.
pub struct ArenaScore {
    protocol: Protocol,
    second_policy: DeclaredResolution,
    evidence: Result<RunEvidence, ArenaFailure>,
    certificate: Result<Certificate, ArenaFailure>,
    accuracy: Result<AccuracyEvidence, ArenaFailure>,
    replay: Result<ReplayEvidence, ArenaFailure>,
    day: Result<DayReport, ArenaFailure>,
}
impl ArenaScore {
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }
    /// The resolution the step-policy gate held beside the core's own.
    pub fn second_policy(&self) -> &DeclaredResolution {
        &self.second_policy
    }
    /// The timed run's identity, seal, resolution, throughput and numerical health. A faulted run's health is in
    /// its `ArenaFailure::Faulted`.
    pub fn evidence(&self) -> Result<&RunEvidence, &ArenaFailure> {
        self.evidence.as_ref()
    }
    pub fn certificate(&self) -> Result<&Certificate, &ArenaFailure> {
        self.certificate.as_ref()
    }
    pub fn accuracy(&self) -> Result<&AccuracyEvidence, &ArenaFailure> {
        self.accuracy.as_ref()
    }
    pub fn replay(&self) -> Result<&ReplayEvidence, &ArenaFailure> {
        self.replay.as_ref()
    }
    pub fn day(&self) -> Result<&DayReport, &ArenaFailure> {
        self.day.as_ref()
    }
}

pub(crate) type Parts = (
    Result<RunEvidence, ArenaFailure>,
    Result<Certificate, ArenaFailure>,
    Result<AccuracyEvidence, ArenaFailure>,
    Result<ReplayEvidence, ArenaFailure>,
    Result<DayReport, ArenaFailure>,
);

/// The certificate must certify the seal the timed run ended on, the accuracy evidence must come from its
/// resolution under the arena's accuracy policy, the replay evidence must name its identity, and the day checks
/// must name its identity and end their fed run on its seal. A part held against no run, because the timed run
/// failed, stays as its gate returned it.
pub(crate) fn bound(
    protocol: Protocol,
    accuracy_policy: Digest,
    second_policy: DeclaredResolution,
    parts: Parts,
) -> ArenaScore {
    let (evidence, mut certificate, mut accuracy, mut replay, mut day) = parts;
    if let Ok(run) = &evidence {
        let held = |of: bool, part| of.then_some(()).ok_or(ArenaFailure::MissingEvidence { part });
        certificate = certificate.and_then(|certificate| {
            held(certificate.seal() == run.seal(), ScorePart::Certificate).map(|()| certificate)
        });
        accuracy = accuracy.and_then(|accuracy| {
            let of =
                accuracy.resolution() == run.resolution().digest() && accuracy.policy() == accuracy_policy;
            held(of, ScorePart::Accuracy).map(|()| accuracy)
        });
        replay =
            replay.and_then(|replay| held(replay.run() == run.run(), ScorePart::Replay).map(|()| replay));
        day = day.and_then(|day| {
            let of = day.run() == run.run() && day.seal() == run.seal().base;
            held(of, ScorePart::Day).map(|()| day)
        });
    }
    ArenaScore {
        protocol,
        second_policy,
        evidence,
        certificate,
        accuracy,
        replay,
        day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use core::cell::Cell;
    use core::num::{NonZeroU32, NonZeroU64};
    use ng_contract::*;
    use ng_record::ConservationView;

    const SECOND: u64 = 1_000_000_000;
    const HOUR: u64 = 3_600 * SECOND;
    const INTERVAL_NS: u64 = 16 * SECOND;
    const END_NS: u64 = 7_000 * SECOND;

    fn measure<U>(value: f64) -> Measure<U> {
        Measure::new(value).expect("finite")
    }

    fn at(seconds: u64) -> SimTime {
        SimTime::from_ns(seconds * SECOND)
    }

    fn case() -> TwoReservoirs {
        let delivery = Delivery {
            id: b"input",
            from: at(2_000),
            to: at(6_000),
            amount: measure(100.0),
        };
        let start = [measure(500_500.0), measure(499_500.0)];
        let case = TwoReservoirs::delivering(start, vec![delivery]).expect("a body");
        case.until(SimTime::from_ns(END_NS))
    }

    fn reference(interval_ns: u64) -> ReferenceIntegrator {
        let loose = ErrorScales::uniform(measure(1e9), measure(1.0)).expect("positive");
        let floor = NonZeroU64::new(100_000).expect("a floor");
        ReferenceIntegrator::new(interval_ns)
            .with_scales(loose)
            .with_floor(floor)
    }

    struct Ticks(Cell<u64>);
    impl WallClock for Ticks {
        fn elapsed_ns(&self) -> Option<u64> {
            self.0.set(self.0.get() + SECOND);
            Some(self.0.get())
        }
        fn source(&self) -> ClockSource {
            ClockSource::Scripted
        }
    }

    struct Readings {
        readings: Vec<u64>,
        next: Cell<usize>,
    }
    impl WallClock for Readings {
        fn elapsed_ns(&self) -> Option<u64> {
            let reading = *self.readings.get(self.next.get())?;
            self.next.set(self.next.get() + 1);
            Some(reading)
        }
        fn source(&self) -> ClockSource {
            ClockSource::Scripted
        }
    }

    fn timed<'m>(
        case: &'m TwoReservoirs,
        core: &'m ReferenceIntegrator,
        store: &'m MemoryStore,
        seed: Seed,
        window_ns: u64,
    ) -> ArenaRun<'m> {
        let body = case.body();
        let channels = (body.rows().iter()).map(|row| ChannelSpec::new("every row", row.kind()));
        let lens = (body.lens(channels.collect(), NonZeroU64::new(HOUR).expect("an hour"))).expect("a lens");
        let spec = ArenaSpec {
            window: Interval::new(at(0), SimTime::from_ns(window_ns)).expect("a window"),
            packet: case.packet().expect("the case's packet"),
            lens,
            boundary: BoundaryPort::new(case.events().expect("its inputs")).expect("a port"),
            seed,
            repeats: NonZeroU32::MIN,
        };
        run(body, core, spec, &Ticks(Cell::new(0)), store).expect("the case runs")
    }

    fn windows() -> SlidingWindows {
        SlidingWindows::new(NonZeroU64::new(HOUR).expect("an hour"), END_NS).expect("a coverage")
    }

    fn certified(view: &ConservationView<'_>) -> Certificate {
        let limits =
            |quantity| QuantityPolicy::new(quantity, measure::<Water>(1e-12), measure(1e-9), measure(1e3));
        let quantities = (view.basis().ids()).map(limits);
        let quantities = quantities
            .collect::<Result<Vec<_>, _>>()
            .expect("a policy of water");
        let policy = CertificatePolicy::new(view.basis(), quantities, ExpectedDrift::Zero, windows())
            .expect("a policy");
        certify(view, &policy).expect("the ledger closes and the body holds")
    }

    fn accurate(interval_ns: u64) -> AccuracyEvidence {
        let policy = AccuracyPolicy::manufactured().expect("the model default");
        accuracy(&reference(interval_ns), &policy).expect("within the policy")
    }

    fn replayed(case: &TwoReservoirs, interval_ns: u64) -> ReplayEvidence {
        let clock = Readings {
            readings: vec![
                0,
                0,
                5 * SECOND,
                5 * SECOND,
                7 * SECOND,
                900 * SECOND,
                90_000 * SECOND,
            ],
            next: Cell::new(0),
        };
        let plan = ReplayPlan {
            clock: &clock,
            simulated_per_wall: 1,
            chunks: vec![3 * SECOND, 11 * SECOND + 1, SECOND / 2],
            resume_every: 100,
            chance: None,
        };
        replay_gates(case, &move || reference(interval_ns), &plan).expect("every gate")
    }

    /// Bound as the arena binds a score, with the manufactured policy standing for the arena's, so no slice window is
    /// built.
    fn scored(parts: Parts) -> ArenaScore {
        let protocol = Protocol {
            windows: windows(),
            repeats: NonZeroU32::MIN,
        };
        let policy = AccuracyPolicy::manufactured()
            .expect("the model default")
            .digest();
        bound(
            protocol,
            policy,
            reference(INTERVAL_NS / 2).resolution().clone(),
            parts,
        )
    }

    fn missing<T>(part: Result<&T, &ArenaFailure>) -> Option<ScorePart> {
        match part {
            Err(ArenaFailure::MissingEvidence { part }) => Some(*part),
            Err(failure) => panic!("{failure:?}"),
            Ok(_) => None,
        }
    }

    fn not<T>(part: ScorePart) -> Result<T, ArenaFailure> {
        Err(ArenaFailure::MissingEvidence { part })
    }

    #[test]
    fn a_score_holds_one_runs_parts_and_refuses_another_cores_or_another_policys_accuracy() {
        let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
        let ran = timed(&case, &core, &store, case.seed(), END_NS);
        let [certificate, again, more] = [(); 3].map(|()| certified(&ran.session().conservation()));
        let (evidence, replay) = (ran.into_evidence(), replayed(&case, INTERVAL_NS));
        let another = accurate(INTERVAL_NS / 2);
        let parts = (
            Ok(evidence.clone()),
            Ok(again),
            Ok(another),
            Ok(replay.clone()),
            not(ScorePart::Day),
        );
        assert_eq!(missing(scored(parts).accuracy()), Some(ScorePart::Accuracy));
        let cases = vec![AccuracyCase::TwoReservoir(Box::new(
            TwoReservoirs::stepped_inflow().expect("a body"),
        ))];
        let other = AccuracyPolicy::new(cases, measure(5.0), measure(0.0), 100 * SECOND).expect("a policy");
        let other = accuracy(&core, &other).expect("within its policy");
        let parts = (
            Ok(evidence.clone()),
            Ok(more),
            Ok(other),
            Ok(replay.clone()),
            not(ScorePart::Day),
        );
        assert_eq!(missing(scored(parts).accuracy()), Some(ScorePart::Accuracy));
        let parts = (
            Ok(evidence),
            Ok(certificate),
            Ok(accurate(INTERVAL_NS)),
            Ok(replay),
            not(ScorePart::Day),
        );
        let score = scored(parts);
        let evidence = score.evidence().expect("the timed run");
        assert_eq!(
            score.certificate().map(Certificate::seal).ok(),
            Some(evidence.seal())
        );
        assert_eq!(score.replay().map(ReplayEvidence::run).ok(), Some(evidence.run()));
        let resolution = score.accuracy().map(AccuracyEvidence::resolution).ok();
        assert_eq!(resolution, Some(evidence.resolution().digest()));
        assert_eq!(score.protocol().windows, windows());
        assert_ne!(score.second_policy().digest(), evidence.resolution().digest());
    }

    #[test]
    fn a_certificate_of_another_run_or_of_the_same_run_at_another_head_is_missing() {
        let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
        let evidence = timed(&case, &core, &store, case.seed(), END_NS).into_evidence();
        let replay = replayed(&case, INTERVAL_NS);
        let reseeded = timed(&case, &core, &store, Seed::new([5, 6, 7, 8]), END_NS);
        let longer = timed(&case, &core, &store, case.seed(), END_NS + HOUR);
        for other in [reseeded, longer] {
            let certificate = Ok(certified(&other.session().conservation()));
            let accuracy = Ok(accurate(INTERVAL_NS));
            let parts = (
                Ok(evidence.clone()),
                certificate,
                accuracy,
                Ok(replay.clone()),
                not(ScorePart::Day),
            );
            assert_eq!(missing(scored(parts).certificate()), Some(ScorePart::Certificate));
        }
    }

    #[test]
    fn replay_evidence_of_another_run_is_missing() {
        let (case, core, store) = (case(), reference(INTERVAL_NS), MemoryStore::default());
        let ran = timed(&case, &core, &store, case.seed(), END_NS);
        let certificate = Ok(certified(&ran.session().conservation()));
        let other = Ok(replayed(&case, INTERVAL_NS / 2));
        let parts = (
            Ok(ran.into_evidence()),
            certificate,
            Ok(accurate(INTERVAL_NS)),
            other,
            not(ScorePart::Day),
        );
        assert_eq!(missing(scored(parts).replay()), Some(ScorePart::Replay));
    }

    #[test]
    fn with_no_timed_run_each_part_stays_as_its_gate_returned_it() {
        let failed = Err(ArenaFailure::MissingThroughput);
        let other = Ok(replayed(&case(), INTERVAL_NS / 2));
        let parts = (
            failed,
            not(ScorePart::Certificate),
            Ok(accurate(INTERVAL_NS / 2)),
            other,
            not(ScorePart::Day),
        );
        let score = scored(parts);
        assert!(matches!(score.evidence(), Err(ArenaFailure::MissingThroughput)));
        assert!(score.accuracy().is_ok() && score.replay().is_ok());
        assert_eq!(missing(score.certificate()), Some(ScorePart::Certificate));
    }

    #[test]
    fn day_checks_of_another_run_or_of_a_fed_run_that_ended_elsewhere_are_missing() {
        let body = arena_body().expect("compiled");
        let core = ng_solver::Core::new(10_000_000);
        let one = SimTime::from_ns(anchor_moment().nanoseconds() + SECOND);
        let window = |end| Interval::new(anchor_moment(), end).expect("a window");
        let store = MemoryStore::default();
        let slice = |end, seed| {
            let spec = ArenaSpec {
                window: window(end),
                packet: arena_packet(&body).expect("a packet"),
                lens: arena_lens(&body).expect("a lens"),
                boundary: BoundaryPort::new(Vec::new()).expect("a port"),
                seed,
                repeats: NonZeroU32::MIN,
            };
            let ran = run(&body, &core, spec, &Ticks(Cell::new(0)), &store).expect("the slice runs");
            ran.into_evidence()
        };
        let spec = DaySpec {
            window: window(one),
            packet: arena_packet(&body).expect("a packet"),
            lens: arena_lens(&body).expect("a lens"),
            seed: arena_seed(),
            plan: DayPlan::reference(NonZeroU32::MIN),
        };
        let day = day_checks(&body, &core, spec).expect("the day checks run");
        let later = SimTime::from_ns(one.nanoseconds() + SECOND);
        let reseeded = Seed::new([5, 6, 7, 8]);
        let runs = [
            (slice(one, arena_seed()), None),
            (slice(later, arena_seed()), Some(ScorePart::Day)),
            (slice(one, reseeded), Some(ScorePart::Day)),
        ];
        for (evidence, expected) in runs {
            let unscored = (
                not(ScorePart::Certificate),
                not(ScorePart::Accuracy),
                not(ScorePart::Replay),
            );
            let parts = (Ok(evidence), unscored.0, unscored.1, unscored.2, Ok(day.clone()));
            assert_eq!(missing(scored(parts).day()), expected);
        }
    }
}
