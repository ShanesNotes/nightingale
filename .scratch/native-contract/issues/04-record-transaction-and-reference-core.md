# 04: Record transaction with a backward Euler reference core

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A session steps the two-reservoir body: the reference core solves, the quadrature and stage witness build the proposal, the record validates and publishes or refuses, and the certificate closes over the run.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] The two-reservoir case with a stepped exterior inflow matches its closed form within the accuracy policy
- [ ] A forward Euler core with h > RC closes every ledger and is rejected by the accuracy gate
- [ ] A forced refusal leaves the accepted image, pending inputs and stream positions byte-identical; the retry applies the delivery once
- [ ] A core that books one sample while its endpoint came from other stages faults with `MethodMismatch`
- [ ] Compensated amounts reach the certificate; closure holds over every sliding window of the run
