use crate::run::advance;
use crate::{MemoryStore, ReplayCase, TwoReservoirs};
use core::cell::RefCell;
use ng_contract::{
    AcceptedStateView, AdmissionError, Attempt, BoundParameters, ColumnId, Compensated, CoupledProblem,
    CoupledSolver, DeclaredResolution, Digest, Dimensionless, EngineFault, InitializationFailure, Interval,
    Measure, NumericalRefusal, RunIdentity, SimTime, StepProposal, StepTicket, TrialFailure, Water,
};
use ng_record::{Curve, Faulted, HistorySegment, PatientRecord, Session};

const SECOND: u64 = 1_000_000_000;

/// A manufactured case: a two-reservoir body with a closed form, and for some an exact identity beside it.
pub enum AccuracyCase {
    TwoReservoir(Box<TwoReservoirs>),
    /// `a` drained to within one percent of empty and refilled from `b`: a core keeps publishing down to the
    /// admissible bound.
    DrainAndRefill,
    /// The route holds exactly zero flow until an inflow restores it, so a core stays defined at rest.
    ZeroFlowRestore,
    /// Overlapping and back-to-back removals, each compared on the boundary's column, so every removed amount
    /// shows in the ledger its receipt seals.
    ControlledRemoval,
    /// After evaluations the contract refused, the same core on the same head publishes the same bits, and one
    /// stage there gives the same residual and Jacobian bits.
    EvaluationIndependence,
}
impl AccuracyCase {
    /// Every manufactured case §7 lists, in its order.
    pub fn manufactured() -> Result<Vec<Self>, AdmissionError> {
        Ok(vec![
            Self::TwoReservoir(Box::new(TwoReservoirs::stepped_inflow()?)),
            Self::DrainAndRefill,
            Self::ZeroFlowRestore,
            Self::ControlledRemoval,
            Self::EvaluationIndependence,
        ])
    }
}

enum Check {
    ClosedForm,
    Independence { refusing: Box<BoundParameters> },
}

struct Manufactured {
    kind: u8,
    fixture: TwoReservoirs,
    check: Check,
}
impl Manufactured {
    fn of(case: AccuracyCase) -> Result<Self, AdmissionError> {
        let plain = |kind, fixture| Self {
            kind,
            fixture,
            check: Check::ClosedForm,
        };
        Ok(match case {
            AccuracyCase::TwoReservoir(fixture) => plain(0, *fixture),
            AccuracyCase::DrainAndRefill => plain(1, TwoReservoirs::drain_and_refill()?),
            AccuracyCase::ZeroFlowRestore => plain(2, TwoReservoirs::zero_flow_restore()?),
            AccuracyCase::ControlledRemoval => plain(3, TwoReservoirs::controlled_removal()?),
            AccuracyCase::EvaluationIndependence => {
                let fixture = TwoReservoirs::closed()?.until(SimTime::from_ns(2_000 * SECOND));
                Self {
                    kind: 4,
                    check: Check::Independence {
                        refusing: Box::new(fixture.overflowing_values()?),
                    },
                    fixture,
                }
            }
        })
    }
}

/// `absolute + relative * |exact|` bounds the error at each compared instant, and `grid_ns` spaces the
/// instants compared inside a segment.
pub struct AccuracyPolicy {
    cases: Vec<Manufactured>,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    grid_ns: u64,
}
impl AccuracyPolicy {
    /// Refuses a zero grid, no case and a case that ends where it starts.
    pub fn new(
        cases: Vec<AccuracyCase>,
        absolute: Measure<Water>,
        relative: Measure<Dimensionless>,
        grid_ns: u64,
    ) -> Result<Self, AdmissionError> {
        let cases = cases
            .into_iter()
            .map(Manufactured::of)
            .collect::<Result<Vec<_>, _>>()?;
        let never_steps = |case: &Manufactured| case.fixture.end() == SimTime::from_ns(0);
        if grid_ns == 0 || cases.is_empty() || cases.iter().any(never_steps) {
            return Err(AdmissionError::EmptyAccuracyPolicy);
        }
        Ok(Self {
            cases,
            absolute: Compensated::of(&absolute),
            relative,
            grid_ns,
        })
    }
    /// Every manufactured case under the model defaults.
    pub fn manufactured() -> Result<Self, AdmissionError> {
        let absolute = Measure::new(5.0)?;
        Self::new(
            AccuracyCase::manufactured()?,
            absolute,
            Measure::new(0.0)?,
            100 * SECOND,
        )
    }
    pub fn digest(&self) -> Digest {
        let Self {
            cases,
            absolute,
            relative,
            grid_ns,
        } = self;
        let mut bytes = grid_ns.to_be_bytes().to_vec();
        for limb in absolute
            .bits()
            .into_iter()
            .chain(Compensated::of(relative).bits())
        {
            bytes.extend_from_slice(&limb.to_be_bytes());
        }
        for Manufactured {
            kind,
            fixture,
            check: _,
        } in cases
        {
            bytes.push(*kind);
            bytes.extend_from_slice(fixture.digest().as_bytes());
        }
        Digest::of_canonical(&bytes)
    }
}
/// The worst error per case. Accepted values only: a trial state is never read.
#[derive(Debug)]
pub struct AccuracyEvidence {
    policy: Digest,
    worst: Vec<Compensated>,
}
impl AccuracyEvidence {
    pub fn policy(&self) -> Digest {
        self.policy
    }
    pub fn worst(&self) -> &[Compensated] {
        &self.worst
    }
}
#[derive(Debug)]
pub enum AccuracyFailure {
    /// The first compared instant above the policy, in time order.
    Exceeded {
        case: usize,
        at: SimTime,
        error: Compensated,
    },
    Faulted(EngineFault),
    Admission(InitializationFailure),
    /// Adjacent refinements' errors are outside the band, or the finer error is zero.
    Order {
        case: usize,
        refinement: usize,
    },
    /// Adjacent refinements ran under different policies.
    MixedEvidence {
        refinement: usize,
    },
    /// The core's attempt on refusing values was not refused for its domain, so no evaluation was refused.
    NothingRefused {
        case: usize,
    },
    /// After refused evaluations, the head published other bits, or its stage gave other residual or Jacobian
    /// bits.
    Dependent {
        case: usize,
    },
}

fn water(value: f64) -> Compensated {
    Compensated::of(&Measure::<Water>::new(value).expect("the closed form of finite constants is finite"))
}

fn net_at(segment: &HistorySegment, column: ColumnId, at: SimTime) -> Compensated {
    let piece = Interval::new(segment.interval().start(), at).expect("a compared instant is after the start");
    let reached = |curve: Curve<'_>| {
        let ends = curve.on(piece).expect("a compared instant is inside the segment");
        ends[ends.len() - 1]
    };
    let (forward, reverse) = segment.exchange(column);
    reached(forward).minus(reached(reverse))
}

fn compared_instants(interval: Interval, grid_ns: u64) -> Vec<SimTime> {
    let (start, end) = (interval.start().nanoseconds(), interval.end().nanoseconds());
    let first = (start / grid_ns + 1) * grid_ns;
    let mut instants: Vec<SimTime> = (first..end)
        .step_by(usize::try_from(grid_ns).unwrap_or(usize::MAX))
        .map(SimTime::from_ns)
        .collect();
    instants.push(interval.end());
    instants
}

fn comparisons(
    case: &TwoReservoirs,
    record: &PatientRecord<'_>,
    segment: &HistorySegment,
    at: SimTime,
) -> Vec<(Compensated, Compensated)> {
    let exact = case.exact(at);
    let dense = record.accepted().last_segment();
    let mut pairs = Vec::with_capacity(4);
    for (row, amount) in case.rows().into_iter().zip(exact.amounts) {
        let value = dense
            .sample(row, at)
            .expect("a compared instant is inside the segment");
        pairs.push((value, water(amount)));
    }
    let circulation = case.circulation().expect("the body circulates");
    pairs.push((net_at(segment, circulation, at), water(exact.circulated)));
    if let Some(exterior) = case.exterior() {
        pairs.push((net_at(segment, exterior, at), water(exact.received)));
    }
    pairs
}

/// Runs each case with the same core. After every accepted step it compares the accepted segment with the
/// closed form: both amounts, the circulation exchange and any boundary exchange. A core that diverges fails
/// here at its first bad instant, before positivity can intervene.
pub fn accuracy(
    solver: &dyn CoupledSolver,
    policy: &AccuracyPolicy,
) -> Result<AccuracyEvidence, AccuracyFailure> {
    let mut worst = Vec::with_capacity(policy.cases.len());
    for (index, case) in policy.cases.iter().enumerate() {
        if let Check::Independence { refusing } = &case.check {
            independent(solver, &case.fixture, refusing, index)?;
        }
        worst.push(closed_form(solver, &case.fixture, policy, index)?);
    }
    Ok(AccuracyEvidence {
        policy: policy.digest(),
        worst,
    })
}

fn faulted(stopped: Session<'_, Faulted>) -> AccuracyFailure {
    AccuracyFailure::Faulted(*stopped.fault())
}

fn closed_form(
    solver: &dyn CoupledSolver,
    case: &TwoReservoirs,
    policy: &AccuracyPolicy,
    index: usize,
) -> Result<Compensated, AccuracyFailure> {
    let store = MemoryStore::default();
    let mut session = case.admit(solver, &store).map_err(AccuracyFailure::Admission)?;
    let mut largest = Compensated::ZERO;
    while session.record().accepted().time() < case.end() {
        session = advance(session).map_err(faulted)?;
        let record = session.record();
        let Some(interval) = record.accepted().last_segment().interval() else {
            continue;
        };
        let segment = session.conservation().head().expect("a step was accepted");
        for at in compared_instants(interval, policy.grid_ns) {
            for (value, exact) in comparisons(case, record, &segment, at) {
                let error = value.minus(exact).abs();
                if !error.at_most(policy.absolute.plus(exact.abs().times(&policy.relative))) {
                    return Err(AccuracyFailure::Exceeded {
                        case: index,
                        at,
                        error,
                    });
                }
                if !error.at_most(largest) {
                    largest = error;
                }
            }
        }
    }
    Ok(largest)
}

fn independent(
    solver: &dyn CoupledSolver,
    case: &TwoReservoirs,
    refusing: &BoundParameters,
    index: usize,
) -> Result<(), AccuracyFailure> {
    let clean_store = MemoryStore::default();
    let clean = case
        .admit(solver, &clean_store)
        .map_err(AccuracyFailure::Admission)?;
    let stage_before = stage_bits(solver, clean.record());
    let clean = advance(clean).map_err(faulted)?;

    let store = MemoryStore::default();
    let session = case.admit(solver, &store).map_err(AccuracyFailure::Admission)?;
    let record = session.record();
    let refused = Refusing {
        core: solver,
        values: refusing,
        identity: record.identity().clone(),
    };
    let span_ns = solver.next_step(record.accepted());
    let reason = match record.attempt(&refused, Attempt::First { span_ns }) {
        Err(TrialFailure::Refused(refusal)) => Some(refusal.reason()),
        Err(TrialFailure::EngineFault(fault)) => fault.last_refusal(),
        Ok(_) => None,
    };
    if !matches!(reason, Some(NumericalRefusal::TrialDomain { .. })) {
        return Err(AccuracyFailure::NothingRefused { case: index });
    }
    let stage_after = stage_bits(solver, record);
    let session = advance(session).map_err(faulted)?;
    let same_stage = stage_before.is_some() && stage_before == stage_after;
    if !same_stage || clean.record().seal() != session.record().seal() {
        return Err(AccuracyFailure::Dependent { case: index });
    }
    Ok(())
}

struct Refusing<'c> {
    core: &'c dyn CoupledSolver,
    values: &'c BoundParameters,
    identity: RunIdentity,
}
impl CoupledSolver for Refusing<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let refusing = CoupledProblem::new(
            problem.body(),
            self.values,
            &self.identity,
            problem.accepted(),
            ticket.digest(),
        );
        self.core.solve(&refusing, ticket)
    }
}

struct StageAtHead<'c> {
    core: &'c dyn CoupledSolver,
    bits: RefCell<Option<Vec<u64>>>,
}
impl CoupledSolver for StageAtHead<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        let length = ticket.length_ns();
        let workspace = problem.workspace();
        let point = workspace.point(ticket.at(length)?);
        let stage = problem.evaluate(&point)?;
        let jacobian = problem.linearize(&point, ticket.shift(length)?)?;
        let values = stage.residuals().iter().chain(jacobian.view().values());
        *self.bits.borrow_mut() = Some(values.map(|value| value.to_bits()).collect());
        Err(ticket.refuse(NumericalRefusal::IterationLimit))
    }
}

fn stage_bits(core: &dyn CoupledSolver, record: &PatientRecord<'_>) -> Option<Vec<u64>> {
    let probe = StageAtHead {
        core,
        bits: RefCell::new(None),
    };
    let span_ns = core.next_step(record.accepted());
    let _published_nothing = record.attempt(&probe, Attempt::First { span_ns });
    probe.bits.into_inner()
}

/// The ratio test for a core's observed order: for a refinement by two and order `p`, each error over the
/// next must lie in `[low, high]` around `2^p`. A ratio states the order with no logarithm.
pub fn observed_order(
    ladder: [&AccuracyEvidence; 3],
    low: Measure<Dimensionless>,
    high: Measure<Dimensionless>,
) -> Result<(), AccuracyFailure> {
    for (refinement, pair) in ladder.windows(2).enumerate() {
        if pair[0].policy != pair[1].policy {
            return Err(AccuracyFailure::MixedEvidence { refinement });
        }
        for (case, (coarse, fine)) in pair[0].worst.iter().zip(&pair[1].worst).enumerate() {
            // An exact answer has no ratio to read, so it fails too.
            let within = !fine.at_most(Compensated::ZERO)
                && fine.times(&low).at_most(*coarse)
                && coarse.at_most(fine.times(&high));
            if !within {
                return Err(AccuracyFailure::Order { case, refinement });
            }
        }
    }
    Ok(())
}
