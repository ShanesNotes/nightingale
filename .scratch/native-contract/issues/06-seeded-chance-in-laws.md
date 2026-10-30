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

From 04: `StreamPositions` is an empty list in `ng-record`, digested by `digests` and the seal, so ticket 04's forced refusal proves only that the field is read. Your extra-evaluation check carries the rest of that box. `bind` refuses any body with a mode variable (`AdmissionError::ModesNotYetRun`), and this ticket, 08 and 10 lift it. `propose` takes no modes yet. In the sketch `CoupledProblem::transitions` now reads two stage evaluations, because a core holds no `Candidate`. A ticket's `after_breakpoint` is set only by deliveries today and must also follow a guard that changed mode.

From 05: `StreamPositions` is in the checkpoint and in each image's digest, so a position is under the seal and `resume` already restores it, and ticket 04's note that `streams` proves only that the field is read stands until a draw moves one. A ticket's `after_breakpoint` now also reaches `StepReceipt::after_breakpoint`, which the resume gate reads to pick its checkpoints, so a guard change must set it or that gate stops pressing the instant. A guard that lands inside a span needs a retry length of its own: `StepTicket::refuse` halves the committed span until it is shorter than the refused ticket, and that rule knows nothing of a guard. A landing that stops on a guard must not read the timeline either, so the rule in record 0006 is the one to extend. `replay_gate` takes `&TwoReservoirs`, so a case with a stream needs the gates generalized here or in 11.
