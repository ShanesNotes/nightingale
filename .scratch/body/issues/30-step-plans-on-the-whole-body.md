# 30: The step plans measured on the whole body

Type: prototype
Status: open
Blocked by: 06, 27

**What to build:** The second box of [ticket 27](27-step-plan.md). Ticket 27 measured its plans on the slice only, because [ticket 06](06-whole-body-on-holds.md)'s body did not exist yet. Measure the same plans on ticket 06's body by ticket 27's protocol, and say whether ticket 27's pick still holds there.

- [ ] Each plan run on the body: the doubled step (the reference integrator still declares it), the halved step (ticket 27's pick, on `main`), and the two plans ticket 27 set aside on the slice. Those are two-stage Radau IIA (branch `claude/body-27-radau`) and the extrapolated step (branch `claude/body-27-lean`). Each run reports its steps, its wall time for each simulated second, `NumericalHealth::most_refusals` and `shortest_retry_ns`, and passes the replay gates and the accuracy gate
- [ ] Why each plan's steps fall where they do: steps a beat by reason (accuracy, cap, landing, retry, growth, after a breakpoint) and solves a step, as ticket 27's Answer reads them on the slice
- [ ] The cost of the body's step by part, as [ticket 07](07-what-limits-the-step.md)'s Answer splits the slice's: the core against the record, and a step's fixed cost. [Ticket 26](26-cost-budget.md)'s first box left it here
- [ ] Whether the halved step's rule holds on every firing the body adds: a ticket after a breakpoint takes the doubled stages
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)
- [ ] Each plan's long-run salt rates on the body, which no box above reads: the body's total sodium, chloride and potassium as rates over the five windows of [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md)'s part 5, awake and with the breath held, at the plan's own stepping and at one finer, and whether main's certificate passes each to 700 s. For the halved step, the step aim and the floor are varied apart. Native-contract 100's part 5 has the halved step's figures at three steppings moved together.
  - Native-contract 100's part 5 also has the halved step with the aim and the floor moved apart, from one run for each setting of a build of main `e5a05b96` (measured). Over window 5, in 1e-3 a second, sodium awake is +1.52360 at main's stepping, +1.48156 with the floor alone at 25 µs, -0.01298 with the aim alone at a tenth and -0.05102 with both; held, +2.77201, +2.71672, +1.20987 and +1.17288. Chloride and potassium have their own figures there. Main's certificate to 700 s certifies the awake body with the aim alone and with both, and refuses by `Drift` on sodium the awake body with the floor alone and the held body with the aim alone, the floor alone and both. The tap counts no steps on the shortest span. *Inferred:* on this body and at these two sizes of change, the aim moves the salts' rates far more than the floor does. Not claimed: a settled rate, a limit or its sign, or anything for another plan.

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

## Comments

From the solver explorations: the plans this ticket compares differ in something none of its boxes reads. A backward Euler step damps an oscillation, the halved step by about `w^2 h / 4` a second and a one-stage step by twice that, with no refusal and inside every gate ([native-contract 91](../../native-contract/issues/91-the-accuracy-gate-holds-no-oscillation.md) has the measurement). [Ticket 27](27-step-plan.md) counted four fewer valve mode changes a beat under the one-stage plan, which is what more damping would do (inferred). So a plan can cost less because it damps more. Each plan's run here could also report, against the same plan at half its cap: the valve mode changes a beat, the pulse pressure, and the largest change in any route's forward or reverse exchange over a beat. Two-stage Radau IIA is third order and damps far less at the same step, so its place in the comparison is not only its cost.
