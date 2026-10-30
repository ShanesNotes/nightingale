//! The accuracy gate: a core against known answers, because a step can close every ledger and be wrong.
use crate::TwoReservoirs;
use crate::run::advance;
use ng_contract::{
    AdmissionError, ColumnId, Compensated, CoupledSolver, Dimensionless, EngineFault, InitializationFailure,
    Interval, Measure, SimTime, Water,
};
use ng_record::{HistorySegment, PatientRecord};

/// A manufactured case with a closed form. Ticket 11 adds the drain, zero-flow, removal, independence and
/// slice cases, the last against the reference integrator on a body it is handed.
pub enum AccuracyCase {
    TwoReservoir(TwoReservoirs),
}

/// A model default reported with every result. `absolute + relative * exact` bounds the error at each
/// compared instant, and `grid_ns` spaces the instants compared inside a segment.
pub struct AccuracyPolicy {
    cases: Vec<AccuracyCase>,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    grid_ns: u64,
}
impl AccuracyPolicy {
    /// Refuses a zero grid and no case.
    pub fn new(
        cases: Vec<AccuracyCase>,
        absolute: Measure<Water>,
        relative: Measure<Dimensionless>,
        grid_ns: u64,
    ) -> Result<Self, AdmissionError> {
        if grid_ns == 0 || cases.is_empty() {
            return Err(AdmissionError::EmptyAccuracyPolicy);
        }
        Ok(Self {
            cases,
            absolute: Compensated::of(&absolute),
            relative,
            grid_ns,
        })
    }
}
/// The worst error per case. Accepted values only: a trial state is never read.
#[derive(Debug)]
pub struct AccuracyEvidence {
    worst: Vec<Compensated>,
}
impl AccuracyEvidence {
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
    /// The core could not finish the case. A must-fail core that reaches this has not failed on accuracy.
    Faulted(EngineFault),
    Admission(InitializationFailure),
    /// Adjacent refinements' errors are outside the band.
    Order {
        case: usize,
        refinement: usize,
    },
}

fn water(value: f64) -> Compensated {
    Compensated::of(&Measure::<Water>::new(value).expect("the closed form of finite constants is finite"))
}

/// The value a column's net extent has reached at `at`, inside `segment`.
fn net_at(segment: &HistorySegment<'_>, column: ColumnId, at: SimTime) -> Compensated {
    let piece = Interval::new(segment.interval().start(), at).expect("a compared instant is after the start");
    let (forward, reverse) = segment.exchange(column);
    let (forward, reverse) = (forward.on(piece), reverse.on(piece));
    forward[forward.len() - 1].minus(reverse[reverse.len() - 1])
}

/// Each grid instant inside a segment, start excluded, then the segment's end, so a breakpoint is always
/// compared.
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

/// Each accepted value at `at` beside its closed form.
fn comparisons(
    case: &TwoReservoirs,
    record: &PatientRecord<'_>,
    at: SimTime,
) -> Vec<(Compensated, Compensated)> {
    let exact = case.exact(at);
    let dense = record.accepted().last_segment();
    let view = record.conservation();
    let segment = view.segments().next_back().expect("a step was accepted");
    let mut pairs = Vec::with_capacity(4);
    for (row, amount) in case.rows().into_iter().zip(exact.amounts) {
        let value = dense
            .sample(row, at)
            .expect("a compared instant is inside the segment");
        pairs.push((value, water(amount)));
    }
    let circulation = case.circulation().expect("the body circulates");
    pairs.push((net_at(&segment, circulation, at), water(exact.circulated)));
    if let Some(exterior) = case.exterior() {
        pairs.push((net_at(&segment, exterior, at), water(exact.received)));
    }
    pairs
}

/// Runs each case with the same core. After every accepted step it compares the accepted dense segment
/// with the closed form: both amounts, the circulation exchange and the gross inflow. A core that diverges fails
/// here at its first bad instant, before positivity can intervene.
pub fn accuracy(
    solver: &dyn CoupledSolver,
    policy: &AccuracyPolicy,
) -> Result<AccuracyEvidence, AccuracyFailure> {
    let mut worst = Vec::with_capacity(policy.cases.len());
    for (index, AccuracyCase::TwoReservoir(case)) in policy.cases.iter().enumerate() {
        let mut session = case.admit(solver).map_err(AccuracyFailure::Admission)?;
        let mut largest = Compensated::ZERO;
        while session.record().accepted().time() < case.end() {
            session = advance(session).map_err(|stopped| AccuracyFailure::Faulted(*stopped.fault()))?;
            let record = session.record();
            let Some(interval) = record.accepted().last_segment().interval() else {
                continue;
            };
            for at in compared_instants(interval, policy.grid_ns) {
                for (value, exact) in comparisons(case, record, at) {
                    let error = value.minus(exact).abs();
                    if error > policy.absolute.plus(exact.abs().times(&policy.relative)) {
                        return Err(AccuracyFailure::Exceeded {
                            case: index,
                            at,
                            error,
                        });
                    }
                    if error > largest {
                        largest = error;
                    }
                }
            }
        }
        worst.push(largest);
    }
    Ok(AccuracyEvidence { worst })
}

/// The ratio test for a core's observed order: for a refinement by two and order `p`, each error over the
/// next must lie in `[low, high]` around `2^p`. A ratio states the order with no logarithm.
pub fn observed_order(
    ladder: [&AccuracyEvidence; 3],
    low: Measure<Dimensionless>,
    high: Measure<Dimensionless>,
) -> Result<(), AccuracyFailure> {
    for (refinement, pair) in ladder.windows(2).enumerate() {
        for (case, (coarse, fine)) in pair[0].worst.iter().zip(&pair[1].worst).enumerate() {
            if *coarse < fine.times(&low) || *coarse > fine.times(&high) {
                return Err(AccuracyFailure::Order { case, refinement });
            }
        }
    }
    Ok(())
}
