# 13: The mutant sweep runs its rows in parallel

Type: task
Status: claimed
Claimed by: sweep-12-13
Blocked by: none

**What to build:** `40-mutants` runs its rows side by side instead of one after another, and the gate means what it means today. One seat does this ticket and [ticket 12](12-mutant-baseline-once.md), because both change the sweep in `tools/mutate.py` and its driver in `tools/gate/gate.py`.

**Why.** The serial sweep, not the machine, bounds a proof. On one uncapped full proof of main on a build worker, `tools/verify.d/40-mutants.sh` replayed its 113 rows one after another and took 1,312 of the proof's 2,042 seconds (64%), while the worker's mean load was 5.5 of its 16 threads. Two whole proofs at once on that worker gave 3.05 proofs an hour against 1.76 for one at a time. Both are measured. The idle threads bear on ticket 12's last box too.

**The trap.** One cargo target directory shared by two checkouts links the other checkout's patched crate. Today a sweep extracts the commit once to `target/mutate/` and builds every row in that one tree and its one target. So each row that runs beside another needs its own extract and its own target, and none may read another's.

What must not change:

- Each row is judged as today: the same verdicts (CAUGHT, CAUGHT at build, SURVIVED, NOT APPLIED, UNCLEAR), printed in the line `gate.py` reads.
- [Ticket 11](11-proof-fails-fast.md)'s order and stop: the rows a change adds or edits run first, and the sweep stops at its first row that is not CAUGHT and names the rows not run.
- A hand run of `tools/mutate.py` gives the same verdicts as the gate's run.

- [ ] `40-mutants` runs more than one row at once, each in its own extract with its own target, and how many run at once follows the machine's threads
- [ ] On one head, a parallel sweep and a serial sweep catch the same rows, row for row, and a check compares the two lists
- [ ] A seeded surviving row stops a parallel sweep as it stops a serial one, and the stop line names the rows not run
- [ ] The sweep's wall time, measured before and after on one build worker under the same other load, with the worker named beside each number
- [ ] The PR body says how many rows the sweep ran at once and what bounds it after the change

**Non-goals:** changing which rows a proof runs, or any row's test. Sharing a build target between rows or across proofs.
