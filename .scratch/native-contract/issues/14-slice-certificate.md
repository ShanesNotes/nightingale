# 14: A short slice window passes the certificate

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The certificate runs on a short slice window with the reference core and passes closure and drift. Its result states the window's coverage. The coverage report lists each held declaration's routes, spaces and validity, each held route's flow per window, and each held pool's conserved combination.

Blocked by: 12

Status: resolved

- [x] A short slice window passes closure and drift with the reference core, and the result states its coverage and its held support per window

## Answer

`crates/ng-validate/tests/slice_certificate.rs` runs two seconds of the slice from admission with the reference core through `run` and certifies `arena.session().conservation()` over every 1 s window. All 21 combinations close to a relative 1e-12 of each window's gross exchange with no absolute allowance; the worst residual is 1.5e-25, on energy. The same walk fails closure with no allowance and drift under a 1e-3 bound. The certificate oracle walks the slice in time order and agrees bit for bit.

The window length is now policy data: `SlidingWindows::new(window, coverage)` replaces `Sliding24Hours`, so a short window passes with its coverage stated, and the arena keeps 24 h over 48 h. A charge combination now takes a policy, in the new `Charge` dimension.

The basis carries a `CoverageManifest` (`ConservedBasis::coverage`): each held declaration as a public `HeldCard` (routes, spaces, validity), one `HeldRoute` per column a held route moves, with its combination and the way its forward extent crosses, and each `HeldPool` with its own combination. The declarations moved there from the layout, so the model hash is unchanged and the basis digest now covers them. The certificate returns its span, the manifest, and `HeldSupport` per held route: the most it carried into and out of the body over any window, each named by window.

## Comments

Decided after the deep review of 02 to 04 (nightingale#13, finding 16): minimum coverage is a field of the certificate policy, with no fixed 48 h floor, so a short window passes with its coverage stated.

From 09: `CompiledBody::held` gives card names only, so the coverage report needs a public read of each declaration's routes, spaces and validity. The certificate has no report of a held route's flow per window yet. Each exchange column's origin names its card and the history keeps its exchange, so the report can be derived. A held pool has a conserved combination of its own, so the coverage report has a line to list. The sketch keeps `HeldDeclaration::reservoir` and `HeldPorts::{receipts, ligand_secretion, ligand_clearance}` for a second book.

From the split of ticket 11: the slice's drift limits are fixture data until the healthy-range set holds the within-subject variation (`crates/ng-validate/src/certificate.rs:10`). The closure floor is a policy value. It takes a model default, set in the validation crate and reported with every result (foundation 07).

From ticket 12: certify the session that `run` returns, `certify(&arena.session().conservation(), policy)`, over `ArenaSpec::window`. Every repeat keeps the same links in the store that `run` takes. `MemoryStore` holds one copy per seal, and the bench's `LinkLog` appends each repeat's links again.
