# 01: The tilt neighborhood

Type: prototype
Status: resolved
Claimed by: claude
Blocked by: none

**Question:** Can a claim graph over the evidence library say what the sustained heart rate and arterial pressure response to passive head-up tilt in healthy adults depends on, find the slice's posture-path couplings without being told them, and name couplings the slice lacks that a sourced check could see?

- [x] An answer key of 19 posture-path couplings and slice laws, the pilot's quarantine, five competency questions, the 2016 cut year and eight pass bars frozen and hashed before the graph was built
- [x] A 100-passage gold slice labelled by two annotators before Jev saw it
- [x] Jev's layer zero on the whole text corpus
- [x] Links proposed from terms named in one sentence, checked by Jev against their passages, and scored by independent works
- [x] Recall, gaps, two-step lift, the Codex blind rating, the outside model, Jev's agreement, the link traps, leakage and cost measured on the whole corpus

The developer reviewer's blind rating is [ticket 02](02-reviewer-blind-rating.md), the largest gap's sourced check is [ticket 03](03-pressure-natriuresis-check.md), and the pipeline faults this run found are [ticket 04](04-library-fixes.md).

## Answer

Yes, with one bar missed. Jev tagged 355,934 of the corpus's 359,561 text chunks on closed facets. The others are the 100 gold passages, tagged apart; 3,521 chunks that repeat another word for word and share its tags; and 6 reference lists too long for Jev's context. Their papers' bodies were tagged; [ticket 04](04-library-fixes.md) splits and tags the lists. Jev marked 15,672 chunks from 431 works as the tilt neighborhood, which name 1,908 terms in three or more works, and checked all 21,327 term pairs named in one sentence. 2,893 directed edges came out, 786 of them in two or more works. Run 1 stopped at 13.6% of the corpus when the API ran out of credits; its column is kept for comparison.

| Measure | Bar | Whole corpus | Run 1 | Verdict |
|---|---|---|---|---|
| Recall of the answer key | at least 0.70 | 16 of 19 (0.84) | 15 of 19 | pass |
| Two-step lift, cut at 2016 | at least 2x random pairs | 6.67x (2 of 20 against 6 of 400) | 1.33x | pass, on two hits |
| Codex blind rating | at least 10 of 20 real and relevant, mean above random | 16 of 20, mean 2.8 against 2.25 | 18 of 20 | pass |
| Jev against the agreed gold labels | kappa at least 0.60 on every facet | 13 of 14; fluids 0.54 | not run | fail |
| Link traps | at least 90% right | 90 of 96 (0.94) | not run | pass |
| Quarantine leaks | none | none | none | pass |
| Top gaps a human measurement would see | at least 1 | 2 of 20 | 1 of 20 | pass |
| Jev cost | at most $60 | $43.43 for 1.03 billion input tokens | $6.16 | pass |

What the graph says:

- **It finds the slice's couplings unprompted.** The three misses are baroreflex resetting, which never became a term because the term step lacks "resetting"; lymph return; and atrial stretch raising natriuretic peptide. Tubuloglomerular feedback, missed in run 1, is found in one work. The renal autoregulation review covers it at length and was tagged, but "feedback" is not one of the term step's quantity nouns, so the phrase never becomes a term ([ticket 05](05-two-step-hub-and-work-families.md)).
- **The largest gap is the kidney's answer to pressure and volume.** Ten of the top 20 gaps are renal and body-fluid control: extracellular volume raising sodium excretion, pressure natriuresis twice, diuresis raising aldosterone, renal sympathetic activity lowering sodium excretion, efferent arteriolar tone, nitric oxide and renal vasodilation setting filtration, urine output lowering extracellular volume, and blood volume and renin. The census has no cardiovascular-to-renal arrow for pressure natriuresis, and [ticket 03](03-pressure-natriuresis-check.md) asks whether the slice's renal laws give it anyway. The two gaps a human perturbation study in the corpus would see are both here: August 1958 (aldosterone loading) and Williams 1972 (posture and volume depletion).
- **Cell-scale acetylcholine edges entered the top 20 with the whole corpus.** Seven rows are acetylcholine's actions, from endothelial nitric oxide to nodal potassium conductance, and two more are tyramine and choline. The engine carries vagal release at the sinus node's M2 receptor through S10, so most sit below its scale or under another name. Three rows are term-step faults, such as "cytoplasmic concentration" and "stimulates renin secretion"; [ticket 04](04-library-fixes.md) tightens the term step.
- **Two-step discovery passes on thin evidence.** Two of the top 20 chains built before 2016 are confirmed after it, both baroreflex control of vascular tone and resistance, against 6 of 400 random pairs. At the baseline rate two or more hits come by chance about 4% of the time, and most chains run through two hubs, arterial pressure and sympathetic activity. The edges dated before the cut grew from run 1's 103 to 275.
- **The Codex rater finds the candidates real, not more relevant.** Candidates score 1.8 of 2 on real against 1.2 for random pairs, and 1.0 against 1.05 on relevance to the tilt itself, the same pattern as run 1. The renal gaps act over hours and the acetylcholine edges at cell scale, while a 16-minute tilt is reflex control.
- **The Guyton circulation model agrees on more of its structure.** 17 of the 27 directed edges read from its component names are in the graph, against 14 in run 1. The missing ten are angiotensin on both renal arterioles, the veins and thirst; natriuretic peptide on the afferent arteriole; autoregulatory feedback on the efferent arteriole; aldosterone on potassium transport; renin activity on heart rate; vasopressin on thirst; and pressure distending the arteries.
- **Six edges conflict in sign** across works, among them baroreflex and heart rate in 11 works, where the sign depends on whether the reflex is loaded or unloaded, and central venous pressure and cardiac output. 25 edges with two or more works rest only on animal studies and are labelled animal-derived.
- **Jev agrees with the annotators on 13 of 14 facets** (kappa 0.66 to 0.97) and gets 90 of 96 link traps right. Fluids fails at 0.54, but the two annotators reached only 0.46 on it, so the facet's wording is at fault.

Decision: the library works as an index to the evidence. It finds the slice's couplings, and it names the kidney's answer to pressure and volume as what the slice lacks, a gap with human studies that can check it. The gap table still needs [ticket 04](04-library-fixes.md)'s term fixes before its top rows can steer work without a person reading them. The largest gap goes to a sourced check, [ticket 03](03-pressure-natriuresis-check.md), before any coupling enters under record 0002. The disease side and MIMIC wait for ticket 04.

Local evidence, outside the repo: the pipeline in `~/nightingale-evidence-store/scripts/graph/`, its data in `~/nightingale-evidence-store/data/graph/`, and the frozen inputs, gold labels, scores, judge packets, screenshots and report in `~/lanes/logs/ng-evidence-library/tilt-slice/` (`final/` for this run, `run1/` for run 1, hashes in `frozen/hashes.txt` and `frozen/pipeline-hashes.txt`). Source text stays local; the page and this ticket name titles and locators only.
