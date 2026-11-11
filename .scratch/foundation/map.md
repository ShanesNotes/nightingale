# Foundation wayfinder map

## Destination

[SRS invariant I19](../../docs/srs.md#4-invariants-each-becomes-a-test) passes on a run of the reference man's reviewed packet, checked against a reviewed healthy-range set. That pass is the first milestone ([record 0004](../../docs/adr/0004-first-milestone.md)).

## Notes

- Prose uses "the map" for the patient's anatomy (SRS section 2) and calls this file the wayfinder map.
- The architecture's layers ([SRS section 1](../../docs/srs.md#1-purpose-and-scope)) come first, and display comes after them. Display work has started outside this effort, as the `clinical-workspace/` prototype, which replays one recorded run and never runs the engine. Its tickets are in the [clinical workspace map](../clinical-workspace/map.md).
- This wayfinder map carries execution as well as decisions. Its tickets include the build work that reaches the destination, not only the choices before it.
- Foundation tickets name native-contract tickets in their `Blocked by:` and `Lands with:` lines, so a frontier scan reads `.scratch/native-contract/issues/` as well as `.scratch/foundation/issues/`.
- Grilling tickets call the grilling and domain-modeling skills.
- Proof time limits how many tickets land in a day, because the tower proves one head at a time. [Ticket 11](issues/11-proof-fails-fast.md) makes a red proof stop at its first failure, and [ticket 12](issues/12-mutant-baseline-once.md) drops the mutant baselines `10-rust` already ran. The diagnosis is `~/lanes/logs/ng-optimize/proof-time.md`.

## Where the route stands

Update this block whenever a ticket on the route resolves. The vault's project master says what is in flight.

The run machinery is built: native-contract 01 to 27 are resolved, and it runs a synthetic slice of four systems (heart and vessels, fluids, kidney and the baroreflex). What still stands between that and the destination, in order:

1. **Before the arena pick** ([foundation 01](issues/01-arena-and-milestone.md), step 2). [Native-contract 28](../native-contract/issues/28-admission-energy-at-balance.md)'s code, admission's energy offset, is merged as nightingale#91, and its last box is open: a probe on the merged head. The developer reviewer approved a synthetic 24 h energy drift limit of 4e5 for the slice if that probe agrees with a drift of about 1.9e5 a day. [Native-contract 31](../native-contract/issues/31-cross-interval-drift-check.md), a cross-interval drift check, is open. [Native-contract 33](../native-contract/issues/33-leg-veins-empty-head-down.md) asks why the slice's leg veins empty within 30 s of head-down, where every core faults. The arena's day never goes head-down, so the pick does not wait on it. Each candidate core needs a row in the contract's crate table and in `tools/gate/paths.toml`.
2. **The pick and the 48 h run** (steps 2 and 3). The pick protocol scores each core, the developer reviewer confirms the pick (SRS rule 8), and the winner runs the milestone protocol once. [Ticket 26](../native-contract/issues/26-arena-picks-on-a-day.md)'s Answer has the cost, and [record 0008](../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md) says what history a run keeps. The first pick, `crates/ng-solver`, was made before the harness existed and is provisional until it is rescored ([ticket 24](../native-contract/issues/24-arena-ready-to-dispatch.md)'s comments).
3. **The body** (step 4): all 13 systems at rung 0. Nine have no crate (respiratory, hepatic, gastrointestinal, endocrine, blood chemistry, hematology, immune, metabolism and thermal, musculoskeletal and skin). The [body map](../body/map.md) orders the work loop by loop, in 28 tickets.
4. **The contract's follow-ups** ([foundation 07](issues/07-contract.md)): the parameter-shift payload, inferred provenance, the day and the healthy-range set.
5. **The clinical numbers** ([research map](../research/map.md)). Research 03 to 10 are open, and no crate reads `data/` until the [reviewed packet builder](../research/issues/10-reviewed-packet-builder.md) exists.
6. **Speed.** The slice runs at 6.8 times real time on the heavy-run machine ([ticket 25](../native-contract/issues/25-history-on-disk.md)), against the aim of 100 with full waveforms ([SRS section 6](../../docs/srs.md#6-fidelity-aims)). [Body ticket 07](../body/issues/07-what-limits-the-step.md) measures what limits the step, and [ticket 27](../body/issues/27-step-plan.md) picks a plan.

## Decisions so far

- [Fast-forward identity (D3)](../../docs/srs.md#3-hard-rules): SRS rule 4 gives identical bits at every pace, fast-forward included. Resolution is part of a run's identity, so a run at a reduced resolution is a different run.
- [Map detail at milestone 1 (D5)](../../docs/adr/0002-map-starts-grouped.md): the map starts grouped, and each split is earned. At stage 1 the map holds only the regions that the anchors and the stage 1 checks name.
- [Chart the foundation map from SRS section 7](issues/02-roadmap.md): this wayfinder map sorts the section, and the ticket keeps the section word for word.
- [First crate](issues/03-first-crate.md): `tools/verify` plans a merge, changed or full run over four `ng-*` crates; root and ticket Markdown count as docs-only; the pin rule waits for a pin without a lockfile. The nightly job is [Add the nightly job](issues/10-nightly-job.md).
- [The proof fails fast](issues/11-proof-fails-fast.md): `tools/verify` runs the cheap steps, then the mutant rows a change adds or edits, then `10-rust` with the edited test targets first, then the other rows and the workspace. A sweep stops at its first row that is not CAUGHT, so a red run reports only its first failure, and a green proof checks what it checked before. [Ticket 12](issues/12-mutant-baseline-once.md), which skips a baseline `10-rust` already ran, can start.
- [Admit the ng-* crates under purity rules](../native-contract/issues/01-ng-purity-rules.md): the contract's crate table marks the physiology closure, which takes no crate from outside the workspace, and no crate may name a host service.
- [Anatomy as data](../native-contract/issues/02-anatomy-as-data.md): `ng-contract` checks a map at `finish` with named reasons, hashes it in key order and derives one water pool per space. A bed must lie on the circulation, no pump is declared, and site ids are names resolved by key.
- [Declaration language and compiler](../native-contract/issues/03-declaration-language-and-compiler.md): laws attach by reference, so compile refuses an omitted law, a second owner and an uncovered mode by name; route families are requested whole; every registry pool is an amount unless the schema leaves its space out with a reason, and one no transfer moves is refused.
- [Record transaction and reference core](../native-contract/issues/04-record-transaction-and-reference-core.md): a core can only propose. `StepProposal::verify` replays its stages on the record's own values, `validate` compares the ticket the session issued, a refused session can only retry, a fault latches, and a ticket freezes its drives, so a refusal changes nothing and a delivery applies once. The certificate walks every sliding 24 h window without sampling, and a forward Euler core past RC closes every ledger and fails the accuracy gate.
- [Native-contract 05 to 27](../native-contract/issues/): the replay gates, seeded chance, the lens and observer, the admission solve, the slice's anatomy and laws, flat memory, throughput and numerical health, the certificate and the accuracy and replay gates on the slice, split measurement, the day's checks, the score, the must-fail table, the arena's entry point, compact history, the pick and milestone protocols, and the fault paths. Each ticket's Answer or Comments holds its decisions. Records 0006 to 0008 in `docs/adr/` hold the three that are hard to reverse.

## Not yet specified

- **A name for "the agent".** The build order in [Chart the foundation map from SRS section 7](issues/02-roadmap.md#srs-section-7-word-for-word) lists "the agent" with the solver and the contracts as architecture. The SRS defines no such term, and what it names is not yet clear. Once a session finds out, "the agent" gets a glossary term or leaves the build order.
- **Where the monitor goes.** [SRS section 6](../../docs/srs.md#6-fidelity-aims) keeps the monitor a requirement, and [record 0004](../../docs/adr/0004-first-milestone.md) keeps the monitor's readings in the lens that the milestone checks. So its readings lie on this route, while its display is out of scope. Which readings it carries, and where it is built, are open. [Parking lot](issues/08-parking-lot.md) parks only the monitor's display until after the milestone.
- **The reference man's reviewed packet and a reviewed healthy-range set.** The destination needs both, along with the healthy-range check and the return check that read them. The [research wayfinder map](../research/map.md) scopes sourcing both, and no ticket yet scopes the build of those two checks. [Contract follow-ups](issues/07-contract.md) adds the day and the healthy-range set to the contract, and the lens decides which readings the set covers.

## Out of scope

This effort leaves out the first milestone's non-goals, which [Chart the foundation map from SRS section 7](issues/02-roadmap.md#srs-section-7-word-for-word) keeps word for word.

- Disease, hazard, intervention and debrief behavior. They belong to scope stages 2 to 5, and I19's healthy adult has nothing done to them. What they will attach to stays in scope, such as the event log, checkpoints and the packet.
- Bedside scenarios as the definition of done. SRS invariants define done, and scenarios belong to later scope stages.
- Display work, the chart included. Display comes after the architecture. The `clinical-workspace/` prototype is its own line of work.
- Map detail finer than the first milestone needs. D5 rules it out.
- A build target other than native Rust (SRS rule 5).
- Scripted outcomes and arbitrary odds. The body computes every number from its physics, and chance fires only at cited odds (SRS section 1).
