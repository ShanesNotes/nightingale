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

From ticket 14: the certificate's closure floor still has no model default: each caller builds `QuantityPolicy` per combination, and the slice passes at absolute 0 and relative 1e-12, with a worst residual of 1.5e-25. The policy names its window and coverage only through `CertificatePolicy::digest`. `Certificate` returns its span, `coverage()` and `support()`, so the score can show the held support each part rests on.

From ticket 12: `RunEvidence::run()` is the digest of the run identity that `run` admitted, so `score` can compare it with the replay evidence's run. `ArenaFailure` derives nothing, because `FaultReport` has no `Debug`.

From 13: `RunEvidence::health()` is the run's `NumericalHealth`, counted over one repeat, and a repeat whose health differs is `RepeatDiffered`. It has no public constructor. `ArenaFailure::Faulted` is now `{ report: Box<FaultReport>, health }`, so a faulted run also shows its refusals up to the fault. A floor fault counts every `NoAdmissibleStep`, whether the floor or the retry budget ended it.

From 18: `SliceCase` admits `arena_packet` at `anchor_moment` with seed [1, 2, 3, 4] through the same solve and admission as `run`, and the run identity reads no boundary port. So `run` on that packet, start, seed and core should name the run that `replay_gates` on `SliceCase` returns (inferred from the code, not run). Every gate's evidence on the slice names one run (`a_slice_window_passes_every_replay_gate_with_the_reference_core`).

From 16: the arena's accuracy policy is `AccuracyPolicy::new` over `AccuracyCase::manufactured()` with `AccuracyCase::SliceWindow(Box::new(SliceWindow::first()?))` added, at absolute 5, relative 0 and grid 100 s. `manufactured()` itself stays without the window, so a score that accepts only `manufactured()`'s digest accepts evidence that never ran the window: bind it to the policy with the window. The cores the window passes step in milliseconds, and the manufactured cases run for thousands of seconds, so an arena core on that policy runs the manufactured cases at its own cap (measure that cost before choosing). Building the window runs the reference three times, about 33 s on the tower, and can fail as an `AccuracyFailure`. Its worst is the largest share of its distance that any quantity or transition reached, and 1 or less passes. Its policy values (the factor of two, the floor of a millionth of each quantity's reach, the 0.1 ms landing tolerance) reach a result only through its digest, which the policy digest seals. Do not hold the window to a first-order `observed_order` band: on the slice its worst barely falls with the step.

From 20: `split_gate` returns only a `SplitQualification`, so none of the runs it measures leaves evidence that `score` could take. The sketch's `EvaluationPlan::UnderMeasurement` is not in the crates yet. Once the block seam lets a declared resolution say a run is under measurement, `score` refuses that run.
