# 26: The arena picks a core on short runs and one day, and the winner runs 48 h

Spec: [Arena specification](../../../docs/architecture/solver-contract.md#7-arena-specification), the gates for each core

Type: task

Status: open

Blocked by: the statecharts PR (agreed order: statechart first). The runs this ticket enables also need a synthetic 24 h energy drift limit for the slice fixture (`.scratch/foundation/issues/01-arena-and-milestone.md`, comments from native-contract 24): until it exists every core fails the certificate on energy, so no scored run starts before it.

Decided (Shane, 2026-11-07): the arena picks a core from the short gates plus one 24 h run, then the winner runs 48 h once as the evidence for SRS invariant I19 (record 0004). The 48 h run stops being a gate every seat pays.

**Why.** The 48 h run is I19's check: drift over every sliding 24 h window needs 48 h, because 24 h windows starting at every time of day span two days. It barely tells two cores apart. Closure, accuracy, replay, step policy, late admission and rewind already run on short windows; speed needs one run that covers a whole day's phases (sleep, meals, posture); and drift mostly judges the body, which every core shares. A full protocol per seat costs about 40 h of tower time and 296 to 342 GB of history after native-contract 25.

**What to build:**

- Section 7's gates for each core: the slice and certificate gate runs 24 continuous hours from 06:55, one window, with the same coverage manifest and held support per window. The speed bar stays 100 times real time, so at most 864 wall seconds for 24 h. A second bullet says the picked core then runs 48 h once, every sliding 24 h window, as I19's evidence, and that a core failing it reopens the pick. Keep the edit to the lines that state the old rule. Usage (c) of the contract ("The arena scores a 48 h run") and the sketch's doctest follow.
- `crates/ng-validate/src/arena.rs`: `ArenaProtocol` gains the pick protocol, 24 h from 06:55 with one window and two repeats, at the same scales and minimum interval, and `full()` stays as the 48 h milestone protocol. Name both in the glossary's words (**Arena protocol**); a new term lands in `GLOSSARY.md` in this PR.
- The seat contract (`~/lanes/briefs/ng-arena-seat-contract.md`, outside the repo) scores seats on the pick protocol and states the 48 h run's cost for the winner only.

- [ ] Section 7 and usage (c) say the pick runs 24 h and the winner runs 48 h once, and the sketch still compiles every doctest.
- [ ] `ArenaProtocol` has a 24 h pick protocol beside the 48 h one, each with a test that reads its windows, coverage and repeats, and the arena refuses a core off either protocol as it does today.
- [ ] No 24 h or 48 h run is started by this ticket. The proof is the gate's tests on short spans; the cost of each protocol is stated by arithmetic from native-contract 25's measured bytes and ticket 21's measured speed.

**Non-goals:** setting the energy drift limit; running the arena; changing any other gate in section 7.

## Comments
