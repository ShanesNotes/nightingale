//! Synthetic composition root and stand-ins, certificate and arena.
//!
//! # Type rules
//!
//! The certificate cannot pair one record's inventories with another's ledger: a record lends no separate
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

mod accuracy;
mod certificate;
mod fixture;
mod reference;
mod run;

pub use accuracy::{
    AccuracyCase, AccuracyEvidence, AccuracyFailure, AccuracyPolicy, accuracy, observed_order,
};
pub use certificate::{
    Certificate, CertificateCheck, CertificateFailure, CertificatePolicy, ExpectedDrift, QuantityPolicy,
    QuantityResult, Sliding24Hours, certify,
};
pub use fixture::{Delivery, TwoReservoirs, admit_fixture};
pub use reference::ReferenceIntegrator;
pub use run::run_until;
