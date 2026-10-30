//! Cores written on the public seam alone. Each must-fail core does one wrong thing, so a gate that passes
//! it has lost its check.
pub mod forced_refusal;
pub mod must_fail;
pub mod stages;

use ng_contract::*;

const MILLISECOND: u64 = 1_000_000;

/// A core's declared settings: fixture values, not physiology.
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

/// A one-step core's continuation: it remembers nothing past the step it just took.
pub fn fresh_history(problem: &CoupledProblem<'_>, ticket: &StepTicket) -> MethodHistory {
    let schema = problem.accepted().method_history().schema();
    MethodHistory::new(schema, ticket.interval().start(), Vec::new())
}
