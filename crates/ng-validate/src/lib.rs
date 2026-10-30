//! Synthetic composition root and stand-ins, certificate and arena.

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
