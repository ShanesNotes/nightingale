# 36: The whole body's filtration and output answer posture the wrong way

Type: task
Status: open
Blocked by: none

**Why.** The slice's posture checks (`crates/ng-validate/tests/arena_body.rs`) had not been run on the whole body. [Ticket 09](09-tissues-respire.md)'s earn run ran them, from one admitted start, with means over 30 to 40 s. Standing, filtration rises to 2.0248 from 1.9979 flat, where the check asks it to fall by at most 5%. Seated, output rises to 81.13 from 78.10 flat while stroke volume falls, where the check asks both to fall, and filtration rises to 2.0411. Every other check passes in both postures: the pressure stands above supine (105.5 standing and 107.1 seated against 99.9), the rate rises, sympathetic traffic rises and stays under 0.8 (0.53 and 0.49), and nothing hunts (swing under 0.02). It was so before arrow 78 was wired, with the heart's work still paid at the receipt.

- [ ] Which held controller or missing law makes each answer, by a run with each candidate restored
- [ ] The posture checks run on the whole body as a test, and each check that cannot yet pass is named beside the ticket that will make it pass

## The certificate under a tilt

Every value is synthetic. One test, which is not in the suite, ran on main in two runs, one for each end time, and each result was measured once. It takes the leaking body, its held packet and its shut and open room branches from `at_a_quarter_the_body_passes_its_policy_and_its_certificate_refuses_each_leak_between_edges` in `crates/ng-validate/tests/body_run.rs`. It holds the drive scale at one and the breath rate at 0.20 a second. From one admission both branches take the same five posture steps, at 30, 38, 46, 54 and 62 s after the anchor moment, each lowering the trunk angle's sine by a fifth of 0.117, so the last ends at -0.117 with the legs horizontal. On one branch a vasopressin leak of +3e-3 a second runs from 60 s, and the patch sets the other three leak routes to zero while the body's own transfers go on.

**Established, measured once on main.**

- Ending at 70 s, the body with no leak passes the body's whole policy over 12 windows between edges, and the leaking body is certified through 70.011 s.
- Between the breath edges at 60.001 and 70.001 s, two breaths, vasopressin changes at -1.817e-3 a second with no leak and at +1.167e-3 a second with it, a net of +2.984e-3. The policy's edge rate for vasopressin is 2e-3 a second (`SPECIES_BOUNDS` in `crates/ng-validate/src/body/packet.rs`), and neither change passes it.
- Ending at 75 s, the body with no leak is itself refused, `EdgeDrift` on vasopressin, in the window between edges from 65.001 to 75.001 s: a change of 2.226e-2 against an allowance of 2.000e-2.
- The leaking branch ending at 75 s is not measured: the test stops at the refusal of the body with no leak.

**Not claimed:** a cause for either result, and a fix.

- [ ] On this protocol, the five steps with the drive scale held at one and the breath rate at 0.20 a second: on the run that ends 5 ms past 75 s, the body with no leak passes the body's whole policy, through the window between edges from 65.000947966 to 75.001047964 s that refuses it on main; and on the run that ends 5 ms past 70 s, the body with a vasopressin leak of +3e-3 a second from 60 s, its net within 5% of +3e-3 a second, is refused on vasopressin, in a window between edges that opens after the settling time. The policy's edges select vasopressin's rate by a window's opening edge: 5e-4 a second for a window that opens before the settling time of 20 s, and 2e-3 a second for one that opens after it (`SPECIES_BOUNDS` in `crates/ng-validate/src/body/packet.rs`, judged in `crates/ng-validate/src/certificate.rs`). How the policy does that is this ticket's to decide by measurement. If it needs a new kind of check in the policy, that is a design round of its own: this ticket says so and stops there

**A note on the lymph thresholds, from [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md)'s fix.** In `crates/ng-validate/src/body/packet.rs` the gut's and the liver's lymph thresholds are `LYMPH_OPENING - ABDOMINAL_PRESSURE`, -6, and the same named value `ABDOMINAL_PRESSURE` sets the abdomen's design pressure, the packet value `gi.abdominal-pressure`. The pulmonary and coronary beds' thresholds are `LYMPH_OPENING - PLEURAL_REST`, +1, over the chest's pressure at rest, which `the_chest_rests_at_the_pressure_its_beds_lymph_thresholds_are_stated_over` in `crates/ng-validate/tests/viscosity.rs` holds to the packet's own design. A change to the abdomen's design value through `ABDOMINAL_PRESSURE` moves the gut's and the liver's thresholds and their admitted lymph, though the lymph law no longer reads the abdomen's pressure. So this ticket states each threshold as its own value, or moves the two apart on purpose. The chest's pressure at rest could be derived from the lung's and the chest wall's mechanics in place of the stated `PLEURAL_REST`: [research 16](../../research/issues/16-constants-and-what-binds-them.md)'s rows RS-001 and RS-002 state those mechanics as `P_A - P_pl = (V - V_L0) / C_L` and `P_pl - P_bs = (V - V_W0) / C_W - P_mus`, and RS-003 and RS-005 derive `V_L0` and the chest wall's relaxed volume supine from the functional residual capacity and the pleural pressure there.
