# 23: Every must-fail row has its gate

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Every row of §7's must-fail table maps to a test that builds that core or model and shows its named gate rejecting it. Each gate's ticket wires its own rows. This ticket lists every row with its test and wires any row that no ticket owns.

Blocked by: 13, 14, 15, 16, 18, 19, 20, 22

Status: resolved

- [x] Every row of §7's must-fail table has a test in which its named gate rejects that core or model

## Answer

`tools/gate/must-fail.toml` pairs each of §7's 27 must-fail rows with what rejects its core: tests of its gate, each named through a mutant row of `tools/gate/mutants.toml` the test must fail under, and compile_fail doctests for a core that does not compile. Most mutants turn the core into one built to pass. Three change the record, the refusal sources or the health count, where no core can be built. 05-seam reads the rows from the contract and refuses a row with no entry, an entry with no row or with neither a mutant nor a doctest, a mutant the table lacks, a test that is no `#[test]` fn of its target, and a doctest that is not there (`seam.must-fail`, `seam.must-fail-test`, each with a probe that bites). 40-mutants replays the 30 mutants. All 30 were caught at 894fb4b, and `gate.py codes` proves the doctests' codes.

What this ticket wired:

- **Late admission now checks the withheld start.** A case whose `admit_withheld` already held an input passed the gate (ticket 18). The gate now refuses an input whose `enqueue` leaves the waiting digest where it was, as `NoLateBoundary`. The test failed on main.
- **Stale coordinates and the sweep limit have slice cores** in `crates/ng-validate/tests/window.rs`. `Sweeps` stops a doubled backward Euler stage after a number of Newton updates and can book blocks back at the coordinates the stage started from. Every block outside the strongly coupled one, held at its start, fails the first window as `ModesApart`. At 5 ms with scales of 1 and 1e-2, validate refuses one update and the stale blocks on their stage residual, while the honest core's step is accepted.
- **The energy row has its core.** `HoldsTheWallPower` solves each stage with the wall power held at the stage's start, and validate refuses it on the wall power row.
- **The posture row has its test.** A history kept across a posture is a method mismatch at the posture, on the closed two-reservoir body.

Three rows rest on the nearest thing the crates can build:

- *Applies a due input before the attempt and keeps it after a refusal*: no core can do it. `attempt` takes `&self`, a core reaches no pending input (E0609), and nothing publishes through a lent record (E0596). The snapshot test's mutant leaves the due input due, and the test sees it applied twice.
- *Holding a concentration whose store is in the slice*: a held declaration holds pools, so the test is a held pool a slice law moves (`HeldPoolMoved`). A held reservoir lands with the second book.
- *A reaction slipped into the schema*: the crate has no reaction type, and a binding is a paired transfer. The compile refusal that names the process is `CargoStranded`, a species one end of a route lacks.

The table at resolution, generated from the two TOML files:

| §7 row | Test that rejects the core (mutant that makes it pass) | compile_fail doctest |
|---|---|---|
| Forward Euler on the two-reservoir case with h > RC and a nonzero initial difference: every ledger closes, the difference grows | `ng-validate` `transaction::forward_euler_past_rc_closes_every_ledger_and_fails_the_accuracy_gate` (`forward-euler-inside-rc`) | none |
| Accepts a step whose normalized error exceeds one at its minimum interval | `ng-validate` `transaction::an_estimate_above_one_at_the_floor_faults_and_never_publishes` (`error-above-one-floor-lowered`) | none |
| Accepts after the sweep limit on a strongly coupled fixture whose change is still above tolerance | `ng-validate` `window::validate_refuses_a_slice_stage_its_core_left_unsolved` (`sweeps-converge-on-the-slice`)<br>`ng-validate` `window::the_first_window_holds_each_core_to_the_references_own_distances` (`sweeps-converge-in-the-window`) | none |
| Evaluates blocks at stale coordinates under the coupled plan | `ng-validate` `window::validate_refuses_a_slice_stage_its_core_left_unsolved` (`no-block-behind-on-the-slice`)<br>`ng-validate` `window::the_first_window_holds_each_core_to_the_references_own_distances` (`no-block-behind-in-the-window`) | none |
| Books one sample at the start while its endpoint came from other stages (rate 1 + t over h = 1: one sample books 1, the trapezoid 1.5) | `ng-validate` `transaction::a_core_that_books_one_sample_while_its_endpoint_came_from_other_stages_faults` (`one-sample-books-heun`) | `crates/ng-contract/src/lib.rs` E0451 `Extents { layout, forward, reverse }` |
| Adds an evaluation from an earlier attempt | `ng-validate` `transaction::an_evaluation_from_an_earlier_attempt_is_a_binding_mismatch` (`earlier-attempt-books-its-own`) | none |
| Qualifies a split from invented samples | none | `crates/ng-record/src/lib.rs` E0599 `SplitTrace::new(id, grid, state, exchange, observable, events)` |
| Applies a due input before the attempt and keeps it after a refusal | `ng-validate` `transaction::a_forced_refusal_changes_nothing_and_the_retry_applies_the_delivery_once` (`due-input-stays-due`) | `crates/ng-contract/src/lib.rs` E0609 `accepted.pending`<br>`crates/ng-record/src/lib.rs` E0596 `session.record().publish(next, store)` |
| Keeps a counter or cache outside its continuation | `ng-validate` `replay::a_core_that_keeps_a_cache_outside_its_continuation_fails_resume_and_rewind` (`cache-never-counts`) | none |
| Spreads its steps toward a known later input | `ng-validate` `replay::a_core_told_the_timeline_in_only_one_run_fails_late_admission` (`told-the-timeline-ignores-it`) | `crates/ng-contract/src/lib.rs` E0609 `accepted.pending` |
| Asks for a huge span to read the next breakpoint | `ng-validate` `replay::a_core_that_tries_to_read_past_the_breakpoint_fails_late_admission` (`huge-span-capped`) | `crates/ng-contract/src/lib.rs` E0599 `problem.ticket(1 << 60)` |
| Reads a cut, refuses it and spreads its steps toward that instant | `ng-validate` `replay::a_core_that_acts_on_a_cut_it_read_has_no_late_boundary` (`cut-reader-starts-short`) | none |
| Clips at caller output times | `ng-validate` `replay::a_core_that_clips_at_caller_output_times_fails_every_gate` (`clipper-ignores-outputs`) | `crates/ng-validate/src/lib.rs` E0061 `session.step(stop)` |
| Reads the host's clock | `ng-validate` `replay::a_core_that_reads_the_host_clock_fails_pace` (`host-clock-unread`) | none |
| Draws chance per solver step or attempt | `ng-validate` `chance::a_core_that_draws_per_step_passes_every_bit_gate_and_fails_the_step_policy_gate` (`step-dice-reach-nothing`) | `crates/ng-contract/src/lib.rs` E0599 `problem.threshold(0, 1)`<br>`crates/ng-contract/src/lib.rs` E0616 `problem.identity` |
| Draws chance per evaluation | `ng-validate` `chance::a_core_that_draws_per_evaluation_fails_the_extra_evaluation_gate` (`evaluation-dice-fixed`) | none |
| Biases every step alike, so its own two policies agree | `ng-validate` `chance::a_core_that_delays_every_event_alike_fails_against_a_reference_policy` (`delay-of-nothing`) | none |
| Refuses every landing, hoping to skip an event | `ng-validate` `chance::a_core_that_refuses_every_landing_faults_and_publishes_nothing_past_the_crossing` (`landings-taken`) | none |
| Keeps its predictor or Jacobian across a firing | `ng-validate` `chance::a_core_that_keeps_its_history_across_a_firing_is_a_method_mismatch` (`firing-restarts-history`) | none |
| Keeps its predictor or Jacobian across a posture breakpoint | `ng-validate` `transaction::a_history_kept_across_a_posture_faults_with_method_mismatch` (`posture-restarts-history`) | none |
| Refuses on a reference range | `ng-validate` `transaction::the_refusal_sources_list_each_rows_set_and_each_laws_operators` (`divisor-not-a-source`)<br>`ng-validate` `throughput::a_slice_window_that_faults_at_the_floor_reports_its_health_up_to_the_fault` (`refusal-not-counted`) | `crates/ng-contract/src/lib.rs` E0599 `Admissible::new(low, high)`<br>`crates/ng-contract/src/lib.rs` E0061 `k.finish(shifted, Admissible::nonnegative(), modes, class)` |
| A pressure-volume update whose volumes pair but whose elastic energy does not close | `ng-validate` `window::validate_refuses_a_slice_stage_its_core_left_unsolved` (`wall-power-solved`) | none |
| A held stand-in using a route its declaration omits, or holding a concentration whose store is in the slice | `ng-contract` `held::a_held_route_its_declaration_omits_is_refused_at_compile` (`held-route-declared`)<br>`ng-contract` `held::a_space_a_law_moves_cannot_be_held` (`held-pool-left-to-the-slice`) | none |
| An orphan pool | `ng-contract` `held::a_pool_no_law_moves_and_no_declaration_holds_is_an_orphan` (`orphan-held`) | none |
| A reaction that creates an element, written outside a scope or slipped into the schema | `ng-contract` `map_reads::a_species_one_end_of_a_route_lacks_is_refused_until_the_route_keeps_it_back` (`cargo-held-at-both-ends`) | `crates/ng-contract/src/lib.rs` E0451 `SiteRoute { family: todo!(), brand: todo!(), kind: todo!() }` |
| A kernel calling host math on an expression | none | `crates/ng-contract/src/lib.rs` E0599 `x.value().exp()` |
| An initializer returning a packet guess that violates a row or anchor | `ng-validate` `admission::an_initializer_that_returns_its_guess_is_refused_with_the_row_named` (`guess-that-solves`)<br>`ng-validate` `admission::a_witness_that_holds_every_row_and_misses_its_anchor_is_refused_with_the_anchor_named` (`anchor-met`) | none |

## Comments

From 08: `admit_fixture` returns the packet's guess, which is §7's must-fail initializer, and `admit_solved` solves.

From 10: a core that breaks the elastic store's closure fails the stage residual on `wall power`, so that is the row for §7's energy must-fail.

From 03: ticket 03's third box waits on the two split probes in ticket 20.

From foundation 07: before either 48 h core runs, the build proves incidence and energy closure, validate-then-publish refusal with nonempty pending inputs, the generated census gate and the replay gates.

From 19: §7's table has no observer row. The observer gates' must-fail observers are in-crate tests, because only a test can build an observer that is not a function of its views: `replay::tests::an_observer_that_drifts_while_the_accepted_state_matches_fails_pace_resume_late_admission_and_rewind` and `an_observer_that_forgets_its_state_on_resume_fails_resume_and_rewind_and_passes_the_rest`.
From 15: the four new accuracy cases are not rows of §7's table, and each has its must-fail core in `crates/ng-validate/tests/accuracy.rs`, in `each_case_rejects_the_core_built_to_fail_it_and_the_stepped_inflow_passes_them_all`: refuses near empty (drain), leak guard (drain and removal), relative Newton stop (zero flow), damping kept after a refused evaluation (independence). `an_independence_case_whose_core_evaluated_nothing_tested_nothing` holds `NothingRefused`.

From 18: the seven replay gates run on `SliceCase` in about 62 s with the reference core at 3 ms (Omarchy). A gate's runs take from about 2 s (extra evaluation) to about 28 s (resume), plus one admission solve of about 6.5 s for each core resolution it needs, so a slice row can run its one gate through `replay_gate`. No gate checks that a case's `admit_withheld` withholds anything. A `SliceCase` wrapper whose withheld start knew both inputs passed the late-admission gate (measured), because `enqueue` takes the same id and payload as the same request. The slice's test checks the waiting digests itself, and a gate could refuse an input whose `enqueue` leaves the waiting digest where it was.

From 16: §7's row "Accepts after the sweep limit on a strongly coupled fixture whose change is still above tolerance" names the accuracy gate beside `validate`. Its accuracy half is `AcceptsAfterSweepLimit` in `crates/ng-validate/tests/window.rs`, which stops each Newton stage after one update and declares scales loose enough for `validate` to accept it. It passes every manufactured case, and the first slice window rejects it as `Outside`. The row "Evaluates blocks at stale coordinates under the coupled plan" has no window core yet.

From 22: §7's table has no row for evidence binding. Its must-fail evidence is in `crates/ng-validate/tests/score.rs`, one test for each kind of evidence taken from another run, and a compile-fail `E0451` for a score built outside `score`.

From 20: §7's row "Qualifies a split from invented samples" is the compile-fail doctest on `SplitTrace::new` (E0599) in `crates/ng-record/src/lib.rs`. The split gate's must-fail core is `a_split_over_its_budget_is_refused` in `crates/ng-validate/tests/split.rs`, a coupled core at 32 times the reference's interval. Both split probes that ticket 03's third box waited on now hold in the crates.
