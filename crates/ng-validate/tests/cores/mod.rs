pub mod forced_refusal;
pub mod must_fail;
pub mod stages;

use ng_contract::*;

const MILLISECOND: u64 = 1_000_000;

pub fn resolution(scheme: StageScheme, tag: &[u8], budget: u32) -> DeclaredResolution {
    let measure = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let scales = ErrorScales::uniform(measure(1.0), measure(1e-5)).expect("positive");
    DeclaredResolution::new(
        Digest::of_canonical(tag),
        MILLISECOND,
        budget,
        ArithmeticProfile::contract_v1(),
        scheme,
        scales,
    )
}

pub fn fresh_history(problem: &CoupledProblem<'_>, ticket: &StepTicket) -> MethodHistory {
    let schema = problem.accepted().method_history().schema();
    MethodHistory::new(schema, ticket.interval().start(), Vec::new())
}
