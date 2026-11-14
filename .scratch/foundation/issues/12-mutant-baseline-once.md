# 12: A mutant baseline is not run twice

Type: task
Status: resolved
Claimed by: sweep-12-fix
Blocked by: none

**What to build:** In a run of `tools/verify`, `40-mutants` skips the baseline of each test command that `10-rust`'s test phase already ran and passed on the same commit with the same build settings. A baseline runs a whole test on the unmutated tree before any row runs. The diagnosis at `~/lanes/logs/ng-optimize/proof-time.md` (section 5, cut 3) measured the baselines at a median of 420 s, 22.7% of a full proof. Every baseline that is not `--release` reruns a test `10-rust` already passed on the same commit at the same `opt-level = 2` profile. The `--release` baselines run a profile `10-rust` never builds, so they stay. So does the baseline for any crate the plan did not test. The diagnosis estimates the saving at 300 to 400 s on a green proof at the 1,871 s median, and 441 to 569 s on the newest proof. The range is wide because the sweep's own cold build rides in its first baselines, and that build moves to the first row.

One check moves. Today the baseline proves a test passes unmutated in the sweep's own extract and target. After this change `10-rust` proves it passes in the checkout at the same profile. So only a difference between the two trees or environments slips through, such as `CARGO_NET_OFFLINE` or a file `git archive` leaves out. A row whose filter matches no test still fails: as SURVIVED, because `cargo test` then exits 0, or, when its mutant does not build, at its baseline, which the sweep then runs. This choice is a model default, open to the developer reviewer's review.

- [x] `10-rust` records what its test phase ran and passed: the commit, the build settings, and the crates and test targets. `40-mutants` skips each baseline that record covers
- [x] A `--release` baseline, a crate the plan did not test, a record from another commit or other build settings, and a missing record each run the baseline as today. A check shows each of these
- [x] A hand run of `tools/mutate.py` runs every baseline, as today. Rows that [ticket 11](11-proof-fails-fast.md) runs before `10-rust` keep their baselines, since `10-rust` has not run yet
- [x] The seconds saved on a full proof, measured in three alternating proofs each way on one head on one build worker under the same other load, with the worker named beside each number. All six show the same CAUGHT count and the same `10-rust` test counts
- [x] One full proof run under `pidstat -u 5` or `sar -u 5`, with the record kept beside its log. It shows whether cores sit idle during the arena test binary and the heavy mutant rows. That decides the next cut. If cores sit idle, running `40-mutants` beside `10-rust` could save up to 766 s, or the arena binary beside the other test binaries up to about 230 s, both estimates. If the arena tests fill the cores, that cut is dropped
- [x] The PR body names the check that moves and marks it a model default open to review

**Non-goals:** changing which tests or rows a full proof runs. The cuts the diagnosis leaves to the developer reviewer: aiming the heavy rows at shorter tests, shortening the arena day protocol, narrowing the `everything` triggers in `tools/gate/paths.toml`, and accepting a docs-only change on top of a proven head. A warm build target shared across proofs, which the diagnosis rejects.

## Answer

`10-rust.sh` clears `target/gate/tested.jsonl` at its start. After each test call that passes, it appends one line through `gate.py tested`. The line holds the commit, the build settings and the call's arguments. The build settings are every `CARGO*` and `RUST*` variable plus `UPDATE_EXPECT`, leaving out the four that `mutate.py` sets itself. Both steps source one scrub, `tools/gate/scrub.sh`, so they clear the same variables.

A skipped baseline is sound only when the sweep's extract holds everything the checkout's tests read. So `10-rust` writes no record when the checkout holds a changed tracked file, an untracked file or an ignored one, other than the build outputs a proof's fresh clone holds at that point: `target/`, `tools/__pycache__/` and the sketch's `target/`. It also writes none when a tracked file is marked assume-unchanged or skip-worktree, which hides a change from `git status`, when a Cargo config sits in `target/.cargo` or `target/mutate/.cargo`, which only the sweep's cargo reads, or when a file carries an `export-ignore` or `export-subst` attribute, since `git archive` drops or rewrites that file. With no record, the sweep runs every baseline.

The sweep checks the tree again. It refuses to run while a Cargo config sits in either place under `target/`. After the extract it refuses when a tracked path is missing from it, which is how `export-ignore` on a directory shows. When it takes the record, it drops the record if the checkout then holds a changed or untracked file or a marked one, so a file edited between the two steps runs every baseline. It does not ask about ignored files there, because a proof's clone holds more build outputs by then, such as `tools/bench/target/`.

Only the rest part of a merge run passes `--tested` to `mutate.py`. There a baseline is skipped when a passed call at this commit and these settings ran the same crate, and either the same test target or the whole crate. A call counts only when it ran every test, or every test with the ignored ones. The sweep then builds each skipped test with `cargo test --no-run`. An extract that does not build still stops the sweep, instead of every row reading as CAUGHT at build. That build also keeps the sweep's cold build out of the first row's 420 s budget.

These still run their baseline:

- a `--release` test
- a doctest, because rustdoc builds one only by running it
- a test with runner flags after `--`
- a crate or target that 10-rust did not run whole
- a record from another commit or other settings
- a missing record
- a row whose mutant does not build, once that row is judged, so its filter is shown to name a test

So does every baseline in a full run, so the nightly keeps them all, and every baseline in the first part and in a hand run. `05-seam` holds each of these rules with a case that fails when the rule breaks: the coverage cases, the build settings, which sweeps each mode and part hands the record, the writer in a scratch repository for each kind of path and mark, a sweep with a stub cargo before and after an ignored file appears, after a tracked file changes, with a directory `git archive` leaves out and with a row caught at build whose filter names no test, a Cargo config in each place under `target/` against the seam, the writer and the sweep, the writer and sweep cases run again under a pre-push hook's `GIT_DIR` from a linked worktree with the scratch repository's config and refs compared, the test wrapper's arguments against the record and its stop on a failed call, a run with no mode set, the reset before the first test call, and both steps sourcing the scrub.

The check that moves is a model default, open to the developer reviewer's review. Before this change, a baseline proved that a test passes unmutated in the sweep's own extract and target. Now `10-rust` proves that in the checkout, and the sweep proves that the extract builds. The writer's rule makes the two trees hold the same files, and `05-seam` keeps a test from reading the environment or the clock.

Three mutants of this change's lines survive the seam, and each is harmless. `40-mutants.sh` alone setting `RUSTC_BOOTSTRAP` below the scrub is harmless to the skip: the settings then differ, so every baseline runs, and nothing says why. `10-rust.sh` alone setting one more variable below the scrub is harmless while a test cannot read the environment. The shared scrub no longer clearing `RUSTC_BOOTSTRAP` is older than this change: the scrub's content had no case on main either.

`40-mutants` does not run beside `10-rust`. The skip needs the record `10-rust` writes first, and running the two together would give back the saving below to gain at most the 766 s estimate.

Six proofs ran on ryzen, alternating between main at `c4760e8` (before) and this branch at `6dd7410` (after). Each ran from a fresh clone through `ryzen-prove.sh`. Other workers' jobs shared ryzen the whole time: sodium benches and up to two other proofs. So the other load was not the same in each run, and the wall times move with it. Beside each run is its mean 1-minute load, read from `status.sh` every 5 minutes, and the mean count of other jobs beside it.

| Run | Worker | Mean load | Other jobs | Wall s | 40-mutants s | Baselines s (count) | Builds s |
|---|---|---|---|---|---|---|---|
| 1 before | ryzen | 9.1 | 2.35 | 5,054 | 3,790 | 841 (231) | 0 |
| 2 after | ryzen | 10.8 | 1.12 | 4,533 | 3,148 | 141 (6) | 116 |
| 3 before | ryzen | 7.8 | 0.88 | 4,718 | 3,428 | 786 (231) | 0 |
| 4 after | ryzen | 7.3 | 0.64 | 4,030 | 2,867 | 108 (6) | 90 |
| 5 before | ryzen | 14.4 | 1.67 | 6,162 | 4,736 | 925 (231) | 0 |
| 6 after | ryzen | 10.1 | 1.20 | 4,487 | 2,967 | 125 (6) | 101 |

All six runs caught all 302 rows and ran the same 10-rust tests: 93 test results with 1,464 tests passed. Each after run skipped 225 baselines and ran 6, two `--release` and four doctests. Baselines plus their builds fell from a mean of 851 s to 227 s, a mean saving of 624 s a proof: 584, 588 and 699 s pair by pair. Mean proof time fell from 5,311 s to 4,350 s, but the before runs had more company, so the rows the change does not touch also ran slower then. The 624 s is the number to trust, measured beside one or two other jobs.

Run 6 ran under `sar -u 5` and `pidstat -u 5`, with another whole proof running beside it. While 10-rust's arena binary ran, 42% of ryzen's 12 threads sat idle, and the binary itself used a mean of 1.9 cores. During the rest sweep, 43% sat idle, and each heavy row's test ran on about one core. Cores sit idle, so rows beside rows stays open as [ticket 13](13-mutant-rows-in-parallel.md). The run's evidence folder holds the logs, the CPU records and the scripts that read them.

The independent check of this change found the limits of the skip. It is sound for a proof taken in a fresh clone, and every proof that merges a PR is taken in one. On a checkout changed by hand, five shapes still let a mutant that changes nothing read CAUGHT. No gate step makes any of them, and the nightly's full run runs every baseline.

- An ignored file that a test reads through `expect_file!` appears between `10-rust` and the sweep. The sweep's second look leaves ignored files out.
- A Cargo config at the root, hidden by `.git/info/exclude`, is written after the record.
- With `core.trustctime` off, a same-size edit that keeps the file's modification time and inode is one `git status` cannot see. So "changed" above means changed as `git status` sees it, and `prove.sh` reads the same status for `worktree: clean`.
- An `export-subst` attribute is added through `.git/info/attributes` after the record.
- HEAD moves between the test call and the writer's reading of it. `prove.sh` checks `head_unchanged`.

None of these is closed. The closes the check names are to hash every tracked file and not trust its stat, to read the export attributes again in the sweep, to look for an ignored `.cargo` between the checkout and the extract, and to read HEAD before the test call.

Three smaller notes. The sweep's second look at the assume-unchanged and skip-worktree marks has no case, and [ticket 13](13-mutant-rows-in-parallel.md) carries one as a box. The line `baselines skipped: N` is printed before the rows run, so it counts a test whose baseline then runs because one of its rows is caught at build: in this change's proof it read 251, and one of those ran later. And "harmless while a test cannot read the environment" holds only for a variable that does not change the build: one that is not compared and picks another `cargo`, such as `PATH`, would not be harmless, and no step sets one.
