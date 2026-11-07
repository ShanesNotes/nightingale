use crate::score::bound;
use crate::window::WindowPolicy;
use crate::{
    AccuracyPolicy, ArenaFailure, ArenaScore, ArenaSpec, CertificatePolicy, ChancePlan, ClockSource,
    CoreFactory, DayPlan, DayReport, DaySpec, ReplayEvidence, ReplayPlan, ScorePart, SliceCase,
    SlidingWindows, WallClock, accuracy, anchor_moment, arena_body, arena_certificate, arena_lens,
    arena_packet, arena_seed, certify, day_checks, replay_gates, run,
};
use core::cell::Cell;
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::{AdmissionError, BoundaryPort, CompiledBody, Digest, Interval, SimTime};
use ng_record::HistoryStore;

const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3_600_000 * MILLISECOND;
const DAY: u64 = 24 * HOUR;

/// What a score spans from the 06:55 admission, and how many times the arena times that span. The windows'
/// coverage is the span; the day plan covers every day it reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Protocol {
    pub windows: SlidingWindows,
    pub repeats: NonZeroU32,
}
impl Protocol {
    /// §7's: every 24 h window over 48 h, timed twice so the throughput shows its spread.
    pub fn full() -> Self {
        let day = NonZeroU64::new(DAY).expect("a day");
        Self {
            windows: SlidingWindows::new(day, 2 * DAY).expect("two days cover one"),
            repeats: NonZeroU32::MIN.saturating_add(1),
        }
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
    protocol: Protocol,
    body: CompiledBody,
    certificate: CertificatePolicy,
    accuracy: AccuracyPolicy,
    accuracy_digest: Digest,
    plan: DayPlan,
    window: WindowPolicy,
}
impl Arena {
    pub fn new(protocol: Protocol) -> Result<Self, ArenaFailure> {
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
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }
    /// Scores one core: `cores` builds it fresh for every run, and `second_policy` builds the same core under
    /// another step policy for the step-policy gate. Every part runs on its own fresh core, and only the timed run
    /// keeps its history in `store`, which the certificate walks. A part that fails leaves the others standing.
    pub fn score(
        &self,
        cores: &dyn CoreFactory,
        second_policy: &dyn CoreFactory,
        clock: &dyn WallClock,
        store: &dyn HistoryStore,
    ) -> ArenaScore {
        let accuracy = accuracy(&*cores.fresh(), &self.accuracy).map_err(ArenaFailure::Accuracy);
        let replay = self.replay(cores, second_policy, clock);
        let core = cores.fresh();
        let ran = self
            .spec()
            .and_then(|spec| run(&self.body, &*core, spec, clock, store));
        let (evidence, certificate, day) = match ran {
            Ok(ran) => {
                let certified = certify(&ran.session().conservation(), &self.certificate);
                let evidence = ran.into_evidence();
                (
                    Ok(evidence),
                    certified.map_err(ArenaFailure::Certificate),
                    self.day(cores),
                )
            }
            Err(failure) => {
                let missing = |part| ArenaFailure::MissingEvidence { part };
                (
                    Err(failure),
                    Err(missing(ScorePart::Certificate)),
                    Err(missing(ScorePart::Day)),
                )
            }
        };
        let second_policy = second_policy.fresh().resolution().clone();
        let parts = (evidence, certificate, accuracy, replay, day);
        bound(self.protocol, self.accuracy_digest, second_policy, parts)
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
    /// The plan `tests/arena_body.rs` passes the reference with. The step-policy gate holds both policies to the
    /// first slice window's finest refinement, within its landing tolerance: the stir's rate is constant, so every
    /// policy lands it within its own floor after it crosses its draw.
    fn replay(
        &self,
        cores: &dyn CoreFactory,
        second_policy: &dyn CoreFactory,
        clock: &dyn WallClock,
    ) -> Result<ReplayEvidence, ArenaFailure> {
        let case = SliceCase::new().map_err(|error| ArenaFailure::Initialization(error.into()))?;
        let window = self.window;
        let reference = move || window.finest();
        let clock = Since {
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
                event_tolerance_ns: window.landing_ns().get(),
            }),
        };
        replay_gates(&case, cores, &plan).map_err(ArenaFailure::Replay)
    }
}

/// The live pace adds what its clock reads to the head its run starts from, so it reads the host's clock from its
/// own first reading, as `WallClock` promises, and not from when the host started it.
struct Since<'c> {
    clock: &'c dyn WallClock,
    origin: Cell<Option<u64>>,
}
impl WallClock for Since<'_> {
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
