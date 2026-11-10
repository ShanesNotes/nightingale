# 27: A step plan that can reach the speed aim

Type: prototype
Status: claimed
Claimed by: 27-step-plan
Blocked by: 07

**What to build:** A way of stepping the body that can reach 100 times real time on all 13 systems. [Ticket 07](07-what-limits-the-step.md) says what limits the step today. On its figures no faster core closes the gap alone: the record's own work caps the slice near 21 times real time, and every row the body adds pays for the heart's step count.

- [ ] Two or three plans prototyped and measured on the slice, each still passing the replay gates and the accuracy gate. Candidates:
  - A scheme of higher order. Today's scheme is first order: backward Euler, stepped twice at half length (`crates/ng-solver/src/lib.rs`). The seam admits any implicit Runge-Kutta scheme whose weights are nonnegative exact ratios ([solver contract](../../../docs/architecture/solver-contract.md), section 3). The scheme has to damp stiff rows: a trapezoidal stage fails on the valve rows, and two-stage Radau IIA, with weights 3/4 and 1/4, is one that [native-contract 28](../../native-contract/issues/28-admission-energy-at-balance.md) names for admission's cycle. A new scheme is one more entry in the contract's closed set, and its plan says what it asks of admission
  - Step control that survives a firing
  - A split where the heart's rows step apart from the slow rows (the split gate of native-contract 20 exists and no core uses it)
  - A cheaper record that keeps the same guarantees
- [ ] The same plans measured on [ticket 06](06-whole-body-on-holds.md)'s body once it exists
- [ ] A pick with its measured gain and what it costs, written as a decision record if it changes the contract. Resolution is part of a run's identity (foundation map), so a plan that changes results is a different run and says so. A pick that falls short of the aim says by how much and what would close the rest
- [ ] The pick is on main, so [ticket 24](24-the-synthetic-day.md)'s day runs on it. Build work the pick needs beyond this ticket's commits is filed as a ticket that ticket 24 waits on
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

This runs as pstack's Hillclimb playbook: one metric, one hypothesis at a time, a decision log, and one commit for each accepted gain.

## Comments

**What the rhythm's firing costs, read from ticket 07's trace.** The trace is `data/v3/out/trace-today.csv` in ticket 07's evidence folder (sha256 `91a5901070752994…`): 120 s on the heavy-run machine, one deterministic run. Twice a beat the rhythm fires, and the step after it asks for the span the core carried from before the firing, 10 ms both times. At the start of systole that step is refused four times, once at Newton's iteration limit and three times on the accuracy of the mitral valve's flow, and it passes at 0.625 ms. At the end of systole it is refused three times and passes at 1.25 ms. The two steps take 3.5% of the wall time.

For a plan the margin matters more than the cost. A step that passes at 0.625 ms has two refusals left: a third would retry at 0.078 ms, below the floor of 0.1 ms, which is the fault `NoAdmissibleStep`. A 48 h run stands at that point 172,800 times.

Two facts of the seam shape any step control that survives a firing. `CoupledSolver::next_step` takes the accepted view, which has no reading that says the head is a breakpoint, so a core commits its span before it can know that a guard fired. And `StepTicket::refuse` halves the span whatever the estimate was, so a step sixteen times too long takes four refused solves to find its length.

`NumericalHealth::most_refusals` and `NumericalHealth::shortest_retry_ns` report this margin for any run through the arena, so each prototype's run shows its own.
