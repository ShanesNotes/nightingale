# 67: The march's settle test reads no size

Spec: `crates/ng-validate/src/initialize.rs` (`march`)

Type: task

Status: open

Blocked by: none

**Why.** Every body test pays a body admission, so admission's time bounds the proofs that carry the loops to the first milestone (SRS invariant I19). [Body 08](../../body/issues/08-blood-carries-gases.md)'s lean prototype admitted in 525 to 574 s against main's 190 to 202 s, and its plan found why (`~/lanes/logs/ng-wave-4/b08-plan/PLAN.md`, section 8).

**What the test asks.** A leg's Newton stops when no unknown moved by more than 8 machine epsilons of itself (`NEWTON_STOP`), with no size term. A row that subtracts two near-equal numbers keeps rounding noise above that, so its leg runs all 32 updates and never settles.

**The rows that cannot meet it, on the prototype.**

- The lung's `sum_at` rows, `DL (P_A - P)`, with heads of 5 and 0.5 mmHg.
- The blood partial pressures `PCO2 = (C_T - HCO3) / alpha`, where dissolved carbon dioxide is about 1.2 of 26 mM, and `PO2 = (C - 4 c_Hb S) / alpha`, where dissolved oxygen is 1 to 3% of the oxygen content.

| Admission | Load | Wall | March | Legs at the 32-update limit | Updates a leg |
|---|---|---|---|---|---|
| main | 2.7 to 2.9 | 201.8 s | 29.4 s | 247 of 4,737 | 12.3 |
| lean | 2.5 to 4.3 | 574.3 s | 315.2 s | 4,722 of 4,732 | 32.0 |
| lean, with both of body 08's fixes | 3.9 to 4.8 | 259.3 s | 53.2 s | 348 | 13.3 |

Main already runs 247 legs to the limit, on rows of the circulation, the liver's sinusoids and the lymph. Body 08's two fixes live in its own laws and leave the test as it is, so any later row with cancellation meets it again.

- [ ] A fixture row with cancellation runs every leg to the limit today
- [ ] The test reads each slot's natural size, as the ring's test does, picked by a prototype, with the slice's and the body's admissions timed before and after on one named build worker
- [ ] Every pin of the slice and the body that moves has its sentence. `initialize.rs` is shared, and a row with no stated size is one unit, so the plan infers the slice's admitted bits and seals move (not run)
- [ ] Oxygen's partial pressure near full saturation admits without a slow march. There it is ill-conditioned in the oxygen content, about 18 times at a saturation of 0.98 (the oversight seat's arithmetic on the Hill form, not a run)
