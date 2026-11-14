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
- [ ] The sweep's wall time, measured before and after on one build worker under the same other load, with the worker named beside each number
- [ ] The PR body says how many rows the sweep ran at once and what bounds it after the change
- [ ] Carried from the check of nightingale#142: a sweep case marks a changed file assume-unchanged after `10-rust` wrote its record, and the sweep then runs every baseline. The case fails when the sweep's second look stops reading the marks

**Non-goals:** changing which rows a proof runs, or any row's test. Sharing a build target between rows or across proofs.
