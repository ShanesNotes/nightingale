# 63: The sliding energy bound's tests hold more protocols

Spec: `crates/ng-validate/src/slice.rs` (`arena_certificate`, `SYNTHETIC_MILESTONE_ENERGY_DRIFT`), `crates/ng-validate/tests/arena_certificate.rs` and [native-contract 52](52-energy-fails-the-sliding-windows.md)

Type: task

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is judged on energy's drift over every sliding 24 h window, and native-contract 52 gave energy 4e6 for any protocol whose coverage is longer than its window. The check of nightingale#171 ran 20 mutants of its own through the two energy tests: 14 were caught, and two of the six that survived show what no test holds (`~/lanes/logs/ng-wave-4/nc52-build/verify-coordinator/VERIFY-pr171.md`, item 3).

- **Between one and two windows.** The arm gives 4e6 to a coverage just past one window, and the test runs only the milestone's two. Its mutant m19, "sliding only from two windows up", survives.
- **`Book::Energy`.** The arm is `Book::Of(Species::Energy) | Book::Energy`, and no test's body has energy as `Book::Energy`, which a body with a reaction takes. Its mutant m12, the sliding bound on `Book::Of(Species::Energy)` alone, survives. It was so before native-contract 52.
- **Windows that are not 24 h.** A sliding protocol with shorter windows, such as the whole body's run test (10 s over its first minute) or `tests/arena.rs` (1 s over 2 s), takes a bound measured on 24 h windows. Over seconds that bound reads nothing.

- [ ] A test holds energy's bound for a coverage between one and two windows
- [ ] A test holds the `Book::Energy` half of the arm, on a body whose energy is that quantity
- [ ] A sliding protocol whose window is not 24 h takes a bound stated for its window, or §7 says why the 24 h one stands
- [ ] Each new test has a mutant row in `tools/gate/mutants.toml`

**Non-goals:** the bound's value; the pick's protocol; native-contract 62's admission.
