# 36: A law can rebuild a set point in its variable's dimension through a width ratio

Spec: [solver contract](../../../docs/architecture/solver-contract.md), the paragraph "Set points are their own dimension", and `crates/ng-contract/src/quantity.rs` (the two deviation rows)

Type: task

Status: open

Blocked by: none

The contract says a set point can never become the variable it guards. [Body ticket 03](../../body/issues/03-whole-body-lists.md) closed the cancellation rows to set points and deviations, and its independent check then found one older path the types still allow. A deviation leaves over a width of its own dimension as a ratio with no dimension. `scale` then multiplies a second parameter, a width declared in the guarded variable's dimension, by that ratio. With the two widths equal in value the product is the deviation as a pressure, and the measured value less that product is the set point as a pressure. The path needs two declared parameters of equal value, so a packet shows it, and no law on main takes it.

**What to build:** the contract's sentence says what the types hold, or the path is closed.

- [ ] A pick, with its reason: reword the sentence (no row relabels a set point or a deviation as the guarded variable; rebuilding one takes two declared widths of equal value) or close the path in the types
- [ ] If the path is closed: a compile-fail doctest with its code and a legal twin, and no law on main changes
- [ ] If the sentence is reworded: the glossary agrees with it

**Non-goals:** any change to how a deviation leaves over its own width, which research 12's thermostat uses.
