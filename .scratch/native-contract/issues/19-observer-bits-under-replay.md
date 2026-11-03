# 19: Replay gates compare observer bits

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Pace, resume, late admission and rewind compare observer digests as well as accepted state, ledger and events, as §7's pace and resume gate asks. The gates thread an observer through `advance`, `step_to`, `revision_to` and `traced`, and each gate's digests go in `GateEvidence::observed`.

Blocked by: None (can start immediately)

Status: ready-for-agent

- [ ] Pace, resume, late admission and rewind compare observer digests, and each rejects an observer built to differ while the accepted state matches

## Comments

From 07: only the non-perturbation gate attaches an observer today. Equal seals give equal views, so these comparisons can only catch an impure observer. Threading one through `advance`, `step_to`, `revision_to` and `traced` is about 40 lines, and `GateEvidence::observed` is where the digests go. A delayed channel holds `delay / (period × window)` readings in each observer checkpoint, so a 45 min delay on a 4 ms grid holds about 16 MB.
