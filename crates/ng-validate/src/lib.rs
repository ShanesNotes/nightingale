//! Synthetic fixtures, the slice's composition root and held stand-ins, the admission solve, the reference
//! integrator, the accuracy gate, the certificate, the replay gates, the arena run, the split gate, the day checks, and
//! the arena that scores a core.
//!
//! # Type rules
//!
//! The certificate cannot pair one record's amounts with another's exchanges: a record lends no separate
//! histories, only one view.
//! ```compile_fail,E0599
//! use ng_record::PatientRecord;
//! use ng_validate::*;
//! fn cross(first: &PatientRecord, second: &PatientRecord, policy: &CertificatePolicy)
//!     -> Result<Certificate, CertificateFailure> {
//!     certify(first.inventory_history(), second.ledger(), policy)
//! }
//! ```
//! ```
//! use ng_record::{Running, Session};
//! use ng_validate::*;
//! fn one(session: &Session<'_, Running>, policy: &CertificatePolicy)
//!     -> Result<Certificate, CertificateFailure> {
//!     certify(&session.conservation(), policy)
//! }
//! ```
//! Charge's drift holds to its closure bound, so a policy cannot give charge a drift bound of its own.
//! ```compile_fail,E0277
//! use ng_contract::*;
//! use ng_validate::QuantityPolicy;
//! fn bounded(quantity: QuantityId, floor: Measure<Charge>, relative: Measure<Dimensionless>, drift: Measure<Charge>)
//!     -> Result<QuantityPolicy, AdmissionError> {
//!     QuantityPolicy::new(quantity, floor, relative, drift)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_validate::QuantityPolicy;
//! fn held(quantity: QuantityId, floor: Measure<Charge>, relative: Measure<Dimensionless>)
//!     -> Result<QuantityPolicy, AdmissionError> {
//!     QuantityPolicy::charge(quantity, floor, relative)
//! }
//! ```
//! A pace is when a host calls `step`, and `step` takes nothing, so a caller's stop cannot cut a step.
//! ```compile_fail,E0061
//! use ng_contract::SimTime;
//! use ng_record::*;
//! fn clipped<'m>(session: Session<'m, Running>, stop: SimTime) -> Step<'m> { session.step(stop) }
//! ```
//! ```
//! use ng_contract::SimTime;
//! use ng_record::*;
//! use ng_validate::*;
//! fn paced<'m>(session: Session<'m, Running>, stop: SimTime)
//!     -> Result<Session<'m, Running>, Session<'m, Faulted>> {
//!     run_until(session, stop)
//! }
//! ```
//! Admission here takes only a synthetic fixture, so a run it admits is nonclinical by type.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! use ng_record::{HistoryStore, Running, Session};
//! use ng_validate::*;
//! fn clinical<'m>(body: &'m CompiledBody, packet: ParameterPacket<Reviewed>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver, store: &'m dyn HistoryStore)
//!     -> Result<Session<'m, Running>, InitializationFailure> {
//!     admit_fixture(body, packet, port, core, store)
//! }
//! ```
//! ```compile_fail,E0308
//! use ng_contract::*;
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn clinical<'m>(body: &'m CompiledBody, packet: ParameterPacket<Reviewed>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver, lens: &ObservationLens, store: &'m dyn HistoryStore)
//!     -> Result<Admitted<'m>, InitializationFailure> {
//!     admit_solved(body, packet, port, core, lens, store)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn synthetic<'m>(body: &'m CompiledBody, packet: ParameterPacket<SyntheticFixture>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver, lens: &ObservationLens, store: &'m dyn HistoryStore)
//!     -> Result<Admitted<'m>, InitializationFailure> {
//!     admit_solved(body, packet, port, core, lens, store)
//! }
//! ```
//! The arena takes only a synthetic fixture, so a run it times is nonclinical by type.
//! ```compile_fail,E0308
//! use core::num::NonZeroU32;
//! use ng_contract::*;
//! use ng_validate::*;
//! fn clinical(window: Interval, packet: ParameterPacket<Reviewed>, lens: ObservationLens, boundary: BoundaryPort,
//!     seed: Seed, repeats: NonZeroU32) -> ArenaSpec {
//!     ArenaSpec { window, packet, lens, boundary, seed, repeats }
//! }
//! ```
//! ```
//! use core::num::NonZeroU32;
//! use ng_contract::*;
//! use ng_validate::*;
//! fn synthetic(window: Interval, packet: ParameterPacket<SyntheticFixture>, lens: ObservationLens,
//!     boundary: BoundaryPort, seed: Seed, repeats: NonZeroU32) -> ArenaSpec {
//!     ArenaSpec { window, packet, lens, boundary, seed, repeats }
//! }
//! ```
//! No throughput number enters the certificate: the run's evidence stands beside the record.
//! ```compile_fail,E0308
//! use ng_validate::*;
//! fn timed(run: ArenaRun<'_>, policy: &CertificatePolicy) -> Result<Certificate, CertificateFailure> {
//!     certify(&run.into_evidence(), policy)
//! }
//! ```
//! ```
//! use ng_validate::*;
//! fn conserved(run: &ArenaRun<'_>, policy: &CertificatePolicy) -> Result<Certificate, CertificateFailure> {
//!     certify(&run.session().conservation(), policy)
//! }
//! ```
//! Evidence comes only from a gate that ran.
//! ```compile_fail,E0451
//! use ng_contract::Digest;
//! use ng_validate::*;
//! fn forged(run: Digest, pace: GateEvidence) -> ReplayEvidence {
//!     ReplayEvidence {
//!         run,
//!         pace: pace.clone(),
//!         resume: pace.clone(),
//!         late_admission: pace.clone(),
//!         rewind: pace.clone(),
//!         extra_evaluation: pace.clone(),
//!         step_policy: pace.clone(),
//!         non_perturbation: pace,
//!     }
//! }
//! ```
//! ```
//! use ng_validate::*;
//! fn ran(case: &TwoReservoirs, cores: &dyn CoreFactory, plan: &ReplayPlan<'_>)
//!     -> Result<ReplayEvidence, ReplayFailure> {
//!     replay_gates(case, cores, plan)
//! }
//! ```
//! Throughput evidence comes only from a run that the host's clock timed.
//! ```compile_fail,E0451
//! use core::num::NonZeroU64;
//! use ng_validate::*;
//! fn forged(clock: ClockSource, wall: NonZeroU64) -> ThroughputEvidence {
//!     ThroughputEvidence {
//!         clock,
//!         admission_wall_ns: 0,
//!         runs_wall_ns: vec![wall],
//!         simulated_ns: 1,
//!         accepted_steps: 1,
//!         waveform_samples: 1,
//!     }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn timed<'m>(body: &'m CompiledBody, core: &'m dyn CoupledSolver, spec: ArenaSpec, clock: &dyn WallClock,
//!     store: &'m dyn HistoryStore) -> Result<ThroughputEvidence, ArenaFailure> {
//!     Ok(run(body, core, spec, clock, store)?.into_evidence().throughput().clone())
//! }
//! ```
//! A day report comes only from the runs its checks read.
//! ```compile_fail,E0451
//! use ng_contract::{Digest, SimTime};
//! use ng_validate::*;
//! fn forged(run: Digest, at: SimTime, checks: Vec<DayCheck>) -> DayReport {
//!     DayReport {
//!         run,
//!         seal: run,
//!         start: at,
//!         end: at,
//!         beats: 0,
//!         checks,
//!     }
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_validate::*;
//! fn checked(body: &CompiledBody, core: &dyn CoupledSolver, spec: DaySpec) -> Result<DayReport, DayFailure> {
//!     day_checks(body, core, spec)
//! }
//! ```
//! A healthy-range report and a return report come only from the checks that read the run.
//! ```compile_fail,E0451
//! use ng_contract::Digest;
//! use ng_validate::*;
//! fn forged(run: Digest, results: Vec<RangeResult>, gaps: Vec<RangeGap>) -> RangeReport {
//!     RangeReport { run, seal: run, set: run, lens: run, day: run, results, gaps }
//! }
//! ```
//! ```compile_fail,E0451
//! use ng_contract::{Digest, PacketHash};
//! use ng_validate::*;
//! fn forged(run: Digest, packet: PacketHash, results: Vec<AnchorReturn>, gaps: Vec<ReturnGap>) -> ReturnReport {
//!     ReturnReport { run, seal: run, set: run, lens: run, packet, results, gaps }
//! }
//! ```
//! ```
//! use ng_validate::*;
//! fn read(checks: ReadingChecks<'_>) -> (RangeReport, ReturnReport) {
//!     checks.finish()
//! }
//! ```
//! A score comes only from `Arena::score`, so every part it holds came from a gate the arena ran.
//! ```compile_fail,E0451
//! use ng_contract::DeclaredResolution;
//! use ng_validate::*;
//! fn assembled(protocol: ArenaProtocol, second_policy: DeclaredResolution, evidence: RunEvidence,
//!     certificate: Certificate, accuracy: AccuracyEvidence, replay: ReplayEvidence, day: DayReport) -> ArenaScore {
//!     ArenaScore {
//!         protocol,
//!         second_policy,
//!         evidence: Ok(evidence),
//!         certificate: Ok(certificate),
//!         accuracy: Ok(accuracy),
//!         replay: Ok(replay),
//!         day: Ok(day),
//!     }
//! }
//! ```
//! ```
//! use ng_contract::CoupledSolver;
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn scored<C: CoupledSolver + 'static>(arena: &Arena, cores: impl Fn() -> C, second_policy: impl Fn() -> C,
//!     clock: &dyn WallClock, store: &dyn HistoryStore) -> Result<ArenaScore, ArenaFailure> {
//!     arena.score(cores, second_policy, clock, store)
//! }
//! ```
//! The second policy is the same core under another step policy, so a score cannot hold the step-policy evidence of
//! another core.
//! ```compile_fail,E0308
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn mixed(arena: &Arena, clock: &dyn WallClock, store: &dyn HistoryStore) -> Result<ArenaScore, ArenaFailure> {
//!     arena.score(|| ng_solver::Core::new(10_000_000), || ReferenceIntegrator::new(5_000_000), clock, store)
//! }
//! ```
//! ```
//! use ng_record::HistoryStore;
//! use ng_validate::*;
//! fn same(arena: &Arena, clock: &dyn WallClock, store: &dyn HistoryStore) -> Result<ArenaScore, ArenaFailure> {
//!     arena.score(|| ng_solver::Core::new(10_000_000), || ng_solver::Core::new(5_000_000), clock, store)
//! }
//! ```

extern crate alloc;

mod accuracy;
mod arena;
mod body;
mod burning;
mod certificate;
mod day;
mod fixture;
mod healthy;
mod initialize;
mod reference;
mod replay;
mod run;
mod score;
mod slice;
mod split;
mod window;

pub use accuracy::{
    AccuracyCase, AccuracyEvidence, AccuracyFailure, AccuracyPolicy, accuracy, observed_order,
};
pub use arena::{Arena, ArenaProtocol};
pub use body::{
    FluidRoute, HOLD_TABLE, Held, Hold, KEEP_BACKS, KeepBack, Release, body_anatomy, body_schema, whole_body,
};
pub use burning::BurningBeds;
pub use certificate::{
    Certificate, CertificateCheck, CertificateFailure, CertificatePolicy, ExpectedDrift, HeldSupport,
    QuantityPolicy, QuantityResult, SlidingWindows, certify,
};
pub use day::{
    DayCheck, DayFailure, DayPlan, DayQuantity, DayReport, DaySpec, Miss, Unavailable, Verdict, day_checks,
};
pub use fixture::{ChanceCase, Delivery, TwoReservoirs, admit_fixture, admit_seeded};
pub use healthy::{
    AnchorReturn, BareLens, CheckVerdict, DayActivities, RangeFailure, RangeGap, RangeReport, RangeResult,
    ReadingCheckError, ReadingChecks, ReturnGap, ReturnReport,
};
pub use initialize::{Admitted, admission_tolerance, admit_at, admit_solved};
pub use reference::ReferenceIntegrator;
pub use replay::{
    ChancePlan, ClockSource, CoreFactory, GateEvidence, MemoryStore, ReplayCase, ReplayEvidence,
    ReplayFailure, ReplayGate, ReplayPlan, WallClock, replay_gate, replay_gates,
};
pub use run::{
    ArenaFailure, ArenaRun, ArenaSpec, NumericalHealth, RunEvidence, ThroughputEvidence, run, run_until,
};
pub use score::{ArenaScore, ScorePart};
pub use slice::{
    SliceCase, anchor_moment, arena_anatomy, arena_body, arena_certificate, arena_lens, arena_organs,
    arena_packet, arena_schema, arena_seed, hold_outside_slice,
};
pub use split::{SplitFailure, draw_split_trace, split_gate};
pub use window::{Compared, SliceWindow, WindowPolicy};
