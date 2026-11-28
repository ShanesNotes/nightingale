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

- [x] For each row above, the Answer names the shortest test that still catches its mutant, or says why none does. A row stays on its long test when only that test sees the mutation
- [x] Each row moved to a shorter test is still CAUGHT, run alone with `tools/mutate.py --only`, and its seconds before and after are measured on one named build worker
- [ ] The sweep's wall time before and after, on one build worker under the same other load, with the worker named beside each number
- [ ] Body 06's two rows land, or the Answer says what a run of one would have to cost first
- [ ] A row holds the run test's comparison of counts with ceilings (body 26), or the Answer says what a run of one would have to cost first
- [x] A row can name its own budget, so a row that runs one body admission is held to its own limit and not to the sweep's one 420 s. Body 06 part 4c left both body rows out for this: each is one rebuild and one body admission, about 300 s on a quiet beelink and up to twice that under load, against the sweep's 420 s. The table's `tools/mutate.py` reads one budget for every row today

The last box lands with [ticket 13](13-mutant-rows-in-parallel.md)'s PR. [Ticket 17](17-the-parallel-sweep-is-measured-on-ryzen.md) found that four lanes halved the sweep with a budget of 1,000 s for every row and not with 420 s. The other boxes are unchanged.

**Non-goals:** running rows in parallel, which is ticket 13. Changing a row's mutation, or what any test checks. Raising the 420 s a cargo test has.

## Comments

Partial progress landed alongside [ticket 13](13-mutant-rows-in-parallel.md) on main `5cf9fac2` (ticket remains `open` for boxes 3, 4 and 5):

- **Box 6 (per-row `budget`):** `tools/mutate.py` now supports an optional positive integer `budget` on `[[mutant]]` rows (`limit = max(--budget, row.budget, 2 * base_s)`, baseline limit `max(4 * --budget, 4 * max_row_budget)`), probed in `gate.py`'s `limit_case`. The 10 whole-body admission `body_run` rows in `tools/gate/mutants.d/00-base.toml` carry `budget = 1000`.
- **Boxes 1 and 2 (empirical audit of the named rows on `beelink`, `debug = 0`, load ~21):**
  - `day-merge-drops-the-last-event`: moved from `-p ng-validate --lib day::tests::a_packet_holding_the_reference_day_admits_the_timeline_the_arenas_port_does` to `-p ng-contract --lib boundary::day::tests::an_event_at_or_after_the_moment_enters_as_the_port_parses_it_and_a_meal_as_its_parts`. Measured alone via `tools/mutate.py --only` on `beelink`: CAUGHT in `17 s` row / `8 s` baseline vs `32 s` row / `33 s` baseline before (-47% row / -76% baseline).
  - `timed-run-knows-no-input`: probed on `-p ng-validate --test arena one_arena_scores_the_reference_and_ng_solver_with_one_call_each` -> `SURVIVED` (`short()` 2 s ends before the 07:00 sit-up). Stays on `past_the_sit_up_the_day_checks_end_on_the_timed_runs_seal`.
  - `day-waits-on-the-timed-run`: `a_clock_with_no_reading_leaves_the_day_checks_and_accuracy_standing` is the only test exercising `run_day` with `&Stopped`. Stays.
  - `window-reads-every-finest-step`: probed on `-p ng-validate --lib window::tests::the_first_window_holds_each_core_to_the_references_own_distances` -> `SURVIVED` (does not vary grid phase in the beat). Stays on `window::tests::honest_cores_pass_wherever_the_grid_falls_in_the_beat`.
  - `sweeps-converge-in-the-window`, `no-block-behind-in-the-window`, `wall-power-solved`: pinned in `tools/gate/must-fail.toml` (lines 27, 31, 125) to `ng-validate --test window`; `wall-power-solved` mutates `tests/window.rs` directly where only `validate_refuses_a_slice_stage_its_core_left_unsolved` exercises `self.wall`. All three stay.
- **Note on Box 5:** `cost-ceiling-lets-one-solve-over` already sits in `tools/gate/mutants.d/00-base.toml` on `-p ng-validate --test body_run a_count_over_its_ceiling_fails_and_names_that_count`; Box 4 (`body-trace-pinned`, `body-held-pool-guess`) and the remaining body 26 lines remain open.
