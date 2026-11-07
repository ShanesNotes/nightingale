use crate::{AccuracyEvidence, AccuracyPolicy, ArenaFailure, Certificate, ReplayEvidence, RunEvidence};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScorePart {
    Certificate,
    Accuracy,
    Replay,
}

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
/// must come from its core and cover `AccuracyPolicy::manufactured`, and the replay evidence must name its identity.
pub fn score(
    certificate: Certificate,
    accuracy: AccuracyEvidence,
    evidence: RunEvidence,
    replay: ReplayEvidence,
) -> Result<ArenaScore, ArenaFailure> {
    let missing = |part| Err(ArenaFailure::MissingEvidence { part });
    if certificate.seal() != evidence.seal() {
        return missing(ScorePart::Certificate);
    }
    let default =
        AccuracyPolicy::manufactured().map_err(|error| ArenaFailure::Initialization(error.into()))?;
    if accuracy.resolution() != evidence.resolution().digest() || !accuracy.covers(&default) {
        return missing(ScorePart::Accuracy);
    }
    if replay.run() != evidence.run() {
        return missing(ScorePart::Replay);
    }
    Ok(ArenaScore {
        certificate,
        accuracy,
        evidence,
        replay,
    })
}
