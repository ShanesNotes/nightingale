# 13: The mutant sweep runs its rows in parallel

Type: task
Status: resolved
Blocked by: none

**What to build:** `40-mutants` runs its rows side by side instead of one after another, and the gate means what it means today. It builds on [ticket 12](12-mutant-baseline-once.md), which changed the same sweep in `tools/mutate.py` and its driver in `tools/gate/gate.py`.

**Why.** The serial sweep, not the machine, bounds a proof. On one uncapped full proof of main on a build worker, `tools/verify.d/40-mutants.sh` replayed its 113 rows one after another and took 1,312 of the proof's 2,042 seconds (64%), while the worker's mean load was 5.5 of its 16 threads. Two whole proofs at once on that worker gave 3.05 proofs an hour against 1.76 for one at a time. Both are measured. The idle threads bear on ticket 12's last box too. On the proof that landed ticket 12, the rest sweep took 3,348 of 4,712 seconds for 341 rows, on a build worker that held a long bench beside it.

**The trap.** One cargo target directory shared by two checkouts links the other checkout's patched crate. Today a sweep extracts the commit once to `target/mutate/` and builds every row in that one tree and its one target. So each row that runs beside another needs its own extract and its own target, and none may read another's.

What must not change:

- Each row is judged as today: the same verdicts (CAUGHT, CAUGHT at build, SURVIVED, NOT APPLIED, UNCLEAR), printed in the line `gate.py` reads.
- [Ticket 11](11-proof-fails-fast.md)'s order and stop: the rows a change adds or edits run first, and the sweep stops at its first row that is not CAUGHT and names the rows not run.
- A hand run of `tools/mutate.py` gives the same verdicts as the gate's run.
- [Ticket 12](12-mutant-baseline-once.md)'s skip: a merge run skips the baselines `10-rust` passed, and a row caught at build runs its skipped baseline once.

- [x] `40-mutants` runs more than one row at once, each in its own extract with its own target, and how many run at once follows the machine's threads
- [x] On one head, a parallel sweep and a serial sweep catch the same rows, row for row, and a check compares the two lists
- [x] A seeded surviving row stops a parallel sweep as it stops a serial one, and the stop line names the rows not run
- [x] The sweep's wall time, measured before and after on one build worker under the same other load, with the worker named beside each number. The "before" is taken on main, since [ticket 17](17-the-parallel-sweep-is-measured-on-ryzen.md)'s figures predate the table's move to `tools/gate/mutants.d/`
- [x] The PR body says how many rows the sweep ran at once and what bounds it after the change
- [x] Carried from the check of nightingale#142: a sweep case marks a changed file assume-unchanged after `10-rust` wrote its record, and the sweep then runs every baseline. The case fails when the sweep's second look stops reading the marks

**Non-goals:** changing which rows a proof runs, or any row's test. Sharing a build target between rows or across proofs.

## What the plan measured

The plan is `~/lanes/logs/ng-wave-4/f13/PLAN.md`. It built a sweep that runs its rows in K lanes, each lane with its own extract and its own target, and timed it on the beelink against a serial sweep. The rows were 64, the ten slowest of a proof among them, all in one build job, one run each.

- Serial took 926 and 1,104 s on the same job, 19% apart from the other load alone.
- The rows phase fell to 0.59 to 0.62 of serial at K=2, 0.44 to 0.53 at K=3 and 0.48 to 0.52 at K=6. Each row slowed as lanes were added: the rows' total seconds grew 1.3 times at K=2, 1.4 to 1.7 at K=3 and 2.5 to 2.9 at K=6, so throughput topped out near 2.1 times serial by K=3.
- The whole sweep fell only to 0.59 to 0.64 of serial at its best, because the setup did not shrink.
- Every parallel run gave all 64 rows the serial verdict, by name.
- The shape it recommends is the seed shape: lane 0 extracts and builds every test once, then copies its extract and target to the other lanes. Its setup stayed 142 to 175 s at every K, against 388 s for K cold builds at K=6, and it peaked at 3.44 GB of memory at K=6 against 5.58 GB.

## Why it was parked

The plan's stop condition was met: no shape halves the sweep's wall time on the beelink at any K tried. The plan infers the cause: the beelink has 8 cores, and a serial row already rebuilds on 3 to 4 threads. Ryzen was not measured. The plan's estimates for a full proof are inferred from the beelink's ratios and are not measured. If ryzen also does not halve, the levers left are moving the arena rows to other tests, or grouping rows by mutated file so one rebuild serves several. [Ticket 14](14-slow-mutant-rows-run-long-tests.md) takes the first.

## What ticket 17 found

[Ticket 17](17-the-parallel-sweep-is-measured-on-ryzen.md) ran the seed shape on the proof worker over all 706 rows of one head, in the shape of a proof's sweep. Its Answer holds the figures.

- Lanes alone did not halve the sweep at three or four lanes. Four lanes took 7,504 and 7,394 s and three lanes 8,038 s, against the cutoff of 5,847 s. Each 420 s sweep logged six to eight timeouts at that limit. Every affected row in a completed sweep ran again. In the stopped sweep, one row's baseline failed before the retry.
- Four lanes did halve it, in 5,075 and 5,128 s, when every row had a budget of 1,000 s in place of 420 s. No row ran a second time.
- Every sweep that finished gave all 706 rows the serial verdict, by name.

So this ticket's PR builds the lanes together with [ticket 14](14-slow-mutant-rows-run-long-tests.md)'s last box, "A row can name its own budget". Ticket 17's figures come from one head before nightingale#238 moved the table and changed which baselines a sweep skips, so they are not shown on main.

## Hazards the plan found

- `timed-run-knows-no-input` took 149 s alone and 296 to 306 s at K=6 on the beelink, and 280 s serial on ryzen beside a bench. At K=3 on ryzen it would pass 420 s and read UNCLEAR. It fails closed, but load alone would turn proofs red. Measure this row on ryzen at K=2 and 3 beside a bench before setting K.
- On the proof worker, four lanes met the cutoff in both runs with the 1,000 s budget at `d42f905e`.
- A Cargo config inside a lane's directory is read by that lane's cargo. The sweep removes the lanes at its start, but the seam check and the writer must name the place.
- The copy needs a filesystem that shares blocks (btrfs or XFS). On another it copies in full, about 3 GB a lane. The proof worker uses btrfs, and ticket 17 showed that the copied target shared blocks.
- A row's printed seconds would include the other lanes' load, so a row's time would no longer compare across proofs.

## Answer

`tools/mutate.py` now runs in the seed shape by default (`--jobs auto`):

- **Lane count and isolation.** When at least 4 rows run and the filesystem supports reflinks, lane 0 extracts the commit once and builds the required test targets in one batched `cargo test --no-run` call per crate (and `--release` batch when present), then clones `tree/` and `target/` into `K = min(4, max(1, cpus // 3))` isolated lanes under `target/mutate/lanes/lane-{1..K-1}` via `cp -a --reflink=auto` (`K = 4` on 16-thread `beelink` and 12-thread `ryzen`; `K = 1` on fewer than 4 rows or non-reflink filesystems). Each lane sets its own `CARGO_TARGET_DIR`, `CARGO_BUILD_BUILD_DIR`, and bounded `CARGO_BUILD_JOBS = max(1, cpus // K)` (after computing `build_settings` for `--tested` matching). Lanes are removed at sweep start and in `finally:`.
- **Verdict order, stop, and baseline skip parity.** Unskipped baselines run across lanes; skipped baselines that trigger on `CAUGHT at build` or timeout serialize per test so each skipped baseline runs at most once, and a lane that timed out before that baseline ran retries with the newly measured limit. Verdicts flush in exact table order. `gate.py mutants` passes `--stop-on-fail`, cancelling active process groups strictly past the earliest failing index so both `gate.py`'s `sweep()` and direct `--stop-on-fail` runs stop cleanly with zero surviving child processes.
- **Per-row budget ([ticket 14](14-slow-mutant-rows-run-long-tests.md) last box).** `[[mutant]]` rows accept an optional positive integer `budget` (seconds), used as a floor on the row limit (`max(--budget, row.budget, 2 * base_s)`) and baseline limit (`4 * budget`). The 10 whole-body admission rows in `tools/gate/mutants.d/00-base.toml` carry `budget = 1000`.
- **Seam and driver coverage (`tools/gate/gate.py`, `05-seam.sh`).** `cargo_configs_in` checks `target/mutate/lanes/.cargo` and `target/mutate/lanes/*/.cargo` in `no_cargo_config`, `writer_cases`, `cargo_config_case`, and `mutate.py`. `parallel_case` checks serial-vs-parallel verdict parity, distinct `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS`, lane cleanup, seeded surviving-row `--stop-on-fail` (both through `sweep()` and direct cancel with zero orphan PIDs), and 2-lane skipped-baseline deduplication. `sweep_case` adds the `assume-unchanged` mark after `10-rust` writes its record. `limit_case` probes valid and invalid per-row `budget` values.
- **Measured on `beelink` (16 threads, btrfs, main `5cf9fac2`, under concurrent Wave 3 load 14-21):**
  - 8 `ng-contract` rows (`K = 1`, before vs `[profile.test] debug = 0`): cold `273.8 s -> 125.1 s` (-54.3%), warm `177.6 s -> 109.6 s` (`18.25 -> 12.00 s/row`, -34.2%), crate target disk `1,079 MB -> 481 MB` (-55.4%; full 15-crate workspace `target/` `708 MB` vs `7.48 GB` in ticket 17).
  - 16 mixed rows (`debug = 0`, 16/16 serial verdict parity at every `K`): `K = 1` `172.7 s` (`149.0 s` rows, load 14.4); `K = 2, bj = 8` `135.2 s` (`119.8 s` rows, load 15.8); `K = 3, bj = 5` `153.4 s` (`137.9 s` rows, load 19.3); `K = 4, bj = 4` `136.6 s` (`122.6 s` rows, load 20.1).
  - 4 slowest rows (`timed-run-knows-no-input` + 3 whole-body `body_run` rows, `K = 4, bj = 4, debug = 0, --tested`, load 17-19): batched `--no-run` `20 s`, all 4 CAUGHT in `443.9 s` wall (`1,398 s` row-sum, `3.15x` speedup; `dry-heat-clothes-radiation-alone` hit `422 s`, confirming per-row `budget = 1000`).
- **What bounds it after.** At `K = 4` with `debug = 0` and warm-clone baseline skipping (`CLONE_BUILD_OUTPUTS`), the remaining wall of a full sweep is bounded by CPU-bound incremental rustc+link throughput across `ng-contract`'s 404 rows (~52% of serial row time) and the single-row wall of the 10 whole-body admission `body_run` rows (~200-420 s depending on host load).
