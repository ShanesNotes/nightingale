# 17: Volume hormones and the kidney's pressure answer

Type: task
Status: open
Blocked by: 06, 26, `.scratch/research/issues/14-law-forms-volume-and-acid.md`

**What to build:** The kidney releases renin, which becomes angiotensin II and raises aldosterone. Wall stress releases a natriuretic peptide, from the owner [ticket 01](01-rung-0-specification.md) names. Vasopressin is already a species with a first-order clearance. Its secretion stops being a held logistic of sodium: it answers osmolality and volume and rises at night. The kidney's sodium excretion answers pressure and these hormones, and its potassium excretion answers aldosterone. The clock scales filtration. Cell water follows tonicity across the cell membrane, which replaces the slice's hold on cell water. A pump and a leak hold potassium inside cells, so what insulin moves in can return, and the pump's energy is charged to metabolism.

**Releases:** the endocrine card's vasopressin hold, its held renin chain and natriuretic peptide, the blood chemistry card's hold on cell water, and coupling arrows 79 (wall stress), 89 (drive from the nervous system) and 98 (renin). It adds the pressure answer that [evidence library ticket 03](../../evidence-library/issues/03-pressure-natriuresis-check.md) found the slice's kidney lacks.

- [ ] Tests: each secretion, conversion and excretion law matches its form at stated points
- [ ] Run: a salt load is excreted over the following day and body sodium returns to its start. With the hormones held, a rise in arterial pressure still raises sodium excretion
- [ ] Run: lying down raises the peptide. Night urine flow falls below the day's, with no law setting urine flow
- [ ] Every link of each loop moves its reader the way the loop needs, on the admitted start
- [ ] The certificate passes with the sodium, potassium, chloride and water books held by these loops, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** thirst as behavior (drinks are scheduled), sodium stored in skin, and renal nerves beyond what the slice has.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md) and [research 14](../../research/issues/14-law-forms-volume-and-acid.md): insulin multiplies the pump's maximum rate in the muscle and fat beds by `rho = 1 + S_K E_ins`, which [ticket 15](15-insulin-and-glucagon.md) writes, so this ticket's pump carries that factor at 1 until ticket 15 lands.

From [ticket 03](03-whole-body-lists.md)'s Answer: the pumps' work is a heat route that draws on the receipt `chemical energy` until [ticket 09](09-tissues-respire.md) pays work by fuel. Once beds burn fuel, the pump law keeps its share out of the bed's demand.

From [ticket 04](04-scopes-ports-and-census.md):

- Every port is read at the instant at rung 0, and a law that needs an average holds it in its own memory (`docs/architecture/solver-contract.md`, section 2, the paragraph "The coupling catalog is generated"). The natriuretic peptide's secretion (research 14:34, arrow 79) is adapted from a model with no beat, so its constants were set against mean atrial pressure. A form linear above an offset gives a different average over a cycle on a pulsatile input than on that input's mean. So this ticket measures the law read at the instant against the same law on the beat mean, on the body's own atrial waveform. If the difference matters, the law averages inside its own memory, never through a port. [Research 05](../../research/issues/05-parameters.md) fits each such constant against the form as the engine evaluates it.
- The sodium pump's energy is no reaction as research 14 writes it (research 14:39; 01:487). One law that moved the ions and burned fuel would span cells and interstitium, which compile refuses (`ReactionInTwoSpaces`). With [ticket 09](09-tissues-respire.md), this ticket picks FL's pump crossing beside MT's burn in cells, or keeps the heat route from the receipt `chemical energy` that [ticket 03](03-whole-body-lists.md) chose.
- Body 01 lists renin secretion under RN with arrow 98 (01:388) and has EN secrete every hormone (01:710). The code follows the second. Renin's secretion is EN's boundary process (`Secretion<Renin>`, `crates/ng-contract/src/scope/processes.rs:100`), and RN sets its rate on arrow 98 (`renin_secretion: Rate<Renin>`).
