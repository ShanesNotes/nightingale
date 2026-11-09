# Evidence library wayfinder map

## Destination

A fresh agent can ask the evidence library which couplings the engine's body lacks and which sources back the ones it has, and each answer names the works and passages behind it. The library is an index to the evidence: what it proposes enters the engine only through `data/` and the doors the SRS already sets.

## Notes

- The library lives in `~/nightingale-evidence-store`, beside the raw collection in `~/nightingale-evidence`. Only numbers and citations cross into this repo, through `data/` and `30-data`.
- MIMIC rows stay local and map through MIMIC's data dictionary in code. Every other text, licensed and paywalled included, may go to Jev.
- Jev tags the whole text corpus on closed facets (layer zero) and checks each proposed link against its passage. Code proposes links from terms named in one sentence and from model structure, and scores them by independent works. Jev is not bit-stable, so every judgment is logged and replayed.
- Healthy-adult sources set baselines. MIMIC is a primary source for the disease and intervention side of the graph.
- The extraction pilot's held-out works are quarantined: the library drops them before tagging, so no claim, tag or score reads them.
- A model's structure, such as the Guyton circulation model, is a reference to compare against, never a source (SRS rule 6 treats Pulse the same way).

## Decisions so far

- [The tilt neighborhood](issues/01-tilt-neighborhood.md): on the whole corpus, the claim graph for the sustained response to passive head-up tilt found 16 of the slice's 19 posture-path couplings unprompted and named the kidney's answer to pressure and volume as the largest coupling the slice lacks. Seven of eight frozen bars pass; Jev's agreement on the fluids facet fails, and so did the annotators'.
- [Library fixes](issues/04-library-fixes.md): long chunks split, reference lists kept out of the claim graph, a rewritten fluids facet both annotators and Jev agree on, and terms typed by Jev with only body-scale terms in the gap list. All eight tilt bars hold and the top gaps read clean. The agent's source check found every top gap supported and the kidney's answer to pressure and volume still the largest missing coupling, but neither two-step hit confirmed ([ticket 05](issues/05-two-step-hub-and-work-families.md)). Nitric oxide's edges return to the gap list in ticket 05's rerun. The fluids retag finished ([ticket 06](issues/06-finish-fluids-retag.md)).
- Depth is chased as couplings first: a new coupling field or census arrow, before finer law forms or map splits.
- A new coupling enters as a split does under [record 0002](../../docs/adr/0002-map-starts-grouped.md): earned by a sourced check that fails without it, or by a clinician's question that brings a sourced target.
- Animal-only evidence may set a law form when no human source exists, and the form is labelled animal-derived.

## Not yet specified

- **Where the library's code lives.** It sits in the store's `scripts/graph/` beside the store's other scripts; whether it moves to `tools/evidence/` is the data sourcing plan's E01 call.
- **The disease side.** How MIMIC's data dictionary maps onto the facets, and which disease and intervention questions the graph answers first.
- **Records for the two evidence rulings.** Whether the coupling bar and the animal-evidence rule become lines in the SRS or a record.

## Out of scope

- Rows in `data/`, inference records and any change to the engine, the census or the glossary. Each enters through its own ticket.
- Sending MIMIC rows to any cloud model.
