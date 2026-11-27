# 88: The census check reads the whole-body rows of the census table

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 2's census table, and `docs/architecture/solver-contract-sketch/evidence/census-check.py`

Type: task

Status: resolved

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) runs the whole body, and its census is what says each arrow has one holder. The census table's whole-body rows read 18 live and 15 held through the full proof of nightingale#200, while the catalog counted 16 and 17. A reader found it, not a check: `census-check.py` counts the catalog against counts and line lists written in the script, and reads no row of the table. It does not read the slice's rows either.

- [x] `census-check.py` fails when a whole-body row's arrows or count differ from the catalog's, shown by a probe that edits one cell

**Non-goals:** the catalog's own const checks in `crates/ng-contract/src/catalog.rs`, which already hold the counts.

## Answer

- `docs/architecture/solver-contract-sketch/evidence/census-check.py` now parses section 2's census table (`| Disposition | Slice | Count | Whole body | Count |`) directly from `docs/architecture/solver-contract.md` and verifies all 7 rows (`Live`, `Input`, `Anatomy relation`, `Sink`, `Held`, `Outside`, `Unresolved`) for both the slice and whole-body columns against the catalog parsed from `crates/ng-contract/src/catalog.rs`: listed arrow lines, inclusive `a to b` ranges, optional endpoint annotations (`77 CV→NS`), strictly ascending uniqueness, internal row counts, and exact catalog match.
- On every run (including `tools/verify.d/20-sketch.sh`), `run_contract_probes` runs 13 structural cell-edit probes against the loaded `solver-contract.md` text (editing one whole-body count or arrow cell, swapping an arrow between `Held` and `Live` with matching counts as in nightingale#200, populating a zero whole-body row, editing a relation cell, reversed range, duplicate arrow, slice count/arrow/endpoint edits, and dropping a row) and fails the gate if any probe passes. Passing `--contract PATH` checks an arbitrary modified contract file directly.
- `tools/gate/paths.toml` already runs `20-sketch` on changes to `docs/architecture/solver-contract.md` and `everything` on changes to `crates/ng-contract/src/catalog.rs`, so edits on either side trigger the check.
