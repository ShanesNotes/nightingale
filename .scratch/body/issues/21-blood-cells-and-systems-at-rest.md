# 21: Blood cells, plasma proteins and the systems at rest

Type: task
Status: open
Blocked by: 08, 14, `.scratch/research/issues/15-law-forms-slow-books.md`

**What to build:** Red cell mass and albumin are stores that lose mass in proportion to their size, with production to match, so each returns to balance by itself. Red cell mass is the hemoglobin amount. Iron enters from the meal by absorption, not production, and leaves with the gut's shed cells through the fecal exit. Coagulation and immunity do nothing a healthy day can show. Each is declared with zero flux, its coupling arrows are held at zero, and it states the behaviors it cannot show (record 0002). This is pass 2.

**Releases:** the hematology card's remaining holds and the immune card's hold, each replaced by a store or by an explicit statement of what is absent.

- [ ] Tests: each store's loss and production match their forms at stated points
- [ ] Run: a store started 5% below balance moves toward balance, and each store ends a synthetic day within its drift bound
- [ ] The body model carries, for coagulation and for immunity, the list of behaviors it cannot show
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** a clotting model, an inflammation model, red cell age, and erythropoietin as a moving hormone.

From [research 15](../../research/issues/15-law-forms-slow-books.md): iron absorption in the gut reads the store in the liver bed's cells, a pool at another site, so it reads that store at a landmark [ticket 03](03-whole-body-lists.md) adds.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the fecal exit leaves at `(1 - f_i) / f_i` times a species' absorption, so iron, whose fecal exit carries the shed iron, declares its absorbed share `f_i`.

A finding of the plan review, for this ticket's run to settle: with that exit in a fixed proportion to absorption, the share of a meal's iron that is absorbed is `f_i` whatever the store holds, and the store's factor changes only how much iron waits in the lumen. [research 15](../../research/issues/15-law-forms-slow-books.md) has absorption fall as the store fills. If a run with a full store cannot show less iron absorbed, iron's exit needs a term that does not follow absorption, such as the onward flow first order in the lumen's contents that research 13 names as the competing form.

From [ticket 04](04-scopes-ports-and-census.md):

- `HematologyPorts::iron_into_cells` crosses iron into the cells of every capillary bed, wider than body 01 line 38, which gives HC the liver and gut beds. The types cannot name two tissues in one grant: a card crosses one wall with one species once, whatever the set, so a set of one bed would block the other. In practice the schema limits the reach by placing iron's pools in cells only at those beds. If this ticket needs the narrower grant, it asks for a site set over two tissues, a new list entry. The first ticket that places an iron pool in another bed's cells, or that needs another card to cross iron there, builds that site set in its PR. Today FL's iron at the muscle bed is refused as `CrossedTwice`, which is one such case.
- [Ticket 06](06-whole-body-on-holds.md)'s body holds HC's iron, so ticket 06 asserts only where iron's pools in cells stand. This ticket builds HC's iron law, so it asserts on the whole body's compiled columns that the crossing stands at the liver and gut beds and at no other bed.
