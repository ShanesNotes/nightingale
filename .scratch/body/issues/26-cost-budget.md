# 26: A cost budget for the whole body

Type: task
Status: open
Blocked by: 06, 07

**What to build:** The budget each loop ticket is held to. Measure [ticket 06](06-whole-body-on-holds.md)'s body against the slice on the heavy-run machine, and from [ticket 07](07-what-limits-the-step.md)'s limiter set how much wall time for a simulated second each loop may add, and a ceiling for the whole body after pass 1.

- [ ] The body on holds measured against the slice under one repeat protocol: unknowns, rows, the cost of a step by part, and wall time for each simulated second
- [ ] A budget for each loop and a ceiling for the synthetic day, in the map's order table, with the run that set them
- [ ] One command that prints a ticket's cost line the same way every time
- [ ] The rule the map states: a ticket over its budget does not merge until the budget's owner records an exception with its reason

**Non-goals:** reaching the speed aim. That is [ticket 27](27-step-plan.md).
