# 105: Admit refuses a foreign workspace without a panic

From: a break-it read of nightingale#226, which made `ng_validate::solve_fixture` public

Spec: `crates/ng-contract/src/admission.rs` (`InitializationSystem::check` and `admit`)

Type: task

Status: resolved

Claimed by: nc105

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) starts every run from an admitted start. `admit` is the only constructor of that start, and its doc promises a refusal for a workspace from another system. A panic there takes the caller down instead.

**What is wrong, on main `b74fad18`.** In `crates/ng-contract/src/admission.rs`:

- The doc of `admit`, lines 1629 and 1630, says: "A workspace from another system, of another shape or with a topology its own values do not give is refused."
- `check`, lines 583 to 593, calls `self.derive(&workspace.topology)` at line 584, before it compares `workspace.system` with `self.identity`.
- `derive` reads this system's guard table with the guard ids the workspace's knots hold, `guards[guard]` at lines 719, 720 and 725. A workspace solved on a body with more guards than this system's body indexes past the end: `index out of bounds: the len is 1 but the index is 1` at line 719.
- `ng_validate::solve_fixture`, public since nightingale#226, hands a caller a solved workspace of one body and the system of another, so a caller outside the crate reaches it.

**Rules.**

- [x] A workspace whose admission identity is not this system's is refused as `AdmissionError::ForeignWorkspace` before anything is derived from it. `check` compares `workspace.system` with `self.identity` first and derives the legs last, so `derive` runs only for a workspace of this identity. Another system means another identity, and a fresh system of the same identity still admits a workspace solved before, as `crates/ng-validate/src/initialize.rs` lines 358 and 359 do. Shown by `admit_refuses_a_fixture_solve_from_a_body_with_another_guard` in `crates/ng-validate/tests/admission_crossings.rs`: it panics at the base with the index panic above, and passes on this PR. Its two controls admit each body's own solve with the system that solved it, and a fresh system of the receiver's identity then refuses the other body's solve.
- [x] The verdicts main gives stay: `Inconsistent` for a crossing shifted off its root, `GuardSweep` for a topology its own values no longer give, and `ForeignWorkspace` for another packet (`crates/ng-validate/tests/admission_crossings.rs` lines 872 to 897 on main). `ForeignWorkspace` keeps its meaning, "A workspace from another compiled body, or of another shape" (`crates/ng-contract/src/failure.rs` line 241), and no other case gains it. Shown by every test of `admission_crossings` passing on this PR, and by the preflight at the PR's head, which runs every test of the workspace that is not ignored.
- [x] Nothing that admitted before is refused, and no bytes move: the slice's and the whole body's pins, hashes and traces are as on main. The change reorders a check and enters no seal. Shown by the preflight at the PR's head, whose pin and trace tests pass unchanged, and by the full proof, which also runs the tests the preflight leaves ignored.
- [x] No bounds check goes into `derive`. A workspace with this system's identity holds indices in range, because only the crate makes one and the identity seals the tables the indices point into. The lines that show it are below.
- [x] One mutant row, `admission-identity-before-derive` in `tools/gate/mutants.toml`, restores the old order. Its replay alone is caught by the regression test, whose failure's first line is the index panic in `derive`. Section 7 of the solver contract and `tools/gate/must-fail.toml` hold no row on a foreign workspace, and this PR adds none.

**Non-goals:** a bounds check that only the crate's own test module could fail, and the other public functions that read a workspace before `check` (the Answer lists them).

## Answer

`check` now compares the workspace's system with this system's identity first, then the vector's length, and derives the legs last, inside the same short-circuit `||`. A foreign workspace is refused as `ForeignWorkspace` before `derive` reads a guard id. The derived legs are compared only for a workspace of this identity, as before.

**Why a workspace of this identity holds indices in range** (rule 4), at main `b74fad18`:

- `AdmissionWorkspace` (`crates/ng-contract/src/admission.rs` lines 382 to 390) is public with seven private fields. `Located` (354 to 358), `Topology` (361 to 365) and `Leg` (368 to 376) are private types with private fields.
- The one struct literal of a workspace is in the private `workspace_of`, lines 729 to 739, which copies `self.identity` into `system` and derives the legs with this system's tables. Public `workspace` (line 801) and every topology operation make a workspace through it or from one.
- The compile-fail doctest `compile_fail,E0616` in `crates/ng-contract/src/lib.rs`, lines 1116 to 1121, refuses a write to a workspace field from outside the crate.
- The identity (`admission.rs` lines 274 to 278) seals the compiled model's digest, the packet's digest, the anchor moment and the cycle. The model's digest seals the layout's modes and guards (`crates/ng-contract/src/body.rs` lines 209 to 242), so an equal identity means the same guard and mode tables, and every guard id a workspace of that identity holds came from them.

**Other public functions that read this system's tables with a workspace before `check` has run** (rule 5, no fix here). Each returns no error to refuse with. In `merit`, `crossings`, `seed`, `apply` and `solve` a foreign workspace can put an index or a slice out of range, which panics. `size` cannot: its table holds one size for each row and is read at the slot modulo the row count, so it returns the size of a slot the foreign workspace's shape picks. A later ticket would need a workspace tied to its system by type, so a foreign one cannot be passed at all.

- `size` (`admission.rs` line 765): cannot panic, as above.
- `merit` (line 772): reads the workspace's crossing guard and passes it to `root_scale`, which indexes this system's guard table.
- `crossings` (line 815): indexes this system's guard table with `crossing.guards[0]`, then its modes and values.
- `seed` (line 1199): slices the workspace's stage values with this system's width and reads its reset rows.
- `apply` (line 1227): indexes the correction's times by the workspace's crossing positions and places offsets with this system's interval.
- `solve` (`crates/ng-contract/src/ring.rs` line 162): combines the workspace's leg count with this system's width and plan.

`evaluate`, `linearize`, `stage` and `stage_residuals` (`admission.rs` lines 1054, 1080, 1161 and 1188), and `sweep`, `relocate` and `settle`, call `check` first, so they now refuse a foreign workspace as `admit` does.
