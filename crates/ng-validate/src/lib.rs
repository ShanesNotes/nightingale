//! Synthetic fixtures, the slice's composition root and held stand-ins, the admission solve, the reference
//! integrator, the accuracy gate, the certificate, the replay gates and the split gate.
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
mod run;
mod slice;
mod split;

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
    ChancePlan, ClockSource, CoreFactory, GateEvidence, MemoryStore, ReplayCase, ReplayEvidence,
    ReplayFailure, ReplayGate, ReplayPlan, WallClock, replay_gate, replay_gates,
};
pub use run::run_until;
pub use slice::{
    anchor_moment, arena_anatomy, arena_body, arena_lens, arena_organs, arena_packet, arena_schema,
    hold_outside_slice,
};
pub use split::{SplitFailure, draw_split_trace, split_gate};
