# 23: How the drive to breathe splits into rate and depth

Type: research
Status: claimed
Claimed by: r23-split
Blocked by: none
Blocks: ../../body/issues/38-the-drive-moves-the-breaths-depth.md

[Body ticket 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md) waits on this Answer to build depth on the whole body. The question reads the literature and needs no merged code.

**Question:** As chemical drive rises from rest in a healthy adult, how much of the rise in ventilation comes from the breath's rate and how much from its depth, and in what form? [Research 11](11-law-forms-gas-loop.md) gives the controller's output as release and no split, and [research 20](20-the-breaths-rhythm.md) asks for the resting rhythm and leaves the controller out. [Body ticket 11](../../body/issues/11-breathing-is-controlled.md) moves only the rate, for cost, with no source for the split.

Today's form, from body 11's plan: the drive scale multiplies the breath's rate and leaves the pump drive's peak alone. Offline on the packet's breath forms, alveolar ventilation's relative sensitivity to the scale is 1.296 through depth and 0.778 through rate, and 0.191 through inspiration's length at a fixed breath.

- [ ] Rate and tidal volume against ventilation over hypercapnic and hypoxic steps from rest, with the population and the method
- [ ] Whether inspiration's length or expiration's carries the rate change
- [ ] Whether the split differs between the central and the peripheral arm, or between waking and NREM sleep
- [x] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it
- [ ] For body 38: the split in a form a law can state, depth and rate each a function of the drive, with typed parameters
- [ ] What bounds depth as the drive rises, and where rate carries the rest of the rise, with the population and the method

**Non-goals:** exercise, and the contract form that would let depth move once a breath ([native-contract 87](../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md)).

## Answer

The store holds no measurement of rate and tidal volume under a hypercapnic or hypoxic step from rest. So the split under a rising drive is a model default, built from what the store does hold: two primary studies of resting sleep, and textbook lines on the inflation reflex and on the timing of the breath. Each statement below carries its tier (SRS section 6) and says whether a primary source would upgrade it. One box is ticked, the rows for research 16. Every other box says what is missing.

The search, for every box below: the store's full-text search for "breathing pattern", "tidal volume frequency carbon dioxide", "VT/TI", "TI/TTOT", "mean inspiratory flow", "inspiratory duration", "Hering-Breuer", "inflation reflex", "rebreathing", "occlusion pressure", "respiratory frequency hypoxia", "depth and rate of breathing", "increase in tidal volume", "Milic-Emili", "von Euler" and "Gothe", the store's source titles and DOIs, and a word filter over every chunk for a carbon dioxide term with "tidal volume", a rate and a timing word. The filter's 19 hits are ventilated or exercising patients and textbook tables. Dahan 1990 (J Physiol 428:485, full text) reports ventilation only, breath by breath, with no tidal volume or rate.

### What the store holds

| Work | Tier | Who, how many, how | What it gives |
|---|---|---|---|
| Douglas 1982, Thorax 37:840 to 844, full text (PDF pp. 1 to 4) | primary | 19 healthy adults, 8 men and 11 women, at Denver's 1,600 m, room air, lying in bed. Expired flow by hot wire anemometer through a sealed face mask with valves, mask dead space 70 to 85 mL. Each stage at least 2 min stable | Breathing in every sleep stage more rapid and shallower than awake. Tidal volume in REM 73% of waking. Mean inspiratory flow (`V_T / T_I`), which the authors take as an index of inspiratory drive (citing Milic-Emili and Grunstein 1976), lower in REM than awake or in non-REM. No difference in inspiration's share of the breath (`T_I / T_TOT`) between any stages. The faster rate asleep came from parallel falls in `T_I` and `T_E` (PDF p. 4). The figures give the values as plots: their numbers were not readable in the text layer |
| Stradling 1985, Thorax 40:364 to 370, full text (table PDF pp. 3 to 4, printed p. 366) | primary | 6 healthy men aged 18 to 32, Oxford, seated in a fixed chair, no mask: respiratory inductance plethysmography calibrated by regression against spirometry. At least 10 min per stage | Awake, slow wave sleep and REM: minute ventilation 6.28, 5.67 and 5.44 L/min; rate 15.1, 15.2 and 14.9 /min; tidal volume 420, 373 and 367 mL; `V_T / T_I` 17.2, 16.7 and 14.7 L/min; `T_I / T_TOT` 0.367, 0.350 and 0.374. The fall in ventilation came almost entirely from tidal volume and `V_T / T_I`, not from timing (PDF p. 3) |
| Guyton and Hall 12e, p. 506 (PDF p. 527) | textbook | Human adults | The inflation reflex is probably not active until tidal volume passes about three times normal, about 1.5 L. A stronger pneumotaxic signal ends inspiration sooner, which shortens expiration too, so rate rises |
| West and Luks, West's Respiratory Physiology 11e, PDF p. 321 | textbook | Human adults | The stretch reflexes are largely inactive in adult humans unless tidal volume exceeds 1 L. Vagal block in awake humans changes neither rate nor volume |
| Eckert Animal Physiology 4e, PDF p. 565 | textbook, mismatched population (mammals in general) | Mammals | The ratio of inspiration's length to expiration's stays constant as the breath's length changes |
| Ganong 26e, PDF pp. 1503 to 1504 | textbook, from animal work | Intact animals | Raising inspiratory neurons' activity raises both rate and depth |

The two sleep studies disagree on rate. Stradling's own Discussion (PDF p. 5) gives the reason: Douglas used a tight mask at 1,600 m, with mild hypoxaemia (end-tidal PO2 about 84 mmHg) and about 80 mL of added dead space, and masks are known to slow the waking rate, an effect that may vanish with sleep. That would make Douglas's rate rise in sleep partly an artefact of the waking baseline. The body takes Stradling's: no mask, sea level, and the body's patient has no mask. The two agree on the part the body needs most: inspiration's share of the breath does not move with sleep, and the falling drive shows in mean inspiratory flow. A primary study of sleep at sea level without a mask in more subjects would upgrade it.

### Rate and tidal volume over hypercapnic and hypoxic steps from rest

Missing. No work on hand measures rate and tidal volume against ventilation under a chemical step from rest, in any population. The textbook lines give the direction only: both rate and depth rise (Ganong PDF pp. 1503 to 1504, from animals). Hey 1966, the classic plot of ventilation against tidal volume under several stimuli, would settle it; it is not in the store, and no reference list the store holds names it, so its locator is unchecked.

### Whether inspiration's length or expiration's carries the rate change

Partly answered, for sleep only. Douglas 1982 found a faster rate asleep carried by parallel falls in `T_I` and `T_E`, with `T_I / T_TOT` unchanged (primary, resting sleep, PDF p. 4). Stradling 1985 found `T_I / T_TOT` unchanged across waking and sleep (primary, PDF pp. 3 to 4). Eckert gives the constant ratio as a mammalian rule (textbook, mismatched population). No work on hand tests it under a chemical step, so the model default below keeps inspiration's share fixed and lets both lengths carry the rate. Clark and von Euler 1972 (J Physiol 222:267 to 295, locator checked in Douglas 1982's reference list) would settle how `T_I` falls once tidal volume reaches its off-switch volume.

### The arms and sleep

The central and the peripheral arm: no source on hand separates their split, by the search above. Dahan 1990 separates the arms' gains and time courses but reports ventilation only. The model default applies one split to the summed drive scale, as the body sums the arms today. A primary study that reports tidal volume and rate under an isocapnic hypoxic step and under a hyperoxic CO2 step would settle it.

Waking against NREM sleep: at rest, the fall in drive with sleep shows in depth and leaves timing alone (Stradling 1985, primary, 6 men). Under a chemical step in sleep, no work on hand measures the split. Gothe 1981 (J Appl Physiol 50:724, CO2-stimulated breathing in quiet sleep) would settle it: the curation study names it, and its registry folder is empty. Its locator is from the curation study's source list and is not checked in a reference list the store holds.

### What bounds depth, and where rate takes the rest

Model default, with textbook figures for the bound. No work on hand measures where depth stops rising under a chemical drive. The off-switch's volume is the bound the textbooks name: the inflation reflex is inactive below a tidal volume of about 1 L (West PDF p. 321) or about 1.5 L, three times normal (Guyton 12e p. 506). Both are textbook tier, adult humans, with no method stated. Above that volume the off-switch ends inspiration sooner and the rate rises (Guyton 12e p. 506). So depth carries the rise until tidal volume reaches a ceiling between 1 and 1.5 L, two to three times the resting tidal volume, and rate carries the rest. That the reflex's onset is depth's ceiling under a chemical drive is an inference from these lines. Hey 1966 would give the measured volume where rate takes over, and Clark and von Euler 1972 the form of the bend.

### The split as a law can state it

Model default with no sourced form, from the rows above. With `C` the drive scale (dimensionless, one at the awake rest), the depth factor `D` and the rate factor `R` are:

- `D(C) = smooth_min(C, k_V, w_V)`, so tidal volume is `V_T = V_T0 D(C)`.
- `R(C) = C / D(C)`, so the rate is `f = f_0 R(C)`, and inspiration's length and expiration's both fall as `1 / R(C)`, keeping `T_I / T_TOT` at its rest.
- So `V_E = V_T f = V_T0 f_0 C`: minute ventilation stays linear in the drive scale, as body 11's form has it, and only its split changes.

| Parameter | Dimension | Meaning |
|---|---|---|
| `C` | dimensionless | The drive scale, computed, not a parameter |
| `V_T0` | volume | Resting tidal volume, research 20's box |
| `f_0` | rate | Resting rate, research 20's box, `ns.breath-rate` today |
| `k_V` | dimensionless | Depth's ceiling over `V_T0`, above one. Textbook figures put it at 2 to 3 |
| `w_V` | dimensionless | The smooth minimum's width, small beside `k_V - 1` so that `D(1)` is one to rounding |

The range: from below the awake rest (sleep, `C < 1`, where the form puts the whole fall in depth with rate held, as Stradling found) up to drives with tidal volume above the ceiling. No source on hand bounds it above, and exercise is a non-goal. The form is evaluated at the instant on the drive scale. Its sources describe mean breaths over minutes (Stradling's 10 min means, Douglas's 2 min stable stages), so `k_V` is fitted at the instant against means: flagged in its row. If native-contract 87's firing sets the peak at each breath edge, depth reads the scale once a breath, which is nearer its sources' means.

### Rows for research 16

Added to research 16's RS table: RS-016, `k_V`, sourced, bound by the tidal volume at which the rise in ventilation under progressive hypercapnia passes from depth to rate, over resting tidal volume; and RS-017, `w_V`, a model default. `V_T0` and `f_0` are research 20's, and their rows wait on its Answer.

### What body 38 should know

The engine today, in `crates/ng-nervous/src/breath.rs`:

- The drive scale speeds the phase in both inspiration and expiration: line 103, `k.scale(rate, scale)`, inside the loop over `[&inspiration, &expiration]` at line 100. So inspiration's share stays fixed (`ns.inspiration-length` and `ns.expiration-length`, lines 56 and 58), which the sleep studies support.
- The pump drive rises toward `ns.drive-peak` (line 59) at the fixed `ns.drive-rise` (line 50, read at lines 89 to 90). A shorter inspiration reaches a lower drive, so depth falls slightly as rate rises.

For depth to rise with the drive, the builder changes the peak: `ns.drive-peak` times `D(C)` at each breath edge, native-contract 87's candidate. The rate keeps its form with `R(C) = C / D(C)` in place of `C` on line 103. The fixed rise speed is the second change, and it is optional: scaling `ns.drive-rise` by the same `R(C)` keeps the drive's shape fixed in phase, so a rate change above the ceiling does not cut the depth. Without it, depth above the ceiling falls as rate rises.

The sensitivities in "Today's form", by the same arithmetic: below the ceiling the split puts the whole scale on depth, so alveolar ventilation's relative sensitivity to the scale is 1.296, through depth, and zero through rate. That is 1.296 / 0.778 = 1.67 times body 11's rate-only sensitivity at rest, so the same gains give a loop about two thirds stronger at rest. Body 38's run box should print that factor beside 11b's figure, and research 22 sizes the gains for it. Above the ceiling the rate carries the scale at 0.778 with today's fixed rise speed, or at one if the rise speed scales with `R(C)` and tidal volume holds at its ceiling.

Checks for the builder, inferred: depth on the body follows the pump drive's peak through the receptor's effect at the pump's junction, which is not linear, so tidal volume against `C` should be read on the fixture, not assumed to be `D(C)`. A body at `C = 1` must keep every awake bit, which needs `w_V` small enough that `D(1)` rounds to one. The glossary's "Drive scale" says only the breath's phase reads it; body 38 makes the peak read it too, and that entry changes in its PR.

### Gaps, each with what it would settle

None of these is in the store, and none is a source here.

- Hey 1966 (locator unchecked, no reference list on hand names it): ventilation against tidal volume under CO2 and hypoxia, so the first, sixth and fifth boxes: depth's rise and the volume where rate takes over.
- Clark and von Euler 1972, J Physiol 222:267 to 295 (checked in Douglas 1982's reference list): tidal volume against inspiration's length, so the second and sixth boxes.
- Milic-Emili and Grunstein 1976, Chest 70 suppl:131 to 133 (checked in Douglas 1982's reference list): the split of ventilation into mean inspiratory flow and inspiration's share, so the fifth box's form.
- Gothe 1981, J Appl Physiol 50:724 (from the curation study's source list, unchecked): the split under CO2 awake against quiet sleep, so the third box.
- Tobin 1983, Chest 84:202 (from the curation study's source list, unchecked): the resting pattern in normal subjects, so research 20's boxes and `V_T0`.

A fetch of all five would close the first, second, third, fifth and sixth boxes, the third only for sleep: the arms' split would still need a study that reports pattern by arm.
