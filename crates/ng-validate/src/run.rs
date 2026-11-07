use crate::initialize::{admit_start, solved_start};
use crate::{
    AccuracyFailure, CertificateFailure, ClockSource, DayFailure, ReplayFailure, ScorePart, WallClock,
};
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::{
    BoundaryPort, CompiledBody, CoupledSolver, DeclaredResolution, Digest, EngineFaultKind,
    InitializationFailure, Interval, NumericalRefusal, ObservationLens, ObservationSeed, ParameterPacket,
    Seed, SimTime, SourceId, SyntheticFixture,
};
use ng_observe::{Observation, ObservedValue, Observer};
use ng_record::{
    CheckpointError, FaultReport, Faulted, HistorySeal, HistoryStore, ObservationRefusal, Running, Session,
    Step, StepReceipt,
};

pub(crate) fn accept<'m>(
    session: Session<'m, Running>,
) -> Result<(Session<'m, Running>, StepReceipt), Session<'m, Faulted>> {
    counted(session, &mut NumericalHealth::default())
}

fn counted<'m>(
    session: Session<'m, Running>,
    health: &mut NumericalHealth,
) -> Result<(Session<'m, Running>, StepReceipt), Session<'m, Faulted>> {
    let mut attempt = session.step();
    loop {
        match attempt {
            Step::Accepted(session, receipt) => {
                health.excursions += receipt.excursions().len() as u64;
                return Ok((session, receipt));
            }
            Step::Refused(session) => {
                health.refused(session.refused().reason());
                attempt = session.retry();
            }
            Step::Faulted(session) => {
                if session.fault().kind() == EngineFaultKind::NoAdmissibleStep {
                    health.floor_faults += 1;
                }
                return Err(session);
            }
        }
    }
}

pub(crate) fn advance<'m>(
    session: Session<'m, Running>,
) -> Result<Session<'m, Running>, Session<'m, Faulted>> {
    accept(session).map(|(session, _)| session)
}

/// Steps until the accepted head reaches `end`, retrying every refusal. A fault ends it with the latch.
pub fn run_until<'m>(
    mut session: Session<'m, Running>,
    end: SimTime,
) -> Result<Session<'m, Running>, Session<'m, Faulted>> {
    while session.record().accepted().time() < end {
        session = advance(session)?;
    }
    Ok(session)
}

/// Admission checks the packet's anchors against `lens`.
pub struct ArenaSpec {
    /// Admission at its start; each repeat runs from that start until the head reaches its end.
    pub window: Interval,
    pub packet: ParameterPacket<SyntheticFixture>,
    pub lens: ObservationLens,
    pub boundary: BoundaryPort,
    pub seed: Seed,
    pub repeats: NonZeroU32,
}

/// `run` compiles nothing, so its host times compilation with the same clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThroughputEvidence {
    clock: ClockSource,
    admission_wall_ns: u64,
    runs_wall_ns: Vec<NonZeroU64>,
    simulated_ns: u64,
    accepted_steps: u64,
    waveform_samples: u64,
}
impl ThroughputEvidence {
    pub fn clock(&self) -> ClockSource {
        self.clock
    }
    pub fn admission_wall_ns(&self) -> u64 {
        self.admission_wall_ns
    }
    /// One per repeat, in order.
    pub fn runs_wall_ns(&self) -> &[NonZeroU64] {
        &self.runs_wall_ns
    }
    /// The head past the window's start, the same in every repeat.
    pub fn simulated_ns(&self) -> u64 {
        self.simulated_ns
    }
    pub fn accepted_steps(&self) -> u64 {
        self.accepted_steps
    }
    /// The values the observer released over one repeat. An absence is no sample, and a reading its delay
    /// held counts once, when its value arrives.
    pub fn waveform_samples(&self) -> u64 {
        self.waveform_samples
    }
}

/// Counted over one repeat, since every repeat must report the same.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NumericalHealth {
    refusals: Vec<(NumericalRefusal, u64)>,
    floor_faults: u64,
    excursions: u64,
}
impl NumericalHealth {
    /// Each reason, with the row or source it names, in the order it first refused.
    pub fn refusals(&self) -> &[(NumericalRefusal, u64)] {
        &self.refusals
    }
    /// The `TrialDomain` refusals per source, in `CompiledBody::refusal_sources` order.
    pub fn by_source(&self) -> Vec<(SourceId, u64)> {
        let mut sources: Vec<(SourceId, u64)> = (self.refusals.iter())
            .filter_map(|&(reason, count)| match reason {
                NumericalRefusal::TrialDomain { source } => Some((source, count)),
                _ => None,
            })
            .collect();
        sources.sort_unstable();
        sources
    }
    /// The `NoAdmissibleStep` faults: at most one, and only in `ArenaFailure::Faulted`.
    pub fn floor_faults(&self) -> u64 {
        self.floor_faults
    }
    /// Over the accepted steps.
    pub fn excursions(&self) -> u64 {
        self.excursions
    }
    fn refused(&mut self, reason: NumericalRefusal) {
        match self.refusals.iter_mut().find(|(seen, _)| *seen == reason) {
            Some((_, count)) => *count += 1,
            None => self.refusals.push((reason, 1)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunEvidence {
    seal: HistorySeal,
    resolution: DeclaredResolution,
    throughput: ThroughputEvidence,
    health: NumericalHealth,
}
impl RunEvidence {
    pub fn run(&self) -> Digest {
        self.seal.run
    }
    pub fn seal(&self) -> HistorySeal {
        self.seal
    }
    pub fn resolution(&self) -> &DeclaredResolution {
        &self.resolution
    }
    pub fn throughput(&self) -> &ThroughputEvidence {
        &self.throughput
    }
    pub fn health(&self) -> &NumericalHealth {
        &self.health
    }
}

pub struct ArenaRun<'m> {
    session: Session<'m, Running>,
    evidence: RunEvidence,
}
impl<'m> ArenaRun<'m> {
    pub fn session(&self) -> &Session<'m, Running> {
        &self.session
    }
    pub fn into_evidence(self) -> RunEvidence {
        self.evidence
    }
}

#[derive(Debug)]
pub enum ArenaFailure {
    Initialization(InitializationFailure),
    /// A latched session, exported for diagnosis, with the health of its repeat up to the fault.
    Faulted {
        report: Box<FaultReport>,
        health: NumericalHealth,
    },
    /// A start the store would not checkpoint or resume, or a fault it would not export.
    Checkpoint(CheckpointError),
    Observation(ObservationRefusal),
    /// A clock with no reading left, one that ran backward, one that stood still over a repeat, or a repeat
    /// whose observer released no value: a throughput that measured nothing.
    MissingThroughput,
    /// A repeat that ended on another seal, waveform count or numerical health than the first, so it timed
    /// other work.
    RepeatDiffered {
        repeat: u32,
    },
    MissingEvidence {
        part: ScorePart,
    },
    /// A core, or its second policy, that declares other error scales or another minimum interval than the
    /// protocol: nothing ran.
    OffProtocol(DeclaredResolution),
    Certificate(CertificateFailure),
    Accuracy(AccuracyFailure),
    Replay(ReplayFailure),
    Day(DayFailure),
}

const WAVEFORM: [u64; 4] = [5, 5, 5, 5];

fn read(clock: &dyn WallClock) -> Result<u64, ArenaFailure> {
    clock.elapsed_ns().ok_or(ArenaFailure::MissingThroughput)
}

fn exported(stopped: Session<'_, Faulted>, health: NumericalHealth) -> ArenaFailure {
    match stopped.export() {
        Ok(report) => ArenaFailure::Faulted {
            report: Box::new(report),
            health,
        },
        Err(error) => ArenaFailure::Checkpoint(error),
    }
}

struct Repeat<'m> {
    session: Session<'m, Running>,
    wall_ns: NonZeroU64,
    samples: u64,
    health: NumericalHealth,
}

fn timed<'m>(
    mut session: Session<'m, Running>,
    lens: &ObservationLens,
    end: SimTime,
    clock: &dyn WallClock,
) -> Result<Repeat<'m>, ArenaFailure> {
    let began = read(clock)?;
    let mut observer = Observer::new(lens, ObservationSeed::new(WAVEFORM));
    let mut samples = 0;
    let mut health = NumericalHealth::default();
    loop {
        let view = session.observation(lens).map_err(ArenaFailure::Observation)?;
        let observed = observer.advance(&view).map_err(ArenaFailure::Observation)?;
        let present = |seen: &&Observation| matches!(seen.value(), ObservedValue::Present(_));
        samples += observed.iter().filter(present).count() as u64;
        if session.record().accepted().time() >= end {
            break;
        }
        session = match counted(session, &mut health) {
            Ok((session, _)) => session,
            Err(stopped) => return Err(exported(stopped, health)),
        };
    }
    let wall_ns = (read(clock)?.checked_sub(began)).and_then(NonZeroU64::new);
    match wall_ns {
        Some(wall_ns) if samples > 0 => Ok(Repeat {
            session,
            wall_ns,
            samples,
            health,
        }),
        _ => Err(ArenaFailure::MissingThroughput),
    }
}

/// Each repeat resumes from the start before its clock reads, so its wall time holds its steps and its observer.
pub fn run<'m>(
    body: &'m CompiledBody,
    solver: &'m dyn CoupledSolver,
    spec: ArenaSpec,
    clock: &dyn WallClock,
    store: &'m dyn HistoryStore,
) -> Result<ArenaRun<'m>, ArenaFailure> {
    let ArenaSpec {
        window,
        packet,
        lens,
        boundary,
        seed,
        repeats,
    } = spec;
    let opened = read(clock)?;
    let (saved, identity, admission_wall_ns) = {
        let start = solved_start(body, packet, boundary, solver, &lens, window.start());
        let session = start.and_then(|start| admit_start(body, solver, start, seed, store));
        let session = session.map_err(ArenaFailure::Initialization)?;
        let admission_wall_ns = (read(clock)?.checked_sub(opened)).ok_or(ArenaFailure::MissingThroughput)?;
        let saved = session.checkpoint().map_err(ArenaFailure::Checkpoint)?;
        (saved, session.record().identity().clone(), admission_wall_ns)
    };
    let repeat = || {
        let resumed =
            Session::resume(&saved, &identity, body, solver, store).map_err(ArenaFailure::Checkpoint);
        timed(resumed?, &lens, window.end(), clock)
    };
    let Repeat {
        mut session,
        wall_ns,
        samples,
        health,
    } = repeat()?;
    let course = (session.record().seal(), samples);
    let mut runs_wall_ns = vec![wall_ns];
    for index in 1..repeats.get() {
        let again = repeat()?;
        if (again.session.record().seal(), again.samples) != course || again.health != health {
            return Err(ArenaFailure::RepeatDiffered { repeat: index });
        }
        runs_wall_ns.push(again.wall_ns);
        session = again.session;
    }
    let record = session.record();
    let throughput = ThroughputEvidence {
        clock: clock.source(),
        admission_wall_ns,
        runs_wall_ns,
        simulated_ns: record.accepted().time().nanoseconds() - window.start().nanoseconds(),
        accepted_steps: record.revision(),
        waveform_samples: samples,
    };
    let evidence = RunEvidence {
        seal: session.conservation().seal(),
        resolution: identity.resolution().clone(),
        throughput,
        health,
    };
    Ok(ArenaRun { session, evidence })
}
