# 60: Just above a collapse power of one a vein's store is rounding, and no door names it

Spec: `crates/ng-cardiovascular/src/walls.rs` (`emptied`), [native-contract 57](57-the-store-below-a-veins-unstressed-volume.md) and [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md)

Type: task

Status: open

Blocked by: 54

**Why.** The first milestone (SRS invariant I19) is judged on the walls' energy ledger. Below its unstressed volume `V_u` a marked vein's store is the work of its pressure ([native-contract 57](57-the-store-below-a-veins-unstressed-volume.md)). Its closed form subtracts two nearly equal volumes and divides by `n - 1`, with `n` the body parameter `cv.collapse-power`. As `n` nears one from above, what is left of the store is rounding.

**What was measured.** The check of nightingale#168 staged the central veins from `V_u` to `V_u / 2` and set the store's growth beside a sum of the vein's own pressure (`~/lanes/logs/ng-wave-4/nc57/verify-coordinator/VERIFY-pr168.md`, item 9). The 0.14 between the two at every power is the soft floor's offset, which the tests allow.

| Power | The store's growth | The work of the pressure | What the doubles lost, of the store |
|---|---|---|---|
| 1.5 | 98.5181 | 98.3800 | 3e-14 |
| 1.01 | 87.1179 | 86.9805 | 6e-14 |
| 1 + 1e-6 | 86.9056 | 86.7683 | 1.0e-9 |
| 1 + 1e-9 | 86.9055 | 86.7683 | 6.7e-7 |
| One rounding step above one | -53.63 | 86.77 | 1.6 times it |

The loss is about 6e-14 / (n - 1). Both store tests pass from 1 + 3e-9 up, and the join test fails below that on its slope. No door names the cause. The bind admits any positive power. Below 1 + 1e-7 admission refused nine of the ten powers tried, as `Inconsistent` on the elastic energy row or on row 1, and it admitted 1 + 3e-9 with the store off by 5e-8 of itself. A power of one or less refuses at initialization by `CV/vein collapse store`. The slice binds 1.5, so no run of the slice is touched.

A failing test exists. A Codex break-it pass on nightingale#168 wrote `a_power_just_above_one_keeps_the_veins_store_on_its_pressure_integral`: it builds the slice on a packet of unit values and fails at a power one rounding step above one, where the store grew -0.499 against a work of 0.193. It is kept outside the repo, in `~/lanes/logs/ng-wave-4/nc57/breakit/vein-store-near-one.patch`.

**What to build.** One of two:

- The exponent leaves the packet, as [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md)'s first box asks. Written at three halves, the store has no divisor that can vanish, and this ticket closes with that one.
- A floor on the power that a door names. `Admissible` has four sets and none takes a number, so a floor at the bind is contract surface. The check's prototype states one inside the law instead: a second key, `cv.collapse-power-floor`, and a divisor `(n - 1 - floor) V_u` in `emptied`, which refuses at initialization by the term's name. With a floor of 1e-6 the store and the first second are unchanged to every printed digit at 1.5, 1.01 and 1 + 3e-6.

Either moves the slice's model hash, so the day is measured again.

- [ ] The Answer picks one, after native-contract 54's first box
- [ ] The failing test is in the repo and passes, or a door refuses its power and names the cause
- [ ] The law's doc says what the form does just above one
- [ ] The PR lists every pin that moves, and the day's table at the new hash

**Non-goals:** a power of one or less, which refuses by name today. Any other law's power.
