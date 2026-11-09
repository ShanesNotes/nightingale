# 03: The whole body's species, spaces and networks

Type: task
Status: open
Blocked by: 01, 02

**What to build:** The contract's closed lists grow once, for all nine systems the slice lacks, so that no later ticket edits them. Today there are nine species (water, sodium, potassium, chloride, albumin, vasopressin, norepinephrine, acetylcholine and energy), no airway or gut network, and no site or space for alveolar gas, the gut's lumen or red cells. The lists come from [ticket 01](01-rung-0-specification.md)'s Answer.

- [ ] Species for the gases, the fuels, urea and creatinine, the acid-base set, the hormones and hemoglobin, each with its dimension, its charge, the books it belongs to and the spaces that may hold it
- [ ] Lumped classes where rung 0 needs no more: one other strong anion, ketones as one species
- [ ] Spaces and site classes for alveolar gas, the gut lumen and whatever ticket 01 names, and networks and edge kinds for the airway and the gut, with the portal circuit on the blood network
- [ ] Landmarks for each new sensor: the chemoreceptors, the thermostat's sensors, the osmoreceptor
- [ ] Temperature at a site, read from the heat it holds
- [ ] One membrane type with its two settings, as SRS section 5 states it, before the first new exchange law ([foundation 07](../../foundation/issues/07-contract.md) lists this as C3)
- [ ] A compile-fail test with its legal twin for each new type rule
- [ ] The slice's model hash and trace do not move

**Non-goals:** any law, any new pool in the slice, and species only a later scope stage needs (drugs, pathogens, clotting factors).

This changes the contract's interface, so it runs pstack `architect` with an Opus and a Codex runner and a Codex judge. Split it into two PRs, species first, if the judge finds the diff too wide to review.
