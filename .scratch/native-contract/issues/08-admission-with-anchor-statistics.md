# 08: Admission solve with anchor statistics

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The admission system solves a manufactured pulsatile case once, fitting a calibrated parameter so that a cycle-mean anchor holds under the periodic goal, and records provenance for every value.

Blocked by: 04

Status: ready-for-agent

- [ ] A cycle-mean anchor on a pulsatile output is met as a cycle mean, not an instant
- [ ] An initializer that returns a guess violating a row or anchor is refused with the row named
- [ ] Provenance is recorded per value (Derived, Calibrated, Sampled, Authored, Fixture); an Authored value lies inside its population record's range (I16), and no parameter is pinned to an anchor
- [ ] The fixture packet type is the only one the validation crate accepts

## Comments

From 03: a mode variable declares its values but no starting value, so initialization picks one consistent with the guards. Sketch probes this ticket owns: lines 46 (an anchor needs a citation, E0061) and 201 (a synthetic value cannot be an anchor, E0308).

From 04: `body.bind(packet)`, `body.initialization(parameters, boundary, request)` and `system.admit(solved, history)` exist, with `InitializationRequest::consistent_instant`. `packet.start(row, measure, note)` carries start values into the packet's hash until anchors replace it, and no `Initializer` trait exists, so `ng_validate::admit_fixture` is the one caller. `admit` refuses a moved amount or memory value and an algebraic residual that is not exactly zero, because no tolerance is declared yet; this ticket owns that tolerance. `bind` refuses a body with a mode variable.

From the deep review of 02 to 04: `admit` now evaluates every law at the start in every body, so a law with no value there is `InitializationFailure::Undefined` whether or not the body has an algebraic row.

Decided after the deep review of 02 to 04 (nightingale#13, findings 12 to 17): finding 15 gets a partial admission check. Admission refuses a constant built only from literals, and a parameter divided by itself, when the constant has no packet entry. This is not a complete check: other ways to build a constant stay possible, and no compile-time check is claimed.

From 05: `ImageParts` replaces `InitialParts`, with `canonical` and `parse` for its bytes, so a new image part lands in that one place and in the checkpoint at once. `PendingInputs::opening` sets the fence one nanosecond past the anchor, because initialization read that instant's drive. A windowed initialization reads a span of drives, so it must raise the opening fence past every instant it read, or a late input at one of them breaks I5.

From 06: `ModesNotYetRun` now refuses a mode variable a plain guard fires or no hazard guard moves, and any mode variable in a body with an algebraic row, because a firing leaves algebraic values as they were. `ParameterPacket::start_mode` carries a first mode into the packet's hash, and `BoundParameters::modes` is the slot initialization fills when it picks one consistent with the guards. Finding 15's partial check landed here: `bind` refuses `UnbackedConstant` for `exp` or `logistic` over literals and for a parameter over itself, so this ticket extends it and does not rebuild it. Admission takes no seed: a forcing reads 1 and a draw 1/2, so a start state and any calibration are a function of the packet alone. A seeded start would need a seed in the request and a check against the identity.
