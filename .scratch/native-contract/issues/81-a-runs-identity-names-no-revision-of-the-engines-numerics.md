# 81: A run's identity names no revision of the engine's numerics

Spec: `docs/srs.md` (the Event log entry) and `docs/architecture/solver-contract.md` (Identity, time and replay)

Type: research

Status: resolved

Blocked by: none

**Why.** The first milestone (SRS invariant I19) rests on replay: a run is rebuilt from its identity and its events. The identity holds the model hash, the packet hash, the seed and the declared resolution. Nothing in it names the landing's rule or the contract's revision. So a change to the engine's own numerics can change a run that keeps its identity. The check of nightingale#196 found one (`~/lanes/logs/ng-wave-4/nc64/verify-coordinator/VERIFY-nc64.md`, item 9).

**What the check found.**

- [Native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md) changed which guard a landing reads. On main and the slice no run changed.
- On the body at `2e1dcb29`, a log made before the change in the stalled state departs when rebuilt after it, at its first stalled landing (1.2809 s). Replay then reports a divergence there instead of refusing up front.

- [x] What replay promises across revisions of the engine
- [x] The options, a revision in the identity or the promise stated within one revision, each with what it costs
- [x] A design round if the identity's shape changes

**Non-goals:** the landing.

## Answer

### 1. What replay promises across revisions of the engine

**Bitwise replay and exact checkpoint continuation are promised only within one engine numerics revision (one build), never across revisions that change landing, sparse-LU elimination, quadrature, or evaluation arithmetic.** Cross-revision regeneration is a comparison against step witnesses, not an equality guarantee: [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md) (lines 3, 15, and 20) scopes regeneration to *"on the build that made it"* and specifies that a run's evidence names the build that made it, so a regeneration that no longer reproduces its seals flags a cross-revision departure that can still be compared step by step against the witnesses. What crosses engine revisions is wire compatibility: links and checkpoints written by an earlier build still load (record 0008 line 24; `RecordFormat::V1` `ngcheck1` SHA-256 and `RecordFormat::V2` `ngcheck2` BLAKE2b-256 in `crates/ng-record/src/checkpoint.rs:161-179` and `docs/architecture/solver-contract.md:334`).

**What `RunIdentity` seals on main (`9a7469ff`).** `RunIdentity` (`crates/ng-contract/src/scheme.rs:364-397`) seals four fields: `ModelHash`, closed `PacketHash`, `Seed`, and `DeclaredResolution` (`policy`, `minimum_interval_ns`, `retry_budget`, `ArithmeticProfile`, `StageScheme`, `ErrorScales`). Within `DeclaredResolution`:

- `ArithmeticProfile::contract_v1()` (`scheme.rs:228-241, 487-507`) hashes only `ROUTINE_BITS_SHA256`, the sampled bit results of `pinned::exp` and `pinned::ln`.
- `DeclaredResolution::policy` tags each solver core's own revision (`b"ng-solver core 1"` in `crates/ng-solver/src/lib.rs:114`; `b"reference integrator 1"` in `crates/ng-validate/src/reference.rs:53`).
- `docs/architecture/solver-contract.md:330` promises *"Bitwise identity within one declared `ArithmeticProfile` at any pace and chunking and for every input `enqueue` accepts; exact continuation from a checkpoint. Nothing is promised across profiles or other targets."* `docs/srs.md` (lines 36, 114, 115, 117, 119, and 129: I2, I3, I5, I7, I17) states bitwise replay and checkpoint continuation without addressing cross-build changes.

**What is unsealed today.** Nothing in `RunIdentity` names the contract's own numerical rules: guard landing (`crates/ng-contract/src/boundary.rs:1025-1069`), sparse-LU pivot and elimination order (`crates/ng-contract/src/elimination.rs`), or floating-point reduction order in evaluation and quadrature. Three merged contract changes show the consequence on unchanged `ModelHash` and unchanged `DeclaredResolution`:

- [Native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md) (`nightingale#196`) changed which crossed guard `land` reads; on `2e1dcb29` in the stalled state, an event log made before the change diverged at 1.2809 s under the same `RunIdentity`.
- [Native-contract 89](89-the-eliminations-order-inside-its-largest-block.md) (`nightingale#236`) and [native-contract 79](79-what-a-vascular-edges-transport-columns-cost.md) (`nightingale#239`) changed sparse-LU elimination order inside diagonal blocks; admitted and run seals moved at rounding scale (about 1 ULP; 0 of 240 printed six-decimal trace values changed) with `ModelHash`, `DeclaredResolution`, and `ArithmeticProfile::contract_v1()` unchanged. Even when fresh admission moves `PacketHash` (via `InitialRevision`'s solved start and witness), `Session::resume` (`crates/ng-record/src/session.rs:132-144`) and `checkpoint::restore` (`crates/ng-record/src/checkpoint.rs:280-290`) check only `expected.model() == body.model_hash()`, `expected.resolution().digest() == solver.resolution().digest()`, and `header.identity == expected.digest()`, so a caller resuming a stored checkpoint with its stored `RunIdentity` passes every check and continues onto a mixed-revision rounding trajectory (noted in local Codex review of `nightingale#239`).

### 2. Options and costs

| Option | Mechanism | What it gives | What it costs |
|---|---|---|---|
| **A. Promise within one engine revision (no identity change)** | Clarify `docs/architecture/solver-contract.md` ("What replay promises") to match record 0008: bitwise replay and checkpoint continuation hold within one engine revision, and run evidence names the build outside `RunIdentity`. | Zero API, seal, checkpoint-header, or pin churn; preserves `RunIdentity`'s four-field shape (`solver-contract.md:340`); keeps `RecordFormat` compatibility orthogonal. | No up-front refusal when resuming a stored checkpoint across a numerics revision (`Session::resume` silently continues onto a different landing or elimination order whenever `ModelHash` and `DeclaredResolution` stay equal); replaying an old event log reports mid-run divergence rather than refusing at start. |
| **B1. Contract numerics revision inside `ArithmeticProfile` or `DeclaredResolution::digest()`** | Seal a bumped contract numerics revision tag alongside `ROUTINE_BITS_SHA256` inside `ArithmeticProfile` (or inside `DeclaredResolution::digest()`), keeping `RunIdentity` at four fields and `DeclaredResolution::policy` for each core. Bump only on intentional trajectory-moving contract numerics changes; keep `RecordFormat` outside it. | Up-front `CheckpointError::IdentityMismatch` in `Session::resume` (`session.rs:140-142`) and replay refusal before stepping; preserves four-field `RunIdentity` and public constructor signatures; non-numerical commits and bit-identical optimizations keep seals and checkpoints valid. | Conservative false refusals (refuses resuming a checkpoint whose model or state would not have moved, such as the slice across `nightingale#196`); moves every `RunIdentity` pin on each bump (extra re-pin churn for changes like `nightingale#196` that otherwise moved zero pins); requires review discipline when a change moves no pinned trace on main (a gate catches unintended trace-pin drift, not a silent-on-main landing change). |
| **B2. Fifth field on `RunIdentity` or new `DeclaredResolution::new` parameter** | Add an explicit contract-revision field to `RunIdentity` or its resolution constructor. | Same refusal semantics as B1. | Strictly worse than B1: breaks the four-field invariant (`scheme.rs:362`, `solver-contract.md:340`) and public constructor signatures across `ng-contract`, `ng-solver`, `ng-validate`, and `docs/architecture/solver-contract-sketch/` with no benefit over B1. |
| **C. Git commit or build hash in `RunIdentity`** | Seal the source commit or compiler build hash into `RunIdentity`. | Automatic refusal across any build change. | Unusable: breaks every `RunIdentity` pin and every stored checkpoint on docs edits (`nightingale#234`, `nightingale#237`), test additions (`nightingale#241`), comment passes, and bit-identical optimizations (`nightingale#235`, `nightingale#240`), violating record 0008 line 24. |

### 3. Recommendation and design round

**Now (before external checkpoint persistence and trimmed-history regeneration):** adopt **Option A**. No checked-in or external persisted checkpoints exist yet ([native-contract 42](42-veins-collapse-branch.md), line 107), and Option A aligns `docs/architecture/solver-contract.md` with record 0008 without moving pins or blocking the route to I19. Because `RunIdentity`'s shape does not change under Option A, **no design round is required now**.

**When to open a design round (Option B1):** before external hosts persist checkpoints or event logs across engine releases, or alongside [native-contract 95](95-the-record-trims-its-history.md) when regeneration from checkpoints becomes operational, open a small design round to settle four questions for Option B1 without altering `RunIdentity`'s four-field surface:

1. Whether the contract numerics revision sits inside `ArithmeticProfile` (widening it from `ROUTINE_BITS_SHA256` to contract arithmetic, landing, and elimination semantics) or as a dedicated version tag sealed inside `DeclaredResolution::digest()`.
2. Keeping `RecordFormat` (`ngcheck1`/`ngcheck2`) orthogonal to the numerics revision and specifying how `rewind` and `recover` behave on mismatched revisions.
3. What gate rule pairs a trace-digest re-pin (`crates/ng-validate/tests/trace.rs`, `body_run.rs`) with a numerics revision bump and what review rule covers silent-on-main landing changes.
4. Recording the engine build revision in `RunEvidence` outside `RunIdentity`, implementing record 0008 line 20.
