# 13: The mutant sweep runs its rows in parallel

Type: task
Status: open
Blocked by: none

**What to build:** `40-mutants` runs its rows side by side instead of one after another, and the gate means what it means today. It builds on [ticket 12](12-mutant-baseline-once.md), which changed the same sweep in `tools/mutate.py` and its driver in `tools/gate/gate.py`.

**Why.** The serial sweep, not the machine, bounds a proof. On one uncapped full proof of main on a build worker, `tools/verify.d/40-mutants.sh` replayed its 113 rows one after another and took 1,312 of the proof's 2,042 seconds (64%), while the worker's mean load was 5.5 of its 16 threads. Two whole proofs at once on that worker gave 3.05 proofs an hour against 1.76 for one at a time. Both are measured. The idle threads bear on ticket 12's last box too. On the proof that landed ticket 12, the rest sweep took 3,348 of 4,712 seconds for 341 rows, on a build worker that held a long bench beside it.

**The trap.** One cargo target directory shared by two checkouts links the other checkout's patched crate. Today a sweep extracts the commit once to `target/mutate/` and builds every row in that one tree and its one target. So each row that runs beside another needs its own extract and its own target, and none may read another's.

What must not change:

- Each row is judged as today: the same verdicts (CAUGHT, CAUGHT at build, SURVIVED, NOT APPLIED, UNCLEAR), printed in the line `gate.py` reads.
- [Ticket 11](11-proof-fails-fast.md)'s order and stop: the rows a change adds or edits run first, and the sweep stops at its first row that is not CAUGHT and names the rows not run.
- A hand run of `tools/mutate.py` gives the same verdicts as the gate's run.
- [Ticket 12](12-mutant-baseline-once.md)'s skip: a merge run skips the baselines `10-rust` passed, and a row caught at build runs its skipped baseline once.

- [ ] `40-mutants` runs more than one row at once, each in its own extract with its own target, and how many run at once follows the machine's threads
- [ ] On one head, a parallel sweep and a serial sweep catch the same rows, row for row, and a check compares the two lists
- [ ] A seeded surviving row stops a parallel sweep as it stops a serial one, and the stop line names the rows not run
- [ ] The sweep's wall time, measured before and after on one build worker under the same other load, with the worker named beside each number. The "before" is taken on main, since [ticket 17](17-the-parallel-sweep-is-measured-on-ryzen.md)'s figures predate the table's move to `tools/gate/mutants.d/`
- [ ] The PR body says how many rows the sweep ran at once and what bounds it after the change
- [ ] Carried from the check of nightingale#142: a sweep case marks a changed file assume-unchanged after `10-rust` wrote its record, and the sweep then runs every baseline. The case fails when the sweep's second look stops reading the marks

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

- Lanes alone did not halve the sweep at three or four lanes. Four lanes took 7,504 and 7,394 s and three lanes 8,038 s, against a half of 5,847 s, and six to eight rows a sweep went over 420 s and ran again after their skipped baseline.
- Four lanes did halve it, in 5,075 and 5,128 s, when every row had a budget of 1,000 s in place of 420 s. No row ran a second time.
- Every sweep that finished gave all 706 rows the serial verdict, by name.

So this ticket's PR builds the lanes together with [ticket 14](14-slow-mutant-rows-run-long-tests.md)'s last box, "A row can name its own budget". Ticket 17's figures come from one head before nightingale#238 moved the table and changed which baselines a sweep skips, so they are not shown on main.

## Hazards the plan found

- `timed-run-knows-no-input` took 149 s alone and 296 to 306 s at K=6 on the beelink, and 280 s serial on ryzen beside a bench. At K=3 on ryzen it would pass 420 s and read UNCLEAR. It fails closed, but load alone would turn proofs red. Measure this row on ryzen at K=2 and 3 beside a bench before setting K.
- On ryzen the halving is shown only with a 1,000 s budget for every row, and only at one head before nightingale#238.
- A Cargo config inside a lane's directory is read by that lane's cargo. The sweep removes the lanes at its start, but the seam check and the writer must name the place.
- The copy needs a filesystem that shares blocks (btrfs or XFS). On another it copies in full, about 3 GB a lane. Ryzen's is btrfs, and ticket 17 showed a copied target sharing its blocks.
- A row's printed seconds would include the other lanes' load, so a row's time would no longer compare across proofs.
