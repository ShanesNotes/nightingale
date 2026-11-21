# 100: The body moves after admission: what moves, and why

From: [body ticket 17](../../body/issues/17-volume-hormones.md)'s plan, at main `89fa470`

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 17](../../body/issues/17-volume-hormones.md)

A fix that moves the body's pins lands after [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)'s second PR, which moves them first.

**Why.** After admission the body leaves salt balance within minutes, and three accounts of why have failed a check: an open loop in the kidney, the breath's slowly rising drive, and a lung bed that drains into the blood. This ticket finds what moves after admission and why, before any loop of body ticket 17 is built to answer it. Every value below is synthetic, and each rate is a difference of cumulative transfers or a mean over the span it names.

**What the runs and the code establish.**

- **The kidney's half.** Over 170, 1,500 and 2,750 s the glomerular bed's pressure holds at 35.0675, 35.0701 and 35.0692, the lumen's pressure rises from 2.09479 to 2.09614, against filtration, and the glomerular bed's albumin falls from 0.814658 to 0.814285 (-0.046%), for filtration. Filtration rises 0.047%, and filtered sodium 0.064% over the 300 s spans 20 to 320 s and 2,548 to 2,846 s. Over those spans the duct's sodium outflow rises 1.50%, from 1.00038 a second to 1.01541: about 23 times the rise in filtered sodium, and about 32 times the rise in filtration. The duct reclaims near its capacity. These are the duct's outflows: the bladder drains at 1e-3 a second, so the urine lags them by about 1,000 s.
- **The lung bed's three routes,** in water a second, differenced at the breath edges nearest each span's ends:

| Span | Starling, blood to the pool | Lymph, out of the pool | The airway's lining, out of the pool | Net |
|---|---|---|---|---|
| 20.091 to 319.961 s | 0.00993458 | 0.00969373 | 0.00421405 | -0.00397319 |
| 2,547.837 to 2,845.972 s | 0.01042846 | 0.00672293 | 0.00421405 | -0.00050852 |

- Over the whole run, 10.037 to 2,897.173 s, 29.623 of water crossed from the blood into the pool, 22.285 went back to the blood by lymph and 12.167 left through the lining. So the blood gave the lung bed a net 7.338 while the pool lost 4.828, and this pool does not dilute the plasma. Cell water's route does not reach this pool.
- **Admission and the breath.** Admission solves the heartbeat with the breath at rest (`crates/ng-validate/src/body/packet.rs`, the breath's start mode `rest`), and it evaluates the lining's transfer on every stage, so the code rules out an omitted lining. Both the main run and the run with the drive scale held begin tidal breathing after admission. At the admitted instant the pool holds 429.188, against the packet's guess of 430, with Starling 0.003788 and lymph 0.005568, which differ from their means under breathing above. So the run with the drive scale held rules out only the drive's slow rise, not the onset of tidal breathing.
- No log names an exit of admission's solve as the cause.

**Not established.** What lowers the glomerular bed's albumin, and how much of what moves after admission is the change from the admitted rest to tidal breathing. The lung pool's time constant of about 1,170 s and its level near 423.8 by about 6,000 s are projections from slopes, not observations.

- [ ] The plasma's water and albumin are balanced route by route over the diagnostic run, and this ticket's Answer names what lowers the glomerular bed's albumin
- [ ] The admitted start is set beside the breathing body: every pool and flow whose mean over whole breaths in the first 300 s differs from its admitted value, listed with its owner, and one run that holds the breath at rest after admission shows which of them the breath's onset moves
- [ ] From those two, what must change so that the admitted start is the breathing body's own, and who owns it: a packet value, a law, or what admission solves. If admission must solve the breath's cycle or state something new, that is a design round of its own: this ticket says so and stops there
- [ ] A fix that is a packet value or a law has a suite test, with its window, its bound and its comparison stated from the second box's measures

**Non-goals:** body ticket 17's loops, and the bounds of the body's certificate ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)).
