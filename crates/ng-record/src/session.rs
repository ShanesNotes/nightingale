use crate::checkpoint::{BranchId, Checkpoint, CheckpointError, Continuing, HistoryStore, restore};
use crate::history::ConservationView;
use crate::record::{AuthorizedInput, InputReceipt, NextImage, PatientRecord, StepReceipt};
use ng_contract::{
    AdmissionError, Attempt, CompiledBody, CoupledSolver, EngineFault, InitialRevision, Refused, RunIdentity,
    TrialFailure,
};

pub struct Session<'m, S> {
    solver: &'m dyn CoupledSolver,
    store: &'m dyn HistoryStore,
    record: Box<PatientRecord<'m>>,
    status: S,
}
pub struct Running;
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

fn settle<'m>(
    solver: &'m dyn CoupledSolver,
    store: &'m dyn HistoryStore,
    mut record: Box<PatientRecord<'m>>,
    refusals: Vec<Refused>,
    outcome: Result<NextImage, TrialFailure>,
) -> Step<'m> {
    match outcome {
        Ok(next) => {
            let receipt = record.publish(next, store);
            let status = Running;
            Step::Accepted(
                Session {
                    solver,
                    store,
                    record,
                    status,
                },
                receipt,
            )
        }
        Err(TrialFailure::Refused(refused)) => Step::Refused(Session {
            solver,
            store,
            record,
            status: Retrying {
                refused,
                earlier: refusals,
            },
        }),
        Err(TrialFailure::EngineFault(fault)) => Step::Faulted(Session {
            solver,
            store,
            record,
            status: Faulted { fault, refusals },
        }),
    }
}

impl<'m, S> Session<'m, S> {
    pub fn record(&self) -> &PatientRecord<'m> {
        &self.record
    }
    pub fn conservation(&self) -> ConservationView<'_> {
        self.record.conservation(self.store)
    }
    fn fork(
        &self,
        saved: &Checkpoint,
        branch: BranchId,
        continuing: Continuing<'_>,
    ) -> Result<Session<'m, Running>, CheckpointError> {
        if self.record.branch().uses(branch.digest()) {
            return Err(CheckpointError::BranchInUse);
        }
        let identity = self.record.identity();
        let record = restore(saved, identity, self.record.body(), self.store, continuing)?;
        Ok(Session {
            solver: self.solver,
            store: self.store,
            record: Box::new(record),
            status: Running,
        })
    }
}
impl<'m> Session<'m, Running> {
    /// Refuses an identity that is not this body's, this origin's and this solver's.
    pub fn admit(
        body: &'m CompiledBody,
        solver: &'m dyn CoupledSolver,
        origin: InitialRevision,
        identity: RunIdentity,
        store: &'m dyn HistoryStore,
    ) -> Result<Self, AdmissionError> {
        let resolution = solver.resolution();
        if identity.model() != body.model_hash()
            || origin.model_hash() != body.model_hash()
            || identity.packet() != origin.packet_hash()
            || identity.resolution().digest() != resolution.digest()
        {
            return Err(AdmissionError::IdentityMismatch);
        }
        Ok(Self {
            solver,
            store,
            record: Box::new(PatientRecord::admit(body, identity, origin)),
            status: Running,
        })
    }
    /// Continues the run `saved` holds, on its branch, in `solver`, which may be freshly built.
    /// Everything is verified before a record exists.
    pub fn resume(
        saved: &Checkpoint,
        expected: &RunIdentity,
        body: &'m CompiledBody,
        solver: &'m dyn CoupledSolver,
        store: &'m dyn HistoryStore,
    ) -> Result<Self, CheckpointError> {
        if expected.model() != body.model_hash()
            || expected.resolution().digest() != solver.resolution().digest()
        {
            return Err(CheckpointError::IdentityMismatch);
        }
        let record = restore(saved, expected, body, store, Continuing::Resume)?;
        Ok(Self {
            solver,
            store,
            record: Box::new(record),
            status: Running,
        })
    }
    /// Adds a late input. A new one stamped before the accepted head or the fence is refused. The same id
    /// with the same payload is the same request, whenever it comes.
    pub fn enqueue(&mut self, input: AuthorizedInput) -> Result<InputReceipt, AdmissionError> {
        self.record.admit_input(input)
    }
    pub fn step(self) -> Step<'m> {
        let Self {
            solver,
            store,
            record,
            status: Running,
        } = self;
        let span_ns = solver.next_step(record.accepted());
        let outcome = record.attempt(solver, Attempt::First { span_ns });
        settle(solver, store, record, Vec::new(), outcome)
    }
    /// A checkpoint of the accepted head, which never cuts an interval. It is refused while the store has not
    /// kept every link, so the bytes it returns are complete with that store.
    pub fn checkpoint(&self) -> Result<Checkpoint, CheckpointError> {
        self.record.checkpoint(false)
    }
    /// A new branch from a checkpoint on this branch's own chain, with ancestry.
    pub fn rewind(
        &self,
        saved: &Checkpoint,
        branch: BranchId,
    ) -> Result<Session<'m, Running>, CheckpointError> {
        let parent = self.record.history();
        self.fork(saved, branch, Continuing::Rewind(branch, parent))
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
            store,
            record,
            status: Retrying { refused, mut earlier },
        } = self;
        let outcome = record.attempt(solver, Attempt::After(&refused));
        earlier.push(refused);
        settle(solver, store, record, earlier, outcome)
    }
}
impl<'m> Session<'m, Faulted> {
    pub fn fault(&self) -> &EngineFault {
        &self.status.fault
    }
    /// The refusals before the fault, in attempt order.
    pub fn refusals(&self) -> &[Refused] {
        &self.status.refusals
    }
    /// The image is marked, so `resume` refuses it: a faulted run goes on only through `recover`.
    pub fn export(&self) -> Result<FaultReport, CheckpointError> {
        Ok(FaultReport {
            last_accepted: self.record.checkpoint(true)?,
            fault: self.status.fault,
            refusals: self.status.refusals.clone(),
        })
    }
    /// The only way on: a new branch from a checkpoint on this branch's own chain, with ancestry.
    pub fn recover(
        &self,
        earlier: &Checkpoint,
        branch: BranchId,
    ) -> Result<Session<'m, Running>, CheckpointError> {
        let parent = self.record.history();
        self.fork(earlier, branch, Continuing::Recover(branch, parent))
    }
}

pub struct FaultReport {
    last_accepted: Checkpoint,
    fault: EngineFault,
    refusals: Vec<Refused>,
}
impl FaultReport {
    pub fn last_accepted(&self) -> &Checkpoint {
        &self.last_accepted
    }
    pub fn fault(&self) -> &EngineFault {
        &self.fault
    }
    pub fn refusals(&self) -> &[Refused] {
        &self.refusals
    }
}
