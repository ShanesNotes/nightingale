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

From 16: the first slice window passes `ng_solver::Core` capped at 10, 5, 2.5 and 1.25 ms, at shares of 0.84, 0.56, 0.50 and 0.44 of its distances. That is measured on the tower, one run each. The window runs §7's events on a compressed day of 6 s. A window over §6's own day, from 06:55 to the 07:30 meal, needs a history store outside the process: by arithmetic its finest rung writes about 63 GB, and its three rungs take about 2.3 hours on the tower.

From 22: `score(certificate, accuracy, evidence, replay)` returns an `ArenaScore`, whose `certificate()`, `accuracy()`, `evidence()` and `replay()` each state the policy values they were judged by. A part of another run is `ArenaFailure::MissingEvidence { part }`. Certify `arena.session().conservation()` before `into_evidence()`, because the certificate must certify the exact `HistorySeal` the run ended on. Accuracy evidence must come from the run's whole `DeclaredResolution` under `AccuracyPolicy::arena()`, every manufactured case and `SliceWindow::first()` at the default bound and grid. `score` reads that from the cases the evidence ran without building the window again, since `first()` is the only window: a second window constructor needs its own identity in that check. The accuracy gate runs the manufactured cases at the arena core's own step. Those cases cover about 34,000 simulated seconds, which is about 6.8 million steps at a 5 ms step, each case in its own `MemoryStore` (arithmetic). The run identity reads no boundary port, so replay evidence on another timeline of the same identity passes. `ArenaFailure` has no `Certificate`, `Accuracy` or `Replay` variant yet, so an entry point that composes the gates needs them.

From ticket 21: `day_checks` returns a `DayReport` for the report to hold whole, beside the score. `tools/bench`'s `day <interval ms> <core> <simulated s>` prints it. On the tower, ng-solver at 10 ms from 06:55 to 07:30 passes rows 1, 3, 5 and 6 and fails 2, 4 and 7, as the arena script did (308 s, 8.3 GB of link log, measured). A run to 11:30 (`day 10 ng-solver 16500`) also runs the 07:30 meal's counterfactual and scores rows 9 and 10. By arithmetic from that run, it needs about 80 min of wall time and about 130 GB of link log.

From 23: `tools/gate/must-fail.toml` pairs every row of §7's must-fail table with the tests and compile_fail doctests that reject its core, and 05-seam refuses a row left unpaired or a named test that is gone. A row added to §7, or a must-fail test renamed, needs its entry in the same PR. The late-admission gate now refuses a case whose withheld start already holds an input, as `NoLateBoundary`, so a seat's own `ReplayCase` must withhold for real. Three rows rest on the nearest the crates can build: a held pool a slice law moves stands for a held concentration until a held reservoir lands with the second book, `CargoStranded` stands for a reaction slipped into the schema, and the due-input row is held by types and the record's snapshot, because no core can apply an input.
