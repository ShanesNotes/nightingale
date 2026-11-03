# 17: The slice carries a hazard and a forcing

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice gets the stochastic fixture that §7 asks for: one declared hazard law with its own stream, and one forcing, both synthetic and both model defaults. Ticket 18 runs them through the pace and resume gates, so replay with variability is proven before a core is chosen.

Blocked by: None (can start immediately)

Status: ready-for-agent

- [ ] The slice declares one hazard law with its own stream and one forcing, both synthetic, and a seeded slice run fires the hazard and reads the forcing

## Comments

From the split of ticket 11: the slice has neither today. `crates/ng-validate/src/slice.rs` declares no hazard and no forcing. Both laws move the slice's trace, so the trace update says why. A new law on the slice can mean retuning `table`. Ticket 21 carries that lead.

From 06: each rate a forcing varies records its rate class (finding 17). On `ChanceCase` an honest 1 s policy stays within 500 ms of a reference at 1/16 s.
