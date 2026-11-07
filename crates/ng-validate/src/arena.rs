use crate::score::bound;
use crate::slice::arena_certificate;
use crate::window::WindowPolicy;
use crate::{
    AccuracyPolicy, ArenaFailure, ArenaScore, ArenaSpec, CertificatePolicy, ChancePlan, ClockSource,
    CoreFactory, DayPlan, DayReport, DaySpec, ReplayEvidence, ReplayPlan, ScorePart, SliceCase,
    SlidingWindows, WallClock, accuracy, anchor_moment, arena_body, arena_lens, arena_packet, arena_seed,
    certify, day_checks, replay_gates, run,
};
use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::{
    AdmissionError, BoundaryPort, CompiledBody, CoupledSolver, DeclaredResolution, Digest, Dimensionless,
    ErrorScales, Interval, Measure, SimTime,
};
use ng_record::HistoryStore;

const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3_600_000 * MILLISECOND;
const DAY: u64 = 24 * HOUR;

/// What a score spans from the 06:55 admission, how many times the arena times that span, and the error scales and
/// minimum interval every core it scores declares: the day checks read their tolerance from the scales, and the
/// step-policy gate its distance from the interval. The windows' coverage is the span; the day plan covers every day
/// it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArenaProtocol {
    pub windows: SlidingWindows,
    pub repeats: NonZeroU32,
    pub scales: ErrorScales,
    pub minimum_interval_ns: NonZeroU64,
}
impl ArenaProtocol {
    /// §7's: every 24 h window over 48 h, timed twice so the throughput shows its spread, at the solver arena's model
    /// defaults for the scales and the interval.
    pub fn full() -> Self {
        let day = NonZeroU64::new(DAY).expect("a day");
        let scale = |value| Measure::<Dimensionless>::new(value).expect("a finite constant");
        Self {
            windows: SlidingWindows::new(day, 2 * DAY).expect("two days cover one"),
            repeats: NonZeroU32::MIN.saturating_add(1),
            scales: ErrorScales::uniform(scale(0.1), scale(1e-3)).expect("positive constants"),
            minimum_interval_ns: NonZeroU64::new(100_000).expect("a positive constant"),
        }
    }
    fn declared_by(&self, resolution: &DeclaredResolution) -> bool {
        *resolution.scales() == self.scales && resolution.minimum_interval_ns() == self.minimum_interval_ns
    }
    fn window(&self) -> Interval {
        let start = anchor_moment();
        let end = start.nanoseconds().saturating_add(self.windows.coverage_ns());
        Interval::new(start, SimTime::from_ns(end))
            .expect("a coverage longer than its window ends after it starts")
    }
    fn days(&self) -> NonZeroU32 {
        let days = self.windows.coverage_ns().div_ceil(DAY);
        NonZeroU32::new(u32::try_from(days).unwrap_or(u32::MAX)).unwrap_or(NonZeroU32::MIN)
    }
}

/// What every core receives, built once: the slice, its certificate policy for the protocol's windows, the accuracy
/// gate's policy with the first slice window measured, and the day plan whose inputs every timed run knows.
pub struct Arena {
    protocol: ArenaProtocol,
    body: CompiledBody,
    certificate: CertificatePolicy,
    accuracy: AccuracyPolicy,
    accuracy_digest: Digest,
    plan: DayPlan,
    window: WindowPolicy,
}
impl Arena {
    pub fn new(protocol: ArenaProtocol) -> Result<Self, ArenaFailure> {
        let admission = |error: AdmissionError| ArenaFailure::Initialization(error.into());
        let body = arena_body().map_err(admission)?;
        let certificate = arena_certificate(&body, protocol.windows).map_err(admission)?;
        let accuracy = AccuracyPolicy::arena().map_err(ArenaFailure::Accuracy)?;
        let window = WindowPolicy::of(body.longest_step_ns());
        Ok(Self {
            protocol,
            certificate,
            accuracy_digest: accuracy.digest(),
            accuracy,
            plan: DayPlan::reference(protocol.days()),
            window,
            body,
        })
    }
    pub fn protocol(&self) -> ArenaProtocol {
        self.protocol
    }
    /// Scores one core: `cores` builds it fresh for every run, and `second_policy` builds the same core under
    /// another step policy for the step-policy gate. Only the timed run keeps its history in `store`, which the
    /// certificate walks. A part that fails leaves the others standing; only a core that declares other scales or
    /// another interval than the protocol is refused whole.
    pub fn score<C: CoupledSolver + 'static>(
        &self,
        cores: impl Fn() -> C,
        second_policy: impl Fn() -> C,
        clock: &dyn WallClock,
        store: &dyn HistoryStore,
    ) -> Result<ArenaScore, ArenaFailure> {
        let (cores, second_policy): (&dyn CoreFactory, &dyn CoreFactory) = (&cores, &second_policy);
        for factory in [cores, second_policy] {
            let resolution = factory.fresh().resolution().clone();
            if !self.protocol.declared_by(&resolution) {
                return Err(ArenaFailure::OffProtocol(resolution));
            }
        }
        let accuracy = accuracy(&*cores.fresh(), &self.accuracy).map_err(ArenaFailure::Accuracy);
        let replay = self.replay(cores, second_policy, clock);
        let core = cores.fresh();
        let ran = self
            .spec()
            .and_then(|spec| run(&self.body, &*core, spec, clock, store));
        let (evidence, certificate) = match ran {
            Ok(ran) => {
                let certified = certify(&ran.session().conservation(), &self.certificate);
                (
                    Ok(ran.into_evidence()),
                    certified.map_err(ArenaFailure::Certificate),
                )
            }
            Err(failure) => {
                let missing = ArenaFailure::MissingEvidence {
                    part: ScorePart::Certificate,
                };
                (Err(failure), Err(missing))
            }
        };
        let day = self.day(cores);
        let second_policy = second_policy.fresh().resolution().clone();
        let parts = (evidence, certificate, accuracy, replay, day);
        Ok(bound(self.protocol, self.accuracy_digest, second_policy, parts))
    }
    fn spec(&self) -> Result<ArenaSpec, ArenaFailure> {
        let admission = |error: AdmissionError| ArenaFailure::Initialization(error.into());
        let events = self.plan.events(anchor_moment()).map_err(admission)?;
        Ok(ArenaSpec {
            window: self.protocol.window(),
            packet: arena_packet(&self.body).map_err(admission)?,
            lens: arena_lens(&self.body).map_err(admission)?,
            boundary: BoundaryPort::new(events).map_err(admission)?,
            seed: arena_seed(),
            repeats: self.protocol.repeats,
        })
    }
    fn day(&self, cores: &dyn CoreFactory) -> Result<DayReport, ArenaFailure> {
        let admission = |error: AdmissionError| ArenaFailure::Initialization(error.into());
        let spec = DaySpec {
            window: self.protocol.window(),
            packet: arena_packet(&self.body).map_err(admission)?,
            lens: arena_lens(&self.body).map_err(admission)?,
            seed: arena_seed(),
            plan: self.plan.clone(),
        };
        day_checks(&self.body, &*cores.fresh(), spec).map_err(ArenaFailure::Day)
    }
    fn replay(
        &self,
        cores: &dyn CoreFactory,
        second_policy: &dyn CoreFactory,
        clock: &dyn WallClock,
    ) -> Result<ReplayEvidence, ArenaFailure> {
        let case = SliceCase::new().map_err(|error| ArenaFailure::Initialization(error.into()))?;
        let window = self.window;
        let reference = move || window.finest();
        let either_floor = window.landing_ns().max(self.protocol.minimum_interval_ns);
        let clock = FromFirstReading {
            clock,
            origin: Cell::new(None),
        };
        let plan = ReplayPlan {
            clock: &clock,
            simulated_per_wall: 1,
            chunks: vec![70 * MILLISECOND, 130 * MILLISECOND + 1],
            resume_every: 1000,
            chance: Some(ChancePlan {
                second_policy,
                reference: &reference,
                event_tolerance_ns: either_floor.get(),
            }),
        };
        replay_gates(&case, cores, &plan).map_err(ArenaFailure::Replay)
    }
}

struct FromFirstReading<'c> {
    clock: &'c dyn WallClock,
    origin: Cell<Option<u64>>,
}
impl WallClock for FromFirstReading<'_> {
    fn elapsed_ns(&self) -> Option<u64> {
        let reading = self.clock.elapsed_ns()?;
        let origin = self.origin.get().unwrap_or(reading);
        self.origin.set(Some(origin));
        reading.checked_sub(origin)
    }
    fn source(&self) -> ClockSource {
        self.clock.source()
    }
}
