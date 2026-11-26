# 67: The march's settle test reads no size

Spec: `crates/ng-validate/src/initialize.rs` (`march`)

Type: task

Status: resolved

Claimed by: lu-order

Lands with: `.scratch/native-contract/issues/89-the-eliminations-order-inside-its-largest-block.md`

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

- [x] A fixture row with cancellation runs every leg to the limit today
- [x] The test reads each slot's natural size, as the ring's test does, picked by a prototype, with the slice's and the body's admissions timed before and after on one named build worker
- [x] Every pin of the slice and the body that moves has its sentence. `initialize.rs` is shared, and a row with no stated size is one unit, so the plan infers the slice's admitted bits and seals move (not run)
- [x] Oxygen's partial pressure near full saturation admits without a slow march. There it is ill-conditioned in the oxygen content, about 18 times at a saturation of 0.98 (the oversight seat's arithmetic on the Hill form, not a run)

## Answer

**What changed.** In `crates/ng-validate/src/initialize.rs::solve_leg`, the `settled` test now reads each slot's natural size beside the unknown: `change.abs() <= NEWTON_STOP * (system.size(workspace, slot) + unknown.abs())`, matching `moved` and the ring's `initialize` loop and failing closed on `NaN`.

**Cancellation fixture.** `initialize::tests::march_settles_a_cancellation_row_against_its_natural_size` solves `(1 + x)^2 - (1 + 1001 * f64::EPSILON) = 0` on a periodic leg where the exact root sits strictly between representable squares at `x ~ 500.5 * eps`. Without the slot's natural size, the rounding floor `0.5 * eps` stays above `NEWTON_STOP * |x|` and `solve_leg` fails to settle (`settled = false`); with `system.size(workspace, slot)`, it settles cleanly within 4 updates.

**Isolated measurements on `beelink` against `9f5ba6ce`.**

| Assembly | Variant | Legs | Settled | Rounding-cycled | Hit 32-update cap | March updates (avg/leg) | March factors | Ring sweeps / updates | Admission wall / total `instructions:u` |
|---|---|---|---|---|---|---|---|---|---|
| Slice | Base (`9f5ba6ce`) | 5,898 | 3,785 | 1,819 | 294 | 70,934 (12.03) | 8,827 | 6 / 11 | 7.7 s (serialized, load 4.6) |
| Slice | NC 67 (`b2bcb05b`) | 5,898 | 5,431 | 439 | 28 (-90.5%) | 52,491 (8.90, -26.0%) | 8,446 | 6 / 11 | 6.8 s (-11.7%, load 6.2) |
| Body | Base (`9f5ba6ce`) | 7,384 | 1,658 | 4,987 | 739 | 123,585 (16.74) | 26,233 | 13 / 40 | 3,530.27 B ins (`cost body 1 1`) |
| Body | NC 67 (`b2bcb05b`) | 7,384 | 4,817 | 2,280 | 287 (-61.2%) | 89,904 (12.18, -27.3%) | 22,890 (-12.7%) | 13 / 40 | 3,305.48 B ins (-224.8 B, -6.37%) |

The whole body (including arterial and pulmonary capillary oxygen near full saturation) admits with 27.3% fewer march updates, 61.2% fewer capped legs and identical ring sweeps and updates (13 sweeps, 40 ring updates).

**Pins moved.** As predicted, `model_hash`, `conserved_basis`, and all 30 printed lines (240 six-decimal values) of `crates/ng-validate/tests/trace.rs` stay unchanged; only the admitted packet digests and run seals move at rounding scale inside the admission tolerance, re-pinned once alongside [native-contract 89](89-the-eliminations-order-inside-its-largest-block.md).
