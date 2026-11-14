# 50: Admission's ring solve costs the whole body about 6 s an update

Spec: `crates/ng-contract/src/ring.rs`, `crates/ng-contract/src/elimination.rs` (`Factors::solve_sides`) and `crates/ng-contract/src/admission.rs` (`InitializationSystem::ring`)

Type: task

Status: claimed

Claimed by: nc50-build

Blocked by: none

The whole body admits since part 4 of [body 06](../../body/issues/06-whole-body-on-holds.md), and admission is most of its cost. On the beelink, one job beside one other, the body admitted in 377 to 423 s against the slice's 9.0 to 9.1 s, about 42 times. The body has 1,372 rows and 8,892 Jacobian nonzeros against the slice's 241 and 1,578.

Where the time goes, from an instrumented admission (`~/lanes/logs/ng-wave-4/body-06-p4/05-liver-diagnose.log`): the march takes about 28 s, then 52 Newton updates on the whole ring and 9 sweeps. Each update takes about 0.2 s to linearize and about 6 s to solve, against the slice's 0.05 s and 0.15 s. The updates' solves take about 312 of 382 s.

A profile of one whole run, 480 s of admission and 51 s of running (`08-perf.log`): `Factors::solve_sides` 47%, `InitializationSystem::ring` 10%, `Elimination::factor` 7.6%, `ring::reduced` (the dense corner) 6.8%, and evaluation about 6%. The ring solves each point's factored block against every column the last block reads. The dense corner is not the limiter. Every loop ticket adds rows, and the corner grows as their cube.

On ryzen, beside one or two other jobs at a load of 5.6 to 13 on 12 threads, the body admitted in 558, 578 and 657 s against the slice's 14.9 to 16.4 s, and its test binary took 886 s.

Two things wait on this ticket. The body's run test (`the_whole_body_admits_runs_its_first_minute_and_holds_its_pins` in `crates/ng-validate/tests/body_run.rs`) is ignored, so only the full gate runs it. It leaves the ignored tier, and a mutant row that replays it (`body-trace-pinned`) can land, once its binary costs no more than the slowest test binary the merge gate runs today. That is `crates/ng-validate/tests/arena.rs`: 303 s in the beelink's proof of `7b56123` and 535 s in ryzen's proof of `e9a0a06`. The body's binary took 470 s on the beelink beside one other job and 886 s on ryzen.

The nightly full gate runs that test since part 4: about 470 s more on the beelink, and 886 s on ryzen beside other jobs. The nightly took 3,650 s at main `28d0d78`, at a low CPU weight, against a limit of 7,200 s. A nightly that nears the limit is answered by this ticket's cut or by a longer limit, never by leaving the test out. When the run test rejoins the merge gate, a mutant row shows its held-pool assertion fail by itself: the independent check of nightingale#155 opened an exit on a held pool, and admission refused that body before the assertion ran.

This ticket measures and chooses. It names no fix as chosen.

- [ ] The ring solve's share of admission on the whole body is measured on one named build worker, with the update count and each update's linearize and solve seconds
- [ ] At least two ways to cut it are compared on that body, each with its admission seconds and the same admitted state, and one is chosen with its reasons
- [ ] The body's test binary costs no more than `tests/arena.rs` on the same worker, so its run test leaves the ignored tier
- [ ] The slice's model hash, basis and the pins of `crates/ng-validate/tests/trace.rs` stay, and the body's pins in `crates/ng-validate/tests/body_run.rs` stay

**Non-goals:** a step's cost in a run, which [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures; any change to what admission solves.
