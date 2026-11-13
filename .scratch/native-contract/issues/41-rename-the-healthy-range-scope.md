# 41: Rename the healthy range's `Scope`, which is not the glossary's Scope

Spec: `crates/ng-contract/src/healthy.rs` (`Scope`), the glossary's **Scope**, and [solver contract](../../../docs/architecture/solver-contract.md)'s tradeoff for a window that belongs to two changes

Type: task

Status: open

Blocked by: none

`ng_contract::Scope`, in `crates/ng-contract/src/healthy.rs`, says when a healthy range holds: during every activity, during one, or across one change of activity. [Body ticket 04](../../body/issues/04-scopes-ports-and-census.md)'s part 3a defined **Scope** in the glossary as what the code's `Assembly` issues each card, and that crate also holds `HeldScope`. [Body ticket 23](../../body/issues/23-healthy-range-and-return-checks.md) merged after it with the other meaning, and the contract's tradeoff for a window that belongs to two changes says "since a scope names one change". Neither PR was wrong alone and no Rust name clashes, but the glossary rule says a word means one thing. The last check of nightingale#133 found it.

**What to build:** the healthy range's type takes a name the glossary does not hold, and the glossary gains it.

- [ ] The type and its uses are renamed: `healthy.rs`, `lib.rs`, `crates/ng-validate/src/healthy/range.rs` and its tests, `crates/ng-validate/tests/healthy.rs`, the sketch's evidence that names it, and the text of each row of `tools/gate/mutants.toml` that quotes it, each row still applying (`tools/mutate.py --check`) and CAUGHT
- [ ] The contract's tradeoff line takes the new word in place of "scope", and no other line of the contract or the glossary uses "scope" for a healthy range
- [ ] The glossary defines the new term in the same PR
- [ ] The set's digest and every pinned seal stay where they are, since a type's name is not sealed

**Non-goals:** any change to what a range covers or how it is judged.
