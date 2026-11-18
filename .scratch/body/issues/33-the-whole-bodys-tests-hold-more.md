# 33: The whole body's tests hold more

Type: task
Status: open
Blocked by: none

**Why.** These are debts in the strength of the whole body's tests, not in the body's model. [Ticket 06](06-whole-body-on-holds.md) resolved once the body marked its veins, so the first loops (tickets 08, 13, 17 and 22) start from there. The debts close before pass 1 is checked, so [ticket 24](24-the-synthetic-day.md) waits on this ticket.

**What to build:** the three tests ticket 06 left owed, and the low notes the independent check of nightingale#164 left open.

- [ ] A core built to break each replay gate runs on the body's case. Today every such core runs on a fixture case (`TwoReservoirs` in `tests/replay.rs`, `ChanceCase` in `tests/chance.rs`), and none runs on the slice. The seven gates on the body are held by the honest pass and its counted evidence alone: pace, resume, late admission, rewind, extra evaluation, step policy and non-perturbation. The independent check of nightingale#164 ran two gate rows at body cost: the honest pass catches a broken resume of the observer (`observer-checkpoint-drops-held`, `Diverged { gate: Resume }` in 251.8 s), and cannot see the late-admission gate's own refusals removed (`withheld-start-unchecked` survives in 319.2 s), which `TwoReservoirs` holds. Low: no test pins the case's bolus length (`ends` one second after its stamp); the check's mutant of it to two seconds survived
- [ ] A keep-back line is tied to its law module. `KEEP_BACKS` mirrors each route's `keeps_back` lines, and an inert line keeps back a species that neither end of its route holds, so deleting Starling's `keeps_back::<Ammonium>` passes every test. Tying the two needs an accessor in the law crates that lists each route's keep-back lines
- [ ] The mutant rows `body-trace-pinned` and `body-held-pool-guess` land. [Foundation 14](../../foundation/issues/14-slow-mutant-rows-run-long-tests.md) already carries them in its boxes, with a row that names its own budget. `body-trace-pinned` moves the afferent filter's start guess, so the admitted start and the pins move. `body-held-pool-guess` moves the held-pool assertion's guess while the packet's start does not, so that assertion fails by itself. Each runs the body's run test, one body admission plus a rebuild. On ryzen at a load of 8 to 10 on 12 threads, `tools/mutate.py --only` caught them in 248 and 270 s, and inside nightingale#160's proof they took 278 and 281 s, against a row's 420 s. A body admission took 396 s on the beelink at a load of 34.6, so on a loaded beelink a row would pass 420 s and read UNCLEAR

From the independent check of nightingale#164, its second round, in its own words:

- [ ] The refusal's test pins its edge and not its reach. One more call in `slice.rs`'s test would close all three: an input at the fence followed by one at the anchor, expecting `EventBeforeFence`
- [ ] The refused id is unpinned. The test's `..` could name it
- [ ] The fixture does not read the case's end. `assembly_fence.rs` could assert `case.end() > fence`, as `arena_body.rs:1058` does on the slice
- [ ] Tickets 06 and 56 and the row's name write "the anchor" for the anchor moment. The glossary's Anchor is a target a quantity must meet

From the independent check of nightingale#192: three of the heart's work's choices are caught fast only by the body's pin, because the staged harness starts in systole and has no diastole leg.

- [ ] The work memory's fall in diastole (`cv.work-speed` toward zero), which the run's rest check sees and no staged test does
- [ ] The packet's oxygen for each unit of energy, `mt.oxygen-per-energy`, which the run's rest check sees and no staged test does
- [ ] The paying bed's basal value, the stated 8 less the work's oxygen, which no staged test sees. The check did not try it against the run's tests

**Non-goals:** any change to the body's model, its packet or its pins. A row budget, which foundation 14 owns.
