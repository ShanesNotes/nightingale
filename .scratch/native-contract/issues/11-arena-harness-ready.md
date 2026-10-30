# 11: Arena harness ready to dispatch

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The validation crate runs the slice for a short window with the reference core under every gate in §7 and every must-fail row wired, and reports the certificate, accuracy, replay and numerical-health evidence separately, so the arena seats can be dispatched against it.

Blocked by: 05, 06, 07, 10

Status: ready-for-agent

- [ ] Every must-fail core or model in the §7 table is rejected by its named gate
- [ ] A short slice window passes certificate, accuracy and all replay gates with the reference core
- [ ] The run reports throughput beside the record, never inside the certificate
- [ ] The arena seat contracts can name this harness and its report schema

## Comments

From 03: sketch probes this ticket owns: lines 218 (a split qualification is not bytes, E0308) and 232 (no split trace from invented samples, E0599). Both are rows of the §7 must-fail table. No ticket scopes split measurement itself, which both of them need.
