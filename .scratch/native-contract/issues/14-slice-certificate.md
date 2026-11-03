# 14: A short slice window passes the certificate

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The certificate runs on a short slice window with the reference core and passes closure and drift. Its result states the window's coverage. The coverage report lists each held declaration's routes, spaces and validity, each held route's flow per window, and each held pool's conserved combination.

Blocked by: 12

Status: ready-for-agent

- [ ] A short slice window passes closure and drift with the reference core, and the result states its coverage and its held support per window

## Comments

Decided after the deep review of 02 to 04 (nightingale#13, finding 16): minimum coverage is a field of the certificate policy, with no fixed 48 h floor, so a short window passes with its coverage stated.

From 09: `CompiledBody::held` gives card names only, so the coverage report needs a public read of each declaration's routes, spaces and validity. The certificate has no report of a held route's flow per window yet. Each exchange column's origin names its card and the history keeps its exchange, so the report can be derived. A held pool has a conserved combination of its own, so the coverage report has a line to list. The sketch keeps `HeldDeclaration::reservoir` and `HeldPorts::{receipts, ligand_secretion, ligand_clearance}` for a second book.

From the split of ticket 11: the slice's drift limits are fixture data until the healthy-range set holds the within-subject variation (`crates/ng-validate/src/certificate.rs:10`). The closure floor is a policy value. It takes a model default, set in the validation crate and reported with every result (foundation 07).
