//! The host's session over one record, bound at admission to one compiled body and one solver. A refused
//! session can only retry, and a fault latches: `Session<Faulted>` has no `step`, and nothing moves a
//! session back to `Running`.
use crate::record::{NextImage, PatientRecord, StepReceipt};
use ng_contract::{
    AdmissionError, CompiledBody, CoupledProblem, CoupledSolver, EngineFault, InitialRevision, Refused,
    RunIdentity, StepTicket, TrialFailure,
};

pub struct Session<'m, S> {
    solver: &'m dyn CoupledSolver,
    record: Box<PatientRecord<'m>>,
    status: S,
}
pub struct Running;
/// A refusal stands: the record is untouched, and the only way on is the smaller retry.
pub struct Retrying {
    refused: Refused,
    earlier: Vec<Refused>,
}
pub struct Faulted {
    fault: EngineFault,
    refusals: Vec<Refused>,
}
pub enum Step<'m> {
    Accepted(Session<'m, Running>, StepReceipt),
    /// Nothing changed; `Session::retry` tries the same base over a shorter interval.
    Refused(Session<'m, Retrying>),
    /// Latched: stop and show it, never death.
    Faulted(Session<'m, Faulted>),
}

/// One attempt, through solve and validate. A refusal that names any ticket but the one issued would spend
/// another attempt's budget, so it is `BindingMismatch`.
fn attempt(
    record: &PatientRecord<'_>,
    solver: &dyn CoupledSolver,
    issue: impl FnOnce(&CoupledProblem<'_>) -> Result<StepTicket, EngineFault>,
) -> Result<NextImage, TrialFailure> {
    let problem = record.problem();
    let ticket = issue(&problem)?;
    let proposal = solver.solve(&problem, &ticket).map_err(|failure| match failure {
        TrialFailure::Refused(refused) if refused.ticket() != ticket.digest() => ticket.mismatch().into(),
        other => other,
    })?;
    record.validate(&ticket, proposal)
}

fn settle<'m>(
    solver: &'m dyn CoupledSolver,
    mut record: Box<PatientRecord<'m>>,
    refusals: Vec<Refused>,
    outcome: Result<NextImage, TrialFailure>,
) -> Step<'m> {
    match outcome {
        Ok(next) => {
            let receipt = record.publish(next);
            let status = Running;
            Step::Accepted(
                Session {
                    solver,
                    record,
                    status,
                },
                receipt,
            )
        }
        Err(TrialFailure::Refused(refused)) => Step::Refused(Session {
            solver,
            record,
            status: Retrying {
                refused,
                earlier: refusals,
            },
        }),
        Err(TrialFailure::EngineFault(fault)) => Step::Faulted(Session {
            solver,
            record,
            status: Faulted { fault, refusals },
        }),
    }
}

impl<'m, S> Session<'m, S> {
    pub fn record(&self) -> &PatientRecord<'m> {
        &self.record
    }
}
impl<'m> Session<'m, Running> {
    /// Refuses an identity that is not this body's, this origin's and this solver's, and a zero floor.
    pub fn admit(
        body: &'m CompiledBody,
        solver: &'m dyn CoupledSolver,
        origin: InitialRevision,
        identity: RunIdentity,
    ) -> Result<Self, AdmissionError> {
        let resolution = solver.resolution();
        if identity.model() != body.model_hash()
            || origin.model_hash() != body.model_hash()
            || identity.packet() != origin.packet_hash()
            || identity.resolution().digest() != resolution.digest()
        {
            return Err(AdmissionError::IdentityMismatch);
        }
        if resolution.minimum_interval_ns() == 0 {
            return Err(AdmissionError::ZeroMinimumInterval);
        }
        Ok(Self {
            solver,
            record: Box::new(PatientRecord::admit(body, identity, origin)),
            status: Running,
        })
    }
    /// `next_step`, `solve`, `validate`, then `publish`. A fault from any of them latches.
    pub fn step(self) -> Step<'m> {
        let Self {
            solver,
            record,
            status: Running,
        } = self;
        let outcome = attempt(&record, solver, |problem| solver.next_step(problem));
        settle(solver, record, Vec::new(), outcome)
    }
}
impl<'m> Session<'m, Retrying> {
    pub fn refused(&self) -> &Refused {
        &self.status.refused
    }
    /// The same base over the refusal's smaller span. The core solves; it does not pick the interval.
    pub fn retry(self) -> Step<'m> {
        let Self {
            solver,
            record,
            status: Retrying { refused, mut earlier },
        } = self;
        let outcome = attempt(&record, solver, |problem| problem.retry(&refused));
        earlier.push(refused);
        settle(solver, record, earlier, outcome)
    }
}
impl Session<'_, Faulted> {
    pub fn fault(&self) -> &EngineFault {
        &self.status.fault
    }
    /// The refusals before the fault, each naming its row or source.
    pub fn refusals(&self) -> &[Refused] {
        &self.status.refusals
    }
}
