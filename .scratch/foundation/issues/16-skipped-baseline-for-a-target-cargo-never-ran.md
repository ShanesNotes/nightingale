# 16: A skipped baseline can stand for a test target cargo never ran

Type: task
Status: open
Blocked by: none

**Why.** A mutant row shows that a test can fail, and every law on the way to the first milestone (SRS invariant I19) is held by such rows. In a merge run the sweep skips a row's baseline when `10-rust` already passed that test on the same commit ([ticket 12](12-mutant-baseline-once.md), nightingale#142). If the record of what `10-rust` passed counts a test it never ran, a baseline that fails reads as a mutant CAUGHT, and the gate passes a row that proves nothing.

**What happens.** `10-rust` calls `cargo test` on a package with cargo's default choice of targets, and the record counts that call as every target of the package. Cargo's default choice leaves out a target whose manifest sets `test = false`, though a call that names the target still runs it. So a row that names a test in such a target has its baseline skipped. A synthetic case shows it: one package, a passing target, and a failing target that sets `test = false`, with a row on the failing one whose mutant changes nothing the test reads. The hand sweep refuses the failing baseline. The merge sweep prints `baselines skipped: 1`, reads the row CAUGHT and exits 0.

**Where it stands.** No manifest in the workspace sets `test = false`, so no proof so far could have read wrong. The gate already reads a target's `test` flag when it plans its test calls, so such a manifest can land without the gate objecting. [Ticket 14](14-slow-mutant-rows-run-long-tests.md) is where one might be reached for, to keep a long test out of the default call.

- [ ] The synthetic case joins the cases nightingale#142 added for the skip, and is red before the fix
- [ ] One of two fixes, chosen with the code in hand: the record counts a default call only for the targets cargo ran, or the seam refuses `test = false` in a workspace manifest and the gate's branch for it goes
- [ ] The Answer says, for every other manifest setting that takes a target out of a default call (`harness`, `autotests` and the like), whether the same gap is open, each with a case or a refusal

**Non-goals:** the sweep's budget and the long rows, which are ticket 14. Running the rows side by side, which is [ticket 13](13-mutant-rows-in-parallel.md).

Evidence: `~/lanes/logs/ng-wave-4/breakit-post/142/`, with the case (`test_disabled_target.py`) and its run (`run-01.log`).
