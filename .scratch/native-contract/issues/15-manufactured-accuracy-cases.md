# 15: The accuracy gate runs every manufactured case

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** `accuracy` runs every manufactured case that §7 lists, not only `AccuracyCase::TwoReservoir`. The new cases are drain and refill to the admissible bound, zero flow and restore, and controlled removal with every removed amount in its receipt. The last is evaluation independence: one stage gives identical residual and Jacobian bits before and after refused evaluations. Each case has a closed form or an exact identity, and each ships with a core built to fail it.

Blocked by: None (can start immediately)

Status: ready-for-agent

- [ ] The reference core passes each manufactured case in §7's accuracy gate, and a core built to fail each case is rejected

## Comments

From 04: `accuracy(solver, policy)` runs `AccuracyCase::TwoReservoir` only. Its comparison of gross inflow cannot fail today, because the frozen drive makes it exact for any step that `verify` passes. It guards the frame itself.

From the split of ticket 11: the accuracy tolerances are policy values. Each takes a model default, set in the validation crate and reported with every result (contract §7, foundation 07).
