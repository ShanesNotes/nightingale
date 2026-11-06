use crate::initialize::{admit_start, solved_start};
use crate::{ClockSource, WallClock};
use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::{
    BoundaryPort, CompiledBody, CoupledSolver, Digest, InitializationFailure, Interval, ObservationLens,
    ObservationSeed, ParameterPacket, Seed, SimTime, SyntheticFixture,
};
use ng_observe::{Observation, ObservedValue, Observer};
use ng_record::{
    CheckpointError, FaultReport, Faulted, HistoryStore, ObservationRefusal, Running, Session, Step, StepReceipt,
};

pub(crate) fn accept<'m>(
    session: Session<'m, Running>,
) -> Result<(Session<'m, Running>, StepReceipt), Session<'m, Faulted>> {
    let mut attempt = session.step();
    loop {
        match attempt {
            Step::Accepted(session, receipt) => return Ok((session, receipt)),
            Step::Refused(session) => attempt = session.retry(),
            Step::Faulted(session) => return Err(session),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunEvidence {
    run: Digest,
    throughput: ThroughputEvidence,
}
impl RunEvidence {
    pub fn run(&self) -> Digest {
        self.run
    }
    pub fn throughput(&self) -> &ThroughputEvidence {
        &self.throughput
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

pub enum ArenaFailure {
    Initialization(InitializationFailure),
    Faulted(FaultReport),
    /// A start the store would not checkpoint or resume, or a fault it would not export.
    Checkpoint(CheckpointError),
    Observation(ObservationRefusal),
    /// A clock with no reading left, one that ran backward, one that stood still over a repeat, or a repeat
    /// whose observer released no value: a throughput that measured nothing.
    MissingThroughput,
    RepeatDiffered {
        repeat: u32,
    },
}

const WAVEFORM: [u64; 4] = [5, 5, 5, 5];

fn read(clock: &dyn WallClock) -> Result<u64, ArenaFailure> {
    clock.elapsed_ns().ok_or(ArenaFailure::MissingThroughput)
}

fn exported(stopped: Session<'_, Faulted>) -> ArenaFailure {
    match stopped.export() {
        Ok(report) => ArenaFailure::Faulted(report),
        Err(error) => ArenaFailure::Checkpoint(error),
    }
}

struct Repeat<'m> {
    session: Session<'m, Running>,
    wall_ns: NonZeroU64,
    samples: u64,
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
    loop {
        let view = session.observation(lens).map_err(ArenaFailure::Observation)?;
        let observed = observer.advance(&view).map_err(ArenaFailure::Observation)?;
        let present = |seen: &&Observation| matches!(seen.value(), ObservedValue::Present(_));
        samples += observed.iter().filter(present).count() as u64;
        if session.record().accepted().time() >= end {
            break;
        }
        session = advance(session).map_err(exported)?;
    }
    let wall_ns = (read(clock)?.checked_sub(began)).and_then(NonZeroU64::new);
    match wall_ns {
        Some(wall_ns) if samples > 0 => Ok(Repeat {
            session,
            wall_ns,
            samples,
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
    } = repeat()?;
    let course = (session.record().seal(), samples);
    let mut runs_wall_ns = vec![wall_ns];
    for index in 1..repeats.get() {
        let again = repeat()?;
        if (again.session.record().seal(), again.samples) != course {
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
        run: record.identity().digest(),
        throughput,
    };
    Ok(ArenaRun { session, evidence })
}
