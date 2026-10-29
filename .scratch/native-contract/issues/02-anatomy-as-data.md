# 02: Anatomy as data, with its graph checks

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An author declares sites of the six site classes with positions, containment, vascular edges (beds in series and a loop), an enclosure with its compliance, a served-by edge and a region, and gets a checked `Anatomy` and a `Schema` whose pool registry is derived from it.

Blocked by: 01

Status: resolved

Claimed by: claude

- [x] The manufactured anatomy (two capillary beds in series on a vascular loop, one enclosure, one junction, one nerve segment) builds and hashes
- [x] `finish` refuses, each with a named reason: an unreachable bed, an undrained bed, a site enclosed twice, a site with no position, a containment cycle, an orphan site
- [x] A region's derived member list is the containment subtree; adding a site changes the model hash
- [x] Every anatomy type and builder method in the sketch's signature walk compiles against the crate

## Answer

`ng-contract` now holds the anatomy. An author declares sites of the six classes with positions and the relations over them; `AnatomyBuilder::finish` checks the whole map at once and returns a checked `Anatomy`, and `Schema::new` takes it and derives the pool registry. All values in the tests are synthetic.

- **Builds and hashes.** `tests/anatomy.rs` declares the manufactured anatomy: a vessel segment and two capillary beds in series on one vascular loop, an enclosed space around the second bed, a junction, a nerve segment the first bed reports stretch to, an organ region, and a tissue site as the whole body. `Anatomy::digest` is SHA-256, written in the crate and checked against the FIPS 180-4 examples, over canonical bytes in key order. It is the map's part of the model hash: ticket 03's compiled body folds it in, so this box is ticked against the anatomy's digest. Declaration order, identical repeats and the sign of a zero leave it unchanged; adding a site or a relation, or moving a site, changes it.
- **Refusals.** Each of the six has its own `AnatomyDefect` variant naming the least key that shows it, and a test that adds that one defect to the valid anatomy. Disabling each check in a copy turns its test red: 17 of 17 mutations caught. `finish` refuses at the first failing check, in variant order, so a broken map is refused the same way whatever order it was declared in.
- **Members and the hash.** `Anatomy::members(region)` is the root's containment subtree in pre-order, children in key order. A bed added under the organ joins its list.
- **The walk.** `tests/walk.rs` is the signature walk's step 0 with only the crate path changed. It compiles and passes every check.

Choices, each made in a pstack `architect` arena with a Claude Opus runner and a Codex runner, judged by Codex (21 to 20 for the Claude candidate as base; the judge shares a vendor with the other runner):

- **Reachable and drained, with no pump.** The map has no pump marker and a heart is a plain vessel segment, so pumping stays a law's parameter. The circulation (glossary) is the largest strongly connected part of the vascular network holding a vessel segment, ties to the least key. A bed blood cannot reach from it is `UnreachableBed`; one it reaches that cannot return blood to it is `UndrainedBed`. With no circulation, a bed fed from any vessel segment is undrained. A loop of beds alone, an open chain and a second, separate loop are all refused. Only beds are checked, so a reservoir of vessel segments with no bed stays legal for ticket 03. A pump marker would need a declaration the walk cannot make, and it would accept two separate loops that each declare a pump.
- **Root and orphan.** Containment's root is the parentless site holding the most sites, ties to the least key; every other parentless site is an `OrphanSite`. The root is never the first site declared, which would make validity depend on order.
- **No position.** `site()` cannot be called without a `Position` (a compile-fail test, E0061), and `Measure::new` refuses NaN and infinities. So `NoPosition` names a site a relation uses but this anatomy never placed: a key it does not hold, or one it holds under another class.
- **Ids are names.** `SiteId` is a key and a class and stays `Copy`; `finish` resolves it by key. An id from another builder names the site declared here under the same key and class, or no site. The judge proposed owner tokens instead, so that a handle belongs to one builder. Declined: everywhere else in the contract a site's identity is its key (parameters bind by key, and the hash is over keys), and the sketch's `SiteId` is `Copy`.
- **Sets and single declarations.** Sites and region keys are declared once (`DeclaredTwice`, `RegionTwice`). Every relation is a set, so an identical repeat is one fact. A repeat that contradicts a single-valued fact is refused: `ContainedTwice`, `EnclosedTwice`, `ComplianceTwice`. Enclosures nest, and a space that encloses its own encloser is an `EnclosureCycle`; a space with no compliance is `NoCompliance`. A vascular edge at a site that is not a bed or a vessel segment is `WrongRelation`. The arena dropped bans on self-edges, on what an enclosure may hold and on which sites report afferents: none follows from the tree, the enclosure chain or key identity. An anatomy with no sites passes.
- **The registry.** `Schema::pools` derives one water pool per space (glossary): a bed's plasma, interstitium and cells, a vessel segment's lumen and an enclosed space's own fluid; a junction, a tissue site and a nerve segment hold none. Solutes join with the routes that move them, in ticket 03. The sketch's rule that a site class with no compartments is refused is dropped, because it would refuse the walk's tissue-site root. Whether a pool no law touches is refused at compile, as the contract's orphan rule reads, or never becomes an amount, as the Codex runner and the judge proposed, is the compiler's call in ticket 03; the registry is the same either way.
- **Subtracted.** `Network` has only `Vascular`: each other network arrives with its own check. `AnatomyBuilder::refines`, `RefinementRule`, `SplitRule`, `PortEquivalence`, `AggregationMap`, `Representation` and `SiteClass::NAME` are not built: none is in the walk or this ticket, foundation 07 rewrites refinement, and aggregation needs a compiled body.
- **Type rules.** Eight compile-fail doctests in `lib.rs`, each beside a legal twin where one exists. The gate checks compile-fail codes only for the sketch, so these eight were checked by stripping the markers in a copy: each fails with the code it names. Ticket 03 needs that check for the sketch's probes.
- **Gate.** `Measure::new(f64)` is the one public bare `f64`, allowed in `tools/gate/paths.toml` as the parser where a raw number becomes a dimensioned measure. The crate is not `#![no_std]`, because `core` has no `sqrt` and ticket 03's instruction set needs it.
