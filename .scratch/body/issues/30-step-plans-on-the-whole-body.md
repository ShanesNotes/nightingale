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
- [ ] A reference for the body's long-run salt rates, which this ticket owns (a model default). A salt rate that a ticket claims is compared with a reference that is not the plan under test, as section 7 of `docs/architecture/solver-contract.md` judges the slice by its accuracy gate: "A core is never compared with itself." The reference is the contract's `ReferenceIntegrator` (`crates/ng-validate/src/reference.rs`), fully coupled backward Euler on a fixed interval, run on the whole body at three refinements, whose intervals the run declares (the accuracy gate's windows take the longest, a quarter of it and a sixteenth). The cost of one whole-body reference window is measured first. Then each salt rate claimed is compared with the reference for the same window of [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md)'s part 5 and the same condition, awake or with the breath held, and the box reports the spread across the three refinements as a spread, not as a bound on the reference's distance from the limit. Where the cost or the evidence prevents that judgment, the sign and the size stay open and this box says so. Until this box is ticked, no ticket takes a sign or a size of the salts' long-run rates from native-contract 100's part 5 or from the box below.
  - **Met, awake only,** by the measurements in [What the reference showed](#what-the-reference-showed) below. The reference is the contract's `ReferenceIntegrator` on the whole body at intervals of 1 ms, 250 µs and 62.5 µs, which each pass declares. The cost of a window was measured first. The halved step's awake rates at its three steppings are compared with the reference over the same five windows, counted by breath edge, and the spread across the three intervals is reported as a spread. **Not met.** The breath-held condition has no reference pass, so the sign and the size of the held rates stay open. The doubled step, two-stage Radau IIA and the extrapolated step have no rate on the body to compare. Nothing here is shown on today's main, because every pass is from a build of main `e5a05b96` and the solver and record crates changed after it (nightingale#235, nightingale#239 and nightingale#240). Neither the cost nor the evidence prevents the held judgment, since the held passes were not run, so this box stays open.
- [ ] Each plan's long-run salt rates on the body: the body's total sodium, chloride and potassium as rates over the five windows of native-contract 100's part 5, awake and with the breath held, at the plan's own stepping and at two finer ones, and whether main's certificate passes each to 700 s. A finer stepping of a plan is its own step aim tightened tenfold, and then tenfold again, with its retry floor held (a model default). All four plans have a step aim: the doubled step here is [ticket 27](27-step-plan.md)'s adaptive baseline, `Core::doubled` in `crates/ng-solver/src/lib.rs`, and not the contract's `ReferenceIntegrator`, which steps a fixed interval and has no aim. For the halved step, the step aim and the floor are also varied apart. The box reports the three rates and the two changes between neighbouring levels as observed changes, never as an error or a bound: three levels can share a bias larger than both changes. It reports the share of steps on the shortest span where the tap can count it, and says where it cannot. Native-contract 100's part 5 has the halved step's figures at three steppings moved together.
  - Native-contract 100's part 5 also has the halved step with the aim and the floor moved apart, from one 1,500 s rate run for each setting of a build of main `e5a05b96`, with the 700 s certificates from separate runs (measured). Over window 5, in 1e-3 a second, sodium awake is +1.52360 at main's stepping, +1.48156 with the floor alone at 25 µs, -0.01298 with the aim alone at a tenth and -0.05102 with both; held, +2.77201, +2.71672, +1.20987 and +1.17288. Chloride and potassium have their own figures there. Main's certificate to 700 s certifies the awake body with the aim alone and with both, and refuses by `Drift` on sodium the awake body with the floor alone and the held body with the aim alone, the floor alone and both. The tap keeps no count of the steps that reached the shortest committed span. *Inferred:* on this body and at these two sizes of change, the aim moves the salts' rates far more than the floor does. Not claimed: a settled rate, a limit or its sign, or anything for another plan.
  - What those passes already give this box for the halved step, awake, with the floor held at 100 µs (measured): three levels of the aim, 0.7, 0.07 and 0.007, over window 5 in 1e-3 a second. Sodium +1.52360, -0.01298 and -0.75480, with changes of 1.53659 and 0.74182 between neighbouring levels; chloride +1.59978, -0.01363 and -0.79254, with changes of 1.61341 and 0.77891; potassium +0.07618, -0.00065 and -0.03774, with changes of 0.07683 and 0.03709. These are observed changes, not an error or a bound. The finest level has no certificate and no count of steps on the shortest span. Held has two levels of the aim.

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

## What the reference showed

Every value in this section is synthetic.

**The passes (measured).** Three passes of the contract's `ReferenceIntegrator` on the whole body, awake, each from its own admission to 1,500 s, one run at each of three fixed intervals, 1 ms, 250 µs and 62.5 µs. They ran main `e5a05b96`, the revision of native-contract 100's part 5, with a measurement tap that was never pushed and that leaves `crates/ng-validate/src/reference.rs` unchanged. All three admitted the same 1,464 rows bit for bit, as part 5's awake passes did. Each pass declares its interval, absolute and relative scales of 1e9 and 1, and a floor of 100 µs, and reads the body at every breath edge.

**Steps, refusals and faults (measured).** The passes took 1,512,653, 6,010,587 and 24,000,000 steps to 1,500 s. Each reached 1,500 s with exit 0 and no engine fault. The 1 ms pass refused four steps by `GuardInside`, in windows 1, 2, 2 and 4, and refused none of them twice. The 250 µs and 62.5 µs passes refused none.

**The cost (measured, one run each, on one build worker).** The 30 s pilots ran first. With admission, a pilot took 12.74 s of wall time a simulated second at 1 ms, 26.69 s at 250 µs and 81.81 s at 62.5 µs. Its stepping alone took 5.16 s, 19.29 s and 74.17 s a simulated second, and its admission 222.0 to 229.2 s. A step took about the same wall time at each interval, 195.3, 207.8 and 215.7 steps a second of stepping wall. The three long passes started together, each in its own job slot, and took 8,405 s, 29,586 s and 113,469 s of wall time. With one run each, no time here has a spread, and no profile names what bounds a step's cost.

**The windows (measured).** Each window runs between the breath edges nearest 5, 300, 600, 900, 1,200 and 1,500 s. In each of the six passes below, those are breath edges 0, 61, 124, 187, 250 and 314, but each pass reaches them at its own times, up to 0.904 s apart. So each row of the salt tables is one window by breath edge, and each pass's figure is over that pass's own window. The window ends of each pass, in s:

| Pass | Breath edge 0 | Breath edge 61 | Breath edge 124 | Breath edge 187 | Breath edge 250 | Breath edge 314 |
|---|---|---|---|---|---|---|
| a1, main's stepping | 5.006 | 300.595 | 601.660 | 900.672 | 1,198.075 | 1,498.935 |
| a2 | 5.006 | 300.574 | 601.666 | 900.728 | 1,198.205 | 1,499.157 |
| a3, the finest | 5.006 | 300.610 | 601.770 | 900.916 | 1,198.493 | 1,499.556 |
| Reference, 1 ms | 5.006 | 300.656 | 601.871 | 901.075 | 1,198.712 | 1,499.839 |
| Reference, 250 µs | 5.006 | 300.637 | 601.840 | 901.035 | 1,198.668 | 1,499.793 |
| Reference, 62.5 µs | 5.006 | 300.632 | 601.831 | 901.024 | 1,198.654 | 1,499.779 |

**Sodium the body keeps, awake, in 1e-3 a second (measured).**

| Window | Reference, 1 ms | 250 µs | 62.5 µs | 1 ms less 250 µs | 250 µs less 62.5 µs | Ratio of the two | a1, main's stepping | a2 | a3, the finest |
|---|---|---|---|---|---|---|---|---|---|
| 1 | -0.08291 | -0.10647 | -0.11255 | +0.02356 | +0.00608 | 3.87 | +0.10117 | -0.02461 | -0.08525 |
| 2 | -0.34934 | -0.45243 | -0.47997 | +0.10310 | +0.02754 | 3.74 | +0.44619 | -0.09937 | -0.36176 |
| 3 | -0.61203 | -0.79850 | -0.84874 | +0.18647 | +0.05024 | 3.71 | +0.80247 | -0.16851 | -0.63690 |
| 4 | -0.78105 | -1.03638 | -1.10499 | +0.25533 | +0.06860 | 3.72 | +1.12918 | -0.18273 | -0.81781 |
| 5 | -0.76914 | -1.07885 | -1.16269 | +0.30971 | +0.08384 | 3.69 | +1.52360 | -0.05102 | -0.81595 |

**Chloride the body keeps, awake, in 1e-3 a second (measured).**

| Window | Reference, 1 ms | 250 µs | 62.5 µs | 1 ms less 250 µs | 250 µs less 62.5 µs | Ratio of the two | a1, main's stepping | a2 | a3, the finest |
|---|---|---|---|---|---|---|---|---|---|
| 1 | -0.08706 | -0.11179 | -0.11818 | +0.02473 | +0.00639 | 3.87 | +0.10623 | -0.02584 | -0.08951 |
| 2 | -0.36680 | -0.47505 | -0.50397 | +0.10825 | +0.02892 | 3.74 | +0.46850 | -0.10433 | -0.37984 |
| 3 | -0.64263 | -0.83842 | -0.89117 | +0.19579 | +0.05275 | 3.71 | +0.84259 | -0.17693 | -0.66874 |
| 4 | -0.82010 | -1.08820 | -1.16023 | +0.26810 | +0.07203 | 3.72 | +1.18564 | -0.19187 | -0.85870 |
| 5 | -0.80760 | -1.13279 | -1.22082 | +0.32519 | +0.08803 | 3.69 | +1.59978 | -0.05358 | -0.85675 |

**Potassium the body keeps, awake, in 1e-3 a second (measured).**

| Window | Reference, 1 ms | 250 µs | 62.5 µs | 1 ms less 250 µs | 250 µs less 62.5 µs | Ratio of the two | a1, main's stepping | a2 | a3, the finest |
|---|---|---|---|---|---|---|---|---|---|
| 1 | -0.00415 | -0.00532 | -0.00563 | +0.00118 | +0.00030 | 3.87 | +0.00506 | -0.00123 | -0.00426 |
| 2 | -0.01747 | -0.02262 | -0.02400 | +0.00515 | +0.00138 | 3.74 | +0.02231 | -0.00497 | -0.01809 |
| 3 | -0.03060 | -0.03992 | -0.04244 | +0.00932 | +0.00251 | 3.71 | +0.04012 | -0.00843 | -0.03184 |
| 4 | -0.03905 | -0.05182 | -0.05525 | +0.01277 | +0.00343 | 3.72 | +0.05646 | -0.00914 | -0.04089 |
| 5 | -0.03846 | -0.05394 | -0.05813 | +0.01549 | +0.00419 | 3.69 | +0.07618 | -0.00255 | -0.04080 |

**What the reference shows.**

- *Measured.* In every window and at every interval the body loses each salt. Its urine carries more than the gut absorbs, and the feces and sweat routes carry nothing over these windows. The loss grows as the interval shrinks, in every window and for each salt.
- *Measured.* The difference between 1 ms and 250 µs is 3.69 to 3.87 times the difference between 250 µs and 62.5 µs, and the ratio is the same for the three salts in each window to five decimals.
- *Measured.* Each of the 45 window balances, three salts over five windows in three passes, closes on its routes. The change of the salt's total less the sum of its routes is at most 7.1e-13 a second.
- The two differences are a spread across the three intervals, not a bound on the reference's distance from the limit. Three levels can share a bias larger than both differences, as the box above says of the plans' levels.
- *Inferred.* A first-order scheme's difference falls fourfold when its interval falls fourfold. These differences fall 3.69 to 3.87 times and keep one sign, so each salt's rate at the limit is inferred to be a loss, as at every interval measured. This section gives no value for the limit and no bound on the distance to it.

### The halved step against the reference, awake

The a1 to a3 columns are [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md)'s part 5 awake passes of the halved step at main's stepping, the middle stepping and the finest, recomputed from their own tables. They match part 5's sodium table to its four decimals. Each difference below is an observed difference between two passes.

- *Measured.* At main's stepping, a1, the halved step keeps each salt in every window, and the reference loses each salt in every window at every interval.
- *Measured.* At the middle stepping, a2, it loses each salt in every window, less than the reference at any interval.
- *Measured.* At the finest stepping, a3, it loses each salt in every window. In every window and for each salt, its loss lies between the reference's at 1 ms and at 250 µs.
- *Measured.* Over window 5, a3 loses 0.81595e-3 sodium a second and the reference at 62.5 µs loses 1.16269e-3, an observed difference of 0.34674e-3 a second. For each salt, a3's window 5 loss is 0.702 of the reference's at 62.5 µs.
- The passes with the aim and the floor moved apart, which the box above quotes, are not in these tables, because none is finer than a3 in the aim or in the floor.
- *Not claimed:* which pass stands nearer the limit, a cause of any difference, anything with the breath held, anything for another plan, or anything on today's main.

## Comments

From the solver explorations: the plans this ticket compares differ in something none of its boxes reads. A backward Euler step damps an oscillation, the halved step by about `w^2 h / 4` a second and a one-stage step by twice that, with no refusal and inside every gate ([native-contract 91](../../native-contract/issues/91-the-accuracy-gate-holds-no-oscillation.md) has the measurement). [Ticket 27](27-step-plan.md) counted four fewer valve mode changes a beat under the one-stage plan, which is what more damping would do (inferred). So a plan can cost less because it damps more. Each plan's run here could also report, against the same plan at half its cap: the valve mode changes a beat, the pulse pressure, and the largest change in any route's forward or reverse exchange over a beat. Two-stage Radau IIA is third order and damps far less at the same step, so its place in the comparison is not only its cost.
