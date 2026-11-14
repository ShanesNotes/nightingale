# 53: A window that starts after admission fails honest cores, and two of the window's rules have no test

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (the accuracy window) and `crates/ng-validate/src/window.rs`

Type: task

Status: open

Blocked by: none

[Native-contract 51](51-per-interval-nets-depend-on-grid-phase.md) made the slice window read every value at each step of the finest refinement. Its independent check passed it and left three things open. None changes what `SliceWindow::first()` judges, which is what the pick uses. The check's report, its probe and its logs are in `~/lanes/logs/ng-wave-4/nc51-build/verify-coordinator/` (`VERIFY-pr158.md`, notes 2 to 4).

**A late window.** A young window reads each exchange's net since admission until it is one coarsest step old (`window.rs`, the net's `back` and `before`). A grid that starts 0.75 s or more after admission fails four honest cores, at 1.07 to 5.77: `Core::new` fails `Outside` on column 63 at 0.750625 s, at 1.068. That is no grid phase, and no shipped path starts such a window. The day checks that judge a core at sleep onset or on the second day's waking will. The cause is read from the code, and its place is measured.

**The grid and the span.** The contract says the metrics are read at every step of the finest refinement, and an exchange by its net over the coarsest step. Grids of 1.25, 2.5 and 5 ms, and nets over 2.5 and 20 ms, pass the window's lib tests and `tests/window.rs`. On the 5 ms grid the 16 s reference fails from an offset of 3.75 ms. The arena test and the gate were not run on those mutants.

**A floor of one millisecond.** `tests/window.rs` expects the 5 ms core with a 1 ms floor to fail the first slice window. Since [native-contract 42](42-veins-collapse-branch.md)'s part 3 it fails as `TransitionMoved` on the stir, 0.30 ms off against 0.2 ms, where main failed four checks first. Whether the window should refuse a 1 ms floor by rule is open: today that rests on where one crossing falls on the core's grid.

**The whole-beat margin.** Through the real window nothing holds the whole-beat distance's margin: a margin a thousand times larger passes `tests/window.rs`. The lib fixture holds it from both sides. The fixed 5 ms reference with a 300 µs landing floor fails only on the whole-beat check at the head, so it can hold the margin through the real window.

- [ ] A window that starts after admission reads a young net from the total at its own start, and the honest cores pass at starts of 0.75 s, a beat and a posture step after admission. A test holds it before any path builds such a window
- [ ] A lib assertion holds the policy's grid to `refinements_ns[2]` and the net's span to `refinements_ns[0]`, each with a mutant row that a coarser grid or another span trips
- [ ] A test through the real window fails when the whole-beat distance's margin grows
- [ ] The Answer says, by a run, what holds the tricuspid valve's memory when its level is wrong and its switch times are right

**Non-goals:** the window's margins and floor; a new tolerance; the whole-beat check as [native-contract 31](31-cross-interval-drift-check.md) built it; the per-instant statistic itself, unless a run shows a core that gets through it.
