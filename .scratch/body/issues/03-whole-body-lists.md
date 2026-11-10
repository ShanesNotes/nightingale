# 03: The whole body's species, spaces and networks

Type: task
Status: open
Blocked by: 01, 02, 29

**What to build:** The contract's closed lists grow once, for all nine systems the slice lacks, so that no later ticket edits them. Today there are nine species (water, sodium, potassium, chloride, albumin, vasopressin, norepinephrine, acetylcholine and energy), no airway or gut network, and no site or space for alveolar gas, the gut's lumen or red cells. The lists come from [ticket 01](01-rung-0-specification.md)'s Answer.

- [ ] Species for the gases, the fuels, urea and creatinine, the acid-base set, the hormones and hemoglobin, and iron, ammonium, glycerol, body protein, the cell anion and the diet's organic anion, each with its dimension, its charge as [ticket 29](29-the-charge-default.md) picks it, the books it belongs to and the spaces that may hold it. Oxygen, carbon dioxide and glucose are in the list already, from [ticket 02](02-reactions.md), and stay
- [ ] Lumped classes where rung 0 needs no more: one other strong anion, ketones as one species
- [ ] From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): glycogen, triglyceride and body protein each get a unit with atoms for their reactions, a glucosyl residue, a triglyceride of stated formula and an amino acid residue, because ticket 01 measures them as masses and a reaction counts whole units of a fixed formula
- [ ] Spaces and site classes for alveolar gas, the gut lumen and whatever ticket 01 names, and networks and edge kinds for the airway and the gut, with the portal circuit on the blood network. The alveolar and airway gas is a space that holds no water, with a gas-law identity, and it holds nitrogen gas and water vapor. Red cells get no space, because hemoglobin is a mass in blood ([ticket 08](08-blood-carries-gases.md))
- [ ] Route families across the cell membrane, heat carried by blood, conduction between heat stores, and an osmotic identity for small solutes, so tickets 12, 15 and 17 edit no shared list
- [ ] Landmarks for each new sensor: the chemoreceptors, the skin thermoreceptor, the osmoreceptor, the volume receptor, the adrenal, the islets and the parathyroid. Core temperature reaches the nervous system by arrow 91, so it needs no landmark. Each new hormone has its secretion and clearance sites
- [ ] Temperature at a site, read from the heat it holds
- [ ] One membrane type with its two settings, as SRS section 5 states it, before the first new exchange law ([foundation 07](../../foundation/issues/07-contract.md) lists this as C3)
- [ ] A compile-fail test with its legal twin for each new type rule
- [ ] The slice's model hash and trace do not move

**Non-goals:** any law, any new pool in the slice, and species only a later scope stage needs (drugs, pathogens, clotting factors).

This changes the contract's interface, so it runs pstack `architect` with an Opus and a Codex runner and a Codex judge. Split it into two PRs, species first, if the judge finds the diff too wide to review.

From [research 12](../../research/issues/12-law-forms-heat.md): the identity table has no row for heat over a heat capacity, a conductance times a temperature difference, or a heat capacity per volume times a flow times a temperature difference, which a heat store's temperature, conduction and the heat blood carries need.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the amino acid species' fixed formula sets the counts of the liver's reaction that turns them into urea, and this ticket writes into each storage and conversion reaction's counts the share of what it uses that the reaction burns, which is the meal's heat. No operator gives a rate of change and no law can hold a value from a delivery's instant, so rung 0 leaves out the first phase of insulin secretion and Visentin's portal term, and the stomach empties without the meal's starting volume.

From [research 14](../../research/issues/14-law-forms-volume-and-acid.md): no row turns each charged species' concentration into a charge concentration, which the strong ion difference and the mass-action terms need, and none takes a flow over a flow, an osmolality per volume times a volume, or a sum of two species' amounts. The lumped anion's makeup is this ticket's to settle, because sulfur is no element the ledger balances.

From [research 15](../../research/issues/15-law-forms-slow-books.md): GI's iron absorption reads the store in the liver bed's cells, so the landmark list needs a place where GI's law reads that store. Ticket 01 lists bone bed cells for calcium and phosphate and not for magnesium, while its MS row gives bone's pool to all three, so this ticket lists bone bed cells for magnesium or [ticket 19](19-minerals-and-bone.md) drops magnesium's bone pool.
