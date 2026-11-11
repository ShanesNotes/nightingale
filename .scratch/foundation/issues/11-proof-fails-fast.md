# 11: The proof fails fast

Type: task
Status: resolved
Claimed by: f11-fail-fast
Blocked by: none

**What to build:** A red proof stops at its first failure, near the start of the run. Today `tools/verify` runs its steps in file order, and `40-mutants` prints a surviving row only when its whole sweep ends. The diagnosis at `~/lanes/logs/ng-optimize/proof-time.md` (section 5, cut 2) measured the cost. Since native-contract 24, 9,030 of 38,143 s of full proofs on the tower (24%) went to red proofs. One red proof failed on a doctest in `20-sketch`, a 5 s step, after 889 s. Another had two surviving rows that took 6 s each, printed after 3,844 s. The diagnosis estimates that the order below would have saved about 8,000 of those 9,030 s. The new order is:

1. The cheap steps: `01-toolchain`, `05-seam`, `10-docs`, `15-f64`, `20-sketch` and `30-data`. Steps 05, 15, 20 and 30 took about 47 s together, measured.
2. In a merge run, the mutant rows the change adds or touches, the ones `--since` selects, with their baselines.
3. `10-rust`, whose test phase runs the test targets the change adds or edits before its other test binaries.
4. The rest of the sweep. Each row runs once, so it skips the rows step 2 ran.
5. `50-workspace`, as today.

The gate's sweep stops at the first row that is not CAUGHT. A green proof still runs every step and every row, so what a full proof checks does not change. A red run loses one thing. It reports only its first failure, so a change with two faults needs a second red run to show the second. A full run has no base, so only the cheap steps move and the sweep stops early.

- [x] `tools/verify` runs the steps in the order above
- [x] The gate's sweep stops at the first row that is not CAUGHT, and prints that row's verdict and the number of rows it did not run. A hand run of `tools/mutate.py` can still ask for every row
- [x] Three seeded faults on a scratch branch, each run on the tower before and after this change and timed to the `verify:` failure line. A failing doctest in `20-sketch` fails within 2 minutes. A failing assert in a test file the change edits fails within 5 minutes. A new row that survives, on a test whose baseline takes 20 s or less, fails within 3 minutes. These limits are estimates from the measured step times, and the PR records each measured time beside its limit
- [x] A green proof on one head, run before and after this change, shows the same steps, the same rows with the same verdicts and the same test counts. The PR shows both wall times
- [x] The verify skill's step list and the header of `tools/mutate.py` say the new order and that a red run reports only its first failure

**Non-goals:** the cuts the diagnosis leaves to the developer reviewer, because each changes what a full proof checks or binds to: aiming the heavy rows at shorter tests, shortening the arena day protocol, narrowing the `everything` triggers in `tools/gate/paths.toml`, and accepting a docs-only change on top of a proven head. Skipping repeated baselines is [ticket 12](12-mutant-baseline-once.md). Running steps side by side waits on the core use that ticket 12 records.

## Answer

`tools/verify` now runs `01-toolchain`, `05-seam`, `10-docs`, `15-f64`, `20-sketch` and `30-data`, then `40-mutants.sh first`, `10-rust`, `40-mutants.sh rest` and `50-workspace`, and it refuses a step file its order does not place. `40-mutants.sh` hands its part to `gate.py mutants`, which runs `tools/mutate.py` and stops the sweep at the first row that is not CAUGHT with `40-mutants: the sweep stops at <row>, <verdict>. Rows not run: <n>`. The body of `mutate.py` is unchanged, so a hand run still reports every row. In a merge run with a base, `gate.py plan` names the test targets the change adds or edits as `first`, and `10-rust` runs them, then the library tests and the other test targets, then the doctests, so each test binary runs once. Its other runs make today's test call.

Step 2 is not what `--since` selects. `--since` keeps every row whose test crate's `tests/` changed, so an edit to one file under `crates/ng-validate/tests/` selects 70 of the 92 rows. The first part takes the rows the change adds or edits and the rows whose `path` it changes, and the rest part takes the run's other rows. Together they are the set each mode sweeps today, which the seam's part cases pin. The changed run that the pre-push hook takes makes the same calls as before, in the new order: its first part checks that every row applies, and its rest part runs nothing.

Measured on the tower, each run in a fresh clone under the heavy lock:

| Seeded fault | Limit | Before | After |
|---|---|---|---|
| A failing doctest in `20-sketch` | 120 s | 1,065 s | 49 s |
| A failing assert in an edited test file | 300 s | 818 s | 104 s |
| A new row that survives, on a test whose baseline is 20 s or less | 180 s | 2,830 s | 75 s |

A green proof of one head before and after this change ran the same nine steps, the same 92 rows all CAUGHT, and the same counts for each test binary and doctest run: 84 runs and 1,163 passing tests. It took 2,865 s before and 2,894 s after. The extra 29 s are 9 s from three test calls in place of one, and 19 s in the sweep, mostly in baselines: its rest part extracts the tree again with fresh file times, so its first baseline rebuilds the workspace crates. The PR has the logs' hashes and each run's numbers.

Two things stay as they are. `gate.py` reads `mutate.py`'s verdict lines through a copy of their format, which a seam case that runs the real `mutate.py` pins. A test command shared by a first row and a rest row runs its baseline in both sweeps, and [ticket 12](12-mutant-baseline-once.md) is where baselines run once.
