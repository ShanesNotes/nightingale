# 31: The slice window misses drift that builds across intervals

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7, and `crates/ng-validate/src/window.rs` (`over_interval`, `Measured`)

Type: task

Status: resolved

Claimed by: nc31-build

Blocked by: none

Ticket 28 changed how the slice window judges an exchange column: by its net over each grid interval (`over_interval`), not by its running total since admission. A running total carried each interval's signed error into the next. Moving the start to the beat onset put the 16 s reference outside the window: ratio 1.058, column 246. That failure was read as the grid's phase in the beat, and it was not: the running total fails the 16 s reference at 1.057 to 1.058 at all sixteen grid phases this ticket's plan measured. On the collecting duct to bladder flow (columns 245 to 247: water, sodium and potassium) the honest cores carry a one-sign error of their own, 1.8 to 2.3 times the coarse refinement's from the third beat edge on, and a running total adds it up. Its size is 3e-5 of the flow for water and 8e-5 for sodium and potassium: a coarser step's own error. The developer reviewer accepted the per-interval statistic for nightingale#91.

The per-interval statistic is blind to slow drift. Suppose a core's error is small enough in every interval to pass, but always the same sign. Its running total then grows without bound, and no row of the window sees it. On main the new statistic is looser for 142 of 257 exchange columns, and the 16 s reference's worst ratio falls from 0.93 to 0.67. The must-fail core still fails, on column 7.

**What to build:** a check that judges each exchange's accumulated error and is still independent of grid phase. Options:

- Compare running totals only at whole beats, where the phase lock cancels.
- Compare each exchange's mean over a longer window, such as one beat or a few, with a distance from the reference rungs at that window.
- A separate drift bound on the cumulative error's slope, judged from the reference rungs' own slopes.

- [x] A core whose per-interval nets pass but whose errors all share one sign fails the new check, on a synthetic fixture. Held by `window::tests::a_one_sign_error_inside_every_interval_fails_on_its_whole_beat`.
- [x] The honest cores at the arena start pass it at any grid phase, not only the one the start falls on. The check reads at the finest refinement's beat edges and at no grid instant, so the grid's phase does not enter it; the plan measured a spread of zero over sixteen phases.
- [x] A mutant row holds it, with a test inside 40-mutants' 420 s budget. Rows `window-judges-whole-beats` and `window-finds-beat-edges`.
- [x] §7's lines on the window change only as much as the new rule needs: one sentence of the Accuracy gate and one must-fail row.

**Non-goals:** the synthetic energy drift limit, 2e5 since ticket 28, does not change here (ticket 28's residual on the working core, `ng_solver::Core::new`: +0.96% of inflow over the first 30 s, +5.8/s over 120 s). No widened window distances, no changed margins.

## Answer

The slice window also compares each exchange by its net over each whole beat of the finest refinement, from one beat edge to the next, beside the per-interval nets, which stay as they were. The finest refinement finds its edges where the heart's phase falls, the rule the day checks use, and every other run reads its totals at those instants. A beat's distance is the coarser two refinements' largest error on those beat nets, or the floor share of the finest's largest beat net, times the margin of 2: no new number. A window whose finest refinement makes fewer than two whole beats is refused (`AccuracyFailure::TooFewBeats`), as the day checks refuse one. The plan and its measurements are in `~/lanes/logs/ng-wave-4/nc31/PLAN.md`.

- The cumulative shapes were refused. Running totals at beat edges, running totals at grid instants and the slope of the cumulative error all fail the honest cores at every grid phase, because they add up the honest one-sign error on columns 245 to 247 described above, and passing it would need a margin above 2. A net over each whole beat starts again at each edge, so that error does not build.
- Through the built check the honest cores' worst shares are `Core::new(16 s)` 0.707, `Core::doubled(16 s)` 0.712, `Core::new(5 ms)` 0.526 and the 16 s reference 0.750 (its per-interval share, the larger). The four must-fail cores of `crates/ng-validate/tests/window.rs` fail as before, on the per-interval values or on modes, which are judged first.
- Plan figures: a constant one-sign bias is caught at a median of 0.095 of the per-interval distance, inside the first whole beat. A one-sign rate below a beat's distance still passes; the plan's section "What stays unseen" lists, column by column, which of those the certificate catches over a day.
- The per-interval nets depend on the grid's phase, and three honest cores fail them 8 ms off a beat edge: [ticket 51](51-per-interval-nets-depend-on-grid-phase.md).
