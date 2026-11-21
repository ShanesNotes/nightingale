# 100: The body moves after admission: what moves, and why

From: [body ticket 17](../../body/issues/17-volume-hormones.md)'s plan, at main `89fa470`

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 17](../../body/issues/17-volume-hormones.md)

A fix that moves the body's pins lands after [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)'s second PR, which moves them first.

**Why.** After admission the body leaves salt balance within minutes, and three accounts of why have failed a check: an open loop in the kidney, the breath's slowly rising drive, and a lung bed that drains into the blood. This ticket finds what moves after admission and why, before any loop of body ticket 17 is built to answer it. Every value below is synthetic, and each rate is a difference of cumulative transfers or a mean over the span it names.

**What the runs and the code establish.**

- **The kidney's half.** Over 170, 1,500 and 2,750 s the glomerular bed's pressure holds at 35.0675, 35.0701 and 35.0692, the lumen's pressure rises from 2.09479 to 2.09614, against filtration, and the glomerular bed's albumin falls from 0.814658 to 0.814285 (-0.046%), for filtration. Over the 300 s spans 20 to 320 s and 2,548 to 2,846 s, differenced at their edges, filtered water rises 0.06844% and the duct's sodium outflow 1.503%, from 1.00038 a second to 1.01541: 22.0 times the rise in filtered water and 23.6 times the rise in filtered sodium. The duct reclaims near its capacity. These are the duct's outflows: the bladder drains at 1e-3 a second, so the urine lags them by about 1,000 s.
- **The lung bed's three routes,** in water a second, differenced at the breath edges nearest each span's ends:

| Span | Starling, blood to the pool | Lymph, out of the pool | The airway's lining, out of the pool | Net |
|---|---|---|---|---|
| 20.091 to 319.961 s | 0.00993458 | 0.00969373 | 0.00421405 | -0.00397319 |
| 2,547.837 to 2,845.972 s | 0.01042846 | 0.00672293 | 0.00421405 | -0.00050852 |

- From 10.037 to 2,897.173 s, an interval that leaves out both ends of the run, 29.623 of water crossed from the blood into the pool, 22.285 went back to the blood by lymph and 12.167 left through the lining. The three transfers reconcile with the pool's change over that interval, and they rule out a net drainage of water from this pool into the blood.
- **Admission and the breath.** Admission solves the heartbeat with the breath at rest (`crates/ng-validate/src/body/packet.rs`, the breath's start mode `rest`), and it evaluates the lining's transfer on every stage, so the code rules out an omitted lining. Both the main run and the run with the drive scale held begin tidal breathing after admission. At the admitted instant the pool holds 429.188, against the packet's guess of 430, with Starling 0.003788 and lymph 0.005568, which differ from their means under breathing above. So the run with the drive scale held rules out only the drive's slow rise, not the onset of tidal breathing.
- No log names an exit of admission's solve as the cause.

**Not established.** What lowers the glomerular bed's albumin, and how much of what moves after admission is the change from the admitted rest to tidal breathing. The lung pool's time constant of about 1,170 s and its level near 423.8 by about 6,000 s are projections from slopes, not observations.

- [x] The plasma's water and albumin are balanced route by route over the diagnostic run, and this ticket's Answer names what lowers the glomerular bed's albumin: the Answer's part 1
- [x] The admitted start is set beside the breathing body: every pool and flow whose mean over whole breaths in the first 300 s differs from its admitted value, listed with its owner. One run that holds the breath at rest after admission covers the same elapsed window as the breathing run, with the same averaging over heartbeats, since the admitted table is one instant of a beat, and it shows which of them the breath's onset moves: the Answer's list, with parts 2 and 3
- [x] From those two, what must change so that the admitted start is the breathing body's own, and who owns it: a packet value, a law, or what admission solves. If admission must solve the body over whole breaths or state something new, that is a design round of its own: this ticket says so and stops there. The Answer's table, with the fix
- [ ] A fix that is a packet value or a law has a suite test, with its window, its bound and its comparison stated from the second box's measures

**Non-goals:** body ticket 17's loops, and the bounds of the body's certificate ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)). The bounds stay ticket 85's, except that the fix's PR re-sizes, by that ticket's rule and on the suite's own reference run, each bound its change makes fail the rule.

## Answer

Every value is synthetic. Each rate is a difference of cumulative transfers at the breath edges nearest a span's ends, over the time between them. Each concentration or pressure is a mean over the breaths that end inside the span. Each statement is one of three kinds: established (a closed balance and a run), inferred (a balance and no run), or a projection (a fit beyond the data). A second read checked the investigation, and where the two differ this Answer takes the read's figures.

**The passes.** Three passes from one build of main `b4a40ef6` with the switches added, each from its own admission and each to 1,500 s, since a sealed packet resumes only under its own admission. The base pass has main's laws and tidal breathing. The switched pass is a prototype, not a law: the pulmonary lymph reads the pleural pressure's rest value, -3, in place of its instant, and nothing else changes. The held pass sets the hazard that starts the first breath to zero, so the breath stays at rest, where admission holds it. The held pass admits the base pass's 1,464 rows bit for bit, and the switched pass agrees with them to 7.1e-13 relative on every row above 1e-9. Part 1's balance is over the plan's diagnostic run of main to 2,900 s.

### Part 1. The plasma (established)

**What lowers the glomerular bed's albumin.** Rows rebuilt from the admitted rows and each column's transfers reproduce every conserved water total to 9e-11 and every albumin total to 1.3e-11 at edges 0, 1 and 613, so the balance closes. From 10.037 to 2,897.173 s the blood gains 3.916839 water and 1.196520 albumin. It gains water faster than albumin in proportion: from 20.091 to 319.961 s it gains 0.003877 water and 0.000879 albumin a second, where keeping its concentration of 0.715136 would need 0.002773 albumin. The gut's intake is fixed at 0.030000 a second, and the kidney's net water moves from -0.015810 to -0.015854 a second between the two spans below. The airway's lining takes 0.00421405 a second out of the lung's interstitial pool, and the pool and the blood share that supply:

| Span | From the blood (Starling less lymph) | From the pool | The blood's share |
|---|---|---|---|
| 20.091 to 319.961 s, a second | 0.000241 | 0.003973 | 5.7% |
| 2,547.837 to 2,845.972 s, a second | 0.003706 | 0.000509 | 87.9% |
| 10.037 to 2,897.173 s, in all | 7.338 (29.623 less 22.285) | 4.829 | 60.3% of 12.167 |

So early the lung's pool pays the lining, and the gut's water that the kidney does not excrete stays in the blood and dilutes its albumin. As the pool falls its pressure and its lymph fall, the blood takes over the lining's supply, and the albumin concentration's rate falls from -5.22e-7 to -1.19e-10 a second between the two spans. The glomerular bed's albumin follows the pooled plasma. The skin's water loss, which draws on the muscle bed's interstitium, is not fixed: it is the held loss times its forcing, 0.009682 a second over the first span and 0.011704 over the second.

**The trigger.** The pulmonary lymph's law reads the lung's tissue pressure, and at the lung's bed that pressure carries the pleural pressure. In the base pass, over the four whole breaths from 602.079 to 621.137 s (3,702 steps), the pleural pressure swings from -4.079 to -2.295 about a mean of -2.993, and the lung's tissue pressure from -3.498 to -1.713 about -2.412. Through the law's soft threshold the swing raises the lymph: the law's mean over the steps is 0.0085343 a second, against 0.0047210 at the mean pressure, 1.81 times. Between the first span and the third of the table below, the switch removes 91.1% of the size of the arterial albumin's drift (-0.00030199 in the base pass, +0.00002699 switched) and 85.4% of the duct's sodium outflow's (+0.010908 a second, against -0.001589). Both residuals reverse sign, so the switch corrects each drift by 108.9% and 114.6% of its signed size.

**The breath held.** The switched pass and the held pass agree closely on the lung's bed and the plasma, and less closely on the duct's sodium and on the pool's small rate:

| Pass, span | Starling | Lymph | Pool | Arterial albumin | Glomerular albumin | Filtered sodium | Duct sodium out |
|---|---|---|---|---|---|---|---|
| Base, 20.091 to 319.961 s | 0.00993458 | 0.00969373 | -0.00397319 | 0.71399844 | 0.81465660 | 199.312844 | 1.000379 |
| Switched, 20.091 to 319.788 s | 0.00983969 | 0.00557274 | +0.00005290 | 0.71409866 | 0.81476662 | 199.297128 | 0.999017 |
| Held, 20.095 to 319.965 s | 0.00983505 | 0.00557208 | +0.00004892 | 0.71410095 | 0.81473672 | 199.258932 | 0.997742 |
| Base, 602.079 to 901.376 s | 0.01016529 | 0.00828067 | -0.00232942 | 0.71378032 | 0.81442224 | 199.354864 | 1.007158 |
| Switched, 601.673 to 900.693 s | 0.00983733 | 0.00558470 | +0.00003858 | 0.71410078 | 0.81477149 | 199.276302 | 0.998396 |
| Held, 602.086 to 901.379 s | 0.00982760 | 0.00558332 | +0.00003023 | 0.71410188 | 0.81473956 | 199.254042 | 0.996130 |
| Base, 1,199.077 to 1,495.538 s | 0.01029631 | 0.00750060 | -0.00141834 | 0.71369645 | 0.81432863 | 199.398550 | 1.011287 |
| Switched, 1,198.105 to 1,498.975 s | 0.00982478 | 0.00559262 | +0.00001811 | 0.71412565 | 0.81479789 | 199.255015 | 0.997428 |
| Held, 1,199.083 to 1,495.541 s | 0.00982700 | 0.00559087 | +0.00002209 | 0.71412273 | 0.81475676 | 199.257676 | 0.995176 |

Rates are water or sodium a second, and the lining takes 0.00421405 a second in every row. By window, switched against held: Starling within 0.047%, 0.099% and 0.023%, lymph within 0.012%, 0.025% and 0.031%, glomerular albumin within 0.005%, filtered sodium within 0.019%, 0.011% and 0.001%, the duct's sodium outflow within 0.128%, 0.227% and 0.226%, and the pool's rate by 8.1%, 27.6% and 18.0% of the held pass's.

### The fix, in a PR of its own

Ruled, a model default: the lymph law reads the tissue pressure over the pressure of the enclosure around its bed, against the edge's threshold. The tissue and its lymph vessels sit inside the same enclosure, the law has no cited source, and the Starling law already reads a difference. Each enclosed bed's threshold is restated over its enclosure's design pressure, so the admitted rows stay within roundoff. That PR ticks the fourth box. Three forms were not chosen:

- A memory of the outside pressure: a new state and a time constant for the same result.
- A flatter law by packet values: it leaves part of the gain (a width of 2 and a conductance of 0.006497 keep the resting lymph and cut the gain from 1.808 to 1.059 on the base pass's saved steps), and it bends the law at rest.
- Admission over whole breaths: a design round that nothing measured needs.

### Part 2. The gases

**What both balances show.** Two balances of the same tables, made apart, agree on every route to its printed digit and on every store within 1e-5 a second. With the breath held the gases are steady: over 5.006 to 300.765 s the body's carbon dioxide changes by -0.0066 a second against a production of 71.999, and its oxygen by +0.0025 against a use of 95.999, and over 1,199.083 to 1,495.541 s by -0.0086 and +0.0008. The arterial carbon dioxide pressure is 29.833 over the first span and 29.834 over the second. Under tidal breathing the body clears less carbon dioxide than it makes, and the tissues hold what is retained:

| Tidal pass, a second | 5.006 to 300.761 s | 1,199.077 to 1,495.538 s |
|---|---|---|
| Carbon dioxide made | 72.005 | 72.014 |
| Exhaled, net (expired less inspired) | 69.105 (70.000 less 0.895) | 70.929 (71.841 less 0.913) |
| Retained | 2.900 | 1.086 |
| Held by the tissues | 2.051 | 0.940 |
| Held by the blood | 0.732 | 0.136 |
| Held by the alveolar gas and the airway | 0.117 | 0.010 |

By 1,495.538 s the base pass holds 2,676 more carbon dioxide than it was admitted with, and the diagnostic run 3,551 more by 2,897.173 s.

**Inferred.** Why tidal breathing at the admitted drive exchanges less than the rest's mixing is inferred in part and not measured. The packet fits the breath's pump by a static form to the rest's mixing exchange, and how much of the shortfall comes from that fit, from the dead space's mixing and from the smoothing of the airway's pressure heads is not measured.

**A projection.** A fit of the form `a - b exp(-t / tau)` from 300 s, weighted by each breath's length, gives a plateau of the arterial carbon dioxide pressure of 30.42, with tau 1,231 s on the diagnostic run to 2,897 s and 1,247 s on the base pass to 1,495 s. That is 0.59 above the held pass's 29.833. The same fit of the body's carbon dioxide gives about 4,000 retained at the plateau. No fix is scheduled: the projected shift is 0.59 and the retained amount is under 0.07% of a day's production, 6.22e6 at 72.0 a second. The run that would settle the split is not run: one awake admission at a breath rate of 0.20 a second, the drive held at one for the first minute and then released for at least 10,000 s, recording each airway interface's flow times its gas fraction in each direction, the breath's depth, the alveolar exchange and each store's slope.

### Part 3. The duct (inferred)

With the breath held the duct's sodium outflow still falls, from 0.997742 to 0.995176 a second between the first span and the third (-0.257%), and its sodium concentration from 63.21485 to 63.06425. Over 20.095 to 319.965 s the sodium delivered to the duct, 12.259768 a second, less its reclaim, 11.265056, less its outflow, 0.997742, is -0.00303: the duct holds excess sodium at admission and gives it up. Its candidate source is the held skin water loss's forcing, which admission holds at its mean, and which runs at 0.012336 a second over 602.086 to 901.379 s in the held pass. That source is untested.

### The second box's list

Over edges 0 to 61 (5.006 to 300.761 s, and 5.006 to 300.765 s held), 378 rows and 189 columns, 567 items, have a mean under tidal breathing that differs from the held pass's by more than its scatter. The scatter is the larger of the two passes' residual standard deviations of per-span values about a fitted straight line. Single owners: metabolism 170, fluids 132, blood chemistry 48, renal 29, nervous 25, hepatic 25, respiratory 19, receptor 18, cardiovascular 11, the boundary 6, and musculoskeletal and skin, hematology and endocrine 1 each. Two or three cards share the other 81. The largest are the breath's own phase and mixing share (nervous, by design), the pulmonary lymph's water and each species it carries (fluids, +75.1%), the airway's inspired and expired gases (respiratory), and the beds' ion exchange (fluids).

The screen removes a fitted slope, so it cannot show that an item has settled. Items whose slope persists beyond the window, under tidal breathing:

- The lung's interstitial pool: -0.00397319, -0.00232942 and -0.00141834 a second over the three spans of part 1.
- The drive scale: 1.03952, 1.04258 and 1.06250 as means of the breaths that end in 200 to 300 s, 300 to 400 s and 1,199 to 1,496 s, where the held pass gives 0.99933, 0.99927 and 0.99944.
- The bladder's sodium: 999.017, 999.158 and 1,004.332 over the same breaths, where the held pass gives 998.579, 998.318 and 996.136.
- The carbon dioxide the body holds, by part 2.

### The third box's table

| What must change | Kind | Owner | When |
|---|---|---|---|
| The lymph's form: the tissue pressure over its enclosure's | A law | The fluids card | Ruled, its own PR, the fourth box |
| The tidal breath's exchange at the admitted drive | A law or a packet value | [Body ticket 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md), by a new box | Not scheduled |
| The skin water loss's forcing | A hold | [Body ticket 12](../../body/issues/12-heat-balance.md) releases that hold | With ticket 12 |
| Admission over whole breaths | What admission solves | Native-contract | Not scheduled |

Admission over whole breaths is not scheduled, since nothing measured needs it once the lymph reads over its enclosure. It reopens on a pool or flow whose mean under breathing stands off its admitted value with no fix by a law's form or a packet value. If it reopens, it is a design round of its own.

This ticket stays open for its fourth box, and it still blocks body ticket 17.
