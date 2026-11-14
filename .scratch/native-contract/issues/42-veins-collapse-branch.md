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

