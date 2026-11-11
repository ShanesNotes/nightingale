# 34: The fluids crate's reflection goes through the one membrane type

Spec: [solver contract](../../../docs/architecture/solver-contract.md), the paragraph "Walls, passages and heat routes are closed lists", and `crates/ng-contract/src/law.rs` (`reflect`, `membrane`)

Type: task

Status: open

Blocked by: none

[Body ticket 03](../../body/issues/03-whole-body-lists.md) landed one `Membrane`, which gives a wall's conductance and its reflection for a species. The fluids crate still declares albumin's reflection with `FragmentBuilder::reflect` (`crates/ng-fluids/src/starling.rs`). On a fluid route the two lower to one record, so a reflection has two spellings.

**What to build:** the fluids crate declares its reflection through `membrane`, and `reflect` goes if nothing else needs it.

- [ ] Every reflection in `crates/ng-fluids` is declared through `membrane`
- [ ] The slice's model hash, conserved basis and first-second traces do not move, or the PR lists each one that moved and why
- [ ] `reflect` is removed with its tests and rows, or the Answer says who still needs it

**Non-goals:** a membrane on a wall's or a passage's own route, where the law reads the reflection as a share.
