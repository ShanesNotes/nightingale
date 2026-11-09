# 26: The arena picks a core on short runs and one day, and the winner runs 48 h

Spec: [Arena specification](../../../docs/architecture/solver-contract.md#7-arena-specification), the gates for each core

Type: task

Status: resolved

Blocked by: the statecharts PR (agreed order: statechart first). The runs this ticket enables also need a synthetic 24 h energy drift limit for the slice fixture (`.scratch/foundation/issues/01-arena-and-milestone.md`, comments from native-contract 24): until it exists every core fails the certificate on energy, so no scored run starts before it.

Decided (Shane, 2026-11-07): the arena picks a core from the short gates plus one 24 h run, then the winner runs 48 h once as the evidence for SRS invariant I19 (record 0004). The 48 h run stops being a gate every seat pays.

**Why.** The 48 h run is I19's check: drift over every sliding 24 h window needs 48 h, because 24 h windows starting at every time of day span two days. It barely tells two cores apart. Closure, accuracy, replay, step policy, late admission and rewind already run on short windows; speed needs one run that covers a whole day's phases (sleep, meals, posture); and drift mostly judges the body, which every core shares. A full protocol per seat costs about 40 h of tower time and 296 to 342 GB of history after native-contract 25.

**What to build:**

- Section 7's gates for each core: the slice and certificate gate runs 24 continuous hours from 06:55, one window, with the same coverage manifest and held support per window. The speed bar stays 100 times real time, so at most 864 wall seconds for 24 h. A second bullet says the picked core then runs 48 h once, every sliding 24 h window, as I19's evidence, and that a core failing it reopens the pick. Keep the edit to the lines that state the old rule. Usage (c) of the contract ("The arena scores a 48 h run") and the sketch's doctest follow.
- `crates/ng-validate/src/arena.rs`: `ArenaProtocol` gains the pick protocol, 24 h from 06:55 with one window and two repeats, at the same scales and minimum interval, and `full()` stays as the 48 h milestone protocol. Name both in the glossary's words (**Arena protocol**); a new term lands in `GLOSSARY.md` in this PR.
- The seat contract (`~/lanes/briefs/ng-arena-seat-contract.md`, outside the repo) scores seats on the pick protocol and states the 48 h run's cost for the winner only.

- [x] Section 7 and usage (c) say the pick runs 24 h and the winner runs 48 h once, and the sketch still compiles every doctest.
- [x] `ArenaProtocol` has a 24 h pick protocol beside the 48 h one, each with a test that reads its windows, coverage and repeats, and the arena refuses a core off either protocol as it does today.
- [x] No 24 h or 48 h run is started by this ticket. The proof is the gate's tests on short spans; the cost of each protocol is stated by arithmetic from native-contract 25's measured bytes and ticket 21's measured speed.

**Non-goals:** setting the energy drift limit; running the arena; changing any other gate in section 7.

## Answer

- **Two protocols, one struct.** `ArenaProtocol::pick()` (24 h from 06:55, one 24 h window, two repeats) and `ArenaProtocol::milestone()` (every 24 h window over 48 h, two repeats; `full()` renamed to the glossary's word) are one line each over a private builder, so they share the scales, the interval, the window and the repeats and differ only in coverage. Which one a score ran is `score.protocol()`. The bench takes `pick` or `milestone`. **Pick protocol** and **Milestone protocol** are in the glossary.
- **The certificate checks one window.** `SlidingWindows::new` refused a coverage equal to its window, and `certify`'s walk checked no window on such a history. Now a coverage of one window is the one window from admission, however far past it the last step reaches, so every core is judged on the same window; a longer coverage slides as before. The walk reads the window's ends through `Curve::on` on the segments holding them (a private `Stretch`), with no new public read. Tests in `crates/ng-validate/tests/transaction.rs` (`a_history_one_window_long_is_certified_on_its_one_window`, `a_coverage_of_one_window_is_the_window_from_admission_however_far_the_history_runs`), the oracle in `certificate_oracle/mod.rs`, which reads the window by index, and a two-second slice window with its held support in `slice_certificate.rs`. Two mutant rows hold the end read and the admission start.
- **Off either protocol is refused whole** before a core steps (`a_core_off_either_protocol_is_refused_whole_before_it_steps`, with a core that panics if stepped).
- **Cost by arithmetic** (`ng_solver::Core` at 10 ms on the tower, ticket 21's 6.808 times real time, native-contract 25's 9,321 bytes a step at 184 to 212 steps a simulated second): the pick is about 15 h a seat (timed run 2 x 3.53 h, day checks 7.9 h) and 148 to 171 GB; the milestone run about 40.5 h and 296 to 342 GB, once. n seats cost about 15n + 40.5 h instead of 40.5n. At the speed bar the pick takes about 1.1 h. No 24 h or 48 h run was started.
- **Design.** pstack architect: an Opus and a Codex runner, judged by Codex. The judge picked the Codex candidate's rule (one window from admission, because a run may step past its end) and the Opus candidate's private end reads; the rename to `milestone()` follows the glossary rule over the ticket's "`full()` stays". Evidence: `~/lanes/logs/ng-nc-26/`.

## Comments
