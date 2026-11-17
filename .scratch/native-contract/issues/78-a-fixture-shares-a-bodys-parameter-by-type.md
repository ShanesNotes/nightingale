# 78: A fixture shares a body's parameter by type

Spec: `crates/ng-contract/src/packet.rs`

Type: task

Status: open

Blocked by: none

**Why.** A test fixture that must run at a value the whole body states, such as the oxygen reach, can share it only as a bare `f64`, which the f64 gate refuses across a crate boundary, or state it a second time, which no check holds. [Body ticket 09](../../body/issues/09-tissues-respire.md)'s third PR states the reach a second time in the gas fixture. A test cannot pass one packet's parameter on to another packet, and a `Measure` gives a test no value and no comparison.

- [ ] A typed way for a test to take one parameter's value from a packet and set it in another, or to compare the two, with no bare `f64` across a crate boundary
- [ ] The gas fixture's reach is the body's by that way, and a mutant row that moves one of the two alone is caught
