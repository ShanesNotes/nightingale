# Foundation wayfinder map

## Destination

[SRS invariant I19](../../docs/srs.md#4-invariants-each-becomes-a-test) passes on a run of the reference man's reviewed packet, checked against a reviewed healthy-range set. That pass is the first milestone ([record 0004](../../docs/adr/0004-first-milestone.md)).

## Notes

- Prose uses "the map" for the patient's anatomy (SRS section 2) and calls this file the wayfinder map.
- The architecture's layers ([SRS section 1](../../docs/srs.md#1-purpose-and-scope)) come first, and display comes after them. Until display work starts, nightingale-pulse's existing monitor covers display.
- This wayfinder map carries execution as well as decisions. Its tickets include the build work that reaches the destination, not only the choices before it.
- Foundation tickets name native-contract tickets in their `Blocked by:` and `Lands with:` lines, so a frontier scan reads `.scratch/native-contract/issues/` as well as `.scratch/foundation/issues/`.
- Grilling tickets call the grilling and domain-modeling skills.

## Decisions so far

- [Fast-forward identity (D3)](../../docs/srs.md#3-hard-rules): SRS rule 4 gives identical bits at every pace, fast-forward included. Resolution is part of a run's identity, so a run at a reduced resolution is a different run.
- [Map detail at milestone 1 (D5)](../../docs/adr/0002-map-starts-grouped.md): the map starts grouped, and each split is earned. At stage 1 the map holds only the regions that the anchors and the stage 1 checks name.
- [Chart the foundation map from SRS section 7](issues/02-roadmap.md): this wayfinder map sorts the section, and the ticket keeps the section word for word.
- [First crate](issues/03-first-crate.md): `tools/verify` plans a merge, changed or full run over four `ng-*` crates; root and ticket Markdown count as docs-only; the pin rule waits for a pin without a lockfile. The nightly job is [Add the nightly job](issues/10-nightly-job.md).
- [Admit the ng-* crates under purity rules](../native-contract/issues/01-ng-purity-rules.md): the contract's crate table marks the physiology closure, which takes no crate from outside the workspace, and no crate may name a host service.
- [Anatomy as data](../native-contract/issues/02-anatomy-as-data.md): `ng-contract` checks a map at `finish` with named reasons, hashes it in key order and derives one water pool per space. A bed must lie on the circulation, no pump is declared, and site ids are names resolved by key.
- [Declaration language and compiler](../native-contract/issues/03-declaration-language-and-compiler.md): laws attach by reference, so compile refuses an omitted law, a second owner and an uncovered mode by name; route families are requested whole; every registry pool is an amount unless the schema leaves its space out with a reason, and one no transfer moves is refused.

## Not yet specified

- **A name for "the agent".** The build order in [Chart the foundation map from SRS section 7](issues/02-roadmap.md#srs-section-7-word-for-word) lists "the agent" with the solver and the contracts as architecture. The SRS defines no such term, and what it names is not yet clear. Once a session finds out, "the agent" gets a glossary term or leaves the build order.
- **Where the monitor goes.** [SRS section 6](../../docs/srs.md#6-fidelity-aims) keeps the monitor a requirement, and [record 0004](../../docs/adr/0004-first-milestone.md) keeps the monitor's readings in the lens that the milestone checks. So its readings lie on this route, while its display is out of scope. Which readings it carries, and where it is built, are open. [Parking lot](issues/08-parking-lot.md) parks only the monitor's display until after the milestone.
- **The reference man's reviewed packet and a reviewed healthy-range set.** The destination needs both, along with the healthy-range check and the return check that read them. No ticket yet scopes the sourcing of either one or the build of those two checks. [Contract follow-ups](issues/07-contract.md) adds the day and the healthy-range set to the contract, and the lens decides which readings the set covers.

## Out of scope

This effort leaves out the first milestone's non-goals, which [Chart the foundation map from SRS section 7](issues/02-roadmap.md#srs-section-7-word-for-word) keeps word for word.

- Disease, hazard, intervention and debrief behavior. They belong to scope stages 2 to 5, and I19's healthy adult has nothing done to them. What they will attach to stays in scope, such as the event log, checkpoints and the packet.
- Bedside scenarios as the definition of done. SRS invariants define done, and scenarios belong to later scope stages.
- Display work, the chart included. Display comes after the architecture.
- Map detail finer than the first milestone needs. D5 rules it out.
- A build target other than native Rust (SRS rule 5).
- Death behavior, scripted outcomes and arbitrary odds. Death behavior waits for a later scope stage (SRS rule 7). The body computes every number from its physics, and chance fires only at cited odds (SRS section 1).
