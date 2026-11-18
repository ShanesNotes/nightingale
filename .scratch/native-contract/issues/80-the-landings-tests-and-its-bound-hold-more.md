# 80: The landing's tests and its bound hold more

Spec: `crates/ng-contract/src/boundary.rs` (`land`), `crates/ng-contract/src/problem.rs` (`landing_guard`) and `crates/ng-contract/src/landing.rs` (`Bracket`)

Type: task

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours. [Native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md) made the landing read the crossed guard whose root comes first, so a step that crosses two guards lands in about 3 calls. The check of nightingale#196 found mutants that no test sees and a bound, inferred from the code, that is looser than it was (`~/lanes/logs/ng-wave-4/nc64/verify-coordinator/VERIFY-nc64.md`, items 2, 4 and 8). Its second check and the second `/no-comments` pass found a test doc that no longer fits `land` and two gaps the root does not place on its line (`VERIFY-nc64-delta.md` in the same folder, item 3, and `~/lanes/logs/ng-wave-4/nc64/no-comments.md`).

**What the checks found.**

- `same` always false is not seen. The case `[C(1,0.5), S(-0.37)]` lands at 586,506,683 ns in 6 solves, and at 585,958,360 ns under the mutant.
- A width that reads the top's whole length while a bottom stands is not seen.
- A NaN gap at the bottom, with the top past zero, gives a NaN root. It is kept when it is the first candidate, and then no later root takes its place.
- A restart clears the bracket's stall rule. So, inferred from the code and shown by no run, a landing's trials are bounded by the span over the floor plus one, 101 on the body's 10 ms step at a 100 µs floor, where the bracket alone stops at 3 × 64. The most any run of the checks took is 9.
- The bracket's secant multiplies before it divides, as the root did before native-contract 64.
- `landing.rs`'s doc on `a_crossing_the_gap_does_not_show_still_closes_on_it` says a crossed trial can report a gap below zero. `land` no longer reaches that state: `landing_guard` decides whether a trial crossed, and the guard a crossed trial is noted on, the same one or the one a restart reads, has its gap at or past zero there. The test still holds that `Bracket` alone, told of a crossing with a gap below zero, closes within its trial cap with the root between its ends.
- A gap of negative infinity at the line's bottom gives a root that is not a number, as a NaN gap does. `landing_guard` keeps that guard when it is the first candidate, and then no later root takes its place, since `root + floor < first` is false against it. After a finite pick it never takes the place.
- When the two gaps' difference overflows, the root is 0. That happens for a finite bottom below zero and a finite top at or past zero whose difference, top less bottom, rounds past the largest finite `f64`, about 1.8e308. The landing then reads the guard as one at its crossing at the bottom: it takes the place when it is the first candidate or when the pick's root lies more than a floor past the bottom, and no later root takes it back. Its bracket's first trial goes to the middle, because the bracket's slope overflows the same way.

- [ ] A test sees `same` always false, with its mutant row
- [ ] A test sees the width read with a bottom standing, or the Answer shows it cannot matter
- [ ] A NaN gap at the bottom no longer holds the pick
- [ ] The stall rule is carried across a restart, or a bound below the span over the floor is shown
- [ ] The bracket's secant divides before it multiplies, with each pin that moves and its sentence
- [ ] The doc on `a_crossing_the_gap_does_not_show_still_closes_on_it` says what the test still holds for `Bracket` alone, or the test goes
- [ ] A test stages a gap of negative infinity at the bottom, or a refusal of such a gap before `land` is shown
- [ ] A test stages two gaps whose difference overflows, or a refusal of such gaps before `land` is shown

**Non-goals:** which guards exist, the accuracy gate, and the floor.
