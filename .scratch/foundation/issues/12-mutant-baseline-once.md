# 12: A mutant baseline is not run twice

Type: task
Status: claimed
Claimed by: sweep-12-fix
Blocked by: none

**What to build:** In a run of `tools/verify`, `40-mutants` skips the baseline of each test command that `10-rust`'s test phase already ran and passed on the same commit with the same build settings. A baseline runs a whole test on the unmutated tree before any row runs. The diagnosis at `~/lanes/logs/ng-optimize/proof-time.md` (section 5, cut 3) measured the baselines at a median of 420 s, 22.7% of a full proof. Every baseline that is not `--release` reruns a test `10-rust` already passed on the same commit at the same `opt-level = 2` profile. The `--release` baselines run a profile `10-rust` never builds, so they stay. So does the baseline for any crate the plan did not test. The diagnosis estimates the saving at 300 to 400 s on a green proof at the 1,871 s median, and 441 to 569 s on the newest proof. The range is wide because the sweep's own cold build rides in its first baselines, and that build moves to the first row.

One guard moves. Today the baseline proves a test passes unmutated in the sweep's own extract and target. After this change `10-rust` proves it passes in the checkout at the same profile. So only a difference between the two trees or environments slips through, such as `CARGO_NET_OFFLINE` or a file `git archive` leaves out. A row whose filter matches no test still fails, as SURVIVED, because `cargo test` then exits 0. This choice is a model default, open to the developer reviewer's review.

- [x] `10-rust` records what its test phase ran and passed: the commit, the build settings, and the crates and test targets. `40-mutants` skips each baseline that record covers
- [x] A `--release` baseline, a crate the plan did not test, a record from another commit or other build settings, and a missing record each run the baseline as today. A check shows each of these
- [x] A hand run of `tools/mutate.py` runs every baseline, as today. Rows that [ticket 11](11-proof-fails-fast.md) runs before `10-rust` keep their baselines, since `10-rust` has not run yet
- [x] The seconds saved on a full proof, measured in three alternating proofs each way on one head on one build worker under the same other load, with the worker named beside each number. All six show the same CAUGHT count and the same `10-rust` test counts
- [x] One full proof run under `pidstat -u 5` or `sar -u 5`, with the record kept beside its log. It shows whether cores sit idle during the arena test binary and the heavy mutant rows. That decides the next cut. If cores sit idle, running `40-mutants` beside `10-rust` could save up to 766 s, or the arena binary beside the other test binaries up to about 230 s, both estimates. If the arena tests fill the cores, that cut is dropped
- [x] The PR body names the guard that moves and marks it a model default open to review

**Non-goals:** changing which tests or rows a full proof runs. The cuts the diagnosis leaves to the developer reviewer: aiming the heavy rows at shorter tests, shortening the arena day protocol, narrowing the `everything` triggers in `tools/gate/paths.toml`, and accepting a docs-only change on top of a proven head. A warm build target shared across proofs, which the diagnosis rejects.

## Answer

`10-rust.sh` clears `target/gate/tested.jsonl` at its start. After each test call that passes, it appends one line through `gate.py tested`. The line holds the commit, the build settings and the call's arguments. It writes nothing when `git status --porcelain` shows that the checkout differs from HEAD. The build settings are every `CARGO*` and `RUST*` variable plus `UPDATE_EXPECT`, leaving out the four that `mutate.py` sets itself. `40-mutants.sh` clears the same variables that `10-rust.sh` clears, so the two match.

Only the rest part passes `--tested` to `mutate.py`. There a baseline is skipped when a passed call at this commit and these settings ran the same crate, and either the same test target or the whole crate. The sweep then builds each skipped test with `cargo test --no-run`. An extract that does not build still stops the sweep, instead of every row reading as CAUGHT at build. That build also keeps the sweep's cold build out of the first row's 420 s budget.

These still run their baseline:

- a `--release` test
- a doctest, because rustdoc builds one only by running it
- a test with runner flags after `--`
- a crate or target that 10-rust did not run whole
- a record from another commit or other settings
- a missing record

So does every baseline in the first part and in a hand run. `05-seam`'s `tested_cases` holds 14 coverage cases and the missing-record case. It also keeps the two scrub blocks equal, line for line.

The guard that moves is a model default, open to the developer reviewer's review. Before this change, a baseline proved that a test passes unmutated in the sweep's own extract and target. Now `10-rust` proves that in the checkout, and the sweep proves only that the extract builds. A difference in how the two trees run slips through.

Six proofs ran on ryzen, alternating between main at `c4760e8` (before) and this branch at `6dd7410` (after). Each ran from a fresh clone through `ryzen-prove.sh`. Other workers' jobs shared ryzen the whole time: sodium benches and up to two other proofs. So the other load was not the same in each run, and the wall times move with it. Beside each run is its mean 1-minute load, read from `status.sh` every 5 minutes.

| Run | Worker | Mean load | Wall s | 40-mutants s | Baselines s (count) | Builds s |
|---|---|---|---|---|---|---|
| 1 before | ryzen | 9.1 | 5,054 | 3,790 | 841 (231) | 0 |
| 2 after | ryzen | 10.8 | 4,533 | 3,148 | 141 (6) | 116 |
| 3 before | ryzen | 7.8 | 4,718 | 3,428 | 786 (231) | 0 |
| 4 after | ryzen | 7.3 | 4,030 | 2,867 | 108 (6) | 90 |
| 5 before | ryzen | 14.4 | 6,162 | 4,736 | 925 (231) | 0 |
| 6 after | ryzen | 13.2 | 4,487 | 2,967 | 125 (6) | 101 |

All six runs caught all 302 rows and ran the same 10-rust tests: 93 test results with 1,464 tests passed. Each after run skipped 225 baselines and ran 6, two `--release` and four doctests. Baselines plus their builds fell from a mean of 851 s to 227 s. The sweep therefore saved about 620 s a proof under this load. Mean proof time fell from 5,311 s to 4,350 s, but the changing load explains part of that gap. The 620 s is the number to trust.

Run 6 ran under `sar -u 5` and `pidstat -u 5`, with another whole proof running beside it. While 10-rust's arena binary ran, 42% of ryzen's 12 threads sat idle, and the binary itself used a mean of 1.9 cores. During the rest sweep, 43% sat idle, and each heavy row's test ran on about one core. Cores sit idle, so the next cut stays open: run `40-mutants` beside `10-rust`, or run rows beside rows, which is [ticket 13](13-mutant-rows-in-parallel.md). The run's evidence folder holds the logs, the CPU records and the scripts that read them.
