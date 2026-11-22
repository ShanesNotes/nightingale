# 36: The whole body's filtration and output answer posture the wrong way

Type: task
Status: open
Blocked by: none

**Why.** The slice's posture checks (`crates/ng-validate/tests/arena_body.rs`) had not been run on the whole body. [Ticket 09](09-tissues-respire.md)'s earn run ran them, from one admitted start, with means over 30 to 40 s. Standing, filtration rises to 2.0248 from 1.9979 flat, where the check asks it to fall by at most 5%. Seated, output rises to 81.13 from 78.10 flat while stroke volume falls, where the check asks both to fall, and filtration rises to 2.0411. Every other check passes in both postures: the pressure stands above supine (105.5 standing and 107.1 seated against 99.9), the rate rises, sympathetic traffic rises and stays under 0.8 (0.53 and 0.49), and nothing hunts (swing under 0.02). It was so before arrow 78 was wired, with the heart's work still paid at the receipt.

- [ ] Which held controller or missing law makes each answer, by a run with each candidate restored
- [ ] The posture checks run on the whole body as a test, and each check that cannot yet pass is named beside the ticket that will make it pass

## The certificate under a tilt

Every value is synthetic. One test, which is not in the suite, ran once on main. It takes the leaking body, its held packet and its shut and open room branches from `at_a_quarter_the_body_passes_its_policy_and_its_certificate_refuses_each_leak_between_edges` in `crates/ng-validate/tests/body_run.rs`. It holds the drive scale at one and the breath rate at 0.20 a second. From one admission both branches take the same five posture steps, at 30, 38, 46, 54 and 62 s after the anchor moment, each lowering the trunk angle's sine by a fifth of 0.117, so the last ends at -0.117 with the legs horizontal. On one branch a vasopressin leak of +3e-3 a second runs from 60 s, and every other route is zero.

**Established, measured once on main.**

- Ending at 70 s, the body with no leak passes the body's whole policy over 12 windows between edges, and the leaking body is certified through 70.011 s.
- Between the breath edges at 60.001 and 70.001 s, two breaths, vasopressin changes at -1.817e-3 a second with no leak and at +1.167e-3 a second with it, a net of +2.984e-3. The policy's edge rate for vasopressin is 2e-3 a second (`SPECIES_BOUNDS` in `crates/ng-validate/src/body/packet.rs`), and neither change passes it.
- Ending at 75 s, the body with no leak is itself refused, `EdgeDrift` on vasopressin, in the window between edges from 65.001 to 75.001 s: a change of 2.226e-2 against an allowance of 2.000e-2.
- The leaking branch ending at 75 s is not measured: the test stops at the refusal of the body with no leak.

**Not claimed:** a cause for either result, and a fix.

- [ ] Under a posture change the body's certificate neither refuses the body with no leak nor certifies a leak of a size [native-contract 85](../../native-contract/issues/85-the-certificates-windows-end-on-breath-edges.md) states, such as vasopressin's 3e-3 a second. How the policy does that is this ticket's to decide by measurement. If it needs a new kind of check in the policy, that is a design round of its own: this ticket says so and stops there
