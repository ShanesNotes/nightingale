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
