# 09: Tissues respire

Type: task
Status: open
Blocked by: 02, 08, `.scratch/research/issues/11-law-forms-gas-loop.md`

**What to build:** Each bed burns fuel with oxygen into carbon dioxide, water and heat, as a reaction at the bed ([ticket 02](02-reactions.md)). Rung 0 burns one mixed fuel at a fixed respiratory quotient until [ticket 16](16-fuel-stores.md) splits it. The heat joins the bed's heat and the water its water. Tissue carbon dioxide has a store, because blood alone settles too fast when breathing changes. When supply falls short of demand, an oxygen debt and lactate build, and the debt reaches the vessels as the signal for local flow. The work of the heart and the vessel walls, which the slice books against metabolism, is charged once.

**Releases:** the metabolism card's hold, and coupling arrows 78 (work and flow to metabolism) and 90 (oxygen debt to the vessels).

- [ ] Tests: the reaction's proportions close every book, and each rate law matches its form at stated points
- [ ] Run: over a window in which a bed's flow changes, oxygen in less oxygen out equals oxygen used plus the change in what the bed stores, and at a steady state that is flow times the difference between arterial and venous content. Carbon dioxide made over oxygen used equals the declared quotient. The energy book closes: heat equals the fuel's energy less work
- [ ] No work is paid from outside the body: the exterior chemical energy that pays the heart and the vessel walls in the slice is replaced by fuel burned at the bed
- [ ] Run: halving one bed's flow raises its extraction, then its debt, then its flow, in that order
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** which fuel a bed burns (ticket 16), temperature's effect on rate, and heat exchange with the room (ticket 12).
