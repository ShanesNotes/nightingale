# 17: The parallel sweep is measured on ryzen, where the proofs run

Type: prototype
Status: open
Blocked by: none

**Question.** Does running mutant rows in lanes, in the seed shape [ticket 13](13-mutant-rows-in-parallel.md) recommends, halve the sweep's wall time on ryzen? Ticket 13 is parked on a beelink measurement, and its own hazards say "The halving is not shown on ryzen."

**Why now.** Proof time bounds how many tickets land on the way to the first milestone (SRS invariant I19). Every code proof now runs on ryzen, and the sweep is most of it. Three recent proofs took 7,212 to 7,575 s, and their `40-mutants` step took 5,528 to 5,886 s, about 77% of the wall, by each proof's `time:` lines. The table holds 601 rows on main `446e29f9`. One proof hit the old 7,200 s wall inside the sweep with 515 of 546 rows caught (nightingale#183). The serial order is forced in `tools/mutate.py`. One extract and one target serve every row (lines 67, 68 and 320 to 326). Each row edits that extract in place, then restores it (lines 368 to 376). A `for` loop runs the rows one at a time (lines 378 to 388), and a lock refuses a second sweep in the same checkout (lines 315 to 319). `tools/gate/gate.py`'s `first_stop` reads one child's lines in order and stops at the first row that is not caught (lines 2911 to 2939).

**What to build.** Rerun ticket 13's prototype in the seed shape (lane 0 extracts and builds once, then copies its extract and target to each other lane) on ryzen at one head. Use one builders job, the desk idle, and no other job in its slots.

- [ ] Serial and K = 2, 3 and 4 sweeps of every row at one head, timed twice each, with ryzen's load and free memory beside each number
- [ ] Every lane run gives each row the serial verdict, by name, and a script compares the lists
- [ ] `timed-run-knows-no-input` and the ten slowest rows of [ticket 14](14-slow-mutant-rows-run-long-tests.md) are timed at each K against each row's own limit, the larger of the 420 s budget and twice its test's baseline, and any that would read UNCLEAR is named
- [ ] Ryzen's filesystem is read, and the copy of a lane's target is shown to share blocks, or its full size is stated
- [ ] The Answer says which K halves the sweep, or that none does. If one does, ticket 13's `Blocked by:` line is set to `none` with this ticket's figures. If none does, ticket 14 becomes the next lever, with its expected saving per proof

**Non-goals.** Changing any row, its test or its budget. Merging the prototype. Running it on the beelink again.
