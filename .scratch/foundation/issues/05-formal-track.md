# 05: Formal proof track for the ledger

Type: research
Status: open
Blocked by: `.scratch/native-contract/issues/04-record-transaction-and-reference-core.md`

**Trigger.** Once the first slice merges with a real ledger in `ng-record`, run pstack `create-formal-verification-skill` from this ticket. It picks the tool, shows each property failing on a broken variant and leaves a project-local skill. Proofs run at tier 3: nightly and before a release.

**Properties.**

1. Every interior column of the compiled incidence `B` has exactly one +1 and one -1 per moiety, so conservation holds at every edge.
2. Across any sequence of accepted steps, pools plus receipt pools equal the start plus declared inflows, with no overflow.
3. Every applied extent debits one pool and credits another pool or a named receipt pool, with no one-ended write path.
4. A refused step leaves the state unchanged: `PatientRecord` has no interior mutability, and `publish`, the only `&mut`, cannot panic.

**Tools.** Kani model-checks the real Rust within bounds, for properties 1, 3 and 4 and overflow. Verus proves property 2 at every size, on a reference ledger. The pstack skill decides between them.

**Reference against fast.** A small reference ledger, a `BTreeMap` of integer amounts, carries the Verus proofs. Kani shows the dense update `B·J + S·ξ` matches the reference within the bound, and the traces (ticket 04) cover longer runs.

**Representation, open.** The contract stores compensated f64 pairs and keeps an i64 ledger with a fractional carry as its measured alternative. Property 2 is exact only on integers. Model recommendation: the i64 ledger, because it makes property 2 an exact, provable equality. Settle it before the arena scores conservation.

## Comments

Triage, 2026-10-29 (developer reviewer's call, delegated): ready. It proves the ledger properties behind SRS invariant I1 and the certificate (record 0004), and starts when its blocker, native-contract 04, lands.
