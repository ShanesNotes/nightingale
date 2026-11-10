# 11: The proof fails fast

Type: task
Status: open
Blocked by: none

**What to build:** A red proof stops at its first failure, near the start of the run. Today `tools/verify` runs its steps in file order, and `40-mutants` prints a surviving row only when its whole sweep ends. The diagnosis at `~/lanes/logs/ng-optimize/proof-time.md` (section 5, cut 2) measured the cost. Since native-contract 24, 9,030 of 38,143 s of full proofs on the tower (24%) went to red proofs. One red proof failed on a doctest in `20-sketch`, a 5 s step, after 889 s. Another had two surviving rows that took 6 s each, printed after 3,844 s. The diagnosis estimates that the order below would have saved about 8,000 of those 9,030 s. The new order is:

1. The cheap steps: `01-toolchain`, `05-seam`, `10-docs`, `15-f64`, `20-sketch` and `30-data`. Steps 05, 15, 20 and 30 took about 47 s together, measured.
2. In a merge run, the mutant rows the change adds or touches, the ones `--since` selects, with their baselines.
3. `10-rust`, whose test phase runs the test targets the change adds or edits before its other test binaries.
4. The rest of the sweep. Each row runs once, so it skips the rows step 2 ran.
5. `50-workspace`, as today.

The gate's sweep stops at the first row that is not CAUGHT. A green proof still runs every step and every row, so what a full proof checks does not change. A red run loses one thing. It reports only its first failure, so a change with two faults needs a second red run to show the second. A full run has no base, so only the cheap steps move and the sweep stops early.

- [ ] `tools/verify` runs the steps in the order above
- [ ] The gate's sweep stops at the first row that is not CAUGHT, and prints that row's verdict and the number of rows it did not run. A hand run of `tools/mutate.py` can still ask for every row
- [ ] Three seeded faults on a scratch branch, each run on the tower before and after this change and timed to the `verify:` failure line. A failing doctest in `20-sketch` fails within 2 minutes. A failing assert in a test file the change edits fails within 5 minutes. A new row that survives, on a test whose baseline takes 20 s or less, fails within 3 minutes. These limits are estimates from the measured step times, and the PR records each measured time beside its limit
- [ ] A green proof on one head, run before and after this change, shows the same steps, the same rows with the same verdicts and the same test counts. The PR shows both wall times
- [ ] The verify skill's step list and the header of `tools/mutate.py` say the new order and that a red run reports only its first failure

**Non-goals:** the cuts the diagnosis leaves to the developer reviewer, because each changes what a full proof checks or binds to: aiming the heavy rows at shorter tests, shortening the arena day protocol, narrowing the `everything` triggers in `tools/gate/paths.toml`, and accepting a docs-only change on top of a proven head. Skipping repeated baselines is [ticket 12](12-mutant-baseline-once.md). Running steps side by side waits on the core use that ticket 12 records.
