# 80: The landing's tests and its bound hold more

Spec: `crates/ng-contract/src/boundary.rs` (`land`), `crates/ng-contract/src/problem.rs` (`landing_guard`) and `crates/ng-contract/src/landing.rs` (`Bracket`)

Type: task

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours. [Native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md) made the landing read the crossed guard whose root comes first, so a step that crosses two guards lands in about 3 calls. The check of nightingale#196 found mutants that no test sees and a bound that is looser than it was (`~/lanes/logs/ng-wave-4/nc64/verify-coordinator/VERIFY-nc64.md`, items 2, 4 and 8).

**What the check found.**

- `same` always false is not seen. The case `[C(1,0.5), S(-0.37)]` lands at 586,506,683 ns in 6 solves, and at 585,958,360 ns under the mutant.
- A width that reads the top's whole length while a bottom stands is not seen.
- A NaN gap at the bottom, with the top past zero, gives a NaN root. It is kept when it is the first candidate, and then no later root takes its place.
- A restart clears the bracket's stall rule. A landing's trials are then bounded by the span over the floor, 101 on the body's 10 ms step, where the bracket alone stops at 3 × 64.
- The bracket's secant multiplies before it divides, as the root did before native-contract 64.

- [ ] A test sees `same` always false, with its mutant row
- [ ] A test sees the width read with a bottom standing, or the Answer shows it cannot matter
- [ ] A NaN gap at the bottom no longer holds the pick
- [ ] The stall rule is carried across a restart, or a bound below the span over the floor is shown
- [ ] The bracket's secant divides before it multiplies, with each pin that moves and its sentence

**Non-goals:** which guards exist, the accuracy gate, and the floor.
