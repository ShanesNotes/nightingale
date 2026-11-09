# 04: Fix what the tilt slice found in the library's steps

Type: task
Status: resolved
Claimed by: claude
Blocked by: none

**What to do:** fix the three faults [ticket 01](01-tilt-neighborhood.md) found in the library's steps, then hold the rebuilt graph to the slice's eight frozen bars and report each bar before and after. This is library organization only, with no organ math, engine or census change.

## Premise check

The six "whole papers" that ticket 01 reported as untagged are six reference lists. Each paper's body was chunked and tagged. Only its bibliography sat in a single chunk past Jev's context, a block of 82,000 to 381,000 characters with no blank line in it. For example, the renal autoregulation review (PMC4551215) has 318 of its 319 chunks tagged.

Two real faults sit behind this:

- The chunker never splits a block longer than its 1,400-character target.
- Reference lists feed the claim graph, where a cited title counts as the citing work's own claim. 374 neighborhood chunks and 64 checked directed rows came from reference lists.

## Scope

- [x] **Chunks.** Split any chunk over 60,000 characters at line breaks, then at sentence ends, to the 1,400-character target, and tag the pieces. At 60,000 characters a chunk fits Jev's 64k-token context with the 2,600-token question block, even at one character per token. Reference lists, and passages that layer zero marks as administrative text, stay out of the claim graph.
- [ ] **Fluids facet.** Rewrite the facet from the 23 gold passages where the annotators or Jev split, against the engine's own fluids boundary (the [FL atlas page](../../../docs/atlas/fluids-endothelium.md)), and freeze the wording before any test.
  - Two annotators who did not write it label the old 100 gold passages and 100 fresh ones drawn after the freeze. They are blind to each other and to Jev. Half the fresh passages carry fluid words and half are drawn at random.
  - The bar: kappa of at least 0.60 between the annotators, and between Jev and their agreed labels, on each set.
  - If it holds, Jev retags the whole corpus on this facet alone. The combined request keeps its old wording, so no other facet's answers move. The wording held and the retag stopped at 56%; [ticket 06](06-finish-fluids-retag.md) finishes it.
- [x] **Terms and the gap list.**
  - Expand abbreviations by the Schwartz–Hearst rule. It finds the one-word long forms the current rule misses, such as ACh, NE and ADH.
  - Match a short form only in its own case, so "NO" no longer turns every "no" into nitric oxide.
  - Resolve each phrase to its longest ending that is a term.
  - Jev types every term with two closed questions: is it a term, and is it at tissue, organ, system or whole-body scale, or a hormone that circulates? The same two annotators label 150 random terms, with the same bar.
  - The gap list admits only edges whose two ends are body-scale terms. Cell-level acetylcholine hits and verb-led or subjectless phrases leave by that rule, not by name.
- [x] **Re-score.**
  - Rebuild candidates and links; only new pairs are sent to Jev.
  - Record the changed pipeline's hashes before scoring.
  - Hold the graph to the eight frozen bars with the sealed answer key, the same traps, and a fresh blind Codex rating of the new top 20 against fresh random edges.
  - Check each find against its source passages and record the verdict with the evidence. This replaces Shane's blind rating ([ticket 02](02-reviewer-blind-rating.md)).

Jev cost: $7.99 so far on top of the slice's $43.43. Finishing the fluids retag adds about $5.90, for about $57.40 in all, inside the $60 bar. This ticket estimated $10, but the retag alone runs about $13.30. MIMIC rows stay local; the text corpus holds none.

## Not in this ticket

[Ticket 05](05-two-step-hub-and-work-families.md) covers the rest:

- the two-step test's hub control;
- the quantity nouns the term step lacks (feedback, resetting, set point);
- counting a textbook's editions and copies as one work;
- the body-scale rule's nitric oxide exception.

## Nitric oxide

The body-scale rule also drops local mediators named alone, so nitric oxide edges leave the gap list along with acetylcholine's. FL owns nitric oxide synthase capacity. Decided after the score: nitric oxide's edges stay. [Ticket 05](05-two-step-hub-and-work-families.md) adds the exception before its rerun, so this ticket's scores stand on the rule they were scored with.

## Answer

The fixes hold. The rebuilt graph passes all eight frozen bars, and the gap list's top 20 has none of ticket 01's cell-scale or badly pulled rows. One step is unfinished: the fluids retag stopped at 199,971 of 360,430 chunks when the TypeSafe credits ran out, and [ticket 06](06-finish-fluids-retag.md) finishes it. No bar reads the fluids tag, so the partial retag leaves the scores as they are.

| Measure | Bar | Ticket 01 | Ticket 04 | Verdict |
|---|---|---|---|---|
| Recall of the answer key | at least 0.70 | 16 of 19 (0.84) | 17 of 19 (0.89) | pass |
| Two-step lift, cut at 2016 | at least 2x random pairs | 6.67x | 4.44x (2 of 20 against 9 of 400) | pass on the count; neither hit holds on reading |
| Codex blind rating | at least 10 of 20 real and relevant, mean above random | 16 of 20 | 19 of 20, mean 3.25 against 2.65 | pass |
| Jev against the agreed gold labels | kappa at least 0.60 on every facet | 13 of 14; fluids 0.54 | 14 of 14; fluids 1.0 on the old set, 0.85 on the fresh | pass |
| Link traps | at least 90% right | 90 of 96 | 90 of 96 | pass |
| Quarantine leaks | none | none | none | pass |
| Top gaps a human measurement would see | at least 1 | 2 of 20 | 4 of 20; 2 hold on reading | pass |
| Jev cost | at most $60 | $43.43 | $51.42; about $57.40 once the retag finishes | pass |

What changed:

- **Chunks.** The 11 chunks over 60,000 characters became 976 pieces, all tagged; the largest chunk is now 57,103 characters. 11,179 chunks are marked as reference lists, and they stay out of the neighborhood with the passages layer zero marks as other design.
- **Fluids facet.** On the new wording the annotators agreed at kappa 0.79 on the old 100 passages and 0.85 (0.70 to 0.97) on 100 fresh ones with 18 agreed positives. Jev against their agreed labels reached 1.0 and 0.85 (0.68 to 0.97).
- **Terms.** The Schwartz–Hearst rule found 3,293 abbreviations in the neighborhood's 421 works. Jev typed 2,634 phrase endings for $0.07, and 1,287 are terms. On 150 random endings Jev agreed with the annotators at kappa 0.88 on whether each is a term and 0.88 on body scale, though the body-scale interval runs from 0.49 to 1.0 because the annotators agreed on only 5 negatives.
- **Gap list.** The body-scale rule removed 81 gaps. Of the 15 rows that left the top 20, ten were cell-scale or badly pulled: seven acetylcholine rows, tyramine, choline and "stimulates renin secretion". Four renal rows were pushed down by stronger ones, and nitric oxide on filtration left by the body-scale rule (see Nitric oxide).
- **Recall** gained atrial stretch raising natriuretic peptide, through central venous pressure, in one work. Baroreflex resetting and lymph return are still missed; [ticket 05](05-two-step-hub-and-work-families.md) adds the resetting noun. The Guyton circulation model's overlap rose from 17 to 22 of its 27 edges.

The agent's source check replaces the developer reviewer's blind rating. Each find's evidence sentences were read against the edge as stated, and then the edge against the answer key and the [solver contract](../../../docs/architecture/solver-contract.md).

- **All 20 top gaps are supported by their passages.** 13 of the 152 passages support their edge only in part or not at all, and none reverses a sign.
- **Ten are renal and body-fluid control the slice lacks.** Extracellular volume acts on sodium excretion, filtration, aldosterone and arterial pressure; pressure natriuresis appears under two names; diuresis raises aldosterone; sodium reabsorption raises arterial pressure; urine output lowers extracellular volume; and vasodilation raises filtration. They stay parked with nightingale#82 for the organ crate phase.
- **Seven restate couplings the engine's design already carries,** under another name or in two steps:
  - Sympathetic activity on blood flow, and on vasodilation, is the arteriolar resistance law.
  - Sympathetic activity on cardiac output runs through the sinus node, venous and filling laws.
  - Plasma volume on stroke volume runs through filling.
  - Sympathetic activity raising contractility is in the solver contract's posture row, where myocardial beta-1 raises elastance, but the answer key leaves it out.
  - Afferent constriction on filtration is the middle step of tubuloglomerular feedback.
  - Extracellular volume on renin acts through the renin inputs the key already has.
- **The other three:** plasma volume on hematocrit is an identity, and extracellular volume on edema is a partition of the body's water. Erythropoietin raising hematocrit acts over weeks.
- **Two of the four human-measurement flags hold:** August 1958 (aldosterone loading) and Williams 1972 (volume depletion), the same two as in ticket 01. The other two rest on a 2024 forearm study's background sentence and a 2025 hyperhydration study's discussion claim.
- **Neither two-step hit holds.** The post-2016 confirmation of sympathetic activity lowering blood flow is that same 2024 sentence, which reports femoral flow unchanged. Baroreflex on vascular tone rests on an undirected discussion sentence in a 2026 study of mice and nonhuman primates. The two-step bar passes on its count, but its confirmations are not evidence until [ticket 05](05-two-step-hub-and-work-families.md) checks each one against its passage.
- **Duplicate editions inflate work counts.** Editions and copies of one textbook raise the count of 16 of the 20 gaps by one or two ([ticket 05](05-two-step-hub-and-work-families.md)).

Decision: the library's steps are fixed. The gap list now reads without a person weeding it, and it still names the kidney's answer to pressure and volume as the largest coupling the slice lacks. The two-step lift and the work counts need ticket 05 before their numbers mean what they say.

Local evidence, outside the repo: `~/lanes/logs/ng-evidence-library/ticket-04/` holds the frozen wording and bars, gold labels, annotator files, scores (`final/`), judge packet and Codex log, and the source check (`final/source-check/source-check.json`, locators and verdicts). Codex ran gpt-6-astra at xhigh, session 01a1bdf9-66d9-7c21-af10-3fc7b70d444e. The same folder holds the pipeline before and after with its diff. Source text stays local.
