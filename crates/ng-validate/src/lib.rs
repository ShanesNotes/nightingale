//! Synthetic composition root and stand-ins, reference integrator, certificate and arena.
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
//! use ng_record::PatientRecord;
//! use ng_validate::*;
//! fn one(record: &PatientRecord, policy: &CertificatePolicy)
//!     -> Result<Certificate, CertificateFailure> {
//!     certify(&record.conservation(), policy)
//! }
//! ```
//! A pace is when a host calls `step`, and `step` takes nothing, so a caller's stop cannot reach a core.
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
//! Validation takes only a synthetic fixture, so the arena is nonclinical by type.
//! ```compile_fail,E0308
//! use ng_contract::*;
//! use ng_record::{Running, Session};
//! use ng_validate::*;
//! fn clinical<'m>(body: &'m CompiledBody, packet: ParameterPacket<Reviewed>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver) -> Result<Session<'m, Running>, InitializationFailure> {
//!     admit_fixture(body, packet, port, core)
//! }
//! ```
//! ```compile_fail,E0308
//! use ng_contract::*;
//! use ng_validate::*;
//! fn clinical<'m>(body: &'m CompiledBody, packet: ParameterPacket<Reviewed>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver, lens: &ObservationLens) -> Result<Admitted<'m>, InitializationFailure> {
//!     admit_solved(body, packet, port, core, lens)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_validate::*;
//! fn synthetic<'m>(body: &'m CompiledBody, packet: ParameterPacket<SyntheticFixture>, port: BoundaryPort,
//!     core: &'m dyn CoupledSolver, lens: &ObservationLens) -> Result<Admitted<'m>, InitializationFailure> {
//!     admit_solved(body, packet, port, core, lens)
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

mod accuracy;
mod certificate;
mod fixture;
mod initialize;
mod reference;
mod replay;
mod ring;
mod run;
mod slice;
mod sparse;

pub use accuracy::{
    AccuracyCase, AccuracyEvidence, AccuracyFailure, AccuracyPolicy, accuracy, observed_order,
};
pub use certificate::{
    Certificate, CertificateCheck, CertificateFailure, CertificatePolicy, ExpectedDrift, QuantityPolicy,
    QuantityResult, Sliding24Hours, certify,
};
pub use fixture::{ChanceCase, Delivery, TwoReservoirs, admit_fixture, admit_seeded};
pub use initialize::{Admitted, admission_tolerance, admit_at, admit_solved};
pub use reference::ReferenceIntegrator;
pub use replay::{
    ChancePlan, ClockSource, CoreFactory, GateEvidence, ReplayCase, ReplayEvidence, ReplayFailure,
    ReplayGate, ReplayPlan, WallClock, replay_gate, replay_gates,
};
pub use run::run_until;
pub use slice::{
    anchor_moment, arena_anatomy, arena_body, arena_lens, arena_packet, arena_schema, hold_outside_slice,
};
