# 14: The liver is the glucose hub

Type: task
Status: open
Blocked by: 02, 13, `.scratch/research/issues/13-law-forms-gut-and-fuels.md`

**What to build:** The liver bed takes portal and arterial blood. It stores glucose as glycogen and releases it, takes up lactate, and makes urea from amino acids as a reaction ([ticket 02](02-reactions.md)), so nitrogen has its way out: the kidney filters urea and reclaims part of it. Albumin synthesis is constant. Insulin and glucagon are read at their portal concentrations, from held values until [ticket 15](15-insulin-and-glucagon.md).

**Releases:** the hepatic card's hold, and coupling arrow 99 (portal pressure). Arrow 100, ammonia to the nervous system, stays an explicit hold.

- [ ] Tests: each law matches its form at stated points, and the urea reaction closes the nitrogen book
- [ ] Run: glycogen rises after a meal and falls overnight. Raising the held insulin lowers glucose output
- [ ] Run: nitrogen absorbed equals urea nitrogen made plus the change in stores
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 07's budget

**Non-goals:** drug clearance, bile, ammonia's effect on the brain, and zones within the liver.
