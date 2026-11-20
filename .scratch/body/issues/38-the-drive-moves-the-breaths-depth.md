# 38: The drive moves the breath's depth as well as its rate

Type: task
Status: open
Blocked by: 11, ../../research/issues/23-how-the-drive-splits-into-rate-and-depth.md, ../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md

**Why.** The developer reviewer ruled the standard: the body's breathing control moves the breath's depth and its rate together. So rate alone is a step. [Ticket 11](11-breathing-is-controlled.md) closes the gas loop on the breath's rate. Ruled at planning, a model default open to the developer reviewer: depth lands before [ticket 28](28-two-synthetic-days.md)'s 48-hour run, through [ticket 24](24-the-synthetic-day.md), which waits on this ticket.

**What to build:** The nervous card's drive scale moves the breath's depth as well as its rate, in the split [research 23](../../research/issues/23-how-the-drive-splits-into-rate-and-depth.md) cites, on the whole body. Ticket 11's parts stay as they are: 11b is the rate and 11c is sleep. [Native-contract 87](../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md)'s form, a firing at each breath edge that sets the pump drive's peak from the drive scale so the breath's depth follows, is the first candidate for the build. Ticket 11's plan saw the drive scale on depth move the admitted rest, and both valve closings with it: if the build moves the rest, [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) is the fix.

The boxes, each with what shows it:

- [ ] Tests: depth and rate each follow the drive in the split research 23 cites, on a fixture, at stated points of the drive scale
- [ ] Run: on the fixture, a 10% rise in carbon dioxide production moves carbon dioxide's pressure by what the built loop's gain predicts, with depth and rate sharing the drive. The closed form comes from the forms, as 11b's does, and the PR prints 11b's figure beside it. No box here asks for a smaller change than rate alone gives: ticket 11 raised rate's gain to give the same loop, and the size of the whole loop is research 22's
- [ ] Every link of the loop moves its reader the right way, each judged where ticket 11 judges it: on the body against the held run, and the central arm's sign on the fixture. Depth and rate are two links here, and each moves with the drive. On the body both runs are read over one span of time, each reading a mean over the whole breaths that end inside it, and the PR shows that two neighbouring spans do not flip a sign
- [ ] A body with both gains zero keeps the drive scale at its rest's bits, so neither its rate nor its depth answers the gases. It has its own admission, as in ticket 11, and it is the held run the link box judges against
- [ ] The certificate passes on the body under [native-contract 85](../../native-contract/issues/85-the-certificates-windows-end-on-breath-edges.md)'s windows between breath edges, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and user instructions a simulated second against [ticket 26](26-cost-budget.md)'s budget, on the body's bench at the PR's base and its head. Ticket 11's plan measured depth on a continuous path as adding 238.2 M instructions a simulated second against rate's 41.2 M, as native-contract 87 quotes it, because the path from the gases to the pump's pressure joins the solver's largest dense block. Native-contract 87's form is the first candidate for paying less. If the prototype cannot bring depth inside the budget, the budget's owner records an exception with its reason in this ticket, under ticket 26's rule. Depth is not dropped
- [ ] One body run of an hour, outside the suite, as evidence in the PR, with each breathing conserved quantity's amount kept at every breath edge as 11b's hour run keeps it. The PR says what the run shows: carbon dioxide's rise in each ten minutes, whether it has settled by the hour, and whether the drive scale swings
- [ ] The pins that move, and why for each: the model hash, the packet, the admitted seal, the first second and the first minute, as each one moves

**Non-goals:** sleep, which is 11c; the gains' sourced values, which are [research 22](../../research/issues/22-the-chemoreflex-gains-awake-and-asleep.md)'s; and periodic breathing.

## Parameters with no source

Every value is synthetic. The split between depth and rate, and what bounds depth as the drive rises, are research 23's. The gains, and the cited steady change in arterial carbon dioxide that sizes the whole loop, are research 22's. This ticket judges nothing against that figure, and binding the gains waits on research 22.
