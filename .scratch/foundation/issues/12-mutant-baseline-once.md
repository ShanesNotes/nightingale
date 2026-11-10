# 12: A mutant baseline is not run twice

Type: task
Status: open
Blocked by: none

**What to build:** In a run of `tools/verify`, `40-mutants` skips the baseline of each test command that `10-rust`'s test phase already ran and passed on the same commit with the same build settings. A baseline runs a whole test on the unmutated tree before any row runs. The diagnosis at `~/lanes/logs/ng-optimize/proof-time.md` (section 5, cut 3) measured the baselines at a median of 420 s, 22.7% of a full proof. Every baseline that is not `--release` reruns a test `10-rust` already passed on the same commit at the same `opt-level = 2` profile. The `--release` baselines run a profile `10-rust` never builds, so they stay. So does the baseline for any crate the plan did not test. The diagnosis estimates the saving at 300 to 400 s on a green proof at the 1,871 s median, and 441 to 569 s on the newest proof. The range is wide because the sweep's own cold build rides in its first baselines, and that build moves to the first row.

One guard moves. Today the baseline proves a test passes unmutated in the sweep's own extract and target. After this change `10-rust` proves it passes in the checkout at the same profile. So only a difference between the two trees or environments slips through, such as `CARGO_NET_OFFLINE` or a file `git archive` leaves out. A row whose filter matches no test still fails, as SURVIVED, because `cargo test` then exits 0. This choice is a model default, open to the developer reviewer's review.

- [ ] `10-rust` records what its test phase ran and passed: the commit, the build settings, and the crates and test targets. `40-mutants` skips each baseline that record covers
- [ ] A `--release` baseline, a crate the plan did not test, a record from another commit or other build settings, and a missing record each run the baseline as today. A check shows each of these
- [ ] A hand run of `tools/mutate.py` runs every baseline, as today. Rows that [ticket 11](11-proof-fails-fast.md) runs before `10-rust` keep their baselines, since `10-rust` has not run yet
- [ ] The seconds saved on a full proof, measured in three alternating proofs each way on one head on the tower. All six show the same CAUGHT count and the same `10-rust` test counts
- [ ] One full proof run under `pidstat -u 5` or `sar -u 5`, with the record kept beside its log. It shows whether cores sit idle during the arena test binary and the heavy mutant rows. That decides the next cut. If cores sit idle, running `40-mutants` beside `10-rust` could save up to 766 s, or the arena binary beside the other test binaries up to about 230 s, both estimates. If the arena tests fill the cores, that cut is dropped
- [ ] The PR body names the guard that moves and marks it a model default open to review

**Non-goals:** changing which tests or rows a full proof runs. The cuts the diagnosis leaves to the developer reviewer: aiming the heavy rows at shorter tests, shortening the arena day protocol, narrowing the `everything` triggers in `tools/gate/paths.toml`, and accepting a docs-only change on top of a proven head. A warm build target shared across proofs, which the diagnosis rejects.
