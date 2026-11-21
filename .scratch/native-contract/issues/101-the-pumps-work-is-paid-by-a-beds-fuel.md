# 101: The pump's work is paid by a bed's fuel

From: [body ticket 17](../../body/issues/17-volume-hormones.md)'s plan, its fork on cell water and the pump

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 17](../../body/issues/17-volume-hormones.md)'s PR for cell water and the pump, and so [body ticket 15](../../body/issues/15-insulin-and-glucagon.md), whose insulin factor multiplies that pump

**Why.** The cells' pumps draw their work from the exterior receipt `chemical energy` (`FluidsPorts::pump_work` in `crates/ng-contract/src/scope.rs`). Two whole-body tests that stand refuse any column that ends at that receipt: the assembly test in `crates/ng-validate/tests/body.rs` and the run test in `crates/ng-validate/tests/body_run.rs`. [Body ticket 09](../../body/issues/09-tissues-respire.md)'s first box extends that rule to the cells' pumps. So the route from the receipt cannot carry body ticket 17's pump, and the pump's work must be paid by fuel burned at its bed.

Paying it so needs a new `Work` variant and route family in the contract crate, which is a new interface. So this ticket opens with a design round.

- [ ] A design round picks the form: FL's pump crossing beside MT's burn in cells, at research 14's `e_ATP J_p / 2`, or another that keeps each reaction in one space (`ReactionInTwoSpaces`), and records what it adds to the contract
- [ ] The contract crate carries the new `Work` variant and its route family, with a compile-fail test for each type rule it adds
- [ ] A fixture shows a pump's work paid from its bed's fuel with no column ending at `chemical energy`, and the assembly and run tests above still hold

**Non-goals:** the pump's law and cell water themselves (body ticket 17), and insulin's factor on the pump (body ticket 15).
