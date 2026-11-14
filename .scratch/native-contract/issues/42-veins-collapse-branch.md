# 42: Give the veins a collapse branch, so no head empties a vein

Spec: `crates/ng-cardiovascular/src/walls.rs` (`Walls::stressed`), the slice's packet in `crates/ng-validate/src/slice.rs`, and [native-contract 33](33-leg-veins-empty-head-down.md)'s Answer

Type: task

Status: resolved

Claimed by: nc42-pr3-build

Blocked by: none

Below its unstressed volume a vein's stressed volume is held at zero, so its pressure stays near zero however much water it loses. Head-down at trunk -0.2 the leg veins sit 0.1 m above the central veins, nothing opposes the 7 mmHg gravity term, and they empty in 29.5 s on every core. [Native-contract 33](33-leg-veins-empty-head-down.md) names the cause and the form.

**What to build:** the collapse branch at the three veins (leg, splanchnic and central). Below `V_u` the stressed volume falls on `share smooth_min((V_u / n) (1 - (V_u / V)^n), 0)`, added to today's `smooth_max(V - V_u, 0)`, with the power written as `exp(n ln(V_u / V))`. Two new parameters: `cv.collapse-share`, a site parameter, 0.05 at the three veins and 0 elsewhere, and `cv.collapse-power`, a body parameter, 1.5. Both are synthetic, sourced to Toro and others 2021 Eqs 3 to 5 (the elastic part) and to Snyder and Rideout's twentyfold collapsed compliance as Shi, Lawford and Hose 2011 restate it. A throwaway patch that does this is `patches/7-collapse-branch-power-1.5.diff` in ticket 33's evidence folder.

- [x] A closed-form test of the wall law below the unstressed volume: at a vein with a share above zero, the pressure equals `E share (V_u / n) (1 - (V_u / V)^n)` at volumes well below `V_u`, and at a site with a share of zero it stays at today's floor. A mutant row that puts the floor back is CAUGHT.
- [x] The term exists only where the share is above zero, or this ticket says why it cannot. A kernel's site set is a class or the whole vascular network, and every vessel and chamber in the slice is a `VesselSegment`, so leaving the term out needs a narrower site set in the contract. Today the term's divide and logarithm read every blood site's volume.
- [x] Ticket 33's spec completes on `ng_solver::Core::new` and on the reference integrator at trunk -0.1, -0.2 and -0.5. Sitting up keeps the leg veins' water within 1.5 mL of main's over every accepted step: the throwaway patch kept it within 1.4 mL.
- [x] The PR lists every pinned value that moves, with one sentence each: the slice's model hash, the arena packet's two new parameters, and each pinned trace. It says whether every record main wrote still loads.
- [x] A source check of Toro 2021's Eqs 3 to 5 and of the share's source (Shi, Lawford and Hose 2011) against the evidence store.

**Non-goals:** a bed's collapse and a waterfall outlet, which trunk -1.0 would need: there the leg veins hold and pull the leg bed's plasma dry. The healthy figure for head-down tilt, which no source on hand gives.

The foundation map's route says when this lands against the arena pick, because it moves what the pick is scored on.

## Part 1 of 3: the vein set

This PR lands the vein set. `AnatomyBuilder::vein` marks a vessel segment as a vein, and `finish` refuses an undeclared mark (`Undeclared`) and one off the vascular network (`VeinOffVascularNetwork`). The site set `Veins` names the marked veins. A sealed relation, `Includes`, says each set includes itself and blood includes the veins and the beds. A law on a set reads the entry of a set that includes it with `at_site_in`, and its site parameter with `site_parameter_in`, and `in_blood` is gone. A parameter on the veins defaults from the vascular network, as one on blood does, so a network value binds it at each marked vein and an unmarked vessel asks for none. A map that marks no vein keeps the digest it had, so no pin moves, and the slice marks no vein yet.

The design first tried a broadened `at_site` and `site_parameter` that take an including set. It left one existing kernel unable to infer its set (E0283, in `map_reads.rs`'s `a_site_rate_on_a_body_memory_is_refused_by_name`), so the separate reads landed.

Two PRs follow. The term: a law writes a site term once on the veins and adds it at each use, and compile places it where the site is a vein, leaving main's bytes at every other site. Then the wall law: the collapse branch on the four veins, with the pins moved once and the cost measured.

What the check of part 1 found for the later parts:

- A plain law on a set with no site still seals its unplaced program, as on main, so a law on the veins in a map with no vein moves the model hash. Part 2's rule that a body with no vein keeps every byte of main is a rule for site terms.
- Part 2 keeps the relation reflexive, and pins that `at_site_in` on a kernel's own set writes `at_site`'s bytes.
- A map with no vein refuses a packet's network value for a vein parameter as `UnknownParameter`, so in part 3 a packet holds the share only where the map marks a vein.

## Part 2 of 3: the site term

This PR lands the site term in `ng-contract`. A law's author declares a term once on a site set with `FragmentBuilder::site_term`, closes it with `Kernel::finish_term`, and a law on a set that includes the term's set adds it to a value with `Kernel::add_from`. The term holds no row. Compile keeps the law's graph as written beside its program. At a site of the term's set it writes a variant with the term added, after every program main has. At every other site the law runs main's program with main's id. Each op only the term computes carries the term's law (`Code::origins`, sealed only when not empty). An op the law also computes is the law's.

What it holds, each with a test in `crates/ng-contract/src/evaluate/terms.rs` and a mutant row:

- A refused step names the law of the first op, in program order, that fails the check that refused. The evaluator finds that op only after a refusal, by running `forward` on prefixes, so a step's code is main's. The prototype named the term when a host op refused before a term's read that was not finite; the search now runs only the steps and reads before such a read.
- With shared subexpressions an op the host and the term both compute is one op, so the reverse sweep adds its two slopes before it scales them. Values are the same. Slopes are those of that one program: bit for bit the law written by hand with the shared op bound once, and not always those of a law that writes the op twice (near 1e308 the first overflows where the second does not).
- A slope that is not finite names the law of the read it belongs to: the law's for a read the law shares with its term, the term's for a read only the term makes.
- The erasure rule is for a site term: a term with no site in its set leaves every byte of the body as it is with no term. A plain law on a set with no site still seals its unplaced program, on main too, and that is unchanged.
- The declaration checks (a day's input with no parameter, a forcing some law must read, a volume with no partial volume, another route's drive) judge the law's own program as main judges it, and each variant a site of the law runs, pruned as main prunes a program. So with no site in the set a body compiles or is refused as with no term, a law cannot borrow its term's parameter, a term cannot hide a read the variant uses, and a read nothing uses neither passes a check nor fails one. A draw an algebraic law reads is refused at admission through a term as by hand.
- A kernel that adds a term stands at sites. Attached where it stands at no site (a body memory's rate, or a guard's zero, which stands at the body or an edge), it is refused as `TermOffSite`, a guard by its mode variable. Written inline, the same site read is `NoSuchRead`.
- `at_site_in` on a kernel's own set writes the op `at_site` writes, so the reflexive case has a test and a row.
- A term may add a term. No set lies strictly inside the veins, so the test takes an outer term on the veins (a site in neither set, and in both) and an outer term on blood (a site in the outer only, and in both). Each refusal names its own law.
- The host forms part 3 uses: a term's read that takes no nerve keeps that, a summed law closes its variant as main closes it, and a column runs its variant at a vein.

What it leaves:

- Carried to [native-contract 59](59-a-term-names-itself-at-compile-and-admission.md): a refusal at compile or admission names the law that adds the term, not the term: `UnemittedField` and `UnbackedConstant` take their law from the whole kernel, and an unused read has no op to carry an origin. Naming the term there needs the law threaded through each read's address.
- An op two terms both compute carries the law of the term added first.
- A term whose ops all merge into its law's leaves no op of its own, so the law's variant lists the law again in the refusal sources, as main lists a law whose programs are not adjacent, and the term's name is not in the model hash. A test pins the sources.
- A term declared and never added is not validated, as a kernel never attached is not. Its name still joins the names checked twice.
- The three-halves power as `r * sqrt(r)` would cut the term's cost from +2.4% to +0.2% seated against main (the plan's bench, four veins, 32 pairs), but needs a square-root op in the evaluator. Part 3 or a later ticket weighs it.

On the slice, with measurement trees not landed: the wall law adding the term with no vein marked keeps main's model hash, conserved basis, the three first-second traces and seals, the parameters and all 332 sources. Its user instructions an accepted step are -0.14% against main (three rounds, the beelink). With four veins marked it admits and runs seated 121 s with no fault.

Part 3 needs: the wall law's collapse branch on the four veins through `add_from`, with its shared reader spelled `at_site_in` on blood and on the vein term, the share bound once on the vascular network, the pins moved once, and the cost measured again on the real law.

## Part 3 of 3: the wall law on the four veins

**Answer.** The collapse branch runs at the slice's four veins, the pulmonary vein, the central veins, the splanchnic veins and the leg veins. Below `V_u` a marked vein's stressed volume falls on `share (V_u / n) (1 - (V_u / V)^n)`, added through the site term `vein collapse` in `Walls::stressed`, so the pressure, the elastic store and stroke work all read it, and every other site keeps main's program. The slice binds `cv.collapse-share` at 0.05 once on the vascular network and `cv.collapse-power` at 1.5 on the body. Both are synthetic fixture values. The law is the form native-contract 42's part 2 measured (its tree `2130fb1`), with one trait renamed. All four veins are marked because a mark says what a vessel is: no run of the slice tells three marked veins from four.

**The fault.** Head-down at trunk -0.2 on `ng_solver::Core::new` the slice now runs past 30 s, and its leg veins keep more than half their water (`tests/veins.rs`). On main, with no vein marked, the same test stops at 29.494 s with `NoAdmissibleStep`, its last refusal at source 258, `Divisor { law: CV/valves }`, as native-contract 33 found. Ticket 33's spec, from the plan's runs at this law and packet (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, item 3):

| Run | Steps | Fault | End leg-vein water | Leg-vein pressure |
|---|---|---|---|---|
| Core::new, trunk -0.1 | 11,215 | none | 460.648 mL | -0.57 to 2.86 mmHg |
| Core::new, trunk -0.2 | 10,999 | none | 391.178 | -3.00 to 2.86 |
| Core::new, trunk -0.5 | 21,117 | none | 281.997 | -11.06 to 2.86 |
| Reference 1 ms, trunk -0.2 | 61,452 | none | 391.788 | -3.01 to 2.87 |

Sitting up for 121 s, the leg veins' water stays within 1.352 mL of main's at every accepted step, under the spec's 1.5 mL.

**The closed form.** Below `V_u` the law less the outside pressure lies between `-E share w^2 / (4 |shrunk|)` and `E w^2 / (4 (V_u - V))` of `E share (V_u / n) (1 - (V_u / V)^n)`, with `w` the soft width. The test stages the central veins at half and a quarter of `V_u`, and the systemic arteries, which are not marked, at half theirs, where the law is main's floor (`tests/veins.rs`). Two mutant rows hold it: `collapse-floor-restored` puts the floor back, and `collapse-power-read` reads the power as one.

**Two slice tests change, and why.** The splanchnic veins rest about 10 mL below their unstressed volume from admission, on main too, so the branch acts at rest. That moves two tests, and the splanchnic veins alone move both.

- The stand and the sit read the swing of the beat means from 20 s to 30 s, the last seconds of the settling after a sit-up ramp that ends at 15 s, and the beat rule started from phase 0, so an edge in the first step was lost. Main passed because its edge fell in that step. The test now reads from 30 s to 40 s and starts the rule from the phase of the first sample, so it reads hunting and not settling. The limit stays 1.0. A test on made-up samples, with an edge in the first step, holds the rule, with the row `beat-rule-keeps-the-first-edge`.
- The window's 1 ms core still fails the first slice window. On main it failed on gravity work's value (`Outside`); with the branch its values pass, and it fails as `TransitionMoved` on the stir, because a floor of 1 ms cannot land a constant-rate hazard within the reference's 0.2 ms. The test now expects that.

**Energy below the unstressed volume.** Head-down at trunk -0.2 the walls' ledger closes at every step and the store is positive. The leg veins' part, `E s^2 / 2`, grows as they empty. From 0 to 11 s on Core::new the walls shed 170,152 as heat and draw 3.62 as muscle work (the slice's energy units); the leg veins fall from 462.2 mL below `V_u`, holding 3.7e-6, to 378.3 mL, holding 10.47. That store is not the work of the pressure below `V_u`, so the difference leaves as heat: [native-contract 57](57-the-store-below-a-veins-unstressed-volume.md).

**An unstressed volume at or below zero.** The term divides by the volume and takes the logarithm of `V_u / V`, so a marked vein needs `cv.unstressed` above `cv.unstressed-gain`, and the law's doc says so. The plan's probes built two packets that break it, a splanchnic base of 400 with a gain of 800 and a central base of 0: each was refused at initialization as `Undefined { law: CV/vein collapse }`, where main admits both. A packet whose `V_u` crosses zero only mid-run would be refused at the step by the same name, read from the code and not run. The test stages a marked vein with no volume and finds the same refusal by name. A test cannot build such a packet: the slice binds both keys at each of its veins and a packet refuses a second binding.

**Pins.** Each moves once, with these values:

| Pin | Main | This part |
|---|---|---|
| The slice's model hash | `8e461394` | `42c60687` |
| The first second's packet hash, the reference run | `eba44c97` | `f57a2a57` |
| Its seal | `cfb87b8d` | `de4f1fd4` |
| The packet hash of the two `ng_solver` runs, and gp-driver's `SMOKE_PACKET` | `2dde20ac` | `28b0fe0d` |
| Their seals, `0b8f63bb` also gp-driver's `SMOKE_SEAL` | `0b8f63bb`, `c4821276` | `f8cf81f2`, `b2a04a45` |
| gp-driver's `SMOKE_HEAD_NS`, the first head at or past 1 s | 1,000,226,001 | 1,000,225,153 |
| Programs, parameters and refusal sources | 96, 283, 332 | 99, 288, 335 |

The conserved basis, the rows (241) and the algebraic rows (76) do not move. The three new sources are `Divisor`, `LogarithmArgument` and `NonFinite` for `CV/vein collapse`, after main's 332. The whole body marks no vein, so its model hash `a24da7b7` and its pins do not move. No record main wrote loads into this slice: a session refuses an identity whose model hash is not the body's, by design. No checked-in record is admitted by the engine.

**The day.** The plan ran the slice's day from 06:55 to 06:55 on Core::new at 10 ms, at this law and packet (model hash `42c60687`): 18,186,491 accepted steps and no fault. No bound from nightingale#149 moves.

| Book | This day's change | Main's change | Bound | Change / bound |
|---|---|---|---|---|
| Sodium | -16,439.11 | -18,316.79 | 4e4 | 0.41 |
| Chloride | -17,262.46 | -19,234.02 | 4e4 | 0.43 |
| Potassium | -823.35 | -917.23 | 2e3 | 0.41 |
| The moving water | -220.62 | -231.97 | 5e2 | 0.44 |
| Vasopressin | +524.24 | +395.84 | 8e2 | 0.66 |
| Energy | -164,356.3 | +83,433.6 | 2e5 | 0.82 |
| Charge | -2.1e-9 | +1.7e-10 | closure floor | 0.005 of it |

Energy comes closest: its change flips sign and doubles, to 0.82 of its bound, the difference of two flows of 1.0e9 each. No run splits the cause, and [native-contract 52](52-energy-fails-the-sliding-windows.md) says so.

**Sources.** Toro and others 2021, equations 3 to 5, give the form: a vein's transmural pressure is a stiffness times the area ratio to the power m, less the ratio to the power n, with n = -1.5 for veins (full text and the rendered page read, PrimaryFitted). Shi, Lawford and Hose 2011 restate Snyder and Rideout's compliance of a collapsed segment, twenty times an uncollapsed one (the paragraph read, Compiled). Their gaps: the joined form is in neither source; Toro's other branch, the power m, is dropped; area is read as volume; the share sets the stiffness and has no source; and the twentyfold holds at `V_u` only, since the slope is `share (V_u / V)^2.5`. Both parameters stay synthetic fixture values. A reviewed packet would need an inference record for each.

**What is left.**

- [Native-contract 57](57-the-store-below-a-veins-unstressed-volume.md): the store below `V_u` as the pressure's potential, before the scored pick or named by the pick's record.
- [Native-contract 58](58-admission-seats-the-splanchnic-veins-below-their-unstressed-volume.md): why admission seats the splanchnic veins below their unstressed volume.
- [Native-contract 59](59-a-term-names-itself-at-compile-and-admission.md): part 2's open box, the order of an unread forcing, and two rows.
- [Body 06](../../body/issues/06-whole-body-on-holds.md): the whole body marks its veins and moves its pins once.
- [Native-contract 54](54-three-halves-power-as-ratio-times-square-root.md) and [55](55-a-term-evaluated-once-and-rows-that-are-definitions.md): the term's cost.
