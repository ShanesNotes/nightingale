# 57: Below its unstressed volume a vein's elastic store is not the work of its pressure

Spec: `crates/ng-cardiovascular/src/walls.rs` (`Walls::elastic_energy` and the term `vein collapse`), and [native-contract 42](42-veins-collapse-branch.md)'s Answer

Type: task

Status: resolved

Claimed by: nc57

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the reference man for 48 hours, and the walls' energy ledger is one of the books the run is judged on. Native-contract 42 gave the four veins a collapse branch: below its unstressed volume `V_u` a vein's stressed volume `s` falls on `share (V_u / n) (1 - (V_u / V)^n)`, and its pressure is `E s`. The walls' store holds `E s^2 / 2` at each blood site. Above `V_u`, where `ds/dV` is one, that is the work the pressure does to fill the wall. Below `V_u`, `ds/dV` is `share (V_u / V)^(n + 1)`, so `E s^2 / 2` is not the integral of the pressure over the volume, and the difference leaves as heat.

**What was measured.** The plan of native-contract 42's part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, item 5) ran the slice head-down at trunk -0.2 on `ng_solver::Core::new` for 61 s on the beelink. At the run's end the leg veins hold 9.2 in the store against 126.3 for `integral E s dV` from `V_u`, and the splanchnic veins 6.7 against 90.1. At trunk -0.5: 124.7 against 891.2, and 48.0 against 427.2. Heat over the run is 1,035,380 in the slice's energy units, so no figure in the ledger moves past its bound, but the store is the wrong one. The store is positive and a vein's part grows as it empties (`tests/veins.rs`, `below_its_unstressed_volume_a_marked_veins_part_of_the_store_grows`, with the rows `store-falls-below` and `store-negative-below`), so its sign is right.

**What to build.** Below `V_u` the store holds the pressure's potential, `E share (V_u / n) ((V - V_u) - (V_u / (1 - n)) ((V / V_u)^(1 - n) - 1))`, added through the same site term, so a vessel off the veins keeps its bytes.

- [x] A closed-form test of the store below `V_u` at a marked vein, against the integral of `E s` from `V_u`, and a mutant row that puts `E s^2 / 2` back
- [x] The ledger closes over a head-down run at trunk -0.2 and -0.5, and the PR states heat and muscle work as figures beside main's
- [x] The PR lists every pin that moves, with one sentence each, and measures the day again if the model hash moves

This lands before the scored arena pick, or the pick's record names the model hash it scored.

**Non-goals:** the collapse branch's pressure, which native-contract 42 holds. The power's form, which is [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md).

## Answer

Below `V_u` a marked vein's wall now holds `E share Psi`, the ticket's closed form, read at `V_u + smooth_min(V - V_u, 0, w)` so it fades above `V_u` and the store joins `E s^2 / 2` there with its slope. Two site terms on the veins add it in `Walls::stored`: `vein collapse store` at the elastance, for the elastic energy, and `vein collapse stroke work` at the contractile elastance, so stroke work reads the same store. A term cannot take the stiffness as an input, and folding `E` in after the add would reorder every unmarked site's ops. The form divides by `(n - 1) V_u`, so a power of one or less refuses at a marked vein by the term's name. Its smoothed volume reaches zero at `w^2 / (4 V_u)`, where the term refuses. With a power above one the store grows without bound as the vein empties. A volume of zero or below refuses as part 3 left it, naming `CV/vein collapse`.

**The tests.** `tests/veins.rs` sums the vein's own staged pressure row by Simpson's rule from `V_u` and holds the store's growth to it at `V_u / 2` and `V_u / 4` (`below_its_unstressed_volume_a_marked_veins_store_is_the_work_of_its_pressure`), and at `V_u - 10` and `V_u + 10` mL with the store's slope against the pressure (`at_its_unstressed_volume_a_marked_veins_store_and_its_slope_join_the_store_above`). Both run at the central veins and at the leg veins, which a nerve reaches. On main the first failed with the store at 16.70 against the work 98.38 at 150 mL of the central veins, and the second with 0.0011 against 0.174 at 290 mL. They replace `below_its_unstressed_volume_a_marked_veins_part_of_the_store_grows`, which held only that `E s^2 / 2` grew, and its two rows. Four rows hold the new store: `store-squared-below` puts `E s^2 / 2` back, `store-negative-below` flips the term's sign, `store-factor-where-a-nerve-reaches` scales the term by `1 + gain / V_u`, which is one at the central veins, and `store-read-at-the-volume-itself` drops the smoothed volume. Each is caught on a value.

**The ledger.** Head-down on `ng_solver::Core::new` from 1 s, as part 3's plan ran it, the store stays on its sum within 8.1e-8 of the store at every step, on main and here. At trunk -0.2 to 61 s heat is 1,035,181.84 against main's 1,035,380.08 and muscle work 19.308 against 19.268. At trunk -0.5 to 121 s heat is 3,102,683.26 against 3,103,842.48 and muscle work 32.019 against 31.578. At the end of the -0.2 run the leg veins' part is 126.27 against 125.76 for the work of `E s` from `V_u`, and the splanchnic veins' 90.10 against 89.96 (main 9.24 and 6.67). At -0.5: 891.14 against 890.55, and 427.16 against 427.00 (main 124.66 and 48.03).

**The pins.** The whole body marks no vein and keeps every pin. The slice's model hash moves from `42c60687` to `93c12728`, its three first-second packet hashes and seals move, and gp-driver's `SMOKE_PACKET` and `SMOKE_SEAL` follow the `ng_solver` run's. In the traces `wall power` moves on 27 lines. On three lines of the two `ng_solver` runs a step end moves by a few ns, and on one of them the arterial pressure and the aortic flow move in the last printed digit. The day ran again at this hash, from 06:55 to 06:55 on `Core::new` at 10 ms: energy changes by -164,356.35, 0.8218 of its 2e5 bound as at part 3 (-164,356.33), every species' change moves by less than 1e-7 of itself, and charge stays at its closure floor.

**The pick.** The scored pick runs at main `20f237380fe9`, before this change, so its record names the slice hash `42c60687`. Against that commit the slice's solution on `Core::new` moves at rounding only: over the first 60 s at rest both take 11,614 steps, blood volumes agree to 7.9 digits and blood pressures within 2.0e-6 mmHg, and nothing grows (the check of nightingale#168, `~/lanes/logs/ng-wave-4/nc57/verify-coordinator/VERIFY-pr168.md`, item 6).

**The check.** It left two tickets. [Native-contract 60](60-a-collapse-power-just-above-one.md): just above a collapse power of one the store is rounding, and no door names it. [Native-contract 61](61-the-vein-stores-tests-hold-more.md): what the store's two tests do not hold.

