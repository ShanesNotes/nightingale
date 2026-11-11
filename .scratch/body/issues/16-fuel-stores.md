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
- [ ] From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): fatty acids, glycerol, lactate, ketones, amino acids, urea and creatinine cross between a bed's interstitium and its cells by the crossing row in research 13's table for ticket 14, which ticket 14 writes at the liver bed and this ticket at every other bed
- [ ] From [research 12](../../research/issues/12-law-forms-heat.md) and [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the meal's heat meets research 12's checks, about 8% of a liquid mixed meal's energy over 8 h and linear in it (D'Alessio), 60, 78 and 91% of a 6 h total by 3, 4 and 5 h (Reed and Hill), and the basal rate again 8 h after dinner (Westerterp). A run that misses D'Alessio's 8% earns the alternative of a fixed share of each macronutrient's energy, paid by an oxidation reaction of its own
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** exercise, amino acids one by one, and lipoproteins.
