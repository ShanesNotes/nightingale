# 32: The slice's outflow valves shut and open again inside one ejection

Spec: `crates/ng-cardiovascular/src/valves.rs` (`valves`), and the valve law of [research 02](../../research/issues/02-law-forms.md)

Type: research

Status: open

Blocked by: none

**Question:** Why do the slice's aortic and pulmonary valves shut and open again before ejection ends, and should they?

[Body ticket 07](../../body/issues/07-what-limits-the-step.md) traced 119 whole beats of the slice. In each one the aortic valve opens at phase 0.01, shuts at 0.10, opens at 0.12, shuts at 0.18, opens at 0.18 and shuts at 0.30. The pulmonary valve opens at 0.00, shuts at 0.16, opens at 0.17 and shuts at 0.31. The slice's numbers are synthetic, so this may come from the numbers and not from the law. It also costs time. The openings and closings past each valve's first pair are 6 of the 15 landings a beat, about 10 ms of each beat's 122 ms of wall time on the heavy-run machine.

- [ ] How often a healthy outflow valve opens and shuts in one beat, with its source
- [ ] The head across each outflow valve and its flow, traced through one beat on main, with each firing marked. The trace shows what brings the head and the flow to zero at the two early shuts
- [ ] The cause, named as one of four: the law's form (a flow with inertance between two compliances can ring), the synthetic parameters (the ring's period against the length of ejection), the guards (the opening head and the reading), or the landing. One change to one input moves the count of firings as the cause predicts
- [ ] Whether the reference man's sourced parameters would do the same, from the candidate fills on hand for the valve's inertance and orifice and the chambers' compliances. If no fill is on hand, the answer names the parameter that decides it
- [ ] A recommendation and what it changes: leave it, change the synthetic numbers, or change the guard or the form. A change of form meets research 02's standard and names its source
- [ ] What the recommendation saves, measured in landings a beat and in wall time for each simulated second, with the machine, the run count and the spread

**Non-goals:** changing the law or its numbers in this ticket. Tuning a number until the count looks right with no cause named. The step plan of [body ticket 27](../../body/issues/27-step-plan.md), which takes this answer as one of its inputs.

## Comments

From the solver explorations, read from [body ticket 07](../../body/issues/07-what-limits-the-step.md)'s traces of the slice (`~/lanes/logs/ng-body-wave-1/07-step-limit`; the counts are in `~/lanes/logs/ng-explore-solver/valve-counts.txt` and `valve-shuts.txt`): the aortic valve shuts three times a beat and the pulmonary twice in all eight variants of the halved step, with caps from 5 to 40 ms and aims from 0.35 to 0.9. The two aortic shuts inside ejection sit 77 to 78 ms apart in every variant, a ring near 12.8 Hz, and the second shut lasts 4.0 to 7.7 ms. A period that holds while the cap moves eightfold points the third box at the law's form or the synthetic parameters, not at the guards or the landing (inferred: no input was changed). One caution for the second box's trace: the halved step takes about `w^2 h / 4` a second off an oscillation ([native-contract 91](91-the-accuracy-gate-holds-no-oscillation.md)), which is 16 a second at 12.8 Hz and a 10 ms step. The ring a trace shows is what is left after that, and a plan that damps less would show more of it.
