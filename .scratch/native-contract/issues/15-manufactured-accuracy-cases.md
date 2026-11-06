# 15: The accuracy gate runs every manufactured case

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** `accuracy` runs every manufactured case that §7 lists, not only `AccuracyCase::TwoReservoir`. The new cases are drain and refill to the admissible bound, zero flow and restore, controlled removal with every removed amount in its receipt, and evaluation independence, where one stage gives identical residual and Jacobian bits before and after refused evaluations. Each case has a closed form or an exact identity, and each ships with a core built to fail it.

Blocked by: None (can start immediately)

Status: resolved

- [x] The reference core passes each manufactured case in §7's accuracy gate, and a core built to fail each case is rejected

## Answer

`AccuracyCase` now names every manufactured case §7 lists, as the sketch does: `TwoReservoir`, `DrainAndRefill`, `ZeroFlowRestore`, `ControlledRemoval` and `EvaluationIndependence`. `AccuracyCase::manufactured()` lists them in §7's order, and `AccuracyPolicy::manufactured()` runs them all under the model defaults: absolute 5, relative 0, grid 100 s. The four new cases carry no data. The policy builds each one's fixture, so a caller cannot run a weaker body under a case's name. Each case is a two-reservoir body checked against the closed form `TwoReservoirs::exact` already gives, and the policy digest seals each case's kind with its fixture.

- **Drain and refill.** One removal of 7550 out of `a` over 500 to 2010 s takes `a` from 5050 to about 37, under one percent of its start, and `b` refills it once the removal stops.
- **Zero flow and restore.** Equal amounts of 500000 hold the route at exactly zero flow until an inflow of 10000 over 1000 to 3000 s restores it.
- **Controlled removal.** Three removals out of `a`: two overlap and the third starts as the first ends. The boundary's column is compared with the declared removals at every compared instant, so every removed amount shows in the ledger that each step's receipt seals.
- **Evaluation independence.** On the closed body, the gate admits the case twice. On the second admission it drives the core by hand (`PatientRecord::attempt`) on values whose evaluations the contract refuses, a conductance of `f64::MAX`, and requires a `TrialDomain` refusal. The first step from each admission must publish the same seal, and one stage at the head, read before and after, must give the same residual and Jacobian bits. `AccuracyFailure::Dependent` and `AccuracyFailure::NothingRefused` are new.

The cores built to fail the cases live in `crates/ng-validate/tests/accuracy.rs`, and one table holds every core against every case. Each core passes the stepped inflow, which is all the gate ran before this ticket.

| Core | Stepped | Drain | Zero flow | Removal | Independence |
|---|---|---|---|---|---|
| reference at 16 s | pass | pass | pass | pass | pass |
| refuses a head where one amount is under a tenth of another | pass | no step | pass | pass | pass |
| takes a falling total for a leak | pass | no step | pass | no step | pass |
| Newton stop relative to the stage's first residual, 0 / 0 at rest | pass | pass | no step | pass | pass |
| halves its Newton damping after a refused evaluation | pass | pass | pass | pass | dependent |

The reference's worst errors at 16, 8 and 4 s steps are 3.69, 1.84 and 0.92 (stepped), 3.81, 1.90 and 0.95 (drain), 3.70, 1.85 and 0.92 (zero flow), 2.82, 1.42 and 0.71 (removal) and 1.48, 0.74 and 0.37 (independence), so every case reads first order. `ng_solver::Core` at a 16 s cap gives the same worst errors and passes all five.

Two checks guard the frame and no core can fail them. A boundary's exchange is the frozen drive times the step for any core that `verify` passes, so the removal column catches a record or quadrature fault, not a core. The stage read at the head reads the contract's evaluator, not the core; the seal comparison is what rejects a core.

## Comments

From 04: `accuracy(solver, policy)` runs `AccuracyCase::TwoReservoir` only. Its comparison of gross inflow cannot fail today, because the frozen drive makes it exact for any step that `verify` passes. It guards the frame itself.

From the split of ticket 11: the accuracy tolerances are policy values. Each takes a model default, set in the validation crate and reported with every result (contract §7, foundation 07).
