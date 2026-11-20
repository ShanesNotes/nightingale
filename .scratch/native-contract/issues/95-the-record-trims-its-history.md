# 95: The record trims its history

Spec: [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md), the [solver contract](../../../docs/architecture/solver-contract.md)'s section 5 and its Checkpoint paragraph, and `crates/ng-record/src/history.rs`

Type: task

Status: open

Blocked by: 94

Blocks: [body ticket 28](../../body/issues/28-two-synthetic-days.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and that run cannot keep every ledger chunk: record 0008 puts one 48 h course of the slice at 296 to 342 GB, and at 74 to 86 GB compressed. [Native-contract 94](94-the-certificate-folds-as-the-run-goes.md) builds the fold, which judges each window as the run goes. The fold alone frees no space, because a run on main still keeps every link. Record 0008 says what a run keeps in their place, main holds none of it, and no other ticket owns it. It lands before the whole body's 48 h run.

Record 0008 rules the order: before any history is trimmed, the fold matches `certify` and the independent oracle on every window shape. That is native-contract 94, both of its PRs.

- [ ] During a run the history store keeps a rolling day of complete links, reaching back to the checkpoint before the oldest window still open, so a failing window's span is whole when the fold refuses it
- [ ] After a run the store keeps a witness for each step, 96 bytes: the step's seal, its image digest and its ledger digest. Each seal recomputes from the seal before it, the revision and those two digests. It also keeps a checkpoint each simulated minute with its head link
- [ ] The flight recorder: on the first closure or drift failure, a replay divergence, an engine fault or a store failure, the run keeps the original links from the checkpoint before the failing window to the last head
- [ ] A trimmed `restore` checks every seal back to admission, the checkpoint's head link and its image, reparses the ledger of no discarded step, and takes the run's opening time from the record's opening, which a witness does not hold
- [ ] A revision between checkpoints replays forward from the checkpoint before it. A regeneration on the build that made the run is checked step by step against the witnesses, and a run's evidence names that build
- [ ] Links and checkpoints that an earlier build wrote still load
- [ ] What one 48 h course of the whole body keeps during the run and after it, in bytes that a run shows, beside record 0008's arithmetic for the slice: 3.05 to 3.52 GB of witnesses, 111 MB of checkpoints and 37 to 43 GB for the rolling day
- [ ] A mutant row for each rule

The stored forms are new state in the record, so the ticket starts with their design: what a witness and a flight recorder's span are in the store, which seal or digest names each of their fields, how a store tells a trimmed run from a complete one, and what `certify` answers on a trimmed history.

**Non-goals:** the fold and its evaluator (native-contract 94), windows between edges (native-contract 85), a store of past state or readings, which record 0008 leaves to a replay, and any change to what a certificate holds.
