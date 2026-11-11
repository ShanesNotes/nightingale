# 35: An unnamed conserved quantity names the reaction that left it open

Spec: `crates/ng-contract/src/conservation.rs` (`CompileDefect::UnnamedConservedQuantity`), and [body ticket 29](../../body/issues/29-the-charge-default.md)'s Answer

Type: task

Status: open

Blocked by: none

Body ticket 29's prototype met the refusal `UnnamedConservedQuantity`, and it named the bed's first reaction, `burn`, where another reaction's ions were the ones the quantities missed. Under the charge default no body on main reaches that case, so nothing is wrong today, but the refusal misleads whoever meets it next.

**What to build:** the refusal names the reaction that leaves the quantity unnamed.

- [ ] A test with two reactions at one bed, the second leaving a quantity unnamed, expects the second's name
- [ ] A mutant row that names the first reaction again is caught by that test

**Non-goals:** any change to which bodies compile.
