# 47: What fires at an admission knot follows the order guards are held in

Spec: `crates/ng-contract/src/body.rs` (`Layout::fired`), `crates/ng-contract/src/admission/crossings.rs` (`knot`), `crates/ng-contract/src/compile.rs` (`bind`, the guards' sort), and the solver contract's paragraph on modes ("One rule fires guards in a run and on the admission ring")

Type: task

Status: resolved

Claimed by: nc47-guard-order

Blocked by: none

Native-contract 42's worker found that marking a fourth vein, whose row reads zero, stops the slice admitting, and that the vein is not the cause. Its diagnosis is in `~/lanes/logs/ng-wave-4/nc42/PLAN.md`, section 8.

The claim, to check first:

- `Layout::fired` takes guards in index order, and a guard that an earlier firing armed fires in the same pass.
- Compile orders guards by a digest of each guard's layout, and that digest holds the unknown indices the guard reads. So a new row can reorder them.
- At a paced cycle's wrap the clock is forced. When the systole guard is crossed there too, the clock fires only if the systole guard comes first. When it drops, the initializer's sweeps cycle on a left-ventricle valve crossing until admission fails.
- The smallest case is `crates/ng-validate/tests/wrap_firing.rs` on native-contract 42's branch `nc42-pv-case` at `3d0e40c`: a paced pacemaker with 0 to 4 inert rows.

The whole body has 1,372 rows where the slice has 241, so its first run depends on admission not turning on row count.

Evidence: `~/lanes/logs/ng-wave-4/nc47-guard-order/`.

- [x] The failing case is a test, red on main and green after the fix
- [x] What fires at one knot no longer depends on the order guards are held in, and a test shows it
- [x] A mutant row that puts the order dependence back is caught by a named test
- [x] The slice's model hash, basis, first-second traces and the pins of `crates/ng-validate/tests/trace.rs` stay, or each change is listed with its reason

**Non-goals:** the vein set and the collapse law (native-contract 42); any law's form or number; the drift bounds.

## Answer

The claim holds. `Layout::fired` read each guard's arming on the modes the firings before it had left, so a guard a firing armed fired in the same pass when it was held after that firing, and waited when it was held before. The run never saw it, because it fires only guards armed before the firing. The knot did: it forces the wrap's clock in its first turn only, and when the last leg ends in systole the clock is armed only after the systole guard fires. Compile holds guards in the order of a digest that names the unknowns each reads, so a row can swap the two. The case `crates/ng-validate/tests/wrap_firing.rs` read 1.5, 1.5, 1.5, 0.25, 0.25 on main.

The fix makes a firing at one knot order-free and leaves the order alone:

- `Layout::fired` fires only a guard the modes before the firing armed. The run already masked its crossings that way, so the run fires as before.
- At a knot the crossing's guards and the clock stand crossed in every turn, not only the first. The clock fires in the turn after the systole guard, whichever is held first.

The other shape, holding guards in an order the laws state, was not taken. It moves the slice's model hash, and the order of declaration would still decide whether the clock fires.

Two tests hold it. `body::tests::a_firing_moves_the_same_guards_whatever_order_they_are_held_in` fires two crossed guards on one variable held both ways. `crates/ng-validate/tests/inert_rows.rs` admits the slice with one to four memory rows before the heart's rows, which hold its twelve guards in four other orders, the clock both before and after the systole guard, and each admits to the slice's start bit for bit. The slice never meets a crossed systole guard at the wrap, so that test passes on main too. Native-contract 42's four-vein body, which failed with `Inconsistent`, admits with the fix.

The slice's model hash, basis and every pin of `crates/ng-validate/tests/trace.rs` stay.

Left open: two guards that leave one value of one variable and cross at one landing still race, and the first in stable order wins. No body declares such a pair. Closing it needs an order the laws state or a refusal, which changes what a guard is.
