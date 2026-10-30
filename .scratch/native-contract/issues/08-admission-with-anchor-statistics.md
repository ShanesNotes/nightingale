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
