# 42: Give the veins a collapse branch, so no head empties a vein

Spec: `crates/ng-cardiovascular/src/walls.rs` (`Walls::stressed`), the slice's packet in `crates/ng-validate/src/slice.rs`, and [native-contract 33](33-leg-veins-empty-head-down.md)'s Answer

Type: task

Status: open

Blocked by: none

Below its unstressed volume a vein's stressed volume is held at zero, so its pressure stays near zero however much water it loses. Head-down at trunk -0.2 the leg veins sit 0.1 m above the central veins, nothing opposes the 7 mmHg gravity term, and they empty in 29.5 s on every core. [Native-contract 33](33-leg-veins-empty-head-down.md) names the cause and the form.

**What to build:** the collapse branch at the three veins (leg, splanchnic and central). Below `V_u` the stressed volume falls on `share smooth_min((V_u / n) (1 - (V_u / V)^n), 0)`, added to today's `smooth_max(V - V_u, 0)`, with the power written as `exp(n ln(V_u / V))`. Two new parameters: `cv.collapse-share`, a site parameter, 0.05 at the three veins and 0 elsewhere, and `cv.collapse-power`, a body parameter, 1.5. Both are synthetic, sourced to Toro and others 2021 Eqs 3 to 5 (the elastic part) and to Snyder and Rideout's twentyfold collapsed compliance as Shi, Lawford and Hose 2011 restate it. A throwaway patch that does this is `patches/7-collapse-branch-power-1.5.diff` in ticket 33's evidence folder.

- [ ] A closed-form test of the wall law below the unstressed volume: at a vein with a share above zero, the pressure equals `E share (V_u / n) (1 - (V_u / V)^n)` at volumes well below `V_u`, and at a site with a share of zero it stays at today's floor. A mutant row that puts the floor back is CAUGHT.
- [ ] The term exists only where the share is above zero, or this ticket says why it cannot. A kernel's site set is a class or the whole vascular network, and every vessel and chamber in the slice is a `VesselSegment`, so leaving the term out needs a narrower site set in the contract. Today the term's divide and logarithm read every blood site's volume.
- [ ] Ticket 33's spec completes on `ng_solver::Core::new` and on the reference integrator at trunk -0.1, -0.2 and -0.5. Sitting up keeps the leg veins' water within 1.5 mL of main's over every accepted step: the throwaway patch kept it within 1.4 mL.
- [ ] The PR lists every pinned value that moves, with one sentence each: the slice's model hash, the arena packet's two new parameters, and each pinned trace. It says whether every record main wrote still loads.
- [ ] A source check of Toro 2021's Eqs 3 to 5 and of the share's source (Shi, Lawford and Hose 2011) against the evidence store.

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
- The declaration checks (a day's input with no parameter, a forcing some law must read, a volume with no partial volume, another route's drive) judge the law's own program as main judges it, and each variant a site runs, as written before pruning. So with no site in the set a body compiles or is refused as with no term, a law cannot borrow its term's parameter, and a term cannot hide a read. Where a site is in the set, the variant is judged as written, dead ops included, so a marked body can be refused for a dead read that main lets pass in the same law. A draw an algebraic law reads is refused at admission through a term as by hand.
- A kernel that adds a term stands at sites. Attached where it stands at no site (a body memory's rate, or a guard's zero, which stands at the body or an edge), it is refused as `TermOffSite`, a guard by its mode variable. Written inline, the same site read is `NoSuchRead`.
- `at_site_in` on a kernel's own set writes the op `at_site` writes, so the reflexive case has a test and a row.
- A term may add a term. No set lies strictly inside the veins, so the test takes an outer term on the veins (a site in neither set, and in both) and an outer term on blood (a site in the outer only, and in both). Each refusal names its own law.
- The host forms part 3 uses: a term's read that takes no nerve keeps that, a summed law closes its variant as main closes it, and a column runs its variant at a vein.

What it leaves:

- [ ] A refusal at compile or admission names the law that adds the term, not the term: `UnemittedField` and `UnbackedConstant` take their law from the whole kernel, and an unused read has no op to carry an origin. Naming the term there needs the law threaded through each read's address.
- An op two terms both compute carries the law of the term added first.
- A term whose ops all merge into its law's leaves no op of its own, so the law's variant lists the law again in the refusal sources, as main lists a law whose programs are not adjacent, and the term's name is not in the model hash. A test pins the sources.
- A term declared and never added is not validated, as a kernel never attached is not. Its name still joins the names checked twice.
- The three-halves power as `r * sqrt(r)` would cut the term's cost from +2.4% to +0.2% seated against main (the plan's bench, four veins, 32 pairs), but needs a square-root op in the evaluator. Part 3 or a later ticket weighs it.

On the slice, with measurement trees not landed: the wall law adding the term with no vein marked keeps main's model hash, conserved basis, the three first-second traces and seals, the parameters and all 332 sources. Its user instructions an accepted step are -0.14% against main (three rounds, the beelink). With four veins marked it admits and runs seated 121 s with no fault.

Part 3 needs: the wall law's collapse branch on the four veins through `add_from`, with its shared reader spelled `at_site_in` on blood and on the vein term, the share bound once on the vascular network, the pins moved once, and the cost measured again on the real law.
