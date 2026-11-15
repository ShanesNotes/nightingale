# 58: Admission seats the splanchnic veins below their unstressed volume

Spec: the slice's packet in `crates/ng-validate/src/slice.rs` (the walls' table and the start guess), the admission solve, and [native-contract 42](42-veins-collapse-branch.md)'s Answer

Type: research

Status: open

Blocked by: none

**Question:** Why does admission seat the slice's splanchnic veins about 10 mL below their unstressed volume at rest, supine, and should the slice's packet and the whole body's seat them above it?

**Why.** The first milestone (SRS invariant I19) needs a man admitted at rest in his own balance. A resting vein below its unstressed volume is partly collapsed. With native-contract 42's collapse branch that state is no longer inert: the branch acts at rest, not only head-down, and it is the only vein that does.

**What is known.** The plan of native-contract 42's part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, items 1 and 2) read every run the slice's tests make. Supine, the splanchnic veins sit 10.2 mL below `V_u` at their least and below it for 36.4 s of 50, with the four veins marked. With no vein marked, as on main before part 3, they sit 11.3 mL below, for 40 of 50 s. Lowering their unstressed volume from 1,064 to 1,034 mL moved their water down with it and left them 40 mL below, so admission follows `V_u` and does not seat them above it. The other three veins stay above at rest: the central veins 8.6 mL at their least seated, the pulmonary vein 23 mL, the leg veins 3.7 mL. Marked alone, the splanchnic veins carry both changes part 3 made to the slice's tests (the seated swing and the window's 1 ms core).

**On the whole body.** The body marks three veins since body 06: the central veins, the portal vein, which is the slice's splanchnic veins, and the leg veins. Its portal vein carries the splanchnic veins' wall values (elastance 0.1, `cv.unstressed` 1,064 mL, `cv.unstressed-gain` 800 mL), and admission seats it above its unstressed volume. At admission the venous junction's norepinephrine effect is 0.4216, so `V_u` is 726.7 mL and the portal vein holds 745.3 mL. Supine over the first minute, read every 0.1 s on `ng_solver::Core::new` at 10 ms, it stays at least 12.3 mL above `V_u`, the leg veins 7.4 mL and the central veins 55.2 mL, and no vein is below for any sample. The same reads with no vein marked differ by under 0.002 mL. So no body vein runs the collapse branch at rest, and the body's case does not show this ticket's fault. The body adds the liver bed downstream of the portal vein, so a cause in the junction's effect or the venous outflow is a guess this ticket's first box can test.

- [ ] What in the admission solve or the packet sets the splanchnic veins' volume: the start guess, the abdominal pressure, the venous junction's effect at rest, or the fixture's numbers. One change to that input moves the volume as the cause predicts
- [ ] Whether a healthy adult's splanchnic veins at rest supine are above their unstressed volume, with a source, or that no source on hand says
- [ ] A recommendation for the slice's packet and for the whole body's, and the pins it would move

**Non-goals:** changing a law or a number in this ticket. The collapse branch's form.
