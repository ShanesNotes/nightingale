# 74: A reach with a missing pool drops unseen

Spec: `crates/ng-contract/src/compile.rs` (`Reaches::of`)

Type: task

Status: open

Blocked by: none

**Why.** Compile keeps a reach only where every pool end is held, so a route whose pool the schema lacks drops out of the compiled body with no word. [Native-contract 70](70-a-beds-heat-pays-the-work.md)'s plan met it: a work route moved to a bed's thermal pool that the schema did not hold would have vanished, and the work with it. That ticket closes both ends of its own two families, stroke and wall work, once a payer is named (`PayerHoldsNoHeat`, `PaidWorkHasNoStore`). Every other route family keeps the pattern, and a schema that loses a pool loses its routes the same way.

- [ ] A list, from the compiled slice and the compiled whole body, of each reach compile drops today and why, so the ones dropped by design are known
- [ ] Compile refuses a reach dropped for a missing pool unless its family says the pool may be absent, with a test for each family
- [ ] The slice's and the whole body's pins do not move
