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

## Comments

From 03: the sketch probe at line 441 (a core cannot see the boundary timeline, E0599) is this ticket's, on the core API that ticket 04 builds.

From 04: `PatientRecord::digests` is what the gates compare, one digest per part, recomputed from the live fields. It reads no seal. `History` and `LedgerChunk` are private behind `ConservationView`, one chunk per accepted step, so fixed-time chunks and a history store change no reader; a chunk holds six compensated values per combination and four per column, so a run at small steps needs them. `PendingInputs` holds the sorted deliveries, a cursor and the active deliveries, and `admit_input` must insert after the cursor. `Session` has no `enqueue`, `checkpoint`, `rewind`, `recover` or `export`, and `FaultReport` can read `Session<Faulted>::{fault, refusals}`. `branch` is a plain digest of the run until branches exist. `CoupledSolver` carries no `Send + Sync`, so the must-fail core can hold a `Cell`. The probe at line 441 is unchanged, and its twin now asks `problem.ticket(span)`. A ticket's length does show a core where it was clipped, but only for the step that reaches that instant.

From the deep review of 02 to 04: the last sentence above undersells what a core can read. `problem.ticket(1 << 60)` from `next_step` returns a ticket ending at the next breakpoint, from any step, and nothing binds the core to that ticket, so the contract's "no core can see the timeline" does not yet hold. At 0 ns and at 16 s of the stepped inflow it reads 2,000 s. The late-admission gate still rejects a core that acts on it. Whether to close it by type, for example with a span the core commits to before the session issues a ticket, is this ticket's call, because it changes `CoupledSolver::next_step` and the sketch.

From the deep review of 02 to 04, on the same point: `MethodHistory::counters` now reads a continuation back, so the core that keeps a counter outside its continuation has a legal twin, `KeepsItsSpan` in `crates/ng-validate/tests/cores/stages.rs`.
