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
- That choice has a gap. A row whose test lives outside its crate's `tests/` is not chosen when only its test's file changes: `day-slow-stay-four-minutes` in `tools/gate/mutants.toml` names `a_slow_onset_stays_out_five_minutes_inside_its_window`, which lives in `crates/ng-validate/src/day/tests.rs`, and a change to that file alone chooses no row. A full proof replays every row, so the gap does not reach a run whose scope is everything, and it does reach one whose scope is affected. A choice by the test function a row names also needs a case for a row that names a whole test binary, such as `--test closure_rounding`, or all of a crate's doctests, such as `--doc -p ng-contract`.
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

- [ ] A table of one full sweep's time by row, grouped by test file and by what each row's test spends its time on, beside the same sweep timed on each build worker, from a warm and from a fresh target, and a proof timed beside another on the same build worker. Beside each row's rebuild and test run, the table holds the costs a row's own run leaves out: each baseline, the build of each test whose baseline was skipped, the skipped baseline a row caught at build then runs, and the skipped baseline and second run of a row that passes its limit, run again with the limit that baseline measures. This box may restate the 5,400 s of the third box, with its reason
- [ ] The pick, by that table and by two or three cheap prototypes where a fork remains
- [ ] The built change, done when a proof's full replay of every row, as the proof script runs it, from a fresh target with nothing else running on the build workers it uses, takes at most 5,400 s of wall time, with every row still replayed whenever its code or its test changes. It closes the gap above before any choice of rows by what changed is relied on. The 5,400 s stays a target, which the first box may restate
- [ ] The nightly run's time limit and the proof scripts' walls restated from the new time

**Non-goals:** which rows the gate keeps, and a row's limit ([native-contract 84](84-a-mutant-rows-budget-follows-its-tests-own-time.md)).
