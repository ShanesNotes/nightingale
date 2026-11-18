# 81: A run's identity names no revision of the engine's numerics

Spec: `docs/srs.md` (the Event log entry) and `docs/architecture/solver-contract.md` (Identity, time and replay)

Type: research

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) rests on replay: a run is rebuilt from its identity and its events. The identity holds the model hash, the packet hash, the seed and the declared resolution. Nothing in it names the landing's rule or the contract's revision. So a change to the engine's own numerics can change a run that keeps its identity. The check of nightingale#196 found one (`~/lanes/logs/ng-wave-4/nc64/verify-coordinator/VERIFY-nc64.md`, item 9).

**What the check found.**

- [Native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md) changed which guard a landing reads. On main and the slice no run changed.
- On the body at `2e1dcb29`, a log made before the change in the stalled state departs when rebuilt after it, at its first stalled landing (1.2809 s). Replay then reports a divergence there instead of refusing up front.

- [ ] What replay promises across revisions of the engine
- [ ] The options, a revision in the identity or the promise stated within one revision, each with what it costs
- [ ] A design round if the identity's shape changes

**Non-goals:** the landing.
