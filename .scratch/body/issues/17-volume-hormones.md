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
- [ ] The certificate passes with the sodium, potassium, chloride and water conserved quantities held by these loops, and the body's trace is updated. The PR also shows the certificate passing on a run at least as long as the measurement note's below, 2,900 s: sampled windows can show a failure and cannot show a pass (the solver contract's section 5)
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs

**Non-goals:** thirst as behavior (drinks are scheduled), sodium stored in skin, and renal nerves beyond what the slice has.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md) and [research 14](../../research/issues/14-law-forms-volume-and-acid.md): insulin multiplies the pump's maximum rate in the muscle and fat beds by `rho = 1 + S_K E_ins`, which [ticket 15](15-insulin-and-glucagon.md) writes, so this ticket's pump carries that factor at 1 until ticket 15 lands.

From [ticket 09](09-tissues-respire.md)'s third PR: the edge from the glomerular bed into the peritubular bed would answer the peritubular bed's oxygen debt, so its debt share is 0 at rung 0, and that vessel's tone is this ticket's to state.

From [ticket 03](03-whole-body-lists.md)'s Answer: the pumps' work is a heat route that draws on the receipt `chemical energy` until [ticket 09](09-tissues-respire.md) pays work by fuel. Once beds burn fuel, the pump law keeps its share out of the bed's demand.

From [ticket 04](04-scopes-ports-and-census.md):

- Every port is read at the instant at rung 0, and a law that needs an average holds it in its own memory (`docs/architecture/solver-contract.md`, section 2, the paragraph "The coupling catalog is generated"). The natriuretic peptide's secretion (research 14:34, arrow 79) is adapted from a model with no beat, so its constants were set against mean atrial pressure. A form linear above an offset gives a different average over a cycle on a pulsatile input than on that input's mean. So this ticket measures the law read at the instant against the same law on the beat mean, on the body's own atrial waveform. If the difference matters, the law averages inside its own memory, never through a port. [Research 05](../../research/issues/05-parameters.md) fits each such constant against the form as the engine evaluates it.
- The sodium pump's energy is no reaction as research 14 writes it (research 14:39; 01:487). One law that moved the ions and burned fuel would span cells and interstitium, which compile refuses (`ReactionInTwoSpaces`). With [ticket 09](09-tissues-respire.md), this ticket picks FL's pump crossing beside MT's burn in cells, or keeps the heat route from the receipt `chemical energy` that [ticket 03](03-whole-body-lists.md) chose.
- Body 01 lists renin secretion under RN with arrow 98 (01:388) and has EN secrete every hormone (01:710). The code follows the second. Renin's secretion is EN's boundary process (`Secretion<Renin>`, `crates/ng-contract/src/scope/processes.rs:100`), and RN sets its rate on arrow 98 (`renin_secretion: Rate<Renin>`).
- FL's water exchange and the Starling route both move water at the capillary wall, and both are FL's, so `CrossedTwice` does not count them. This ticket is the next to edit FL's wall laws.

From a measurement run before [native-contract 85](../../native-contract/issues/85-the-certificates-windows-end-on-breath-edges.md)'s second PR, at main `e2d5c6ba`, to 2,900 s with sleep from 1,800 s. Every value is synthetic, and each rate is a mean over the span it names.

- With this loop open the body leaves salt balance within minutes. Main's certificate refuses the run at the ten-second window that opens 109.7 s after admission: chloride changes 3.0001e-4 against its 3e-4. No suite run of the body passes 90 s, so no test sees it.
- Sodium comes in at exactly 1.0000 a second throughout, by the gut's held absorption. Sodium out is 0.99997 a second from 108 to 161 s, crosses 1.0000 near 230 s, and is 1.0011 from 602 to 650 s, 1.0034 from 1,001 to 1,048 s, 1.0069 from 1,599 to 1,650 s, 1.0089 from 2,000 to 2,048 s and 1.0124 from 2,802 to 2,849 s. It gains most between 1,000 and 1,200 s, 0.0013 in 200 s, and gains 0.0008 in the 200 s to 2,800 s. By the last breath edge, at 2,895 s, the body has lost 16.8 of sodium, 17.6 of chloride and 0.84 of potassium.
- Chloride and potassium rise in step with it: from 2,802 to 2,849 s chloride out is 1.0631 a second against 1.0500 in, and potassium out 0.05062 against 0.05000. Each salt is then 1.24% over what comes in.
- Sleep's onset at 1,800 s leaves no mark on the rise.
- Water stays near balance to the last breath edge, 0.11 lost of about 90 taken in. As means over spans of about 50 s, the water leaving swings between 2.86e-2 and 3.41e-2 a second, and vasopressin's secretion less its clearance between -1.2e-3 and +1.5e-3. The swing turns in 1,000 s or more, so the run holds too few turns to say whether it decays.
- What moves the excretion is not found. The three salts rise in step. That fits a change in the filtered load, and it fits as well a change in the reclaim they share: both cations cross by one saturating transporter and chloride crosses with them (`crates/ng-renal/src/shared.rs`). The run does not tell the two apart.

The PR that closes the loop says what moved the excretion on main.
