use crate::run::accept;
use crate::{MemoryStore, ReferenceIntegrator, ReplayCase};
use ng_contract::{
    CoupledSolver, EngineFault, InitializationFailure, ObservationLens, ReferenceError, SplitBudget,
    SplitMeasurement, SplitQualification, SplitRejected, SplitTrace, TraceDraw, TraceGrid,
};

#[derive(Debug)]
pub enum SplitFailure {
    Admission(InitializationFailure),
    Faulted(EngineFault),
    Rejected(SplitRejected),
}
impl From<SplitRejected> for SplitFailure {
    fn from(rejected: SplitRejected) -> Self {
        Self::Rejected(rejected)
    }
}

/// The split gate (contract §7). The split is measured against the reference at half its interval, whose own
/// error is its distance from the reference at its interval. The gate returns the qualification and nothing
/// else, so no run it measured is ever scored.
pub fn split_gate(
    case: &dyn ReplayCase,
    split: &dyn CoupledSolver,
    reference: &ReferenceIntegrator,
    lens: &ObservationLens,
    grid: TraceGrid,
    budget: SplitBudget,
) -> Result<SplitQualification, SplitFailure> {
    let coarse = draw_split_trace(case, reference, lens, grid)?;
    let expected = draw_split_trace(case, &reference.refined(), lens, grid)?;
    let error = ReferenceError::self_convergence(&coarse, &expected)?;
    let measured = draw_split_trace(case, split, lens, grid)?;
    Ok(SplitMeasurement::new(expected, measured, error, budget)?.qualify()?)
}

pub fn draw_split_trace(
    case: &dyn ReplayCase,
    core: &dyn CoupledSolver,
    lens: &ObservationLens,
    grid: TraceGrid,
) -> Result<SplitTrace, SplitFailure> {
    let store = MemoryStore::default();
    let mut session = case.admit(core, &store).map_err(SplitFailure::Admission)?;
    let mut draw = TraceDraw::new(case.body(), lens, grid)?;
    session.record().split_trace(&mut draw, None)?;
    while session.record().accepted().time() < grid.end() {
        let (next, receipt) = accept(session).map_err(|stopped| SplitFailure::Faulted(*stopped.fault()))?;
        next.record().split_trace(&mut draw, Some(&receipt))?;
        session = next;
    }
    Ok(draw.finish()?)
}

#[cfg(test)]
mod tests {
    use super::{SplitFailure, split_gate};
    use crate::run::advance;
    use crate::{MemoryStore, ReferenceIntegrator, ReplayCase, TwoReservoirs};
    use core::num::NonZeroU64;
    use ng_contract::{
        ChannelSpec, Compensated, Dimensionless, ErrorScales, Measure, SimTime, SplitBudget, SplitRejected,
        TraceGrid, Water,
    };

    const SECOND: u64 = 1_000_000_000;

    fn every(ns: u64) -> NonZeroU64 {
        NonZeroU64::new(ns).expect("not zero")
    }

    /// The worst distance of either amount from the closed form at the grid's instants.
    fn closed_form_error(case: &TwoReservoirs, core: &ReferenceIntegrator, period_ns: u64) -> Compensated {
        let store = MemoryStore::default();
        let mut session = case.admit(core, &store).expect("admitted");
        let mut worst = Compensated::ZERO;
        for index in 0..1_000 {
            let at = SimTime::from_ns(index * period_ns);
            while session.record().accepted().time() < at {
                let Ok(next) = advance(session) else {
                    panic!("faulted")
                };
                session = next;
            }
            let view = session.record().accepted();
            for (row, exact) in case.rows().into_iter().zip(case.exact(at).amounts) {
                let value = if view.time() == at {
                    view.value(row)
                } else {
                    (view.last_segment().sample(row, at)).expect("an instant inside the step")
                };
                let error = value.minus(Compensated::of(&measure::<Water>(exact))).abs();
                if !error.at_most(worst) {
                    worst = error;
                }
            }
        }
        worst
    }

    fn measure<U>(value: f64) -> Measure<U> {
        Measure::new(value).expect("finite")
    }

    /// The closed form says how far each run is from the truth. The gate measures the split against the
    /// reference's half, so its reference error must cover the half's own error and its total the split's own.
    #[test]
    fn a_split_whose_closed_form_error_is_over_its_budget_is_refused() {
        let case = TwoReservoirs::closed().expect("a synthetic case");
        let kind = (case.body().rows().iter())
            .find(|row| row.id() == case.rows()[0])
            .expect("a row")
            .kind();
        let lens = (case.body())
            .lens(vec![ChannelSpec::new("a synthetic gauge", kind)], every(SECOND))
            .expect("a lens");
        let period_ns = 7 * SECOND;
        let grid = TraceGrid::new(SimTime::from_ns(0), every(period_ns), 1_000).expect("a grid");
        let scale = 1.2;
        let scales = ErrorScales::uniform(measure(scale), measure(0.0)).expect("a positive scale");
        let budget = SplitBudget::new(scales, scales, scales, measure(1.0), measure(0.5)).expect("a share");
        let (reference, half, split) = (
            ReferenceIntegrator::new(8 * SECOND),
            ReferenceIntegrator::new(4 * SECOND),
            ReferenceIntegrator::new(16 * SECOND),
        );
        let normalized =
            |core| closed_form_error(&case, core, period_ns).times(&measure::<Dimensionless>(1.0 / scale));
        let (own, split_own) = (normalized(&half), normalized(&split));
        assert!(
            !split_own.at_most(Compensated::of(&measure::<Dimensionless>(1.0))),
            "the split's own error is over the budget"
        );
        let assessment = match split_gate(&case, &split, &reference, &lens, grid, budget) {
            Err(SplitFailure::Rejected(SplitRejected::OverBudget(assessment))) => assessment,
            other => panic!("not refused over the budget: {other:?}"),
        };
        assert!(own.times(&measure(0.9)).at_most(assessment.reference_error()));
        assert!(split_own.times(&measure(0.95)).at_most(assessment.worst_total()));
    }
}
