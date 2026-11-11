# 18: The acid loop

Type: task
Status: open
Blocked by: 08, 09, 11, 14, 17, 29, `.scratch/research/issues/14-law-forms-volume-and-acid.md`

**What to build:** Every space states its acid-base balance from what it holds: the strong ion difference, the weak acids and carbon dioxide, with one lumped anion that keeps charge at zero. Tissues post the fixed acid the diet makes. The kidney excretes it as a feedback on the strong ion difference. Acidity is read from the amounts and never set.

**Releases:** the blood chemistry card's remaining holds.

- [ ] Tests: a rise in carbon dioxide lowers the acidity reading as the form says, and charge closes with the new species
- [ ] Run: with acid production and the kidney's feedback both on, the lumped anion's amount ends a day within its drift bound
- [ ] Run: after a step up in acid production, the kidney's feedback brings the total carbon dioxide reading back inside its bound. With the feedback clamped at its rate from before the step, the same run fails
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] From [research 14](../../research/issues/14-law-forms-volume-and-acid.md): the run after a step in acid production allows days, because ammonium's rise peaks at 5 to 6 days, or takes a shorter time constant as a model default
- [ ] From [research 14](../../research/issues/14-law-forms-volume-and-acid.md): cells hold phosphate, a weak acid, so this ticket counts phosphate's term in cells or drops it, and decides whether research 11's tissue store reads this balance. With no weak acid, a cell's balance has a root only while its strong ion difference lies between 0 and its total carbon dioxide
- [ ] From [research 14](../../research/issues/14-law-forms-volume-and-acid.md) and [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the lumped anion's exit from the liver bed's cells, the diet's organic anion's entry into cells, and the lumped anion's filtering and reclaim take the forms of research 14's crossings row. The diet's organic anion is absorbed by research 13's chloride row, solvent drag plus a first-order term in the luminal concentration
- [ ] From [ticket 29](29-the-charge-default.md): the certificate holds charge as each space's balance after speciation, which electroneutrality holds at zero, and the sum of fixed charges, the body's strong ion total, gets a drift rule of its own. That sum moves by design when a meal brings a cation with phosphate or the diet's organic anion, and when the kidney sends ammonium out with chloride. This changes the contract's lines on charge in the certificate, and this ticket checks whether record 0004's charge line changes with them. It goes to the developer reviewer before the build
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** buffering inside cells, bone as a buffer, and the rise in bicarbonate after a meal unless research 14 sources its size.

From [ticket 03](03-whole-body-lists.md)'s Answer: the lumped anion is the species `fixed acid`, which is carried, holds no fixed charge and is -1 after speciation. The diet's organic anion is the species `diet base`, which states acetic acid's atoms and is -1 after speciation. They share no book, because the fixed acid states no atoms, and this ticket's balance reads both. The species table's column for charge after speciation gives the direct relabels into a charge sum, for the fixed and strong rows. A weak acid reaches the sum through a declared conversion in charge per that species, by the cancellation rows, and `Osmoles` has the same two ways in.

From [ticket 04](04-scopes-ports-and-census.md):

- Ammonium has a one-space form. The peritubular bed's cells make it in a reaction under RN's grant at `Beds<Tubule>`, and RN's secretion moves it on to the tubule lumen, a route to another site (research 14:44; 01:209, 235). Compile refuses a reaction whose terms sit in two spaces (`ReactionInTwoSpaces`). So the reaction makes its new total carbon dioxide in the cells too, and MT's crossings carry it to plasma, rather than posting it straight into the peritubular plasma.
- **Hard part:** who moves ammonium across the cell membrane? Body 01 treats it as an ion, which FL moves (01:38, and the potassium shift's owner at 01:709). [Ticket 03](03-whole-body-lists.md) made it a fuel (`crates/ng-contract/src/species.rs:259`), and 01:38 gives the fuels' crossings of the cell membrane to MT. Ticket 04's `CellFuel` leaves ammonium out, so today only FL's `cell_membrane` port can carry it.
- **Hard part:** who moves the diet base across the cell membrane? Each bed's cells take it in to burn it (research 14:43; 01:256), and no card is named for that crossing. After speciation it is an anion, one of the other solutes that 01:38 gives to FL. By its atoms it is a fuel (`crates/ng-contract/src/species.rs:264`), which 01:38 gives to MT. Ticket 04's `CellFuel` leaves it out, so today only FL's `cell_membrane` port can carry it. [Ticket 13](13-gut-absorbs-a-meal.md) points here.
- Body 01 gives the diet base no fecal exit (01:262), though GI's exit takes each meal's unabsorbed share (01:423). Ticket 04 adds no `Feces<DietBase>`. If this ticket's absorption is partial, the exit is one more boundary process.
