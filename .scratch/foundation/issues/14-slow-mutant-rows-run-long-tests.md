# 14: The slowest mutant rows each run a long test

Type: task
Status: open
Blocked by: none

**Why.** Proof time bounds how many body tickets land in a day, and the body tickets are the road to the first milestone (SRS invariant I19). A mutant row costs its rebuild plus its test. Most rows run fast tests, but a few run a test that takes minutes, so they cost more than their share of the sweep and they are the rows that load can push past a cargo test's 420 s. [Ticket 13](13-mutant-rows-in-parallel.md) did not halve the sweep on the beelink, and its plan names moving these rows to other tests as one of the levers left.

**What was measured.** The plan for ticket 13 (`~/lanes/logs/ng-wave-4/f13/PLAN.md`, section 1) read one proof's sweep on ryzen, beside a 14-hour bench: 341 rows and 3,106 s of rows, a median row of 7 s and a 90th percentile of 13 s. The slowest rows:

| Row | Seconds |
|---|---|
| `timed-run-knows-no-input` (the arena test) | 280 |
| `day-waits-on-the-timed-run` | 86 |
| three window rows | 62 to 63 each |
| `day-merge-drops-the-last-event` | 61 |
| `wall-power-solved` | 53 |

The ten slowest took 718 s, 23% of the rows' time. The plan does not measure what a shorter test would save for any one row. The 718 s is the most these ten could save. The same plan found that `timed-run-knows-no-input` took 296 to 306 s at K=6 on the beelink, so beside other load it nears the 420 s at which a row reads UNCLEAR.

The newest case is two rows that left nightingale#160 and waited on [body 06](../../body/issues/06-whole-body-on-holds.md), which resolved with nightingale#172: `body-trace-pinned` and `body-held-pool-guess`. Each runs the body's run test, which is one body admission plus a rebuild. They took 248 and 270 s on ryzen, and 278 and 281 s inside that PR's proof ([native-contract 50](../../native-contract/issues/50-admission-ring-solve-cost.md), its last paragraph). On a loaded beelink they would pass 420 s.

The sweep is now most of a proof. In the proof of nightingale#167 on the beelink, beside one other job, it took 3,025 s for 419 rows of a 4,135 s proof, against 2,864 and 2,887 s in the two beelink proofs before it. That proof is over the 3,600 s that [body 26](../../body/issues/26-cost-budget.md) reports as a full run's ceiling there. Body 26 also left three lines of the body's run test that no row holds, because each mutant needs a body admission: the comparison of the four counts with their ceilings, the count of solves, and the accessor that feeds the Jacobian's nonzeros (read in that PR's check, not run).

- [ ] For each row above, the Answer names the shortest test that still catches its mutant, or says why none does. A row stays on its long test when only that test sees the mutation
- [ ] Each row moved to a shorter test is still CAUGHT, run alone with `tools/mutate.py --only`, and its seconds before and after are measured on one named build worker
- [ ] The sweep's wall time before and after, on one build worker under the same other load, with the worker named beside each number
- [ ] Body 06's two rows land, or the Answer says what a run of one would have to cost first
- [ ] A row holds the run test's comparison of counts with ceilings (body 26), or the Answer says what a run of one would have to cost first
- [ ] A row can name its own budget, so a row that runs one body admission is held to its own limit and not to the sweep's one 420 s. Body 06 part 4c left both body rows out for this: each is one rebuild and one body admission, about 300 s on a quiet beelink and up to twice that under load, against the sweep's 420 s. The table's `tools/mutate.py` reads one budget for every row today

**Non-goals:** running rows in parallel, which is ticket 13. Changing a row's mutation, or what any test checks. Raising the 420 s a cargo test has.
