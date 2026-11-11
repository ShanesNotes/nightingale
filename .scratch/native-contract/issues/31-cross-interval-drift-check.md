# 31: The slice window misses drift that builds across intervals

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7, and `crates/ng-validate/src/window.rs` (`over_interval`, `Measured`)

Type: task

Status: open

Blocked by: none

Ticket 28 changed how the slice window judges an exchange column: by its net over each grid interval (`over_interval`), not by its running total since admission. A running total carried each interval's signed error into the next. Because the 10 ms grid is phase-locked to the 1 s beat, that error cancelled or grew depending on where the grid fell in the beat. Moving the start to the beat onset put the 16 s reference outside the window: ratio 1.058, column 246. Shane accepted the per-interval statistic for nightingale#91 on 2026-11-09.

The per-interval statistic is blind to slow drift. Suppose a core's error is small enough in every interval to pass, but always the same sign. Its running total then grows without bound, and no row of the window sees it. On main the new statistic is looser for 142 of 257 exchange columns, and the 16 s reference's worst ratio falls from 0.93 to 0.67. The must-fail core still fails, on column 7.

**What to build:** a check that judges each exchange's accumulated error and is still independent of grid phase. Options:

- Compare running totals only at whole beats, where the phase lock cancels.
- Compare each exchange's mean over a longer window, such as one beat or a few, with a distance from the reference rungs at that window.
- A separate drift bound on the cumulative error's slope, judged from the reference rungs' own slopes.

- [ ] A core whose per-interval nets pass but whose errors all share one sign fails the new check, on a synthetic fixture.
- [ ] The honest cores at the arena start pass it at any grid phase, not only the one the start falls on.
- [ ] A mutant row holds it, with a test inside 40-mutants' 420 s budget.
- [ ] §7's lines on the window change only as much as the new rule needs.

**Non-goals:** the synthetic energy drift limit, 2e5 since ticket 28, does not change here (ticket 28's residual on the working core, `ng_solver::Core::new`: +0.96% of inflow over the first 30 s, +5.8/s over 120 s). No widened window distances, no changed margins.
