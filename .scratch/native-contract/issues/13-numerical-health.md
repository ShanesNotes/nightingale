# 13: The arena run reports its numerical health

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** `RunEvidence` carries a `NumericalHealth` report beside the certificate, as the sketch has it (`docs/architecture/solver-contract-sketch/src/ng_validate.rs:348`). The report counts refusals by reason and by refusal source, faults at the minimum interval and validity excursions. A step's receipt and the session's refusals feed it.

Blocked by: 12

Status: ready-for-agent

- [ ] A short slice window's `run` reports refusals by reason and by source, floor faults and validity excursions, apart from the certificate

## Comments

From 04: `NumericalHealth` is not built. A receipt's excursions and a session's refusals feed it.

From 10: a stage past a guard is `GuardInside`, a new refusal that the report counts.

From 09 and 10, on finding 17: a step stops at the body's tightest rate class and at a forcing's knot, and the record enforces both, so the report can list the recorded classes. An unmoved row's rate class is the slowest band with no limit, so a list of longest intervals skips it.

From the engine bench (nightingale#31): on the slice at 5 ms, 29 percent of attempts were refused and an accepted step took 20 Newton updates, so the refusal counts are worth reading beside throughput.
