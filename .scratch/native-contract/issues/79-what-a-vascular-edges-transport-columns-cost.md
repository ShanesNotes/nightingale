# 79: What a vascular edge's transport columns cost

Spec: [solver contract](../../../docs/architecture/solver-contract.md) and `crates/ng-validate/src/body/packet.rs` (the shunt, the body's newest vascular edge)

Type: prototype

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and what a step costs sets how long that takes. The check of nightingale#194 found that one new vascular edge costs the whole body more than its one flow (`~/lanes/logs/ng-wave-4/b10a/verify-coordinator/VERIFY-10a.md`, items 1, 10 and 12).

**What the check found.**

- The shunt, one new vascular edge, added one flow row and 33 columns to the whole body: 32 that carry blood's 31 solutes and its water, and one for gravity's work. It added 194 nonzeros.
- By the instructions a solve costs, it costs about 138.8 M a simulated second, 2.6% of the body's line. That is inferred from the solve count, not run at a fixed count.
- No species can be left off the edge without breaking the blood's composition across it.
- The body has 22 such edges, and the organs still to come add more.

- [ ] A count, from the compiled body, of how many of its columns are edges' transport columns and what share of a solve's instructions they take
- [ ] A prototype of a cheaper statement of an edge's transport that keeps every book, with solves by kind and the cost line on the whole body
- [ ] Replay stays exact, or the Answer says what it would take
- [ ] The Answer says whether the contract's layout would change, and if so asks for a design round

**Non-goals:** which species blood carries, and any change to a law.
