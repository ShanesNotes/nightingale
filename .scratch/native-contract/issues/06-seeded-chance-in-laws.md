# 06: Seeded chance in the laws: streams, hazards, forcing

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A law declares a stream and a hazard guard that fires a mode change when the integrated hazard crosses the stream's draw, and a forcing that varies a rate on a knotted spline; replay holds with both on.

Blocked by: 05

Status: ready-for-agent

- [ ] The stream's draw depends only on seed, key and event ordinal: adding a second stream leaves the first's draws unchanged
- [ ] A forced extra evaluation before the hazard event leaves its draw and ordinal unchanged
- [ ] Two step policies give the same event time to the replay tolerance, and a must-fail core that draws per step is rejected
- [ ] Admission holds the forcing at its mean; every replay gate in ticket 05 passes with the stream and the forcing on

## Comments

From 03: `FragmentBuilder` has no `stream`, `forcing` or `hazard` yet, and the walk's step 1c waits for this ticket (`crates/ng-contract/tests/walk.rs` marks it). Laws attach by reference, and a declaration records what it owes when it is made, so streams and hazards can follow the same pattern. Sketch probe this ticket owns: line 457 (E0599).
