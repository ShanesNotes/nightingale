# 91: The accuracy gate holds no oscillation whose amplitude the step decides

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 7 (the accuracy gate and its must-fail table), `crates/ng-validate/src/accuracy.rs` and `crates/ng-validate/tests/accuracy.rs`

Type: task

Status: open

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

- [ ] A manufactured case holds a self-sustained oscillation whose amplitude has a closed form, and the gate compares the amplitude a core reaches with it under a declared tolerance
- [ ] The core built to fail it is honest: the halved step or the reference scheme at a cap past four times the growth rate over `w^2`. Every step passes `verify` and every ledger closes, as in the table above, and this case alone refuses it. Its row joins section 7's must-fail table and `tools/gate/must-fail.toml`
- [ ] The tolerance, and the step at which the reference itself passes, are stated with the damping they allow, so the case says how long a step a core may declare for an oscillation of a given period
- [ ] Whether any gate reads an exchange one way. `net_at` compares an exchange's forward less its reverse, and [native-contract 15](15-manufactured-accuracy-cases.md)'s comment says the two-reservoir case's gross inflow cannot fail. A solver exploration measured a gross exchange 50% high over one accepted 1 s step on a ramped drive (`~/lanes/logs/ng-solver-exploration-5fe4e63/`, read and not rerun). The Answer names each reader of a forward or a reverse exchange by itself, or says there is none

**Non-goals:** a new step plan, the cap's value, and any change to what step acceptance reads. [Body ticket 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures the plans on the body.
