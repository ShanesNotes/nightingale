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
