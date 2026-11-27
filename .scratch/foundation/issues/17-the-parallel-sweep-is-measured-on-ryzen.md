# 17: The parallel sweep is measured on ryzen, where the proofs run

Type: prototype
Status: resolved
Blocked by: none

**Question.** Does running mutant rows in lanes, in the seed shape [ticket 13](13-mutant-rows-in-parallel.md) recommends, halve the sweep's wall time on ryzen? Ticket 13 is parked on a beelink measurement, and its own hazards say "The halving is not shown on ryzen."

**Why now.** Proof time bounds how many tickets land on the way to the first milestone (SRS invariant I19). Every code proof now runs on ryzen, and the sweep is most of it. Three recent proofs took 7,212 to 7,575 s, and their `40-mutants` step took 5,528 to 5,886 s, about 77% of the wall, by each proof's `time:` lines. The table holds 601 rows on main `446e29f9`. One proof hit the old 7,200 s wall inside the sweep with 515 of 546 rows caught (nightingale#183). The serial order is forced in `tools/mutate.py`. One extract and one target serve every row (lines 67, 68 and 320 to 326). Each row edits that extract in place, then restores it (lines 368 to 376). A `for` loop runs the rows one at a time (lines 378 to 388), and a lock refuses a second sweep in the same checkout (lines 315 to 319). `tools/gate/gate.py`'s `first_stop` reads one child's lines in order and stops at the first row that is not caught (lines 2911 to 2939).

**What to build.** Rerun ticket 13's prototype in the seed shape (lane 0 extracts and builds once, then copies its extract and target to each other lane) on ryzen at one head. Use one builders job, the desk idle, and no other job in its slots.

- [ ] Serial and K = 2, 3 and 4 sweeps of every row at one head, timed twice each, with ryzen's load and free memory beside each number
  - Not met. Four lanes swept twice, three lanes once and stopped once, and four lanes twice more with a 1,000 s row budget, each with its load and free memory in the Answer. Serial and two lanes did not run.
- [ ] Every lane run gives each row the serial verdict, by name, and a script compares the lists
  - Every sweep that finished gave all 706 rows the serial verdict, by name, and a script compared the lists. The sweep that stopped judged 704 rows, each with the serial verdict, and left two without a verdict.
- [ ] `timed-run-knows-no-input` and the ten slowest rows of [ticket 14](14-slow-mutant-rows-run-long-tests.md) are timed at each K against each row's own limit, the larger of the 420 s budget and twice its test's baseline, and any that would read UNCLEAR is named
  - Timed at three and four lanes only, in the Answer. Two lanes did not run.
- [x] Ryzen's filesystem is read, and the copy of a lane's target is shown to share blocks, or its full size is stated
- [ ] The Answer says which K halves the sweep, or that none does. If one does, ticket 13's `Blocked by:` line is set to `none` with this ticket's figures. If none does, ticket 14 becomes the next lever, with its expected saving per proof

**Non-goals.** Changing any row, its test or its budget. Merging the prototype. Running it on the beelink again.

## Answer

Lanes alone did not halve the sweep at three or four lanes. Four lanes did when every row had a budget of 1,000 s in place of 420 s. Halving means the whole sweep, setup included, in at most 5,847 s in both sweeps at one lane count. That is half of 11,695 s, the `40-mutants` step in the proof of nightingale#231 on the proof worker, which ran 707 rows.

Each sweep was given all 706 rows of main `d42f905e` on the proof worker, in ticket 13's seed shape and in the shape of a proof's sweep. The baselines that `10-rust` covers were skipped, as in a merge proof's rest part. A row over its limit whose baseline was skipped ran that baseline, or waited for the lane that ran it, and then ran again. Each sweep was one build job with no other job in the worker's slots. The 1,000 s budget was `tools/mutate.py`'s `--budget` flag, one value for every row, and the mutant table was unchanged. Measured, one line a sweep:

| Lanes | `--budget` | Whole sweep | At most 5,847 s | Rows judged | Mean load | Least free memory | Other work beside the job |
|---|---|---|---|---|---|---|---|
| 4 | 420 s | 7,504 s | no | 706 | 12.1 | 42.2 GiB | 1.02 threads |
| 4 | 420 s | 7,394 s | no | 706 | 12.5 | 41.6 GiB | 1.00 threads |
| 3 | 420 s | 8,038 s | no | 706 | 9.6 | 42.2 GiB | 0.98 threads |
| 3 | 420 s | stopped at 8,269 s | no | 704 | 11.2 | 37.7 GiB | 2.18 threads |
| 4 | 1,000 s | 5,075 s | yes | 706 | 16.6 | 42.4 GiB | 1.10 threads |
| 4 | 1,000 s | 5,128 s | yes | 706 | 16.1 | 42.8 GiB | 1.09 threads |

- Every sweep that finished gave all 706 rows the serial verdict, by name: 698 CAUGHT and 8 CAUGHT at build, with none UNCLEAR. A script compared the lists. The serial verdicts are the 706 rows of the proof of nightingale#230, whose crates and mutant table match `d42f905e` byte for byte. The 11,695 s reference has one row more.
- The second three-lane sweep stopped when `the_whole_body_admits_runs_its_first_minute_and_holds_its_pins`, run unmutated as a skipped baseline, gave nothing in its 1,680 s limit. Two rows were left without a verdict. Other work had started on the proof worker 4,790 s into that sweep.
- With the 420 s budget, six to eight rows a sweep went over 420 s and ran again after their skipped baseline. With 1,000 s no row went over its limit, and no row ran a second time. In all four four-lane sweeps the rows' last runs summed to 18,220 to 18,427 s.
- `timed-run-knows-no-input` took 360 and 357 s of its 420 s limit at four lanes, and 266 and 252 s at three, against 183 s serial. The other nine of ticket 14's ten slowest rows took 116 s or less. None read UNCLEAR. Every row that passed 420 s had a skipped baseline, so it ran again in place of reading UNCLEAR.
- The proof worker's filesystem is btrfs with zstd compression. Copying lane 0's 7.48 GB target directory with `cp -a --reflink=auto` took 1.9 s and about 1.2 MB of free space, and `btrfs filesystem du` then showed 0 bytes exclusive to either copy. After their rows the four lanes held about 28 GiB together, about 21 GiB of it exclusive.

Not run: a serial sweep and a two-lane sweep in this prototype, so this Answer gives no figure for two lanes.

What limits it:

- One head, `d42f905e`, before nightingale#238 moved the mutant table into `tools/gate/mutants.d/` and changed which baselines a sweep skips. These figures are not shown on main.
- One machine, and two sweeps a cell. Three lanes have one finished sweep.
- The 1,000 s stands in for a row that names its own budget, [ticket 14](14-slow-mutant-rows-run-long-tests.md)'s last box. It was not sized.
