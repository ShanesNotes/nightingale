# 88: The census check reads the whole-body rows of the census table

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 2's census table, and `docs/architecture/solver-contract-sketch/evidence/census-check.py`

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) runs the whole body, and its census is what says each arrow has one holder. The census table's whole-body rows read 18 live and 15 held through the full proof of nightingale#200, while the catalog counted 16 and 17. A reader found it, not a check: `census-check.py` counts the catalog against counts and line lists written in the script, and reads no row of the table. It does not read the slice's rows either.

- [ ] `census-check.py` fails when a whole-body row's arrows or count differ from the catalog's, shown by a probe that edits one cell

**Non-goals:** the catalog's own const checks in `crates/ng-contract/src/catalog.rs`, which already hold the counts.
