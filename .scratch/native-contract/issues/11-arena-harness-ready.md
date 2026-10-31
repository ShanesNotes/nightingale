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

From 04: `accuracy(solver, policy)` runs `AccuracyCase::TwoReservoir` only, and `NumericalHealth` is not built; a receipt's excursions and a session's refusals feed it. In the sketch block scope is now additive: `evaluate_block`, `linearize_block` and `quadrature_for` sit beside the whole-body calls, so a whole-scope core never changes. Measure speed early: a whole step costs about 40 µs on the two-row bench, about 3,500 s for 48 h at a 2 ms step, against the 1,728 s budget, and `chain` rehashes the whole image per step. The `accuracy` comparison of gross inflow cannot fail today, because the frozen drive makes it exact for any step `verify` passes. It guards the frame itself.

Decided after the deep review of 02 to 04 (nightingale#13, findings 12 to 17): finding 16: minimum coverage becomes a field of the certificate policy, with no fixed 48 h floor. The short slice window here passes with its coverage stated in the report. Finding 17: rate-class enforcement is designed in ticket 10 after its speed cost is measured, so this harness reports the recorded classes and does not enforce them.

From 05: the gates take `&TwoReservoirs`, with no case trait, so this ticket adds one if 06 has not. `ReplayFailure::Diverged` maps to `ArenaFailure::Replay`, and `ReplayEvidence` has no public constructor. `crates/ng-validate/src/replay.rs` holds the store only in memory; the slice's run needs a durable `HistoryStore`, and a resume reads every link. Each load in the resume gate runs to the end of the case, so choose `resume_every` for the slice's length: `k` loads cost about `k / 2` runs. Storage stands open and larger: one link per accepted step adds 64 bytes to the chunk, so ticket 04's 19 GB at a 2 ms step over 48 h gains about 5.5 GB. Fixed-time chunks do not answer it, because grouping moves no bytes and merged chords do not bound the curve, so measure on the slice before choosing a store. `WallClock` has no implementation in the workspace, by rule, so the live pace needs a host crate outside it, and 05's evidence ran one of about 30 lines.

From 06: the gates take `&dyn ReplayCase`, so the slice is one more implementor. `ChancePlan` needs a second policy, a `ReferenceIntegrator` factory and a distance, and `GateEvidence::tolerance_ns` reports that distance. The distance is the gate's limit: a core that draws per step and moves its events less than it passes, so choose it from the reference's own refinement on the slice. On `ChanceCase` an honest 1 s policy stays within 500 ms of a reference at 1/16 s. Measure a landing's cost on the slice: on two vessels it costs 2.8 to 6.6 extra solves, each a whole `solve`, `verify` and ledger chunk, and bisection cost 10 to 30. A step on the two-row bench went from 26.6 to about 27.5 microseconds. The late-admission gate now costs one run per input and one more, because it reads each arrival off a run that does not hold that input.
