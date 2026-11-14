# 50: Admission's ring solve costs the whole body about 6 s an update

Spec: `crates/ng-contract/src/ring.rs`, `crates/ng-contract/src/elimination.rs` (`Factors::solve_sides`) and `crates/ng-contract/src/admission.rs` (`InitializationSystem::ring`)

Type: task

Status: resolved

Claimed by: nc50-build

Blocked by: none

The whole body admits since part 4 of [body 06](../../body/issues/06-whole-body-on-holds.md), and admission is most of its cost. On the beelink, one job beside one other, the body admitted in 377 to 423 s against the slice's 9.0 to 9.1 s, about 42 times. The body has 1,372 rows and 8,892 Jacobian nonzeros against the slice's 241 and 1,578.

Where the time goes, from an instrumented admission (`~/lanes/logs/ng-wave-4/body-06-p4/05-liver-diagnose.log`): the march takes about 28 s, then 52 Newton updates on the whole ring and 9 sweeps. Each update takes about 0.2 s to linearize and about 6 s to solve, against the slice's 0.05 s and 0.15 s. The updates' solves take about 312 of 382 s.

A profile of one whole run, 480 s of admission and 51 s of running (`08-perf.log`): `Factors::solve_sides` 47%, `InitializationSystem::ring` 10%, `Elimination::factor` 7.6%, `ring::reduced` (the dense corner) 6.8%, and evaluation about 6%. The ring solves each point's factored block against every column the last block reads. The dense corner is not the limiter. Every loop ticket adds rows, and the corner grows as their cube.

On ryzen, beside one or two other jobs at a load of 5.6 to 13 on 12 threads, the body admitted in 558, 578 and 657 s against the slice's 14.9 to 16.4 s, and its test binary took 886 s.

Two things wait on this ticket. The body's run test (`the_whole_body_admits_runs_its_first_minute_and_holds_its_pins` in `crates/ng-validate/tests/body_run.rs`) is ignored, so only the full gate runs it. It leaves the ignored tier, and a mutant row that replays it (`body-trace-pinned`) can land, once its binary costs no more than the slowest test binary the merge gate runs today. That is `crates/ng-validate/tests/arena.rs`: 303 s in the beelink's proof of `7b56123` and 535 s in ryzen's proof of `e9a0a06`. The body's binary took 470 s on the beelink beside one other job and 886 s on ryzen.

The nightly full gate runs that test since part 4: about 470 s more on the beelink, and 886 s on ryzen beside other jobs. The nightly took 3,650 s at main `28d0d78`, at a low CPU weight, against a limit of 7,200 s. A nightly that nears the limit is answered by this ticket's cut or by a longer limit, never by leaving the test out. When the run test rejoins the merge gate, a mutant row shows its held-pool assertion fail by itself: the independent check of nightingale#155 opened an exit on a held pool, and admission refused that body before the assertion ran.

This ticket measures and chooses. It names no fix as chosen.

- [x] The ring solve's share of admission on the whole body is measured on one named build worker, with the update count and each update's linearize and solve seconds
- [x] At least two ways to cut it are compared on that body, each with its admission seconds and the same admitted state, and one is chosen with its reasons
- [x] The body's test binary costs no more than `tests/arena.rs` on the same worker, so its run test leaves the ignored tier
- [x] The slice's model hash, basis and the pins of `crates/ng-validate/tests/trace.rs` stay, and the body's pins in `crates/ng-validate/tests/body_run.rs` stay

**Non-goals:** a step's cost in a run, which [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures; any change to what admission solves.

## Answer

Chosen: the plan's way T, built in three parts. What admission solves does not change, and every admitted bit holds: the body's seal `c28be00fec6e1426` and the slice's `0b8f63bb10c36e97` read the same in every timed run, and no pin or expect string in `tests/trace.rs` or `tests/body_run.rs` moved.

What the plan found. On the beelink, with census off, the body's admission took 390 to 403 s: the march 30.4 s, then 52 Newton updates, each linearized in 0.19 s and solved in 6.44 s on average (334.7 s in all, 83% of admission). The cost was the memory layout, not the arithmetic: the old solve gathered eight side columns at a time out of a 28 MB row-major matrix, and that gather alone took 27% of a profile. Eleven ways were built and timed on the body and the slice with the same seals; walks that stayed row-major (ways A to C) stayed at 327 to 381 s. The plan, its table, profiles and logs are kept outside the repo with the run's evidence.

What was built, each part a commit that holds every pin:

1. The dense corner eliminates a panel of 16 pivots at a time. Each cell takes the same terms in the same order as a pivot at a time gives it. Held by `ring::tests::a_panel_at_a_time_keeps_every_bit_of_a_pivot_at_a_time` (mutant row `panel-order-swapped`).
2. The walk keeps each group of eight sides together, each group's slab in the block's pivot order, and walks only the rows a nonzero side reaches through a nonzero factor. A value or factor that is not finite takes the full walk. Held by `elimination::tests::each_side_keeps_the_bits_it_had_alone_or_beside_others` (its digest unchanged), `a_group_walks_only_the_rows_its_sides_reach` (row `reach-walks-every-row`) and `a_value_that_is_not_finite_takes_the_full_walk_and_keeps_its_bits`. A ring of one leg or of no rows takes the dense path, so the walk needs no refusal for a missing stage plan.
3. Block 0 orders the side columns by the rows each reaches, and each seed's or fit's term goes to its own lane. Held by `ring::tests::a_ring_keeps_the_bits_it_had_on_zeros_of_both_signs`, whose digest is what main's ring gave synthetic rings with zeros of both signs (row `zero-term-skips-the-right-side`). It also holds the order: main's natural order and the reach order give the same digest.

Measured on this branch's head against main, one admission and one simulated second of `tools/bench`, alternating, user instructions under `perf stat`. Wall seconds carry the load; the instruction counts repeat to within 1e6.

| Worker, round | Load (1 min) main / head | Body admission s, main / head | Body instructions, main / head | Slice admission s, main / head | Slice instructions, main / head |
|---|---|---|---|---|---|
| beelink 1 | 18.5 / 11.1 | 604.7 / 276.3 | 4.513e12 / 2.206e12 | 15.4 / 14.9 | 149.35e9 / 145.82e9 |
| beelink 2 | 9.0 / 34.6 | 618.9 / 396.0 | 4.513e12 / 2.206e12 | 11.4 / 19.5 | 149.35e9 / 145.82e9 |
| beelink 3 | 23.3 / 7.4 | 581.8 / 216.5 | 4.513e12 / 2.206e12 | 22.4 / 9.7 | 149.35e9 / 145.82e9 |
| ryzen 1 | 10.0 / 12.3 | 555.1 / 366.6 | 4.512e12 / 2.206e12 | 18.4 / 17.7 | 149.34e9 / 145.81e9 |

The body's admission takes 51% fewer instructions and 37% to 63% fewer cycles; the slice's takes 2.4% fewer instructions. Beside the beelink's other seats' jobs (one other seat's job in one slot, the head's own runs in another), wall seconds moved with the load.

The bar. On ryzen, in one job at a load of 8.7 to 9.7 on 12 threads, `tests/arena.rs` took 593 s and the body's run test binary 407 s, so the run test leaves the ignored tier. On the beelink the run test passed at the last code commit in 380 s beside the timing runs.

When the corner becomes the limiter, an estimate from the plan's profile shares and not measured: the corner is dense and grows as the cube of the last block's rows and the fits, 7.0e9 multiply-adds an update today and 6.4% of the profile after the panel. The walk grows near linearly while new rows form new networks. The corner rules at about four to six times today's rows. The next cut there is to reduce the last block's own sparse part before the dense corner, which changes the pivots and so the bits, so it belongs to a later ticket.

Not built, and why: taking held pools' columns out of the sides (no column is zero over a whole block, and taking them out changes the corner's pivots and so the bits); reusing a block's work across updates (each update relinearizes every block); fewer updates from a better start (it moves pins, and the bar is met without it); reusing the slab buffers across blocks (a small follow-up, not timed); threads (none are allowed, and none are needed).

Two mutant rows replay the run test, and each was timed once on ryzen with `tools/mutate.py --only` beside its other jobs at a load of 8 to 10 on 12 threads: `body-trace-pinned` (the afferent filter's start guess moves, so the admitted start and the pins move) was caught in 248 s, and `body-held-pool-guess` (the held-pool assertion's guess moves while the packet's start does not, so that assertion fails by itself) in 270 s. Both are under 300 s, so both landed. The run test's own baseline took 365 s there; a merge run skips it once `10-rust` has passed the test.

