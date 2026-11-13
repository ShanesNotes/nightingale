# 16: Fuel stores close over a day

Type: task
Status: open
Blocked by: 09, 15, `.scratch/research/issues/13-law-forms-gut-and-fuels.md`

**What to build:** Fat is stored after meals and released overnight. Muscle holds glycogen. Body protein turns over at its smallest, with creatinine made from muscle, filtered by the kidney and not reclaimed. Ketones are one species, made by the liver from fat overnight and burned by the tissues. Each bed's fuel mix replaces the single mixed fuel of [ticket 09](09-tissues-respire.md), so the respiratory quotient follows the day, and heat production rises after a meal.

**Releases:** the musculoskeletal card's holds on its stores, and the metabolism card's hold on fuel uptake.

- [ ] Tests: each storage and release law matches its form at stated points
- [ ] Run: over a synthetic day whose intake equals what the body spends, each store ends within its drift bound of where it started
- [ ] Run: the respiratory quotient rises after meals and falls overnight. Nitrogen in equals nitrogen out over the day. Creatinine made equals creatinine excreted
- [ ] Run: the respiratory quotient run once with the triglyceride store in the muscle bed's cells and no fat bed. The fat bed stays only if the run fails without it (record 0002)
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): fatty acids, glycerol, lactate, ketones, amino acids, urea and creatinine cross between a bed's interstitium and its cells by the crossing row in research 13's table for ticket 14, which ticket 14 writes at the liver bed, except fatty acids and ketones, which this ticket writes there (01:201-202), and this ticket at every other bed
- [ ] From [research 12](../../research/issues/12-law-forms-heat.md) and [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the meal's heat meets research 12's checks, about 8% of a liquid mixed meal's energy over 8 h and linear in it (D'Alessio), 60, 78 and 91% of a 6 h total by 3, 4 and 5 h (Reed and Hill), and the basal rate again 8 h after dinner (Westerterp). A run that misses D'Alessio's 8% earns the alternative of a fixed share of each macronutrient's energy, paid by an oxidation reaction of its own
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** exercise, amino acids one by one, and lipoproteins.

From [ticket 03](03-whole-body-lists.md)'s Answer: a reaction counts its reference species once. So creatinine from body protein compiles only with creatinine as the reference, to run on a negative rate, and a single storage reaction that burns a share below one half in its own counts cannot be written. [Ticket 04](04-scopes-ports-and-census.md) decides whether the reference may be counted N times. The species rule is by kind of space, so triglyceride may sit in any cells, and the run with the store in the muscle bed's cells needs no change to the lists.

From [ticket 04](04-scopes-ports-and-census.md):

- A reaction counts its reference species N times, used or made, and the contract divides the rate by N ([record 0010](../../../docs/adr/0010-a-reaction-counts-its-reference-n-times.md)). So this ticket writes each reaction forward, its rate in the species its source states. Record 0010 gives these forms:
  - Glycogen stored with its burned share, at the muscle bed: `Proportions::<Glucose, 20>::using`, 20 glucose and 6 oxygen into 19 glycogen, 6 carbon dioxide and 25 water, its rate the glucose used per second. A source that states glycogen made writes `Proportions::<Glycogen, 19>::making` and `uses::<Glucose, 20>` instead.
  - Creatinine from body protein: `Proportions::<Creatinine>::making`, 3 body protein and 6 oxygen into 1 creatinine, 5 carbon dioxide and 4 water, its rate the creatinine made per second.

  These counts are the bench's. They balance the elements, and synthetic energy contents set the heat. This ticket sources its own proportions, its burned share and its fuel, and keeps the form. A bench that holds storage alone is refused as `UnnamedConservedQuantity`, because one direction of its pools has no conserved quantity to name it. In the body the glucose burn beside it names that direction, and a bench declares one at a rate of zero.
- A reaction at one site has one owner (record 0010). Compile refuses two cards' reactions with the same chemistry where both stand (`ReactionOwnedTwice`), and a reaction with its counts scaled is the same chemistry. So storage carries its burned share in its own counts, as above, or MT's own burn pays it. A separate burn that MS declares with MT's terms at the muscle bed is refused.
- Urea and creatinine cross the cell membrane as other solutes, which 01:38 gives to FL. So the box above on research 13's crossing row covers them through FL, which writes each crossing once at every bed, and not through MT or HP.
- The musculoskeletal card reacts at `Beds<Muscle>` only (`ReactsAt`). Body 01 puts protein turnover in the muscle and liver beds' cells under MS (01:207, 233). So this ticket decides who writes the liver's share. HP may declare it at the liver bed, which the one-owner rule allows because the two sites differ, though body 01 names MS. Or MS gets a wider grant, which is a reviewed change to the contract.
- Fat stored after a meal has a one-space form. The move into the fat bed's cells is a crossing, and the storage burn that posts its cost is a reaction in the fat bed's cells beside it (research 13:70, 79). By 01:38, FL crosses the capillary wall and MT the cell membrane. Compile refuses a reaction whose terms sit in two spaces (`ReactionInTwoSpaces`), so no reaction moves triglyceride between spaces.
- **Hard part:** who moves triglyceride into the fat bed's cells? Body 01 gives triglyceride no pool in the fat bed's interstitium (01:535), and research 13 moves it from the fat bed's plasma into its cells in one step (research 13:70). That step crosses both walls, which 01:38 splits between FL at the capillary wall and MT at the cell membrane. Ticket 04's default follows research 13. MT crosses both walls at once (`MetabolismPorts::fat_into_cells`, the endothelial route, at `Beds<Fat>` or `Beds<Muscle>`), and compile counts that route as a crossing of each wall, so FL cannot cross triglyceride there too. The candidate owners are MT alone in one step, or FL and MT through an interstitial pool, which [ticket 03](03-whole-body-lists.md)'s spaces allow.
