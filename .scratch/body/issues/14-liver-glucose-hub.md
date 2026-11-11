# 14: The liver is the glucose hub

Type: task
Status: open
Blocked by: 02, 13, `.scratch/research/issues/13-law-forms-gut-and-fuels.md`

**What to build:** The liver bed takes portal and arterial blood. It stores glucose as glycogen and releases it, makes glucose overnight from lactate, amino acids and glycerol as reactions, and makes urea from amino acids as a reaction ([ticket 02](02-reactions.md)) that also carries the sulfur ticket 18 posts as fixed acid, so nitrogen has its way out: the kidney filters urea and reclaims part of it. Albumin synthesis is constant, with a matching held loss until ticket 21. Insulin and glucagon are read at their portal concentrations, from held values until [ticket 15](15-insulin-and-glucagon.md).

**Releases:** the hepatic card's hold, and coupling arrow 99 (portal pressure). Arrow 100, ammonia to the nervous system, stays an explicit hold.

- [ ] Tests: each law matches its form at stated points, and the urea reaction closes the nitrogen book
- [ ] Run: glycogen rises after a meal and falls overnight. Raising the held insulin lowers glucose output
- [ ] Run: nitrogen absorbed equals urea nitrogen made plus the change in stores
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** drug clearance, bile, ammonia's effect on the brain, and zones within the liver.

From [research 12](../../research/issues/12-law-forms-heat.md) and [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the meal's heat comes from the storage and conversion reactions, so each storage and conversion reaction this ticket writes burns a share of what it uses in its own counts and posts that heat into the liver bed's thermal pool, by the row "The meal's heat" in research 13's table for ticket 16.

From [ticket 29](29-the-charge-default.md): the liver's conversions, glucose from lactate, urea from amino acids and the fixed acid from their sulfur, are written with neutral moieties and no hydrogen ion or counter-charge term. Every term's fixed charge is zero, so only elements and energy can leave such a reaction open.

From [ticket 03](03-whole-body-lists.md)'s Answer: a reaction counts its reference species once. So glucose from lactate has no forward form as the contract stands, urea from the amino acids compiles only with urea as the reference, to run on a negative rate, and a single storage reaction that burns a share below one half in its own counts cannot be written. [Ticket 04](04-scopes-ports-and-census.md) decides whether the reference may be counted N times, and this ticket writes those reactions in the form its Answer gives. The fixed acid is a carried species, so it is no term of the urea reaction: the hepatic card makes it through a synthesis route at research 14's rate `s J_aa`.
