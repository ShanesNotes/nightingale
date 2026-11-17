# 73: A law takes a reaction's count as its constant

Spec: `crates/ng-contract/src/law.rs`

Type: task

Status: open

Blocked by: none

**Why.** [Body 09](../../body/issues/09-tissues-respire.md)'s oxygen debt is three units of oxygen for each unit of the cells' lactate, and the three is the lactate burn's own count. A kernel takes no constant that nothing backs (`UnbackedConstant`), so the debt reads the figure as a site parameter, `mt.oxygen-per-lactate`. The whole body's packet takes it from the compiled burn's counts, but the packet is still where it lives: a site entry over the class entry binds, and that bed's debt then reads another multiple of its lactate than its burn takes. A break-it pass on nightingale#182 wrote the test. [Native-contract 54](54-three-halves-power-as-ratio-times-square-root.md) asks the same of an exponent: a figure that is part of a law's form leaves the packet.

- [ ] A kernel reads the ratio of two species' counts in a reaction its card owns, as a constant the reaction backs, typed as the quotient of the two, with its test and a compile-fail test for a reaction another card owns
- [ ] The oxygen debt reads the lactate burn's ratio, `mt.oxygen-per-lactate` leaves the packet and the fixture, and the body's pins move once
- [ ] The break-it pass's test passes: no packet entry changes the debt's multiple
