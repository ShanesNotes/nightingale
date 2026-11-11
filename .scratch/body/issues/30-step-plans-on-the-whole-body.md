# 30: The step plans measured on the whole body

Type: prototype
Status: open
Blocked by: 06, 27

**What to build:** The second box of [ticket 27](27-step-plan.md). Ticket 27 measured its plans on the slice only, because [ticket 06](06-whole-body-on-holds.md)'s body did not exist yet. Measure the same plans on ticket 06's body by ticket 27's protocol, and say whether ticket 27's pick still holds there.

- [ ] Each plan run on the body: the doubled step (the reference integrator still declares it), the halved step (ticket 27's pick, on `main`), and the two plans ticket 27 set aside on the slice. Those are two-stage Radau IIA (branch `claude/body-27-radau`) and the extrapolated step (branch `claude/body-27-lean`). Each run reports its steps, its wall time for each simulated second, `NumericalHealth::most_refusals` and `shortest_retry_ns`, and passes the replay gates and the accuracy gate
- [ ] Why each plan's steps fall where they do: steps a beat by reason (accuracy, cap, landing, retry, growth, after a breakpoint) and solves a step, as ticket 27's Answer reads them on the slice
- [ ] Whether the halved step's rule holds on every firing the body adds: a ticket after a breakpoint takes the doubled stages
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.
