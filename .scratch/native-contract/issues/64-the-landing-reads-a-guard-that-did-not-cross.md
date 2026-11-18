# 64: The landing reads a guard that did not cross

Spec: `crates/ng-contract/src/boundary.rs` (`land`) and `crates/ng-contract/src/problem.rs` (`crossed`)

Type: task

Status: resolved

Claimed by: nc64-build

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and each pass 1 loop may add 12% of the user instructions a simulated second it starts from ([body 26](../../body/issues/26-cost-budget.md)). The check of nightingale#172 found the body's cost up 5.0%, from 5,960.2 to 6,257.2 M a simulated second, and the rise is not the laws' work. It is extra landing solves on one step a beat (`~/lanes/logs/ng-wave-4/b06-veins/verify-coordinator/VERIFY-pr172.md`, item 6). While it stands, a loop's cost line moves with the landing by about 5% either way.

**What the check found.**

- The landing (`boundary.rs`, `land`) re-solves shorter tickets to land a crossing. It reads one guard's gap: the first armed guard, in layout order, whose gap is at or past zero at the trial's end (`problem.rs`, `crossed`).
- On the whole body, 59 steps of the first minute carry the extra solves, one a beat from the second beat: at 1.279 s, then every 0.998 to 0.999 s. Each is the step that crosses the end of systole, about 280 ms into the beat.
- With the pulmonary vein unmarked, that step lands in 3 calls, and the secant aims at 2.09 ms, as the phase gap predicts. With the four veins marked, it takes 9 calls on the first such beat and 14 to 16 on the other 58, and the secant aims at 9.74 ms. So the landing reads a guard whose gap the crossing does not show, and the bracket stalls and halves.
- Those 59 steps hold 824 extra calls, and the rest of the run differs by -70, so the net is 754: 15,264 core solves against 14,510. Accepted steps (11,794 against 11,797) and refusals (725, with the same reasons) do not move.
- What switches it: the pulmonary vein's mark, whose collapse term's tail moves pressures by about 1e-5 mmHg. Which guard the landing reads is inferred, not run: most likely a valve guard whose gap sits near zero at that instant.
- Body 08's prototype ran with the knife-edge partly off: 2,109 landing solves against main's 2,745 (`~/lanes/logs/ng-wave-4/b08-plan/PLAN.md`, F8 and section 5). So its cost line moved with the landing, not only with its laws.

Inferred, not built: a landing that read the crossed guard whose root is earliest, or the guard that crossed inside the stages, would close in about 3 calls on this step.

- [x] A probe on main names the step and the guard the landing reads on it
- [x] The Answer names that guard and why its gap is at or past zero at the trial's end
- [x] The landing reads the guard that crossed, picked from two prototypes: the crossed guard whose root is earliest, and the guard that crossed inside the stages. Each reports solves by kind, ordinary and landing, on the slice and on the whole body
- [x] Replay stays exact, and each pin that moves has its sentence
- [x] A test fails on today's landing, with its mutant row
- [x] Cost lines before and after, on the slice and on the whole body

**Check.** The contract crate's: this ticket changes `ng-contract`.

**Non-goals:** which guards exist, and the accuracy gate.

## Answer

**The guard.** On the commit before body 10's first PR (`2e1dcb29`), the end-systole step spans about 9.87 ms and crosses two guards. The rhythm's systole-to-diastole guard crosses at 27% of the span. The leg-vein valve (leg veins to central veins, open to shut) crosses at 93%, with its gap going from -9.9e-3 at the base to +7.2e-4 at the end. The valve comes first in layout order, so the landing read it. Its gap is past zero at the end of the span because the valve shuts at the end of each beat, with a gap that lies close to zero at a shallow slope. The pulmonary vein's mark moves pressures by about 1e-5 mmHg, and that decides whether the valve's crossing falls inside the span or just after it. Inside the span, every trial where only the rhythm crossed shows the valve below zero. The bracket then stalls and halves, taking 14 to 16 calls on 58 beats. On main, the valve stays below zero at that step's end: the minute has one landing with two guards crossed, and layout order already reads the earlier root there. A probe of every landing named the guard (`~/lanes/logs/ng-wave-4/nc64/`, `probe-*.log`).

**The fix.** The landing reads the crossed guard whose root comes first. The root is placed on a straight line from the last trial that did not cross (the base at first) to the crossed end. Each trial that crosses reads the crossed guards again. Roots less than a floor apart keep stable order, hazards first, so a pair the floor cannot tell apart lands where layout order put it. Two prototypes were tried, the earliest root and the guard first seen in the stages. They agreed exactly: one seal, 14,510 solves, 1,988 landing solves on the dear commit. The earliest root was picked because it needs no change to the quadrature.

**What moves.** On main and on the slice, nothing moves. Each run is bit for bit the same as before: body seal `818f1809`, slice seal `a50fdfdb`, the same steps, solves and refusals. No pin moves. On the dear commit, 845 firings in 61 beats were none lost and none added, with the same order inside every beat. The beat drifts about 1.6 us a beat, reaching 128 us at 60 s. The reason is that the rhythm restarts at each landed instant. Two landings of one crossing can each sit anywhere inside the floor of 100 us, so they part beat by beat. Measured from the last landed rhythm crossing, every firing but one lands within the floor of today's. The exception is the leg valve's shut at 2.285 s, at 100.88 us. That is the knife-edge guard itself, whose shallow gap moves its root more than the state that moves it. Measured from the beat's start, two landings can part by up to two floors.

**Identity.** No declared version or rule names the landing. The declared resolution's event rule is its floor: a crossing is located to within it, and every landing still is. So no model hash, policy digest or run identity moves. A run saved before this change resumes on the new landing under the same identity, and on main and the slice it continues bit for bit.

**Cost.** Lines on the beelink, back to back in one job. On the dear commit the body falls from 5,386.6 M to 5,157.1 M a simulated second: solves go from 15,264 to 14,510, landing solves from 2,745 to 1,988, and steps from 11,794 to 11,797. On main the body moves from 5,174.3 M to 5,175.8 M and the slice from 902.6 M to 903.4 M, at the same counts. With the pulmonary vein's mark on and off, the end-systole step lands in 3 calls on every beat either way, and the minute takes 1,988 landing solves either way.
