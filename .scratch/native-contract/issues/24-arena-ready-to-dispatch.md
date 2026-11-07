# 24: Arena ready to dispatch

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An arena seat contract can name the validation crate's entry point and the report it returns, so solver-A and solver-B are built and scored against one known interface. The report keeps the certificate, accuracy, replay, numerical health, throughput and day-check results apart.

Blocked by: 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23

Status: ready-for-agent

- [ ] An arena seat contract names the validation crate's entry point and report schema, and both exist on main

## Comments

From the split of ticket 11: this ticket keeps the last check of the old ticket 11, "Arena harness ready to dispatch". Tickets 11 to 23 hold its other claims, one each, so each lands in a small PR. Foundation 01 waits on this ticket.

From foundation 04: each arena seat carries its own trace, and the pick PR keeps the winner's.

From the split of ticket 11: two open issues may move the slice's bits before dispatch, nightingale#38 (fire admission guards at their exact zero) and nightingale#39 (fluid route request order reaches the model hash).

From the solver arena (in this PR): the first pick, the core in `crates/ng-solver`, was made before this harness existed. A throwaway harness outside the repo scored two candidate cores with main's own checks, a 06:55 to 07:30 slice run and a script for the body-response checks. Rescore that pick here once tickets 21 and 24 land, and record whether it stands. The PR links the evidence.

From ticket 14: the arena's certificate policy is `SlidingWindows::new(24 h, 48 h)`. A short slice window certifies with a shorter window (`crates/ng-validate/tests/slice_certificate.rs`). Drift limits there are synthetic, a uniform 1e4 in each unit, because the heart's work moves energy by about 2.3e3 a second.

From ticket 12: `ArenaSpec::repeats` sets how many times the window runs, and the evidence keeps each repeat's wall time. `tools/bench`'s `arena <interval ms> <core> <simulated s> <repeats>` mode prints that evidence under the host clock. Three speed-gate items are still open. No machine is named. §7's checkpoint cadence is not part of the timed run. The slowest steps' wall time that SRS section 5 asks for is not reported (foundation 09).

From 18: the slice's replay entry is `replay_gates(&SliceCase::new()?, cores, plan)`. The plan the reference core passes is in `a_slice_window_passes_every_replay_gate_with_the_reference_core` (`crates/ng-validate/tests/arena_body.rs`): a scripted clock, chunks of 70 ms and 130 ms + 1 ns, `resume_every` 1000, a second policy at twice the step and the floor as the distance. All seven gates take about 62 s at 3 ms on Omarchy, the resume gate about 28 s of it. `SliceCase` keeps one start per core resolution, with and without the inputs, and each new resolution costs one admission solve, about 6.5 s.

From 22: `score(certificate, accuracy, evidence, replay)` returns an `ArenaScore`, whose `certificate()`, `accuracy()`, `evidence()` and `replay()` each state the policy values they were judged by. A part of another run is `ArenaFailure::MissingEvidence { part }`. Certify `arena.session().conservation()` before `into_evidence()`, because the certificate must certify the exact `HistorySeal` the run ended on. Accuracy evidence must come from the run's whole `DeclaredResolution` and must have run every manufactured case under the default bound and grid. Cases beside them are allowed, and nothing requires the slice window yet: tighten `score` to require it once ticket 16's `SliceWindow` lands. The accuracy gate runs the manufactured cases at the arena core's own step. Those cases cover about 34,000 simulated seconds, which is about 6.8 million steps at a 5 ms step, each case in its own `MemoryStore` (arithmetic). The run identity reads no boundary port, so replay evidence on another timeline of the same identity passes. `ArenaFailure` has no `Certificate`, `Accuracy` or `Replay` variant yet, so an entry point that composes the gates needs them.
