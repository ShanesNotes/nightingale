# 20: A split is measured from accepted history

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The crates build split measurement as the contract lays it out: `PatientRecord::split_trace` draws a trace from accepted history, and `SplitMeasurement::qualify` is the only producer of a `SplitQualification`. It scores the four-family metric (state, exchange integral, observable, event time) against the reference integrator. §7's split gate then reads it, and runs under measurement are never scored.

Blocked by: None (can start immediately)

Status: ready-for-agent

- [ ] A split qualification comes only from a measurement over traces from accepted history, scored against the reference integrator, and the sketch's two split probes hold in the crates

## Comments

From 03: the sketch's probes at `docs/architecture/solver-contract-sketch/src/lib.rs:218` (a split qualification is not bytes, E0308) and `:232` (no split trace from invented samples, E0599) are rows of §7's must-fail table. No ticket scoped split measurement itself, which both of them need. Ticket 03's third box stays open on them.

From 04: block scope is additive in the sketch. `evaluate_block`, `linearize_block` and `quadrature_for` sit beside the whole-body calls, so a whole-scope core never changes.

From the split of ticket 11: the split gate judges solver-B (contract §7). Whether solver-B passes the full metric without dt-aware organ updates and incomplete sweep acceptance is a question only the arena answers (foundation 07).
