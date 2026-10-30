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

mod checkpoint;
mod history;
mod record;
mod session;

pub use checkpoint::{BranchFork, BranchId, Checkpoint, CheckpointError, HistoryStore};
pub use history::{ConservationView, Curve, HistorySeal, HistorySegment, Segments};
pub use record::{AuthorizedInput, InputReceipt, NextImage, PatientRecord, RecordDigests, Seal, StepReceipt};
pub use session::{FaultReport, Faulted, Retrying, Running, Session, Step};
