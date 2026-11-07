use crate::{AccuracyEvidence, ArenaFailure, Certificate, ReplayEvidence, RunEvidence};

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

/// The run evidence names the run. The certificate must certify the history it ended on, the replay evidence must
/// name its identity, and the accuracy evidence must come from its core under `AccuracyPolicy::arena`.
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
    if replay.run() != evidence.run() {
        return missing(ScorePart::Replay);
    }
    let arena = accuracy
        .of_the_arena()
        .map_err(|error| ArenaFailure::Initialization(error.into()))?;
    if accuracy.resolution() != evidence.resolution().digest() || !arena {
        return missing(ScorePart::Accuracy);
    }
    Ok(ArenaScore {
        certificate,
        accuracy,
        evidence,
        replay,
    })
}
