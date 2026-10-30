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
