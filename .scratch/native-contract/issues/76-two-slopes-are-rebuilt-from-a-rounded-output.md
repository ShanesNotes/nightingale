# 76: Two slopes are rebuilt from a rounded output

Spec: `crates/ng-contract/src/evaluate.rs` and `crates/ng-contract/src/evaluate/enclosure.rs`

Type: task

Status: open

Blocked by: none

**Why.** A slope computed from an op's rounded output loses what the rounding dropped. [Native-contract 66](66-what-the-gas-loop-works-around.md)'s review found it in `expm1`, whose slope read 0 under an argument of about -37, and fixed it there. Its check then measured the same loss in two ops that were on main before it (`~/lanes/logs/ng-wave-4/nc66/verify-coordinator/VERIFY-pr183.md`, "The other slopes rebuilt from an output").

- The logistic's slope, `v (1 - v)`. Digits kept at an argument of 5, 10, 18, 25 and 35: 13.7, 12.0, 9.1, 5.4 and 1.2. From 37 up it reads 0, where the true slope is 8.5e-17 at 37. Below zero it keeps every digit. A slope from the argument, `e^-x v^2`, keeps 15.6 digits or more at each of these.
- The smooth forms' slopes, `0.5 (1 -/+ along)`, on the side whose slope is small. Digits kept where the operands are 100, 1e4, 1e6 and 5e7 widths apart: 12.1, 7.6, 4.1 and 0.4. From 1e8 widths it reads 0, where the true slope is 2.5e-17.

A slope that reads 0 drops a dependency from the Jacobian, and a solve that leans on it stalls or takes a wrong step. Oxygen saturation is a logistic, and the oxygen rate and glycolysis are smooth forms.

- [ ] Each slope is computed from the op's arguments, with a test at the points above against references written as literals
- [ ] The enclosure's slope bounds for both ops hold the true slope on intervals that reach those arguments
- [ ] Every arm of `evaluate/enclosure.rs` has a test that drives it and a gate row: the check found no row for any op
- [ ] Each pin that moves has its sentence: a slope that changes in its last digits can move a solve's path, on the slice and on the whole body
