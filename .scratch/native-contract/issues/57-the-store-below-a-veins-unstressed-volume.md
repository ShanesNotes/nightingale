# 57: Below its unstressed volume a vein's elastic store is not the work of its pressure

Spec: `crates/ng-cardiovascular/src/walls.rs` (`Walls::elastic_energy` and the term `vein collapse`), and [native-contract 42](42-veins-collapse-branch.md)'s Answer

Type: task

Status: claimed

Claimed by: nc57

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the reference man for 48 hours, and the walls' energy ledger is one of the books the run is judged on. Native-contract 42 gave the four veins a collapse branch: below its unstressed volume `V_u` a vein's stressed volume `s` falls on `share (V_u / n) (1 - (V_u / V)^n)`, and its pressure is `E s`. The walls' store holds `E s^2 / 2` at each blood site. Above `V_u`, where `ds/dV` is one, that is the work the pressure does to fill the wall. Below `V_u`, `ds/dV` is `share (V_u / V)^(n + 1)`, so `E s^2 / 2` is not the integral of the pressure over the volume, and the difference leaves as heat.

**What was measured.** The plan of native-contract 42's part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, item 5) ran the slice head-down at trunk -0.2 on `ng_solver::Core::new` for 61 s on the beelink. At the run's end the leg veins hold 9.2 in the store against 126.3 for `integral E s dV` from `V_u`, and the splanchnic veins 6.7 against 90.1. At trunk -0.5: 124.7 against 891.2, and 48.0 against 427.2. Heat over the run is 1,035,380 in the slice's energy units, so no figure in the ledger moves past its bound, but the store is the wrong one. The store is positive and a vein's part grows as it empties (`tests/veins.rs`, `below_its_unstressed_volume_a_marked_veins_part_of_the_store_grows`, with the rows `store-falls-below` and `store-negative-below`), so its sign is right.

**What to build.** Below `V_u` the store holds the pressure's potential, `E share (V_u / n) ((V - V_u) - (V_u / (1 - n)) ((V / V_u)^(1 - n) - 1))`, added through the same site term, so a vessel off the veins keeps its bytes.

- [ ] A closed-form test of the store below `V_u` at a marked vein, against the integral of `E s` from `V_u`, and a mutant row that puts `E s^2 / 2` back
- [ ] The ledger closes over a head-down run at trunk -0.2 and -0.5, and the PR states heat and muscle work as figures beside main's
- [ ] The PR lists every pin that moves, with one sentence each, and measures the day again if the model hash moves

This lands before the scored arena pick, or the pick's record names the model hash it scored.

**Non-goals:** the collapse branch's pressure, which native-contract 42 holds. The power's form, which is [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md).
