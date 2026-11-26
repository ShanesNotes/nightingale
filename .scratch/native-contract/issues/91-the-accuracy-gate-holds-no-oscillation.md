# 91: The accuracy gate holds no oscillation whose amplitude the step decides

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 7 (the accuracy gate and its must-fail table), `crates/ng-validate/src/accuracy.rs` and `crates/ng-validate/tests/accuracy.rs`

Type: task

Status: resolved

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and its cost will press the step longer. A backward Euler step damps what it integrates. The halved step takes about `w^2 h / 4` a second off an oscillation of angular frequency `w` at step `h`, and a one-stage step twice that, so a growing oscillation decays once `h` passes four times its growth rate over `w^2`. No check reads it. Step acceptance reads the scheme's own second difference, which stays small on a smooth decay. Each manufactured case of the accuracy gate is a two-reservoir body, and none oscillates. The slice's windows are judged against the `ReferenceIntegrator`, which is backward Euler at three refinements: where all three steps are past that length, they agree on an oscillation that has died (inferred, not run).

Measured on a two-row fixture through `ng_solver::Core` on main at `b91ead22`, one run each (`~/lanes/logs/ng-explore-solver/probe.rs` and `probe.log`):

| Case | Cap | Read at the end | Exact | Refusals |
|---|---|---|---|---|
| Grows 0.01 a second, period 60 s, read at 600 s | 6 s | 0.0103 | 201.7 | 0 |
| The same | 0.1 s | 171.4 | 201.7 | 0 |
| Self-sustained, amplitude 1, period 60 s | 6 s | 1.6e-9 | 1 | 0 |
| The same | 3 s | 0.415 | 1 | 0 |
| The same | 1 s | 0.852 | 1 | 0 |
| The same | 0.1 s | 0.986 | 1 | 0 |

At the body's 10 ms cap the form gives 0.001 a second at 0.1 Hz, 0.1 at 1 Hz and 16 at 12.8 Hz, the ring [native-contract 32](32-valves-shut-inside-ejection.md) asks about (inferred from the form, not run on the body). The beat is paced by a phase that a guard resets, so it is not damped this way. What is at risk is a loop that oscillates by itself, such as a feedback with a delay, and any ring a law's inertance makes. The risk grows with the step.

**What to build.** One manufactured case in the accuracy gate, judged against its closed form as the two-reservoir case is, with the core built to fail it.

- [x] A manufactured case holds a self-sustained oscillation whose amplitude has a closed form, and the gate compares the amplitude a core reaches with it under a declared tolerance
- [x] The core built to fail it is honest: the halved step or the reference scheme at a cap past four times the growth rate over `w^2`. Every step passes `verify` and every ledger closes, as in the table above, and this case alone refuses it. Its row joins section 7's must-fail table and `tools/gate/must-fail.toml`
- [x] The tolerance, and the step at which the reference itself passes, are stated with the damping they allow, so the case says how long a step a core may declare for an oscillation of a given period
- [x] Whether any gate reads an exchange one way. `net_at` compares an exchange's forward less its reverse, and [native-contract 15](15-manufactured-accuracy-cases.md)'s comment says the two-reservoir case's gross inflow cannot fail. A solver exploration measured a gross exchange 50% high over one accepted 1 s step on a ramped drive (`~/lanes/logs/ng-solver-exploration-5fe4e63/`, read and not rerun). The Answer names each reader of a forward or a reverse exchange by itself, or says there is none

**Non-goals:** a new step plan, the cap's value, and any change to what step acceptance reads. [Body ticket 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures the plans on the body.

## Answer

- **Manufactured case (`AccuracyCase::SelfSustainedOscillation`).** `Oscillation::self_sustained()` (`crates/ng-validate/src/oscillation.rs`) joins `AccuracyCase::manufactured()` and `AccuracyPolicy::arena()`. Two dimensionless memory rows `x` and `y` obey the Stuart-Landau limit cycle `x' = (g - p (x^2 + y^2)) x + w y`, `y' = (g - p (x^2 + y^2)) y - w x` (`T = 60 s`, `w = 2 pi / 60`, `g = p = 0.01 / s`, start `(0.5, 0)`, end `1200 s`) while driving a closed two-vessel water transfer `q = c x` (`c = 1 / s`, `a(0) = b(0) = 500000`) so the body holds a conserved water ledger. The exact amplitude `r(t) = sqrt(g / (p + (g / r_0^2 - p) exp(-2 g t)))` grows monotonically from `0.5` toward limit-cycle amplitude `1`. At every compared instant `accuracy` compares `sqrt(x^2 + y^2)` against `r(t)` under the declared tolerance `0.1`, reported on `AccuracyEvidence::amplitude_tolerance()` and sealed into `AccuracyPolicy::digest()`.
- **Must-fail core.** Both `ng_solver::Core::new(6 s)` and `ReferenceIntegrator::new(6 s)` (cap `6 s`, past `4 g / w^2 = 3.648 s`) run the full `1200 s` with `0` refusals on `verify` and `0` closure error on the water ledger under `certify`, yet decay the amplitude to `1.82e-4` against exact `1`. Both pass all five two-reservoir manufactured cases and are refused by `SelfSustainedOscillation` alone (`AccuracyFailure::Exceeded`). Its row joins section 7's must-fail table and `tools/gate/must-fail.toml`, backed by mutant rows `oscillation-cap-below-damping-bound` and `oscillation-amplitude-unchecked` in `tools/gate/mutants.d/nc91-oscillation.toml`.
- **Tolerance, passing reference step, and allowed damping.** Holding steady amplitude within tolerance `tau = 0.1` of `1` (`r >= 0.9`) permits numerical damping up to `(2 tau - tau^2) g = 0.19 g = 0.0019 / s`. For halved/doubled backward Euler (damping roughly `w^2 h / 4` per second), that gives a necessary steady-state step bound of `h <= 4 * 0.19 g / w^2 = 0.76 g / w^2` (19% of the decay cutoff `4 g / w^2`, or about `0.693 s` at period `60 s`; half that for one-stage backward Euler). Because the gate judges the growth transient from `r_0 = 0.5` as well, the measured full-trajectory passing cutoff for halved/doubled backward Euler lies between `600 ms` (passes, worst error `0.09635`) and `625 ms` (fails at `156.25 s`, error `0.100007`). `ReferenceIntegrator` passes at `0.5 s` (damping `0.00137 / s`, steady amplitude `0.9289`, worst amplitude error `0.0799 <= 0.1`), and both `ReferenceIntegrator` and `ng_solver::Core` pass all six manufactured cases at `0.5 s` and show first-order convergence (`[1.9, 2.1]`) across `16 s / 8 s / 4 s` on the five two-reservoir cases and `0.5 s / 0.25 s / 0.125 s` on `SelfSustainedOscillation` (errors `0.0799`, `0.0395`, `0.0196`; ratios `2.02` and `2.01`).
- **One-way exchange readers across the codebase.** No gate judges or rejects a core on a column's `forward` or `reverse` exchange against continuous gross exchange by itself.
  - Every accuracy, split, and day gate reads only net (`forward - reverse`): `crates/ng-validate/src/accuracy.rs` (`net_at`), `crates/ng-validate/src/window.rs` (via `net_at`), `crates/ng-contract/src/split.rs` (`TraceDraw::advance`), and `crates/ng-validate/src/day/trace.rs` (`sodium_excreted_per_second`).
  - Four non-gate library and driver paths read `forward` and `reverse` separately, and none checks a step's quadrature accuracy against the continuous solution:
    - `crates/ng-contract/src/quadrature.rs` (`StepProposal::verify`): compares booked `extents.forward` and `extents.reverse` bit-for-bit against the core's own replayed stage evaluations to enforce `MethodMismatch`.
    - `crates/ng-record/src/history.rs` (`LedgerChunk::after`) into `crates/ng-validate/src/certificate.rs` (`pair`, `balance`): sums boundary `forward` and `reverse` into cumulative gross `inflow` and `outflow` per conserved quantity; `balance` uses `inflow - outflow` for closure and `inflow + outflow` only to widen the relative closure allowance (`absolute + gross * relative`).
    - `crates/ng-validate/src/certificate.rs` (`pair`): reads held-route `forward` and `reverse` separately into `HeldSupport` (`Certificate::support()`) for reporting only, with no failure bound.
    - `tools/generative-physiology/driver/src/capture.rs` (lines 595–599): writes each column's cumulative `forward` and `reverse` extents separately to the teacher capture stream without judging a solver.
  - Directional assertions in `crates/ng-validate/tests/{arena_body,burning,replay,slice_certificate,transaction}.rs` (including `slice_certificate.rs:147–164` bounding held-route `inflow`/`outflow` on `Certificate::support()`) are fixture and ledger unit tests, not solver accuracy gates.
