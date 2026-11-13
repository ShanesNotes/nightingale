# 48: Compile refuses two guards that leave one value of one variable

Spec: `crates/ng-contract/src/compile.rs` (`check_guards`), the solver contract's paragraph on modes ("One rule fires guards in a run and on the admission ring") and the must-fail table of its section 7, and `tools/gate/mutants.toml` (the row `first-wins`)

Type: task

Status: open

Blocked by: none

[Native-contract 47](47-guard-order-at-a-knot.md) made a firing at one admission knot order-free, and it left one race open. When two guards leave one value of one mode variable and both cross at one landing, the guard held first wins, in a run and on the admission ring. Compile holds guards in the order of a digest that names the unknowns each reads, so one added row can change the winner. That is the fault 47 was opened for. Nothing would show it: the tests that declare such a pair assert only that one guard fired, or that the ring and the run agree, and both hold in either order.

The independent check of nightingale#152 ran it at that PR's head on the pair `crates/ng-validate/tests/firing.rs` declares on purpose. In compile's order the run entered `second`. With the order reversed the run and the ring both took `first`. Its report is `~/lanes/logs/ng-wave-4/nc47-guard-order/verify-coordinator/VERIFY-pr152.md`, section 10.5 and finding 1.

No body declares such a pair today. The heart's two plain guards (`crates/ng-cardiovascular/src/rhythm/phase.rs`) and each valve's two (`crates/ng-cardiovascular/src/valves.rs`) leave two different values, and the whole body's holds add one hazard on its own variable. So a refusal costs no body anything now.

What changes:

- `check_guards` refuses two plain guards of one fragment that leave one value of one mode variable, and names both guards and the value.
- The contract's line 124 drops "of two on one variable the first in stable order wins" and says compile refuses the pair.
- The row `first-wins` and its test `of_two_guards_on_one_variable_only_the_first_fires` (`firing.rs`) hold a rule that no longer exists. The row goes, or it is rewritten to remove the refusal.
- Three tests declare a pair on purpose: `firing.rs` through `two_guards(false)` and in `of_two_guards_on_one_variable_only_the_first_fires`, and `crates/ng-validate/tests/guard_ordering.rs` (its `before` to `early` and `before` to `later`). Each becomes a refusal test or gives its second guard another value to leave.
- Section 7's must-fail table gains a row for the pair, with its entry in `tools/gate/must-fail.toml`.

- [ ] Compile refuses two plain guards that leave one value of one mode variable, and a test shows the refusal
- [ ] A mutant row that removes the refusal is caught by that test
- [ ] The contract's line 124, the row `first-wins` and the three tests say what the code does
- [ ] Section 7's must-fail table has the row, with its entry in `tools/gate/must-fail.toml`
- [ ] The slice's model hash, basis and the pins of `crates/ng-validate/tests/trace.rs` stay

**Non-goals:** hazard guards racing for one mode variable, which the contract's paragraph on chance states on purpose; an order the laws state, which 47's Answer turned down; the knot's turns.
