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

From [ticket 04](04-scopes-ports-and-census.md):

- A reaction counts its reference species N times, used or made, and the contract divides the rate by N ([record 0010](../../../docs/adr/0010-a-reaction-counts-its-reference-n-times.md)). So this ticket writes each reaction forward, its rate in the species its source states. Record 0010 gives these forms:
  - Glycogen stored with its burned share, at the liver bed: `Proportions::<Glucose, 20>::using`, 20 glucose and 6 oxygen into 19 glycogen, 6 carbon dioxide and 25 water, its rate the glucose used per second. A source that states glycogen made writes `Proportions::<Glycogen, 19>::making` and `uses::<Glucose, 20>` instead.
  - Urea from the amino acids: `Proportions::<Urea>::making`, 2 amino acids and 6 oxygen into 1 urea, 5 carbon dioxide and 5 water, its rate the urea made per second.
  - Glucose from lactate, its energy paid by fuel: `Proportions::<Glucose>::making`, 3 lactate and 3 oxygen into 1 glucose, 3 carbon dioxide and 3 water, its rate the glucose made per second. Its uses hold more energy than its makes, so it warms its bed.

  These counts are the bench's. They balance the elements, and synthetic energy contents set the heat. This ticket sources its own proportions, its burned share and its fuel, and keeps the form. A bench that holds storage or glucose from lactate alone is refused as `UnnamedConservedQuantity`, because one direction of its pools has no conserved quantity to name it. In the body the glucose burn beside it names that direction, and a bench declares one at a rate of zero.
- A reaction at one site has one owner (record 0010). Compile refuses two cards' reactions with the same chemistry where both stand (`ReactionOwnedTwice`), and a reaction with its counts scaled is the same chemistry. So storage carries its burned share in its own counts, as above, or MT's own burn pays it. A separate burn that HP declares with MT's terms at the liver bed is refused. Glucose from lactate is written forward with `making` and its own fuel terms. Written backward, as glucose to 2 lactate on a negative rate, it would carry MT's glycolysis terms at the liver bed and be refused.
- Urea and creatinine cross the cell membrane as other solutes, which 01:38 gives to FL, and not as fuels. So FL writes each of their crossings once, at every bed, the liver's included. HP's `fuel_into_cells` takes only `CellFuel`, which leaves them out.
- Body 01 states no first pass for glucagon at the liver, so HP's only first-pass process is `InsulinFirstPass`. This ticket adds a process for glucagon if a source gives it a first pass.
