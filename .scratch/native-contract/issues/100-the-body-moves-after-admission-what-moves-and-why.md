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

**Not established when this ticket was written.** What lowers the glomerular bed's albumin, which the Answer's part 1 has since established; how much of what moves after admission is the change from the admitted rest to tidal breathing, which the Answer's list counts against the breath held; and which pools and flows move with the breath held against the admitted beat, which the Answer's part 4 has since measured. Still not established: what leaves part 4's residual at the tightest step aim. The lung pool's time constant of about 1,170 s and its level near 423.8 by about 6,000 s are projections from slopes, not observations.

- [x] The plasma's water and albumin are balanced route by route over the diagnostic run, and this ticket's Answer names what lowers the glomerular bed's albumin: the Answer's part 1
- [x] The admitted start is set beside the breathing body, as the investigation compared them: the tidal pass against a pass that holds the breath at rest after admission, over the same elapsed window in the first 300 s, each item's mean compared, and the items that differ counted by the card whose laws move each, with the largest named: the Answer's list, with parts 2 and 3
- [x] The held pass against the admitted beat, at the same phase of the heartbeat: every pool and flow that does not return to its admitted value over whole heartbeats with the breath held, beyond the tolerance of admission's recheck, listed with the law that moves it, and what must change for each: the Answer's part 4
- [x] From those two, what must change so that the admitted start is the breathing body's own, and who owns it: a packet value, a law, or what admission solves. If admission must solve the body over whole breaths or state something new, that is a design round of its own: this ticket says so and stops there. The Answer's table, with the fix, for what the first two boxes found. The box on the admitted beat adds to that table if it finds more
- [ ] A fix that is a packet value or a law has a suite test, with its window, its bound and its comparison stated from the second box's measures

**Non-goals:** body ticket 17's loops, and the bounds of the body's certificate ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)). The bounds stay ticket 85's, except that the fix's PR re-sizes, by that ticket's rule and on the suite's own reference run, each bound its change makes fail the rule.

## Answer

Every value is synthetic. Each rate is a difference of cumulative transfers at the breath edges nearest a span's ends, over the time between them. A law's rate is a time-weighted mean over the solver's steps. Each albumin concentration is the ratio of two span-weighted means of amounts, albumin over water. Each other concentration or pressure is a mean over the breaths that end inside the span. The held pass has no breaths, so its spans end at clock marks at or after the base pass's edge times. Each statement is one of three kinds: established (a closed balance and a run), inferred (a balance and no run), or a projection (a fit beyond the data). A second read checked the investigation, and where the two differ this Answer takes the read's figures.

**The passes.** Three passes from one build of main `b4a40ef6` with the switches added, each from its own admission and each to 1,500 s, since a sealed packet resumes only under its own admission. The base pass has main's laws and tidal breathing. The switched pass is a prototype, not a law: the pulmonary lymph reads the pleural pressure's rest value, -3, in place of its instant, and nothing else changes. The held pass sets the hazard that starts the first breath to zero, so the breath stays at rest, where admission holds it. The held pass admits the base pass's 1,464 rows bit for bit, and on every row above 1e-9 the switched pass's admitted rows differ from them by at most 7.14e-13 relative (7.1328e-13 at row 774). Part 1's balance is over the plan's diagnostic run of main to 2,900 s.

### Part 1. The plasma (established)

**What lowers the glomerular bed's albumin.** Rows rebuilt from the admitted rows and each column's transfers reproduce the summed water total and the summed albumin total at edges 0, 1 and 613, so the balance closes: at edge 613 the residuals are 9.1e-11 for water and 1.3e-11 for albumin. From 10.037 to 2,897.173 s the blood gains 3.916839 water and 1.196520 albumin. It gains water faster than albumin in proportion: from 20.091 to 319.961 s it gains 0.003877 water and 0.000879 albumin a second, where keeping its concentration of 0.715136 would need 0.002772 albumin. The gut's intake is fixed at 0.030000 a second, and the kidney's net water moves from -0.015810 to -0.015854 a second between the two spans below. The airway's lining takes 0.00421405 a second out of the lung's interstitial pool, and the pool and the blood share that supply:

| Span | From the blood (Starling less lymph) | From the pool | The blood's share |
|---|---|---|---|
| 20.091 to 319.961 s, a second | 0.000241 | 0.003973 | 5.7% |
| 2,547.837 to 2,845.972 s, a second | 0.003706 | 0.000509 | 87.9% |
| 10.037 to 2,897.173 s, in all | 7.338 (29.623 less 22.285) | 4.828 | 60.3% of 12.167 |

So early the lung's pool pays the lining, and the gut's water that the kidney does not excrete stays in the blood and dilutes its albumin. As the pool falls its pressure and its lymph fall, the blood takes over the lining's supply, and the albumin concentration's rate shrinks toward zero, from -5.22e-7 to -1.19e-10 a second, between the two spans. The glomerular bed's albumin follows the pooled plasma. The skin's water loss, which draws on the muscle bed's interstitium, is not fixed: it is the held loss times its forcing, 0.009682 a second over the first span and 0.011704 over the second.

**The trigger.** The pulmonary lymph's law reads the lung's tissue pressure, and at the lung's bed that pressure carries the pleural pressure. In the base pass, over the four whole breaths from 602.079 to 621.137 s (3,701 steps), the pleural pressure swings from -4.079 to -2.295 about a mean of -2.993, and the lung's tissue pressure from -3.498 to -1.713 about -2.412. Through the law's soft threshold the swing raises the lymph: the law's time-weighted mean over the steps is 0.0085343 a second, against 0.0047210 at the mean pressure, 1.81 times. Between the first span and the third of the table below, the switch removes 91.1% of the size of the arterial albumin's drift (-0.00030199 in the base pass, +0.00002699 switched) and 85.4% of the duct's sodium outflow's (+0.010908 a second, against -0.001589). Both residuals reverse sign, so the switch corrects each drift by 108.9% and 114.6% of its signed size.

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

Rates are water or sodium a second, and the lining takes 0.00421405 a second in every row. By window, switched against held, each relative difference rounded: Starling about 0.047%, 0.099% and 0.023%, lymph about 0.012%, 0.025% and 0.031%, glomerular albumin about 0.004%, 0.004% and 0.005%, filtered sodium about 0.019%, 0.011% and 0.001%, the duct's sodium outflow about 0.128%, 0.227% and 0.226%, and the pool's rate about 8.1%, 27.6% and 18.0% of the held pass's.

### The fix, in a PR of its own

Ruled, a model default: the lymph law reads the tissue pressure over the pressure of the enclosure around its bed, against the edge's threshold. The tissue and its lymph vessels sit inside the same enclosure, the law has no cited source, and the Starling law already reads a difference. Each enclosed bed's threshold is to be restated over its enclosure's design pressure, and that PR must show that the admitted rows stay within roundoff on every enclosed bed: only the pulmonary lymph was switched in a run. That PR ticks the suite test's box. Three forms were not chosen:

- A memory of the outside pressure: a new state and a time constant, with no run behind it. On the switched pass's saved steps over the whole breaths from 601.673 to 620.714 s, the law with the pleural pressure's true time-weighted mean in place of -3 gives a lymph about 1.2% higher, 0.00565025 a second against 0.00558223.
- A flatter law by packet values: it leaves part of the gain (a width of 2 and a conductance of 0.006497 keep the resting lymph and cut the gain from 1.808 to 1.059 on the base pass's saved steps over 602.079 to 621.137 s), and it bends the law at rest.
- Admission over whole breaths: a design round, which the plasma's drift does not need if the fix's PR shows on the whole body what the switch showed on one bed.

### Part 2. The gases

**What both balances show.** Two balances of the same tables, made apart, agree on each route that both print to its printed digit, and on each carbon dioxide store that both print within about 1e-5 a second. The recompute leaves out two routes the balance on hand prints, from the blood to the interstitium and from the interstitium to the cells, and it prints no oxygen store. With the breath held the gases are steady: over 5.006 to 300.765 s the body's carbon dioxide changes by -0.0066 a second against a production of 71.999, and its oxygen by +0.0025 against a use of 95.999, and over 1,199.083 to 1,495.541 s by -0.0086 and +0.0008. The arterial carbon dioxide pressure is 29.833 over the first span and 29.834 over the second. Under tidal breathing the body clears less carbon dioxide than it makes, and the tissues hold what is retained:

| Tidal pass, a second | 5.006 to 300.761 s | 1,199.077 to 1,495.538 s |
|---|---|---|
| Carbon dioxide made | 72.005 | 72.014 |
| Exhaled, net (expired less inspired) | 69.105 (70.000 less 0.895) | 70.929 (71.841 less 0.913) |
| Retained | 2.900 | 1.086 |
| Held by the tissues | 2.051 | 0.940 |
| Held by the blood | 0.732 | 0.136 |
| Held by the alveolar gas and the airway | 0.117 | 0.010 |

By 1,495.538 s the base pass holds 2,676 more carbon dioxide than it was admitted with, and the diagnostic run 3,551 more by 2,897.173 s.

**Inferred.** Why tidal breathing exchanges less than the breath held at rest, whose exchange includes a smoothed bulk flow, is inferred in part and not measured. The packet fits the breath's pump by a static form to the rest's mixing exchange, and how much of the shortfall comes from that fit, from the dead space's mixing and from the smoothing of the airway's pressure heads is not measured.

**A projection.** This is a projection over hours from under 50 minutes of data. A fit of the form `a - b exp(-t / tau)` from 300 s, with each breath weighted by its length, gives a plateau of the arterial carbon dioxide pressure of 30.42, with tau 1,231 s on the diagnostic run to 2,897 s and 1,247 s on the base pass to 1,495 s. That is 0.59 above the held pass's 29.833. The same form fitted to the body's carbon dioxide at each edge from 300 s, each edge weighted equally, gives about 4,000 retained at the plateau. No fix is scheduled: the projected shift is 0.59 and the retained amount is under 0.07% of a day's production, 6.22e6 at 72.0 a second. A run not yet made would measure the shortfall at a drive of one and test whether the retention converges, though it would not by itself give each mechanism's share: one awake admission at a breath rate of 0.20 a second, the drive held at one for the first minute and then released for at least 10,000 s, recording each airway interface's flow times its gas fraction in each direction, the breath's depth, the alveolar exchange and each store's slope.

### Part 3. The duct

With the breath held the duct's sodium outflow still falls, from 0.997742 to 0.995176 a second between the first span and the third (-0.257%), and its sodium concentration from 63.21485 to 63.06425. Over 20.095 to 319.965 s the sodium delivered to the duct, 12.259768 a second, less its reclaim, 11.265056, less its outflow, 0.997742, is -0.00303: the duct's balance shows its own sodium falling at that rate. Inferred from that balance: the duct holds excess sodium at admission and gives it up.

**The held skin water loss is refuted as its source** over the 302 beats, about 302 s, that part 4's runs cover (established). The loss's forcing is a factor with no unit and a mean of one, which admission holds at one. With the run's own forcing (part 4's R1) the loss runs at 0.011542 a second over beat 1 and 0.008404 over beat 301; with the forcing held at one (R2) it runs at 0.010000 in every beat. Across all 302 beats the duct's sodium change over a beat differs between the two by at most 3.09% of R1's (beat 263), and by at most 0.0014% over beats 1 to 10. In every beat of every pass, at each step aim, the duct's sodium falls, and the tighter aims shrink the fall: over beat 301 it falls by 0.00137397 at the run's aim, 0.000603261 at a tenth and 0.000235808 at a hundredth. In each pass the duct's balance closes over every beat to within 6.9e-13. Still unexplained: the fall that is left at the tightest aim (part 4's residual), and the spans from 602 s on, where the loss runs at 0.012336 a second, which these runs do not reach.

### Part 4. The held pass against the admitted beat

**The runs.** Four held passes, each to 302 s, from one build of main `b4a40ef6` with run-time switches. No file under `crates/` or `tools/` has changed on main since. Each pass has an edge at every beat edge, the first accepted step at which the cardiac phase falls below its last value, so each beat is read from one beat edge to the next with no interpolation. The phase reads 0 at all 302 edges of each pass, so a step lands on each reset. R1 keeps the run's own inputs. R2 holds the skin water loss's forcing at one, as admission holds it. R3 and R4 are R2 with the step aim at a tenth and a hundredth: the step aim is `TARGET` in `crates/ng-solver/src/lib.rs`, 0.7, the share of its error scales that the solver core aims each step's error estimate at. The error scales, which the core's identity seals, stay as they are. R1 and R2 run from one admission, and R3 and R4 from a second, whose admission record is byte for byte the first's. All four passes start from the same admitted rows, all 1,464 bit for bit.

**What admission solves, and what its recheck tests.** Admission solves one cycle of the heart, one second on a grid of 100 equal intervals, split at each guard crossing it locates, each leg two stages of Radau IIA (`crates/ng-validate/src/body/packet.rs`, the packet's paced `Cycle`; `crates/ng-contract/src/admission.rs`, `InitializationSystem`). Its recheck tests each stage's defect against `ErrorScales::at`, the absolute scale times the row's size plus the relative scale times its value (`crates/ng-contract/src/scheme.rs`), at the packet's 1e-9 and 1e-10. It tests no row's change over a beat. The counts below hold each amount row's change over one beat to that scale, with the row's size taken as 1 and its value its admitted start: 0.131 for the body's heat, 0.000184 for the coronary bed's heat and 3.33e-8 for the duct's sodium.

**The run's inputs: refuted.** Over all 302 beats R2's change of the body's heat over a beat differs from R1's by at most 0.16% of R1's (beat 269), and by at most 6.9e-6 of it over beats 1 to 10: -159.14 in beat 1 in both, and -106.384 in R1 against -106.221 in R2 over beat 301. The forcing moves the loss by 15% of its held rate in beat 1 and by 16% in beat 301, and it moves the body's heat by less than 0.2%.

**The run's stepping: established for most of the gap.** With the forcing held, at each step aim, each change is over one beat:

| Over the beat | The run's aim (R2) | A tenth (R3) | A hundredth (R4) | Recheck's tolerance |
|---|---|---|---|---|
| Body heat, beat 1 | -159.14 | -43.6144 | -31.8728 | 0.131 |
| Body heat, beat 301 | -106.221 | -28.9701 | -23.9865 | 0.131 |
| Duct sodium, beat 1 | -0.00416835 | -0.00227893 | -0.000729901 | 3.33e-8 |
| Duct sodium, beat 301 | -0.00137397 | -0.000603261 | -0.000235808 | 3.33e-8 |
| Coronary heat, beat 1 | +112.416 | +25.8741 | +22.119 | 0.000184 |
| Coronary heat, beat 301 | -0.000716481 | -0.000495821 | -0.000264084 | 0.000184 |
| The beat's length, beat 1, s | 0.999246 | 0.999609 | 0.999886 | |
| The beat's length, beat 301, s | 0.999344 | 0.999823 | 0.999952 | |
| Amount rows within, beat 1, of 1,251 | 428 | 429 | 434 | |
| Amount rows within, beat 301, of 1,251 | 624 | 676 | 723 | |
| Steps from admission to the last beat edge | 58,887 | 148,883 | 373,874 | |

Beat 301 ends between 300.80 and 300.99 s, and admission's cycle is one second. R1 has 428 rows within over beat 1 and 763 over beat 301. The hundredth aim removes 80.0% of the body heat's change over beat 1 and 77.4% over beat 301, 82.5% and 82.8% of the duct sodium's, and 80.3% of the coronary heat's over beat 1. Each store then nears a beat of the run's own at its own speed. The coronary bed's heat changes by +112.416 over R2's beat 1 and by -0.000716481 over beat 301. The body's heat changes by less each beat at the pace of its exit: over the last 100 beats of each pass, about 203 to 302 s, its mean change a beat over the slope of that change gives 1,009 s at the run's aim, 1,023 s at a tenth and 1,015 s at a hundredth, beside the 1,000 s of the heat exit's law, which relaxes the store at `bd.heat-speed`, 1e-3 a second. That is a local ratio over those 100 beats, not a fit. At beat 301 of R4 the 528 amount rows outside the tolerance are moved by the laws of the cardiovascular card alone (197), the cardiovascular and hepatic cards (66), the fluids card alone (43), the cardiovascular and fluids cards (42), the metabolism card alone (42) and the hepatic card alone (25), and 113 in 21 other groups of cards.

**A residual: not explained, and not opened.** At the hundredth aim beat 301 still changes the body's heat by -23.9865, 182 times its tolerance, and the duct's sodium by -0.000235808, 7,086 times its tolerance. From a tenth to a hundredth the body heat's change over beat 301 moves by 4.98, against 77.25 from the run's aim to a tenth. The duct sodium's moves by 0.00077 and then 0.00037, so these runs do not show it settling. This ticket names no cause. It is opened by a certificate bound that it makes fail, or by an anchor that misses its return because of it.

**What must change: nothing in a law, a packet value or what admission solves.** The kind is the run's step aim, which is a matter of accuracy. Section 7 of `docs/architecture/solver-contract.md` judges a run's accuracy by its accuracy gate, against the reference integrator, where "a window also compares each exchange's net over each whole beat of the finest refinement, from one beat edge to the next." No change to the step aim is scheduled: to the last beat edge near 302 s a tenth takes 2.53 times the steps and a hundredth 6.35 times.

### The list

Over edges 0 to 61 (5.006 to 300.761 s, and 5.006 to 300.765 s held), 378 rows and 189 columns, 567 items, have a mean under tidal breathing that differs from the held pass's by more than its scatter. The scatter is the larger of the two passes' residual standard deviations of per-span values about a fitted straight line. Each item is counted by the card whose laws move it, since cards own laws and never amounts. Items that one card's laws alone move: metabolism 170, fluids 132, blood chemistry 48, renal 29, nervous 25, hepatic 25, respiratory 19, receptor 18, cardiovascular 11, the boundary's laws 6, and musculoskeletal and skin, hematology and endocrine 1 each. The laws of two or three cards move the other 81. The twenty largest, by the size of the relative difference of the two means over edges 0 to 61, each a mean a second for a transfer:

| Item | Card whose laws move it | Tidal mean | Held mean | Relative difference |
|---|---|---|---|---|
| The breath's phase (a memory) | Nervous, by design | 0.49958391 | 0 | none, the held mean is 0 |
| The breath's mixing share (a memory) | Nervous, by design | 2.2906351e-5 | 1 | -100.0% |
| The breath's mixing share (an algebraic row) | Nervous, by design | 2.2906351e-5 | 1 | -100.0% |
| The pulmonary lymph's water, its transfer | Fluids | 0.0097555873 | 0.0055716875 | +75.09% |
| The pulmonary lymph's sodium | Fluids | 0.97252608 | 0.55543747 | +75.09% |
| The pulmonary lymph's potassium | Fluids | 0.048626304 | 0.027771874 | +75.09% |
| The pulmonary lymph's chloride | Fluids | 1.0211524 | 0.58320935 | +75.09% |
| The pulmonary lymph's water, its law's rate (an algebraic row) | Fluids | 0.0097554798 | 0.0055716879 | +75.09% |
| The pulmonary lymph's albumin | Fluids | 0.0020981512 | 0.0011983319 | +75.09% |
| Expired oxygen | Respiratory | 374.60205 | 603.77488 | -37.96% |
| Expired nitrogen | Respiratory | 1,765.7302 | 2,631.1559 | -32.89% |
| Inspired carbon dioxide | Respiratory | 0.89476773 | 1.3329076 | -32.87% |
| Inspired oxygen | Respiratory | 469.75306 | 699.77647 | -32.87% |
| Inspired nitrogen | Respiratory | 1,766.2715 | 2,631.1595 | -32.87% |
| The gut bed's sodium exchange, plasma to interstitium | Fluids | 0.0015924726 | 0.0023499073 | -32.23% |
| The gut bed's chloride exchange | Fluids | 0.0016720963 | 0.0024674027 | -32.23% |
| The gut bed's potassium exchange | Fluids | 7.9623632e-5 | 1.1749537e-4 | -32.23% |
| The liver bed's chloride exchange, plasma to interstitium | Fluids | 0.0013094548 | 0.0016925782 | -22.64% |
| The liver bed's sodium exchange | Fluids | 0.0012470998 | 0.0016119792 | -22.64% |
| The liver bed's potassium exchange | Fluids | 6.2354992e-5 | 8.0598962e-5 | -22.64% |

The next is the conducting airway's carbon dioxide, +19.18%, respiratory.

The screen removes a fitted slope, so it cannot show that an item has settled. Items whose slope persists beyond the window, under tidal breathing:

- The lung's interstitial pool: -0.00397319, -0.00232942 and -0.00141834 a second over the three spans of part 1.
- The drive scale: 1.03952, 1.04258 and 1.06250 as means of the breaths that end in 200 to 300 s, 300 to 400 s and 1,199 to 1,496 s, where the held pass gives 0.99933, 0.99927 and 0.99944.
- The bladder's sodium: 999.017, 999.158 and 1,004.332 over the same breaths, where the held pass gives 998.579, 998.318 and 996.136.
- The carbon dioxide the body holds, by part 2.

### What must change, and who owns it

| What must change | Kind | Owner | When |
|---|---|---|---|
| The lymph's form: the tissue pressure over its enclosure's | A law | The fluids card | Ruled, its own PR, the suite test's box |
| The tidal breath's exchange at the admitted drive | A law or a packet value | [Body ticket 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md), by a new box | Not scheduled |
| The held skin water loss, with its forcing | A hold | [Body ticket 12](../../body/issues/12-heat-balance.md) moves the loss and its forcing to the evaporation law | With ticket 12 |
| Admission over whole breaths | What admission solves | Native-contract | Not scheduled |
| The run's step aim, which leaves the held body off its admitted beat (part 4) | Accuracy, not a law, a packet value or what admission solves | The solver core, judged by the accuracy gate of the solver contract's section 7 | Not scheduled: a tenth of the aim takes 2.53 times the steps, a hundredth 6.35 times |

Admission over whole breaths is not scheduled: the plasma's drift has a fix by a law's form, and the gases' shortfall may have one by a law or a packet value, which body ticket 38's box owns. It reopens on a pool or flow whose mean under breathing stands off its admitted value and that no law's form or packet value brings back. If it reopens, it is a design round of its own.

This ticket stays open for the suite test's box alone, and it still blocks body ticket 17.
