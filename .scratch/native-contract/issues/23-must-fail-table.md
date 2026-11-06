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

From 20: §7's row "Qualifies a split from invented samples" is the compile-fail doctest on `SplitTrace::new` (E0599) in `crates/ng-record/src/lib.rs`. The split gate's must-fail core is `a_split_over_its_budget_is_refused` in `crates/ng-validate/tests/split.rs`, a coupled core at 32 times the reference's interval. Both split probes that ticket 03's third box waited on now hold in the crates.
