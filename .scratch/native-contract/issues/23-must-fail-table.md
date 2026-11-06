# 23: Every must-fail row has its gate

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Every row of §7's must-fail table maps to a test that builds that core or model and shows its named gate rejecting it. Each gate's ticket wires its own rows. This ticket lists every row with its test and wires any row that no ticket owns.

Blocked by: 13, 14, 15, 16, 18, 19, 20, 22

Status: ready-for-agent

- [ ] Every row of §7's must-fail table has a test in which its named gate rejects that core or model

## Comments

From 08: `admit_fixture` returns the packet's guess, which is §7's must-fail initializer, and `admit_solved` solves.

From 10: a core that breaks the elastic store's closure fails the stage residual on `wall power`, so that is the row for §7's energy must-fail.

From 03: ticket 03's third box waits on the two split probes in ticket 20.

From foundation 07: before either 48 h core runs, the build proves incidence and energy closure, validate-then-publish refusal with nonempty pending inputs, the generated census gate and the replay gates.

From 19: §7's table has no observer row. The observer gates' must-fail observers are in-crate tests, because only a test can build an observer that is not a function of its views: `replay::tests::an_observer_that_drifts_while_the_accepted_state_matches_fails_pace_resume_late_admission_and_rewind` and `an_observer_that_forgets_its_state_on_resume_fails_resume_and_rewind_and_passes_the_rest`.
From 15: the four new accuracy cases are not rows of §7's table, and each has its must-fail core in `crates/ng-validate/tests/accuracy.rs`, in `each_case_rejects_the_core_built_to_fail_it_and_the_stepped_inflow_passes_them_all`: refuses near empty (drain), leak guard (drain and removal), relative Newton stop (zero flow), damping kept after a refused evaluation (independence). `an_independence_case_whose_core_evaluated_nothing_tested_nothing` holds `NothingRefused`.

From 18: the seven replay gates run on `SliceCase` in about 62 s with the reference core at 3 ms (Omarchy). A gate's runs take from about 2 s (extra evaluation) to about 28 s (resume), plus one admission solve of about 6.5 s for each core resolution it needs, so a slice row can run its one gate through `replay_gate`. No gate checks that a case's `admit_withheld` withholds anything. A `SliceCase` wrapper whose withheld start knew both inputs passed the late-admission gate (measured), because `enqueue` takes the same id and payload as the same request. The slice's test checks the waiting digests itself, and a gate could refuse an input whose `enqueue` leaves the waiting digest where it was.
