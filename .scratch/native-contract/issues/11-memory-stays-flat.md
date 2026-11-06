# 11: A run's memory stays flat as it grows

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A run's resident memory stops growing with its accepted steps, and the history that a 48 h slice run writes has a measured size. Today `History::push` (`crates/ng-record/src/history.rs:279`) keeps one `Link` per accepted step in memory: a ledger chunk, an image digest and a seal. Nothing drops a link. The validation crate's `MemoryStore` (`crates/ng-validate/src/replay.rs:435`) keeps every link that a checkpoint hands it. The slice's links go to a history store as each step is accepted, and the record keeps in memory only what the next step needs. The certificate and a resume read the rest through the store.

Blocked by: None (can start immediately)

Status: resolved

- [x] `tools/bench` reads the same peak memory, within its stated margin, for two slice runs of different lengths, and reports the history's bytes per accepted step

## Answer

A run's resident memory no longer grows with its accepted steps. Each accepted step hands its link to the session's history store, and the record keeps only the chain's head.

- **`tools/bench` reads the same peak memory, within its stated margin, for two slice runs of different lengths, and reports the history's bytes per accepted step.** `python3 -I tools/bench/run.py memory` runs the slice on the reference core at 5 ms for 5 s (1154 accepted steps) and 50 s (11578), five fresh processes each, taking turns. Measured on the tower: run peaks of 4808 kB and 4816 kB (medians), 8 kB apart against a 512 kB margin, eight times the 64 kB run-to-run spread. That is a slope of 1 byte a step. The history is 18584 bytes a step on disk, so by arithmetic a 48 h slice run at the arena's 212 steps a simulated second writes about 680 GB, and a meal check to 11:30 about 65 GB. A resume from the long run's checkpoint peaks at 4748 kB and lands on the run's seal. Main at b8990a5 read 26968 kB at 1154 steps and 103912 kB at 5046 (whole-process peak), 19.8 KB a step. The bench resets the high-water mark after admission and returns free heap first, because freed heap that stays resident hid a planted 256-byte-a-step leak until it did.
- The session holds the store from admission (`Session::admit`, `admit_at` and every fixture take one), beside its solver, so the record stays `Sync`. A link the store refuses waits in the record, and `checkpoint` and `export` refuse with `StoreFailed` until a later step's store keeps it.
- Resume, rewind, recover and the certificate read older links newest first through one checked walk. A fork proves the checkpoint is on its parent's chain by walking that chain through the store. `certify` walks backward with two cursors a day apart, and today's forward walk stays in the tests as an oracle every certificate test agrees with.
- `RecordDigests.ledger` now digests the opening, the incidence and the head chunk, the ledger's account at the head; the seal still covers every chunk. The ledger pins in `WATER_DIGESTS`, `SOLVER_DIGESTS` and `CHANCE_DIGESTS` moved, and a run resumed from main's own checkpoint and links reports the new `WATER_DIGESTS`.
- Inside the workspace every store holds its links in memory (`MemoryStore`), because no workspace crate names `std`. Flat memory needs a host whose store is outside the process, as the bench's `LinkLog` is.

## Comments

From the split of ticket 11: this ticket comes first because memory stops a long run before speed does.

From the engine bench (nightingale#31): peak memory read 26.3 MB at 1154 accepted steps and 101.2 MB at 5046. Each reading is one release run of 5 simulated seconds on the slice, at 5 ms and at 1 ms steps. That is about 19.7 KB per accepted step, or about 375 GiB per simulated day at 5 ms steps by arithmetic. Two readings of peak memory are a weak measurement, so measure again first. Reading the code puts one link at about 17.5 KB on the slice, which fits ticket 04's count of six compensated values per combination and four per column.

From 04 and 05: ticket 04's 19 GB for 48 h at a 2 ms step was for its two-row body. One link per accepted step adds 64 bytes to each chunk, about 5.5 GB more. Fixed-time chunks do not answer it, because grouping moves no bytes and merged chords do not bound the curve, so measure on the slice before choosing a store. A resume reads every link, so the resume gate's cost grows with the number of loads.

From the split of ticket 11: the certificate reads one sealed history (contract §5), and a resume walks the chain link by link, so both keep working through the store.

From the solver arena (in this PR): at the arena's scales (0.1, 1e-3) and a 10 ms cap, 06:55 to 07:30 took about 446,000 accepted steps, about 212 a simulated second, and a history link took 15.6 to 18.5 KB, so about 7 GB for that window. A meal check needs runs to 11:30 with and without the meal, about 4.6 simulated hours each and 55 GB or more each at that rate, so rows 9 and 10 wait on this ticket.
