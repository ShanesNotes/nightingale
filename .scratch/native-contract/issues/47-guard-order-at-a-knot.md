# 47: What fires at an admission knot follows the order guards are held in

Spec: `crates/ng-contract/src/body.rs` (`Layout::fired`), `crates/ng-contract/src/admission/crossings.rs` (`knot`), `crates/ng-contract/src/compile.rs` (`bind`, the guards' sort), and the solver contract's paragraph on modes ("One rule fires guards in a run and on the admission ring")

Type: task

Status: claimed

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

- [ ] The failing case is a test, red on main and green after the fix
- [ ] What fires at one knot no longer depends on the order guards are held in, and a test shows it
- [ ] A mutant row that puts the order dependence back is caught by a named test
- [ ] The slice's model hash, basis, first-second traces and the pins of `crates/ng-validate/tests/trace.rs` stay, or each change is listed with its reason

**Non-goals:** the vein set and the collapse law (native-contract 42); any law's form or number; the drift bounds.
