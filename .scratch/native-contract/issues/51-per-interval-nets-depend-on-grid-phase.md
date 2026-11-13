# 51: The slice window's per-interval nets depend on where its grid falls in the beat

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (the Accuracy gate and its windows), and `crates/ng-validate/src/window.rs` (`over_interval`, `Measured::new`)

Type: task

Status: open

Blocked by: none

Since [ticket 28](28-admission-energy-at-balance.md) the slice window compares each exchange by its net over each 10 ms grid interval. That statistic's distance is the coarser refinements' largest error at the sampled instants, and where those instants fall in the valve events moves it. The window's grid starts at the arena's start, which is a beat edge only because admission's paced cycle starts at the wrap.

[Ticket 31](31-cross-interval-drift-check.md)'s plan measured the window at sixteen grid phases on main 7f0cbfe: 0 to 9 ms by 1 ms, and the six offsets where the finest refinement's beat edges fall. At a phase of 8 ms three honest cores fail the per-interval statistic on column 22, stroke work drawn on chemical energy: `ng_solver::Core::new` at 1.165 and `Core::doubled` at 1.172, both at the body's longest step, and the 16 s reference under the pick's scales at 1.194. Their worst interval ends at 2.898 s, beside the third beat edge, where the honest cores' systole starts about 4 ms before the finest refinement's. At phase 0, where the window runs, they pass at 0.693, 0.697 and 0.750. Over the sixteen phases the fixed 5 ms reference ranges 0.419 to 0.840 and the adapting 5 ms reference 0.472 to 0.932. The plan, its figures and the analysis are in `~/lanes/logs/ng-wave-4/nc31/` (`PLAN.md`, `analysis-2.txt`, `phase8.py`).

The window passes these cores only because its grid starts on a beat edge. A window that starts mid-beat, such as §7's sleep onset or the second day's waking, can fail an honest core on this statistic alone.

No fix is chosen. One option: ticket 31's whole-beat nets fail all four must-fail cores of `crates/ng-validate/tests/window.rs` on their own (plan figures: 4.78 on column 7, 96.7, 41.9 and 206), and they read at beat edges, not at grid instants, so they could replace the per-interval nets. Others would change what the per-interval distance samples.

- [ ] The honest cores of the window test, `Core::new`, `Core::doubled` and the 16 s reference pass the window at every grid phase, held by a test or a measured run named here
- [ ] The four must-fail cores still fail the window
- [ ] §7's lines on the window change only as much as the rule needs

This closes before the scored pick, and before any window that starts off a beat edge is built ([ticket 16](16-slice-accuracy-window.md) and [ticket 21](21-day-checks-read-the-trace.md) name those windows).

**Non-goals:** the whole-beat check ticket 31 added; the energy drift limit; the window's margins.
