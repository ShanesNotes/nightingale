# 04: Record transaction with a backward Euler reference core

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A session steps the two-reservoir body: the reference core solves, the quadrature and stage witness build the proposal, the record validates and publishes or refuses, and the certificate closes over the run.

Blocked by: 03

Status: ready-for-agent

- [ ] The two-reservoir case with a stepped exterior inflow matches its closed form within the accuracy policy
- [ ] A forward Euler core with h > RC closes every ledger and is rejected by the accuracy gate
- [ ] A forced refusal leaves the accepted image, pending inputs and stream positions byte-identical; the retry applies the delivery once
- [ ] A core that books one sample while its endpoint came from other stages faults with `MethodMismatch`
- [ ] Compensated amounts reach the certificate; closure holds over every sliding window of the run

## Comments

From 03: the compiled body keeps, private to `ng-contract`, what the reference core needs. That is each program's instructions with inputs by declared name (`law.rs`, `Op` and `Input`), and each instance's bound inputs (`body.rs`, `Bound`: a parameter slot, an unknown, or a schema boundary's drive in name order). It also keeps the parameter layout with default bindings, the modes, the guards and the validity ranges. Evaluation is this ticket's, with the pinned `exp` and `ln`, the square root inside smooth max and min, the derivatives and `CompiledBody::refusal_sources` (a divisor, a logarithm's argument and each row's admissible set). The two-reservoir body is `crates/ng-contract/tests/reservoirs.rs`. Its closed form under a stepped inflow is in the arena's base design (`~/nightingale-evidence/nc03-declaration/architect/candidate-codex/design.md`, line 78). A block is the strongly connected part of the row graph after matching, with the fastest rate class of its laws. It is structure, never permission to lag a dependency. Sketch probes this ticket owns: lines 25, 54, 149, 166, 177 and 309.
