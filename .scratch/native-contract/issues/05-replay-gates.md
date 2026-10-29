# 05: Replay gates: pace, resume, late admission, rewind

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The validation crate runs the manufactured body under each replay gate and reports digests. The gates are pace (unbounded, wall clock, chunked), resume into a freshly built core through stored checkpoint bytes and a history store, an input admitted mid-run, and an in-process rewind after the parent ran ahead.

Blocked by: 04

Status: ready-for-agent

- [ ] All four gates pass for the reference core, with identical accepted-state, ledger and event digests
- [ ] A core that keeps a cache outside its continuation fails resume and rewind
- [ ] A core cannot see the timeline (the contract clips each ticket); a core that clips at caller output times fails late admission
- [ ] `admit_input` refuses an input stamped before the accepted head
- [ ] Corrupted checkpoint bytes are refused before any body byte is decoded
