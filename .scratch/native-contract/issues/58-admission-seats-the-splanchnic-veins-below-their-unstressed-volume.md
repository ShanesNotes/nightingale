# 58: Admission seats the splanchnic veins below their unstressed volume

Spec: the slice's packet in `crates/ng-validate/src/slice.rs` (the walls' table and the start guess), the admission solve, and [native-contract 42](42-veins-collapse-branch.md)'s Answer

Type: research

Status: open

Blocked by: none

**Question:** Why does admission seat the slice's splanchnic veins about 10 mL below their unstressed volume at rest, supine, and should the slice's packet and the whole body's seat them above it?

**Why.** The first milestone (SRS invariant I19) needs a man admitted at rest in his own balance. A resting vein below its unstressed volume is partly collapsed. With native-contract 42's collapse branch that state is no longer inert: the branch acts at rest, not only head-down, and it is the only vein that does.

**What is known.** The plan of native-contract 42's part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, items 1 and 2) read every run the slice's tests make. Supine, the splanchnic veins sit 10.2 mL below `V_u` at their least and below it for 36.4 s of 50, with the four veins marked. With no vein marked, as on main before part 3, they sit 11.3 mL below, for 40 of 50 s. Lowering their unstressed volume from 1,064 to 1,034 mL moved their water down with it and left them 40 mL below, so admission follows `V_u` and does not seat them above it. The other three veins stay above at rest: the central veins 8.6 mL at their least seated, the pulmonary vein 23 mL, the leg veins 3.7 mL. Marked alone, the splanchnic veins carry both changes part 3 made to the slice's tests (the seated swing and the window's 1 ms core).

- [ ] What in the admission solve or the packet sets the splanchnic veins' volume: the start guess, the abdominal pressure, the venous junction's effect at rest, or the fixture's numbers. One change to that input moves the volume as the cause predicts
- [ ] Whether a healthy adult's splanchnic veins at rest supine are above their unstressed volume, with a source, or that no source on hand says
- [ ] A recommendation for the slice's packet and for the whole body's, and the pins it would move

**Non-goals:** changing a law or a number in this ticket. The collapse branch's form.
