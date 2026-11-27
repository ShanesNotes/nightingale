# 16: A skipped baseline can stand for a test target cargo never ran

Type: task
Status: resolved
Blocked by: none

**Why.** A mutant row shows that a test can fail, and every law on the way to the first milestone (SRS invariant I19) is held by such rows. In a merge run the sweep skips a row's baseline when `10-rust` already passed that test on the same commit ([ticket 12](12-mutant-baseline-once.md), nightingale#142). If the record of what `10-rust` passed counts a test it never ran, a baseline that fails reads as a mutant CAUGHT, and the gate passes a row that proves nothing.

**What happens.** `10-rust` calls `cargo test` on a package with cargo's default choice of targets, and the record counts that call as every target of the package. Cargo's default choice leaves out a target whose manifest sets `test = false`, though a call that names the target still runs it. So a row that names a test in such a target has its baseline skipped. A synthetic case shows it: one package, a passing target, and a failing target that sets `test = false`, with a row on the failing one whose mutant changes nothing the test reads. The hand sweep refuses the failing baseline. The merge sweep prints `baselines skipped: 1`, reads the row CAUGHT and exits 0.

**Where it stands.** No manifest in the workspace sets `test = false`, so no proof so far could have read wrong. The gate already reads a target's `test` flag when it plans its test calls, so such a manifest can land without the gate objecting. [Ticket 14](14-slow-mutant-rows-run-long-tests.md) is where one might be reached for, to keep a long test out of the default call.

- [x] The synthetic case joins the cases nightingale#142 added for the skip, and is red before the fix
- [x] One of two fixes, chosen with the code in hand: the record counts a default call only for the targets cargo ran, or the seam refuses `test = false` in a workspace manifest and the gate's branch for it goes
- [x] The Answer says, for every other manifest setting that takes a target out of a default call (`harness`, `autotests` and the like), whether the same gap is open, each with a case or a refusal

**Non-goals:** the sweep's budget and the long rows, which are ticket 14. Running the rows side by side, which is [ticket 13](13-mutant-rows-in-parallel.md).

Evidence: `~/lanes/logs/ng-wave-4/breakit-post/142/`, with the case (`test_disabled_target.py`) and its run (`run-01.log`).

## Answer

- **Fix chosen: record only the libtest targets cargo ran.** `gate.py`'s `tested` now writes a `targets` map (`{crate: [...]}`) into `target/gate/tested.jsonl` computed from `cargo metadata --format-version 1 --no-deps --offline --locked` and each package's `Cargo.toml`. `tools/mutate.py`'s `passed_calls` and `covered` match only recorded per-crate targets, never a blanket `"all"`, and fail closed when a record entry has no `targets` map or a mutant row names no target.
- **Red before fix, green after.** Before the fix, running `cargo test --locked -p ng-a` on a synthetic workspace whose `[[test]] name = "beta"` sets `test = false` and panics exited 0, and `mutate.py --tested` printed `baselines skipped: 1`, read the no-op mutant `CAUGHT`, and exited 0 while the hand sweep refused the baseline. After the fix, `disabled_target_case` in `tools/gate/gate.py` (`DRIVER_CASES`, run by `05-seam.sh`) asserts that `beta`'s baseline is not skipped and fails closed while a passing target (`alpha`) still skips its baseline and catches its mutant.
- **Audit of every other manifest setting:**
  - `[[test]] test = false`: omitted from default `cargo test`; excluded from `targets` for default calls (still recorded if explicitly named via `--test`). Covered by `disabled_target_case` (`fail_test_false.toml`).
  - `[lib] test = false`: omitted from default `cargo test`; excluded from `targets` for default calls. Covered by `disabled_target_case` (`fail_lib_false.toml`) and `TESTED_CASES`.
  - `[[test]] harness = false` and `[lib] harness = false`: run by default `cargo test` with zero arguments without libtest, so a mutant row passing a filter argument can fail or abort even though the default run exited 0 (reproduced with a `harness = false` test that aborts when given a filter). Excluded from `targets` for both default and explicit calls; covered by `disabled_target_case` (`fail_harness_false.toml`).
  - `required-features = [...]`: omitted from default `cargo test`; excluded from `targets` (and refused by `cargo test --no-run`). Covered by `disabled_target_case` (`fail_req_features.toml`).
  - `autotests = false`: `tests/*.rs` files are not registered as targets by Cargo, absent from `cargo metadata` and `targets`, and refused at baseline. Covered by `disabled_target_case` (`fail_autotests_false.toml`).
  - `[lib] doctest = false`: `--doc` baselines are never skipped (`covered` rejects `"doc"` and `_ran_targets` never records `"doc"`). Covered by `TESTED_CASES`.
  - `[[bin]]` (`test = false` / `autobins = false`), `[[example]]` (`autoexamples = false`), `[[bench]]` (`autobenches = false`): `mutate.py`'s `selection` only accepts `--lib`, `--test` and `--doc`, and `_ran_targets` records only `lib` and `test` targets, so none can skip a baseline.
  - `workspace.default-members`: `10-rust.sh` always passes `-p <crate>` and `selection` requires `-p`/`--package`, so `default-members` never affects coverage.
