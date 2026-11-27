# 103: The mutant sweep of a body PR fits its time

From: the full proofs of nightingale#222 and nightingale#224, and the nightly run on main `dfc9cf05`

Spec: `tools/verify.d/40-mutants.sh`, `tools/gate/gate.py` (the mutant sweep) and `tools/mutate.py`

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) is held by its gate, and the gate's mutant sweep is how a test that once missed a mistake keeps seeing it. The sweep is now most of a proof's time, and a code PR may add rows to it.

**What the proofs show.** Established, from each run's `time:` lines and its sweep's summary lines.

- Three full proofs, each with gate scope everything. nightingale#222's two heads took 11,865 s and 12,360 s, as two jobs on the same build worker, and the two ran together for 5,140 s. nightingale#224's head took 13,123 s with neither of them beside it, while the nightly run below ran on the other build worker. The logs show the overlap and not what it cost either proof.
- In the last, `40-mutants` took 10,585 s, its first part 2,681 s and its rest 7,904 s, and `10-rust` took 2,381 s. It replayed all 673 rows: 40 in the first part, all caught, and 633 in the rest, 625 caught and 8 caught at build.
- The nightly run of the whole gate on main `dfc9cf05` passed in 7,829 s on another build worker, from a warm target: `40-mutants` took 6,142 s, its first part 2 s and its rest 6,140 s, over 670 rows, 662 caught and 8 caught at build, and `10-rust` took 1,585 s.
- Not claimed: a cause for the difference between the proof's sweep and the nightly run's. The two runs differ in the build worker, in a warm against a fresh target, and in how the rows fall between the sweep's two parts.

What follows. The proof scripts' walls and the nightly run's time limit are sized from a proof's time, so each grows with the sweep.

**How the sweep chooses and runs rows today.**

- `tools/verify` runs `40-mutants` in two parts, the first before `10-rust` and the rest after it, and `tools/verify.d/40-mutants.sh` runs `gate.py mutants` under `tools/gate/scrub.sh`.
- `sweep_parts` (`tools/gate/gate.py`, lines 2883 to 2895) chooses the rows a run replays. In a merge run whose plan's scope is everything, it chooses every row. Where the scope is affected, it chooses a row when its path changed or a file under its test crate's `tests/` did (`touched`, `tools/mutate.py`, lines 107 to 113), so a PR that touches the body's suite file, `crates/ng-validate/tests/body_run.rs`, takes every row whose test names `ng-validate`.
- That choice has a gap. A row whose test lives in a separate source file outside its crate's `tests/`, which `touched` does not read as a test file, is not chosen when only that file changes: `day-slow-stay-four-minutes` in `tools/gate/mutants.toml` names `a_slow_onset_stays_out_five_minutes_inside_its_window`, which lives in `crates/ng-validate/src/day/tests.rs`, and a change to that file alone chooses no row. A row whose test shares its file with its mutation is chosen: a change to `crates/ng-contract/src/evaluate.rs` chooses `read-is-checked`, whose test lives in that file. A proof whose gate scope is everything replays every row, as the three timed proofs above did, so the gap does not reach it. It does reach a merge proof whose scope is affected, which chooses rows by `touched`, and a docs-only proof shows `full_proof: yes` with neither part of the sweep selected. A choice by the test function a row names also needs a case for a row that names a whole test binary, such as `--test closure_rounding`, or all of a crate's doctests, such as `--doc -p ng-contract`.
- The first part takes the chosen rows that the change adds or edits, or whose path it changes, and the rest takes the others. A full run has no base, so its first part is empty and every row is in the rest (`mutants`, lines 2942 to 2967).
- `tools/mutate.py` runs on one extract, one row at a time (lines 343 to 391): each test's baseline once, then each row with its mutation applied and its own `cargo test` run, so each row pays its own rebuild, and each row its build does not catch pays its test's run: in the last proof 8 rows were caught at build. Rows whose test runs and admits the whole body each pay an admission.
- [Native-contract 84](84-a-mutant-rows-budget-follows-its-tests-own-time.md) gives a row a limit of twice its test's baseline in the same sweep, never under 420 s, and a baseline 1,680 s. In a merge run's rest part the sweep skips each baseline whose test `10-rust` passed whole on the same commit and settings, and builds it instead.
- [Native-contract 97](97-two-prs-that-add-mutant-rows-merge-without-a-conflict.md) holds that every row lives in one file, `tools/gate/mutants.toml`, and asks how two PRs add rows without a conflict.
- A sweep stops at its first row that is not caught.

Foundation tickets 13 (parked), 14 and 17, in `.scratch/foundation/issues/`, hold earlier measurements of rows run in parallel and of the slow rows that run long tests.

**Options, none chosen.**

- Replay rows on both build workers at once.
- Replay a row only when the test function it names or the code it changes moved.
- Split the body's suite file so fewer rows share one test binary.
- Let a file's rows share one admission.

The rule each option keeps is the repo's: a test that once missed a mistake keeps seeing it.

- [ ] A table of one full sweep's time by row, grouped by test file and by what each row's test spends its time on, beside the same sweep timed on each build worker, from a warm and from a fresh target, and a proof timed beside another on the same build worker. Beside each row's rebuild and test run, the table holds the costs a row's own run leaves out: each baseline, each distinct build the tests whose baselines were skipped need, counted once where tests share a target, as the tests of the rows `body-settling-time-apart-from-rule` and `held-packet-keeps-the-rhythms-breath-rate` share `-p ng-validate --test body_run`, the skipped baseline a row caught at build then runs, and the skipped baseline and second run of a row that passes its limit, run again with the limit that baseline measures. This box may restate the 5,400 s of the third box, with its reason
- [x] The pick, by that table and by two or three cheap prototypes where a fork remains
- [ ] The built change, done when a replay of the proof whose `40-mutants` took 10,585 s above, the same head, nightingale#224's `f03600a12b83e96a74115dec5d6b9687ae086caf`, on the same base, main `dfc9cf0501e9ba55ff8bef25df7feedfbc5b6e41`, as the proof script ran it, from a fresh target with nothing else running on the build workers it uses, replays every row in at most 5,400 s of wall time, with every row still replayed whenever its code or its test changes. It closes the gap above before any choice of rows by what changed is relied on. The 5,400 s stays a target, which the first box may restate
- [ ] The nightly run's time limit and the proof scripts' walls restated from the new time

**Non-goals:** which rows the gate keeps, and a row's limit ([native-contract 84](84-a-mutant-rows-budget-follows-its-tests-own-time.md)).

## Comments

### Full-sweep breakdown (`ffcd1a7a` nightly on `beelink`, warm target, 727 rows)

Measured from `20261127T033246Z-ffcd1a7a3f72.log` (`verify: OK`, total `11,345 s`, `10-rust` `1,518 s`, `40-mutants` `9,688 s` = `first` `3 s` + `rest` `9,685 s`):

- **Baselines (`519` runs = `2,298 s`, `23.7%` of `rest`):** `tools/gate/nightly.sh` runs `tools/verify --full`, which never passes `--tested`, so this `2,298 s` is the full-mode baseline cost breakdown (not a skipped-baseline miss on nightly). Top baseline targets: `ng-validate --test body_run` (`8` tests = `1,735 s` / `75.5%`), `--test arena` (`2` = `115 s`), `ng-validate --lib` (`52` = `68 s`), `--test window` (`2` = `38 s`), `--test arena_body` (`4` dev `29 s` + `1` rel `27 s`), `ng-contract --lib` (`131` = `23 s`).
- **Mutant rows (`727` rows = `7,348 s`, `75.9%` of `rest`; `8` caught at build; `~39 s` harness overhead):**

| Target group | Rows | Total (s) | Share | Dominant cost |
|---|---:|---:|---:|---|
| `ng-validate --test body_run` | 16 | 2,141 | 29.1% | 10 whole-body admission rows (`148-259 s`, `2,110 s` sum) |
| `ng-contract --lib` | 182 | 2,106 | 28.7% | ~98% incremental rustc + link (`med 12 s`) |
| `ng-contract --test scopes` | 51 | 386 | 5.3% | incremental rustc + link (`avg 7.6 s`) |
| `ng-validate --lib` | 74 | 330 | 4.5% | incremental rebuild + day/window tests |
| `ng-contract --test routes` | 13 | 115 | 1.6% | incremental rustc + link |
| `ng-validate --test rhythm_certificate` | 27 | 114 | 1.6% | incremental rebuild + cert test |
| `ng-validate --test arena` | 2 | 111 | 1.5% | `timed-run-knows-no-input` (`80 s`) + `day-waits` (`31 s`) |
| `ng-validate --test window` | 6 | 101 | 1.4% | 3 window rows (`31 s` each) |
| Other targets (all remaining crates/tests) | 356 | 1,944 | 26.5% | `ng-contract` tests/doctests + leaf crates (`1-15 s/row`) |

By mutated crate: `ng-contract` `404` rows (`3,833 s`, `52.2%`), `ng-validate` `223` (`2,067 s`, `28.1%`), `ng-metabolism` `22` (`971 s`, `13.2%`), `ng-fluids` `5` (`229 s`, `3.1%`), all other 9 crates `73` (`248 s`, `3.4%`). Cross-worker reference (`ryzen` proof worker, fresh target, 706 rows on `d42f905e`, [foundation 17](../../foundation/issues/17-the-parallel-sweep-is-measured-on-ryzen.md)): serial `11,695 s`, 4 lanes at `420 s` `7,394-7,504 s` (6-8 retries), 4 lanes at `1,000 s` `5,075-5,128 s` (0 retries). Box 1 stays open because `ffcd1a7a` (warm `beelink`, 727 rows) and `d42f905e` (fresh `ryzen`, 706 rows) differ in head, cache and worker.

### Prototypes (`beelink`, `5cf9fac2`, load 14-21) and landed pick

1. **`[profile.test] debug = 0` in `Cargo.toml`:** 8 `ng-contract` rows (`K = 1`): cold `273.8 s -> 125.1 s` (-54.3%), warm `177.6 s -> 109.6 s` (`18.25 -> 12.00 s/row`, -34.2%), crate target `1,079 -> 481 MB` (`708 MB` full workspace vs `7.48 GB` at `debug = 2`).
2. **Warm-clone merge baseline skip (`CLONE_BUILD_OUTPUTS` in `tools/gate/gate.py`):** adds `tools/{gate,bench}/__pycache__/`, `tools/bench/target/`, `tools/generative-physiology/{__pycache__,driver/target}/`, and `clinical-workspace/{node_modules,dist}/` so `10-rust` writes `tested.jsonl` on warm-clone merge runs and `40-mutants rest` skips covered baselines (nightlies run `--full` and never pass `--tested`).
3. **Exact `touched()` in `tools/mutate.py`:** closes the separate-file `src/day/tests.rs` gap (`0 -> 11` rows on base `5cf9fac2`, `10` at HEAD after moving `day-merge-drops-the-last-event` to `ng-contract`, covering all 10 separate `src/` test files), passes `--no-renames` in `git diff`, follows imported local helpers and shared subdirs for `--test <bin>` (failing closed when a test binary entry is absent), handles `--lib` module prefixes (fail-closed to `src/`), `--doc`, `--release`, and `Cargo.toml`, and cuts `body_run.rs` scope from `356 -> 16` rows (`gate.py`'s `touched_cases`).
4. **Seed-shape parallel lanes + per-row `budget` ([foundation 13](../../foundation/issues/13-mutant-rows-in-parallel.md) & [foundation 14](../../foundation/issues/14-slow-mutant-rows-run-long-tests.md) Box 6):** `K = min(4, max(1, cpus // 3))`, batched per-crate `--no-run` with per-batch timeout, capped `CARGO_BUILD_JOBS`, ordered verdict flush + `--stop-on-fail`, `budget = 1000` on the 10 slow `body_run` rows (`SLOW_4` at `K = 4, bj = 4, debug = 0`: `443.9 s` wall for `1,398 s` row-sum, `3.15x` speedup), and `day-merge-drops-the-last-event` moved to `ng-contract` (`32 -> 17 s` row).

Boxes 1 (matched same-head cross-worker warm/fresh table), 3 (historical `nightingale#224` replay on an idle proof worker) and 4 (restating `machine-state` nightly/proof walls) stay open.
