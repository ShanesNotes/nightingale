# 95: The record trims its history

Spec: [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md), the [solver contract](../../../docs/architecture/solver-contract.md)'s section 5 and its Checkpoint paragraph, and `crates/ng-record/src/history.rs`

Type: task

Status: open

Blocked by: 94

Blocks: [body ticket 28](../../body/issues/28-two-synthetic-days.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and that run cannot keep every ledger chunk: record 0008 puts one 48 h course of the slice at 296 to 342 GB, and at 74 to 86 GB compressed. [Native-contract 94](94-the-certificate-folds-as-the-run-goes.md) builds the fold, which judges each window as the run goes. The fold alone frees no space: on main a store that keeps a link keeps it complete, compacted or not, and the day runner's `KeepsNothing` keeps none. Record 0008 says what a run keeps in their place, main holds none of it, and no other ticket owns it. It lands before the whole body's 48 h run.

Record 0008 rules the order: before any history is trimmed, the fold matches `certify` and the independent oracle on every window shape. The first box holds it.

- [ ] Before anything is trimmed, the fold matches `certify` and the independent oracle on every window shape main then has. For the fixed windows native-contract 94's tests show it. For the windows between edges, where [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md) has merged, a test this ticket names or adds shows it
- [ ] During a run the history store keeps a rolling day of complete links, reaching back to the checkpoint before the oldest window still open, so a failing window's span is whole when the fold refuses it
- [ ] After a run the store keeps a witness for each step, 96 bytes: the step's seal, its image digest and its ledger digest. Each seal recomputes from the seal before it, the revision and those two digests, with the hash the run already uses. It also keeps a checkpoint each simulated minute with its head link
- [ ] The flight recorder: on the first closure or drift failure, a replay divergence, an engine fault or a store failure, the run keeps the original links from the checkpoint before the failing window to the last head
- [ ] What a finished run let go is checked as well as what it kept. After a run that passed, a listing of the store shows the witnesses, the checkpoints and their head links, and no other complete link, and the run still has its certificate over every window and its event log. After a run that failed it shows the flight recorder's span as well, the one span that is still checked from the store alone
- [ ] A trimmed `restore` checks every seal back to admission, the checkpoint's head link and its image, reparses the ledger of no discarded step, and takes the run's opening time from the record's opening, which a witness does not hold
- [ ] A revision between checkpoints replays forward from the checkpoint before it, and every replay gate keeps its live comparison on fresh cores. A regeneration on the build that made the run, which the run's evidence names, is checked step by step against the witnesses and gives back the parse of each discarded step's ledger. One that no longer reproduces its seals is reported as a finding
- [ ] `certify` runs no core. A trimmed run's certificate is recomputed only from a rerun of the pinned build or a regenerated history, and a corrected healthy-range set or policy is rechecked the same way. Record 0008's one exception has a test: under the same basis, windows and closure limits, a complete run that passed closure shows from the worst drift it kept for each combination that it passes a loosened drift bound, and a failure under the new bound needs the rerun to name its first window
- [ ] Links and checkpoints that an earlier build wrote still load
- [ ] What a course keeps during the run and after it, in bytes that a run shows: at least a simulated hour of the body main then has, scaled to 48 h beside record 0008's arithmetic for the slice, which is 3.05 to 3.52 GB of witnesses, 111 MB of checkpoints, and 37 to 43 GB of compressed links for the rolling day plus 26 to 30 MB back to its checkpoint. The 48 h course's own bytes are [body ticket 28](../../body/issues/28-two-synthetic-days.md)'s to report
- [ ] A mutant row for each rule

The stored forms are new state in the record, so the ticket starts with their design: what a witness and a flight recorder's span are in the store, which seal or digest names each of their fields, how a store tells a trimmed run from a complete one, and how `certify` reports a history it cannot recompute. On main a certificate's result for each combination already holds its worst drift (`worst_drift` in `crates/ng-validate/src/certificate.rs`).

**Non-goals:** the fold and its evaluator (native-contract 94), windows between edges (native-contract 85), a store of past state or readings, which record 0008 leaves to a replay, and any change to what a certificate holds.

**The body's bytes, measured before this ticket starts.** On main's whole body at `e2d5c6ba`, each accepted step's link is 131,920 bytes raw and 65,972 after `ng_record::compact` (771,863,920 and 386,003,771 bytes over a 30 s run's 5,851 links), and a compressing filesystem saved about 5% more. The body accepts 349,076 steps in its first 1,800 s, awake, and 348,830 in the next 1,800 s, asleep. So an hour is about 92 GB raw and 46 GB compacted, and a 48 h course about 4.4 TB and 2.2 TB. Record 0008's arithmetic, made on the slice, is 296 to 342 GB and 74 to 86 GB compressed, so the body's course is 13 to 15 times the record's raw figure and 26 to 30 times its compressed one. A rolling day of complete links is then about 1.1 TB, against the record's 37 to 43 GB, and more than the build worker that measured it has free. So this ticket's design opens with what the rolling day keeps for the body: complete links as the record has them, with where they are kept, or less, with what a refused window's span is rebuilt from and a record that amends 0008. Certifying from a disk store took 1.47 ms a link.
