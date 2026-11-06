# 22: The score binds one run's evidence

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** `score` takes one run's certificate, accuracy evidence, run evidence and replay evidence and returns an `ArenaScore`, as the sketch's `score` does (`docs/architecture/solver-contract-sketch/src/ng_validate.rs:428`). Evidence from another run, or from a gate that did not run, is `ArenaFailure::MissingEvidence`, never a pass. `replay_gates` returns evidence bound to the run's identity. The score reports every policy value that its parts rest on.

Blocked by: 12, 13

Status: ready-for-agent

- [ ] `score` returns an `ArenaScore` for one run's certificate, accuracy, run and replay evidence, and refuses each kind taken from another run

## Comments

From 05: `ReplayFailure::Diverged` maps to `ArenaFailure::Replay`, and `ReplayEvidence` has no public constructor.

From the split of ticket 11: the contract says policy values are reported with every result (`docs/architecture/solver-contract.md:652`), but the reference's scales reach a result only through the resolution digest (`crates/ng-contract/src/scheme.rs:260`). The score is where each policy value shows. Ticket 21's checks report beside the score, and if they join `ArenaScore`, the sketch changes in the same PR.

From 15: `AccuracyPolicy::manufactured()` is the model-default policy with every manufactured case, and `AccuracyEvidence` names its policy only by digest. A score can refuse accuracy evidence whose policy digest is not that default's (or, after ticket 16, the default with the slice window) as `MissingEvidence`, since a policy that leaves a case out is a gate that did not run. The tolerances (absolute 5, relative 0, grid 100 s) reach a result only through that digest today.

From ticket 12: `RunEvidence::run()` is the digest of the run identity that `run` admitted, so `score` can compare it with the replay evidence's run. `ArenaFailure` derives nothing, because `FaultReport` has no `Debug`.

From 20: `split_gate` returns only a `SplitQualification`, so none of the runs it measures leaves evidence that `score` could take. The sketch's `EvaluationPlan::UnderMeasurement` is not in the crates yet. Once the block seam lets a declared resolution say a run is under measurement, `score` refuses that run.
