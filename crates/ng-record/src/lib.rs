//! Accepted image, validate and publish, session latch, history, checkpoint and observation view.
//!
//! # Type rules
//!
//! A vital is an observation; the accepted record has no setter for it.
//! ```compile_fail,E0599
//! use ng_record::PatientRecord;
//! fn overwrite(record: &mut PatientRecord) { record.set_mean_arterial_pressure(1.0); }
//! ```
//! ```
//! use ng_contract::SimTime;
//! use ng_record::PatientRecord;
//! fn read(record: &PatientRecord) -> SimTime { record.accepted().time() }
//! ```
//! A faulted session is latched: it can be read, never stepped.
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn again(stopped: Session<'_, Faulted>) -> Step<'_> { stopped.step() }
//! ```
//! ```
//! use ng_record::*;
//! fn once(running: Session<'_, Running>) -> Step<'_> { running.step() }
//! ```
//! A refused session can only retry, so a host cannot drop a refusal and restart the budget.
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn restart(refused: Session<'_, Retrying>) -> Step<'_> { refused.step() }
//! ```
//! ```
//! use ng_record::*;
//! fn again(refused: Session<'_, Retrying>) -> Step<'_> { refused.retry() }
//! ```
//! A lent record is read-only, so publication happens only inside the session.
//! ```compile_fail,E0596
//! use ng_record::*;
//! fn sneak(session: &Session<'_, Running>, next: NextImage) -> StepReceipt {
//!     session.record().publish(next)
//! }
//! ```
//! ```
//! use ng_record::*;
//! fn legal(session: Session<'_, Running>) -> Step<'_> { session.step() }
//! ```
//! A refused session is mid-step and a faulted one has no next step, so neither takes an input or a save.
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn late(refused: &mut Session<'_, Retrying>, input: AuthorizedInput) -> Result<InputReceipt, ng_contract::AdmissionError> {
//!     refused.enqueue(input)
//! }
//! ```
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn late(stopped: &mut Session<'_, Faulted>, input: AuthorizedInput) -> Result<InputReceipt, ng_contract::AdmissionError> {
//!     stopped.enqueue(input)
//! }
//! ```
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn save(refused: &Session<'_, Retrying>, store: &mut dyn HistoryStore) -> Result<Checkpoint, CheckpointError> {
//!     refused.checkpoint(store)
//! }
//! ```
//! ```
//! use ng_record::*;
//! fn late(running: &mut Session<'_, Running>, input: AuthorizedInput) -> Result<InputReceipt, ng_contract::AdmissionError> {
//!     running.enqueue(input)
//! }
//! fn save(running: &Session<'_, Running>, store: &mut dyn HistoryStore) -> Result<Checkpoint, CheckpointError> {
//!     running.checkpoint(store)
//! }
//! ```
//! A lent record takes no input: the one path from a clinician to the body goes through the session.
//! ```compile_fail,E0596
//! use ng_record::*;
//! fn sneak(session: &Session<'_, Running>, input: AuthorizedInput) -> Result<InputReceipt, ng_contract::AdmissionError> {
//!     session.enqueue(input)
//! }
//! ```
//! ```
//! use ng_contract::SimTime;
//! use ng_record::*;
//! fn stamp(session: &Session<'_, Running>) -> SimTime { session.record().earliest_stamp() }
//! ```
//! An input names the command that authorized it, through the one constructor.
//! ```compile_fail,E0451
//! use ng_contract::*;
//! use ng_record::*;
//! fn forged(command: Digest, event: BoundaryEvent) -> AuthorizedInput { AuthorizedInput { command, event } }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_record::*;
//! fn authorized(command: Digest, event: BoundaryEvent) -> AuthorizedInput {
//!     AuthorizedInput::from_command_receipt(command, event)
//! }
//! ```
//! A mode and a stream's ordinal are accepted state in the contract's image, so a record has no second copy to
//! write: only a step moves them.
//! ```compile_fail,E0599
//! use ng_record::*;
//! fn advance(record: &mut PatientRecord<'_>) { record.advance_stream(0); }
//! ```
//! ```
//! use ng_contract::Digest;
//! use ng_record::*;
//! fn read(record: &PatientRecord<'_>) -> (Digest, Digest) {
//!     let digests = record.digests();
//!     (digests.modes, digests.streams)
//! }
//! ```
//! A host cannot hand the record a ticket: an attempt issues its own.
//! ```compile_fail,E0624
//! use ng_contract::*;
//! use ng_record::*;
//! fn swap(record: &PatientRecord<'_>, ticket: &StepTicket, proposal: StepProposal)
//!     -> Result<NextImage, TrialFailure> {
//!     record.validate(ticket, proposal)
//! }
//! ```
//! ```
//! use ng_contract::*;
//! use ng_record::*;
//! fn by_hand(record: &PatientRecord<'_>, core: &dyn CoupledSolver) -> Result<NextImage, TrialFailure> {
//!     let span_ns = core.next_step(record.accepted());
//!     record.attempt(core, Attempt::First { span_ns })
//! }
//! ```

mod checkpoint;
mod history;
mod record;
mod session;

pub use checkpoint::{BranchFork, BranchId, Checkpoint, CheckpointError, HistoryStore};
pub use history::{ConservationView, Curve, HistorySeal, HistorySegment, Segments};
pub use record::{AuthorizedInput, InputReceipt, NextImage, PatientRecord, RecordDigests, Seal, StepReceipt};
pub use session::{FaultReport, Faulted, Retrying, Running, Session, Step};
