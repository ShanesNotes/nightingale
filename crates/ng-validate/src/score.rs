use crate::{AccuracyEvidence, AccuracyPolicy, ArenaFailure, Certificate, ReplayEvidence, RunEvidence};

/// The part of a score that was not of its run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// A certificate of another history than the one the run ended on.
    Certificate,
    /// Accuracy evidence of another core, or under a policy other than the model default, which leaves a case
    /// out or holds it to another bound.
    Accuracy,
    /// Replay evidence of another run identity.
    Replay,
}

/// One run's certificate, accuracy, run and replay evidence. Each part states the policy values it was judged
/// by: the certificate its limits and windows, the accuracy evidence its bound and grid, the run evidence its
/// core's resolution, and the replay evidence each gate's tolerance.
pub struct ArenaScore {
    certificate: Certificate,
    accuracy: AccuracyEvidence,
    evidence: RunEvidence,
    replay: ReplayEvidence,
}
impl ArenaScore {
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }
    pub fn accuracy(&self) -> &AccuracyEvidence {
        &self.accuracy
    }
    pub fn evidence(&self) -> &RunEvidence {
        &self.evidence
    }
    pub fn replay(&self) -> &ReplayEvidence {
        &self.replay
    }
}

/// The run evidence names the run. The certificate must certify the history it ended on, the accuracy evidence
/// must come from its core under `AccuracyPolicy::manufactured`, and the replay evidence must name its identity.
pub fn score(
    certificate: Certificate,
    accuracy: AccuracyEvidence,
    evidence: RunEvidence,
    replay: ReplayEvidence,
) -> Result<ArenaScore, ArenaFailure> {
    let missing = |evidence| Err(ArenaFailure::MissingEvidence { evidence });
    if certificate.seal() != evidence.seal() {
        return missing(Evidence::Certificate);
    }
    // A default that does not build has no evidence under it.
    let default = AccuracyPolicy::manufactured().ok().map(|policy| policy.digest());
    if accuracy.resolution() != evidence.resolution().digest() || default != Some(accuracy.policy()) {
        return missing(Evidence::Accuracy);
    }
    if replay.run() != evidence.run() {
        return missing(Evidence::Replay);
    }
    Ok(ArenaScore {
        certificate,
        accuracy,
        evidence,
        replay,
    })
}
